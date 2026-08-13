use std::{f32::consts::TAU, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, ElementId, Hsla, IntoElement, Pixels, Point, SharedString, Window,
    point, px,
};
use gpui_component_macros::IntoPlot;

use crate::{
    ActiveTheme as _,
    plot::{
        Plot,
        label::{PlotLabel, Text, plot_text_size},
        shape::{Arc, ArcData, Pie},
        tooltip::{Tooltip, TooltipState},
    },
};

struct RadialSeries<T> {
    value: Rc<dyn Fn(&T) -> f32>,
    name: SharedString,
    color: Option<Hsla>,
}

/// A radial bar chart supporting concentric and stacked series.
#[derive(IntoPlot)]
pub struct RadialChart<T: 'static> {
    data: Vec<T>,
    label: Option<Rc<dyn Fn(&T) -> SharedString>>,
    series: Vec<RadialSeries<T>>,
    start_angle: f32,
    end_angle: f32,
    inner_radius: f32,
    outer_radius: f32,
    ring_gap: f32,
    pad_angle: f32,
    background: bool,
    stacked: bool,
    center_title: Option<SharedString>,
    center_description: Option<SharedString>,
    id: Option<ElementId>,
}

impl<T> RadialChart<T> {
    /// Creates an empty radial chart over the supplied data categories.
    pub fn new(data: impl IntoIterator<Item = T>) -> Self {
        Self {
            data: data.into_iter().collect(),
            label: None,
            series: Vec::new(),
            start_angle: 0.,
            end_angle: TAU,
            inner_radius: 0.,
            outer_radius: 0.,
            ring_gap: 4.,
            pad_angle: 0.02,
            background: false,
            stacked: false,
            center_title: None,
            center_description: None,
            id: None,
        }
    }

    /// Enables hover tooltips using a stable sibling-unique identifier.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the category label used by tooltips.
    pub fn label(mut self, label: impl Fn(&T) -> SharedString + 'static) -> Self {
        self.label = Some(Rc::new(label));
        self
    }

    /// Appends a named radial series.
    pub fn series(
        mut self,
        name: impl Into<SharedString>,
        value: impl Fn(&T) -> f32 + 'static,
    ) -> Self {
        self.series.push(RadialSeries {
            value: Rc::new(value),
            name: name.into(),
            color: None,
        });
        self
    }

    /// Sets the color of the most recently appended series.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        if let Some(series) = self.series.last_mut() {
            series.color = Some(color.into());
        }
        self
    }

    /// Sets the angular range in radians, measured clockwise from 12 o'clock.
    pub fn angles(mut self, start: f32, end: f32) -> Self {
        self.start_angle = start;
        self.end_angle = end;
        self
    }

    /// Sets fixed inner and outer radii. Zero values resolve from the chart bounds.
    pub fn radii(mut self, inner: f32, outer: f32) -> Self {
        self.inner_radius = inner;
        self.outer_radius = outer;
        self
    }

    /// Sets the gap between concentric series rings.
    pub fn ring_gap(mut self, gap: f32) -> Self {
        self.ring_gap = gap.max(0.);
        self
    }

    /// Sets the angular gap between adjacent radial segments.
    pub fn pad_angle(mut self, angle: f32) -> Self {
        self.pad_angle = angle.max(0.);
        self
    }

    /// Draws a muted full-ring track behind every series.
    pub fn background(mut self, background: bool) -> Self {
        self.background = background;
        self
    }

    /// Places all series in one ring instead of separate concentric rings.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// Sets the two-line label painted in the chart center.
    pub fn center_label(
        mut self,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        self.center_title = Some(title.into());
        self.center_description = Some(description.into());
        self
    }

    fn radii_for(&self, bounds: Bounds<Pixels>, series_index: usize) -> (f32, f32) {
        let outer = if self.outer_radius > 0. {
            self.outer_radius
        } else {
            bounds.size.width.min(bounds.size.height).as_f32() * 0.42
        };
        let inner = if self.inner_radius > 0. {
            self.inner_radius
        } else {
            outer * 0.35
        };
        if self.stacked || self.series.len() <= 1 {
            return (inner, outer);
        }

        let available = (outer - inner).max(0.);
        let total_gap = self.ring_gap * self.series.len().saturating_sub(1) as f32;
        let width = ((available - total_gap) / self.series.len() as f32).max(1.);
        let ring_inner = inner + series_index as f32 * (width + self.ring_gap);
        (ring_inner, ring_inner + width)
    }

    fn series_color(&self, index: usize, cx: &App) -> Hsla {
        self.series[index].color.unwrap_or(match index % 5 {
            0 => cx.theme().chart_1,
            1 => cx.theme().chart_2,
            2 => cx.theme().chart_3,
            3 => cx.theme().chart_4,
            _ => cx.theme().chart_5,
        })
    }

    fn hovered_index(&self, position: Point<Pixels>, bounds: Bounds<Pixels>) -> Option<usize> {
        if self.data.is_empty() || self.series.is_empty() {
            return None;
        }
        if self.stacked {
            let total = self
                .data
                .iter()
                .flat_map(|datum| self.series.iter().map(move |series| (series.value)(datum)))
                .map(|value| value.max(0.))
                .sum::<f32>();
            if total <= f32::EPSILON {
                return None;
            }
            let (inner, outer) = self.radii_for(bounds, 0);
            let shape = Arc::new().inner_radius(inner).outer_radius(outer);
            let sweep = self.end_angle - self.start_angle;
            let mut angle = self.start_angle;
            for (datum_index, datum) in self.data.iter().enumerate() {
                for series in &self.series {
                    let value = (series.value)(datum).max(0.);
                    if value <= f32::EPSILON {
                        continue;
                    }
                    let end_angle = angle + sweep * value / total;
                    let segment = ArcData {
                        data: datum,
                        index: datum_index,
                        value,
                        start_angle: angle,
                        end_angle,
                        pad_angle: self.pad_angle,
                    };
                    if shape.contains(&segment, position, &bounds, None, None) {
                        return Some(datum_index);
                    }
                    angle = end_angle;
                }
            }
            return None;
        }

        for (series_index, series) in self.series.iter().enumerate() {
            let (inner, outer) = self.radii_for(bounds, series_index);
            let shape = Arc::new().inner_radius(inner).outer_radius(outer);
            let value = series.value.clone();
            let arcs = Pie::new()
                .value(move |datum| Some(value(datum)))
                .start_angle(self.start_angle)
                .end_angle(self.end_angle)
                .pad_angle(self.pad_angle)
                .arcs(&self.data);
            if let Some(segment) = arcs
                .iter()
                .find(|segment| shape.contains(segment, position, &bounds, None, None))
            {
                return Some(segment.index);
            }
        }
        None
    }
}

impl<T> Plot for RadialChart<T> {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        if self.data.is_empty() || self.series.is_empty() {
            return;
        }

        if self.stacked {
            let (inner, outer) = self.radii_for(bounds, 0);
            let arc = Arc::new().inner_radius(inner).outer_radius(outer);
            if self.background {
                let unit = ();
                arc.paint(
                    &ArcData {
                        data: &unit,
                        index: 0,
                        value: 1.,
                        start_angle: self.start_angle,
                        end_angle: self.end_angle,
                        pad_angle: 0.,
                    },
                    cx.theme().muted,
                    None,
                    None,
                    &bounds,
                    window,
                );
            }

            let total = self
                .data
                .iter()
                .flat_map(|datum| self.series.iter().map(move |series| (series.value)(datum)))
                .map(|value| value.max(0.))
                .sum::<f32>();
            if total > f32::EPSILON {
                let sweep = self.end_angle - self.start_angle;
                let mut angle = self.start_angle;
                for (datum_index, datum) in self.data.iter().enumerate() {
                    for (series_index, series) in self.series.iter().enumerate() {
                        let value = (series.value)(datum).max(0.);
                        if value <= f32::EPSILON {
                            continue;
                        }
                        let end_angle = angle + sweep * value / total;
                        arc.paint(
                            &ArcData {
                                data: datum,
                                index: datum_index,
                                value,
                                start_angle: angle,
                                end_angle,
                                pad_angle: self.pad_angle,
                            },
                            self.series_color(series_index, cx),
                            None,
                            None,
                            &bounds,
                            window,
                        );
                        angle = end_angle;
                    }
                }
            }
        } else {
            for (series_index, series) in self.series.iter().enumerate() {
                let (inner, outer) = self.radii_for(bounds, series_index);
                let arc = Arc::new().inner_radius(inner).outer_radius(outer);

                if self.background {
                    let unit = ();
                    arc.paint(
                        &ArcData {
                            data: &unit,
                            index: 0,
                            value: 1.,
                            start_angle: self.start_angle,
                            end_angle: self.end_angle,
                            pad_angle: 0.,
                        },
                        cx.theme().muted,
                        None,
                        None,
                        &bounds,
                        window,
                    );
                }

                let value = series.value.clone();
                let arcs = Pie::new()
                    .value(move |datum| Some(value(datum)))
                    .start_angle(self.start_angle)
                    .end_angle(self.end_angle)
                    .pad_angle(self.pad_angle)
                    .arcs(&self.data);
                let color = self.series_color(series_index, cx);
                for segment in &arcs {
                    arc.paint(segment, color, None, None, &bounds, window);
                }
            }
        }

        let center = point(
            bounds.size.width.as_f32() / 2.,
            bounds.size.height.as_f32() / 2.,
        );
        let mut labels = Vec::new();
        let text_size = plot_text_size(cx);
        if let Some(title) = self.center_title.clone() {
            labels.push(
                Text::new(
                    title,
                    point(center.x, center.y - 10.),
                    cx.theme().foreground,
                )
                .font_size(cx.theme().font_size)
                .align(gpui::TextAlign::Center),
            );
        }
        if let Some(description) = self.center_description.clone() {
            labels.push(
                Text::new(
                    description,
                    point(center.x, center.y + 10.),
                    cx.theme().muted_foreground,
                )
                .font_size(text_size)
                .align(gpui::TextAlign::Center),
            );
        }
        PlotLabel::new(labels).paint(&bounds, window, cx);
    }

    fn id(&self) -> Option<ElementId> {
        self.id.clone()
    }

    fn tooltip_state(
        &self,
        position: Point<Pixels>,
        bounds: Bounds<Pixels>,
        _cx: &App,
    ) -> Option<TooltipState> {
        Some(TooltipState::new(
            self.hovered_index(position, bounds)?,
            position,
            Vec::new(),
        ))
    }

    fn tooltip(
        &self,
        state: &TooltipState,
        cursor: Point<Pixels>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let datum = self.data.get(state.index)?;
        let title = self
            .label
            .as_ref()
            .map(|label| label(datum))
            .unwrap_or_else(|| format!("Item {}", state.index + 1).into());
        let mut tooltip = Tooltip::new(cursor, bounds.size).gap(px(8.)).title(title);
        for (index, series) in self.series.iter().enumerate() {
            tooltip = tooltip.row(
                self.series_color(index, cx),
                series.name.clone(),
                format!("{}", (series.value)(datum)),
            );
        }
        Some(tooltip.into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_chart_builder_supports_series_and_layout() {
        let chart = RadialChart::new([1., 2.])
            .series("Visitors", |value| *value)
            .color(gpui::black())
            .radii(20., 80.)
            .ring_gap(6.)
            .pad_angle(0.04)
            .background(true)
            .stacked(true)
            .center_label("3", "Visitors");

        assert_eq!(chart.series.len(), 1);
        assert_eq!(chart.inner_radius, 20.);
        assert_eq!(chart.outer_radius, 80.);
        assert!(chart.background);
        assert!(chart.stacked);
    }

    #[test]
    fn radial_hit_test_uses_weighted_segments_and_partial_angles() {
        let chart = RadialChart::new([10., 90.])
            .series("Visitors", |value| *value)
            .radii(40., 80.)
            .angles(0., std::f32::consts::PI);
        let bounds = Bounds::from_corners(point(px(0.), px(0.)), point(px(200.), px(200.)));

        assert_eq!(
            chart.hovered_index(point(px(109.), px(41.)), bounds),
            Some(0)
        );
        assert_eq!(
            chart.hovered_index(point(px(157.), px(119.)), bounds),
            Some(1)
        );
        assert_eq!(chart.hovered_index(point(px(40.), px(100.)), bounds), None);
    }
}
