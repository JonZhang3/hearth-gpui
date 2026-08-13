use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ElementId, Hsla, IntoElement, Pixels, Point, SharedString, TextAlign,
    Window, point,
};
use gpui_component_macros::IntoPlot;
use num_traits::Zero;

use crate::{
    ActiveTheme,
    plot::{
        Plot,
        label::{PlotLabel, TEXT_GAP, Text, plot_text_size},
        polygon,
        shape::{Arc, ArcData, Pie},
        tooltip::{Tooltip, TooltipState},
    },
};

/// The default extra gap (in pixels) between `outer_radius` and the label radius.
const DEFAULT_LABEL_GAP: f32 = 15.;

#[derive(IntoPlot)]
pub struct PieChart<T: 'static> {
    data: Vec<T>,
    inner_radius: f32,
    inner_radius_fn: Option<Rc<dyn Fn(&ArcData<T>) -> f32 + 'static>>,
    outer_radius: f32,
    outer_radius_fn: Option<Rc<dyn Fn(&ArcData<T>) -> f32 + 'static>>,
    pad_angle: f32,
    value: Option<Rc<dyn Fn(&T) -> f32>>,
    color: Option<Rc<dyn Fn(&T) -> Hsla>>,
    label: Option<Rc<dyn Fn(&T) -> SharedString + 'static>>,
    label_line_color: Option<Rc<dyn Fn(&T) -> Hsla + 'static>>,
    label_color: Option<Hsla>,
    label_gap: f32,
    active_index: Option<usize>,
    active_offset: f32,
    center_title: Option<SharedString>,
    center_description: Option<SharedString>,
    id: Option<ElementId>,
}

impl<T> PieChart<T> {
    pub fn new<I>(data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            data: data.into_iter().collect(),
            inner_radius: 0.,
            inner_radius_fn: None,
            outer_radius: 0.,
            outer_radius_fn: None,
            pad_angle: 0.,
            value: None,
            color: None,
            label: None,
            label_line_color: None,
            label_color: None,
            label_gap: DEFAULT_LABEL_GAP,
            active_index: None,
            active_offset: 4.,
            center_title: None,
            center_description: None,
            id: None,
        }
    }

    /// Set the inner radius of the pie chart.
    pub fn inner_radius(mut self, inner_radius: f32) -> Self {
        self.inner_radius = inner_radius;
        self
    }

    /// Set the inner radius of the pie chart based on the arc data.
    pub fn inner_radius_fn(
        mut self,
        inner_radius_fn: impl Fn(&ArcData<T>) -> f32 + 'static,
    ) -> Self {
        self.inner_radius_fn = Some(Rc::new(inner_radius_fn));
        self
    }

    fn get_inner_radius(&self, arc: &ArcData<T>) -> f32 {
        if let Some(inner_radius_fn) = self.inner_radius_fn.as_ref() {
            inner_radius_fn(arc)
        } else {
            self.inner_radius
        }
    }

    /// Set the outer radius of the pie chart.
    pub fn outer_radius(mut self, outer_radius: f32) -> Self {
        self.outer_radius = outer_radius;
        self
    }

    /// Set the outer radius of the pie chart based on the arc data.
    pub fn outer_radius_fn(
        mut self,
        outer_radius_fn: impl Fn(&ArcData<T>) -> f32 + 'static,
    ) -> Self {
        self.outer_radius_fn = Some(Rc::new(outer_radius_fn));
        self
    }

    fn get_outer_radius(&self, arc: &ArcData<T>) -> f32 {
        if let Some(outer_radius_fn) = self.outer_radius_fn.as_ref() {
            outer_radius_fn(arc)
        } else {
            self.outer_radius
        }
    }

    /// Set the pad angle of the pie chart.
    pub fn pad_angle(mut self, pad_angle: f32) -> Self {
        self.pad_angle = pad_angle;
        self
    }

    pub fn value(mut self, value: impl Fn(&T) -> f32 + 'static) -> Self {
        self.value = Some(Rc::new(value));
        self
    }

    /// Set the color of the pie chart.
    pub fn color<H>(mut self, color: impl Fn(&T) -> H + 'static) -> Self
    where
        H: Into<Hsla> + 'static,
    {
        self.color = Some(Rc::new(move |t| color(t).into()));
        self
    }

    /// Set the label text for each slice.
    ///
    /// Once set, a "leader line + text" is drawn outside the ring for every
    /// slice.
    pub fn label(mut self, label: impl Fn(&T) -> SharedString + 'static) -> Self {
        self.label = Some(Rc::new(label));
        self
    }

    /// Set the leader line color per slice (defaults to `cx.theme().border`).
    pub fn label_line_color(mut self, color: impl Fn(&T) -> Hsla + 'static) -> Self {
        self.label_line_color = Some(Rc::new(color));
        self
    }

    /// Set the label text color (defaults to `cx.theme().foreground`).
    pub fn label_color(mut self, color: Hsla) -> Self {
        self.label_color = Some(color);
        self
    }

    /// Set the extra gap between `outer_radius` and the label radius
    /// (defaults to 15px).
    pub fn label_gap(mut self, gap: f32) -> Self {
        self.label_gap = gap;
        self
    }

    /// Expands one controlled slice to expose an active state.
    pub fn active_index(mut self, index: Option<usize>) -> Self {
        self.active_index = index;
        self
    }

    /// Sets the radial expansion applied to the active slice.
    pub fn active_offset(mut self, offset: f32) -> Self {
        self.active_offset = offset.max(0.);
        self
    }

    /// Sets a two-line label painted inside a donut chart.
    pub fn center_label(
        mut self,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        self.center_title = Some(title.into());
        self.center_description = Some(description.into());
        self
    }

    /// Enables slice tooltips using a stable sibling-unique identifier.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    fn arcs(&self) -> Option<Vec<ArcData<'_, T>>> {
        let value = self.value.as_ref()?.clone();
        Some(
            Pie::new()
                .value(move |datum| Some(value(datum)))
                .pad_angle(self.pad_angle)
                .arcs(&self.data),
        )
    }

    fn hovered_index(&self, position: Point<Pixels>, bounds: Bounds<Pixels>) -> Option<usize> {
        let arcs = self.arcs()?;
        let default_outer_radius = bounds.size.height.as_f32() * 0.4;
        let shape = Arc::new();
        arcs.iter()
            .find(|arc| {
                let configured_outer_radius = self.get_outer_radius(arc);
                let mut outer_radius = if configured_outer_radius.is_zero() {
                    default_outer_radius
                } else {
                    configured_outer_radius
                };
                if self.active_index == Some(arc.index) {
                    outer_radius += self.active_offset;
                }
                shape.contains(
                    arc,
                    position,
                    &bounds,
                    Some(self.get_inner_radius(arc)),
                    Some(outer_radius),
                )
            })
            .map(|arc| arc.index)
    }
}

impl<T> Plot for PieChart<T> {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(value_fn) = self.value.as_ref() else {
            return;
        };

        let outer_radius = if self.outer_radius.is_zero() {
            bounds.size.height.as_f32() * 0.4
        } else {
            self.outer_radius
        };

        let arc = Arc::new()
            .inner_radius(self.inner_radius)
            .outer_radius(outer_radius);
        let value_fn = value_fn.clone();
        let arcs = Pie::<T>::new()
            .value(move |d| Some(value_fn(d)))
            .pad_angle(self.pad_angle)
            .arcs(&self.data);

        for a in &arcs {
            let inner_radius = self.get_inner_radius(a);
            let configured_outer_radius = self.get_outer_radius(a);
            let outer_radius = if configured_outer_radius.is_zero() {
                outer_radius
            } else {
                configured_outer_radius
            } + if self.active_index == Some(a.index) {
                self.active_offset
            } else {
                0.
            };
            arc.paint(
                a,
                if let Some(color_fn) = self.color.as_ref() {
                    color_fn(a.data)
                } else {
                    cx.theme().chart_2
                },
                Some(inner_radius),
                Some(outer_radius),
                &bounds,
                window,
            );
        }

        let center = point(
            bounds.size.width.as_f32() / 2.,
            bounds.size.height.as_f32() / 2.,
        );
        let mut center_labels = Vec::new();
        let text_size = plot_text_size(cx);
        if let Some(title) = self.center_title.clone() {
            center_labels.push(
                Text::new(
                    title,
                    point(center.x, center.y - 10.),
                    cx.theme().foreground,
                )
                .font_size(cx.theme().font_size)
                .align(TextAlign::Center),
            );
        }
        if let Some(description) = self.center_description.clone() {
            center_labels.push(
                Text::new(
                    description,
                    point(center.x, center.y + 10.),
                    cx.theme().muted_foreground,
                )
                .font_size(text_size)
                .align(TextAlign::Center),
            );
        }
        PlotLabel::new(center_labels).paint(&bounds, window, cx);

        // Draw leader-line labels outside the ring (only when `label` is set).
        let Some(label_fn) = self.label.as_ref() else {
            return;
        };

        let label_radius = outer_radius + self.label_gap;
        let center_x = bounds.size.width.as_f32() / 2.;
        let center_y = bounds.size.height.as_f32() / 2.;
        let label_arc = Arc::new()
            .inner_radius(label_radius)
            .outer_radius(label_radius);
        let edge_arc = Arc::new()
            .inner_radius(outer_radius)
            .outer_radius(outer_radius);

        let label_color = self.label_color.unwrap_or(cx.theme().foreground);
        let default_line_color = cx.theme().border;

        // First pass: collect a layout candidate per visible slice, split by
        // side. `y` is the target vertical position relative to the center and
        // gets adjusted later to remove overlaps.
        let mut right: Vec<LabelLayout> = vec![];
        let mut left: Vec<LabelLayout> = vec![];
        for a in &arcs {
            // Skip tiny slices (< 0.5°) that are too thin to label.
            if a.end_angle - a.start_angle < std::f32::consts::PI / 360. {
                continue;
            }

            let centroid = label_arc.centroid(a);
            let edge = edge_arc.centroid(a);
            let is_right = centroid.x > 0.;
            let line_color = self
                .label_line_color
                .as_ref()
                .map(|f| f(a.data))
                .unwrap_or(default_line_color);

            let layout = LabelLayout {
                arc_x: edge.x,
                arc_y: edge.y,
                label_x: centroid.x,
                y: centroid.y,
                text: label_fn(a.data),
                line_color,
            };
            if is_right { &mut right } else { &mut left }.push(layout);
        }

        // Second pass: spread labels on each side so neighbors keep at least one
        // text height apart, clamped within the vertical bounds.
        let text_size = plot_text_size(cx).as_f32();
        let text_height = text_size + TEXT_GAP;
        let top = -center_y + text_height / 2.;
        let bottom = center_y - text_height / 2.;
        spread_labels(&mut right, top, bottom, text_height);
        spread_labels(&mut left, top, bottom, text_height);

        // Third pass: paint leader lines first, then the text on top.
        let mut labels = vec![];
        for (side, items) in [(1., &right), (-1., &left)] {
            for item in items {
                // Leader line: ring edge -> label anchor -> horizontal pull to
                // ±label_radius.
                let pts = [
                    point(item.arc_x + center_x, item.arc_y + center_y),
                    point(item.label_x + center_x, item.y + center_y),
                    point(side * label_radius + center_x, item.y + center_y),
                ];
                if let Some(p) = polygon(&pts, &bounds) {
                    window.paint_path(p, item.line_color);
                }

                // Text sits 4px further out, aligned by side.
                let origin = point(
                    side * (label_radius + 4.) + center_x,
                    item.y - text_size / 2. + center_y,
                );
                let align = if side > 0. {
                    TextAlign::Left
                } else {
                    TextAlign::Right
                };
                labels.push(
                    Text::new(item.text.clone(), origin, label_color)
                        .font_size(plot_text_size(cx))
                        .align(align),
                );
            }
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
        let value = self.value.as_ref()?(datum);
        let label = self
            .label
            .as_ref()
            .map(|label| label(datum))
            .unwrap_or_else(|| format!("Item {}", state.index + 1).into());
        let color = self
            .color
            .as_ref()
            .map(|color| color(datum))
            .unwrap_or(cx.theme().chart_2);
        Some(
            Tooltip::new(cursor, bounds.size)
                .row(color, label, format!("{value}"))
                .into_any_element(),
        )
    }
}

/// A resolved label position before overlap adjustment.
struct LabelLayout {
    /// Anchor on the ring edge (relative to center).
    arc_x: f32,
    arc_y: f32,
    /// Centroid x at the label radius (relative to center).
    label_x: f32,
    /// Target/adjusted vertical position (relative to center).
    y: f32,
    text: SharedString,
    line_color: Hsla,
}

/// Spread `items` vertically so adjacent labels keep one resolved text line apart.
///
/// Uses a two-direction relaxation: a top-down pass pushes crowded labels down,
/// then a bottom-up pass (anchored at `bottom`) pushes them back up. This
/// resolves cascading overlaps that a single-neighbor nudge cannot.
fn spread_labels(items: &mut [LabelLayout], top: f32, bottom: f32, text_height: f32) {
    let n = items.len();
    if n == 0 {
        return;
    }

    // Sort by target position so neighbors in the slice are neighbors in y.
    items.sort_by(|a, b| a.y.total_cmp(&b.y));

    // Top-down: enforce the minimum gap by pushing labels down.
    for i in 1..n {
        let min_y = items[i - 1].y + text_height;
        if items[i].y < min_y {
            items[i].y = min_y;
        }
    }

    // Bottom-up: clamp the bottom-most label, then pull overflowing labels up.
    if items[n - 1].y > bottom {
        items[n - 1].y = bottom;
    }
    for i in (0..n - 1).rev() {
        let max_y = items[i + 1].y - text_height;
        if items[i].y > max_y {
            items[i].y = max_y;
        }
    }

    // Keep the top-most label within bounds.
    if items[0].y < top {
        items[0].y = top;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    #[test]
    fn pie_chart_builder_supports_active_and_center_content() {
        let chart = PieChart::new([1., 2.])
            .value(|value| *value)
            .inner_radius(40.)
            .active_index(Some(1))
            .active_offset(6.)
            .center_label("3", "Total")
            .id("pie");

        assert_eq!(chart.active_index, Some(1));
        assert_eq!(chart.active_offset, 6.);
        assert_eq!(chart.center_title.as_deref(), Some("3"));
        assert_eq!(chart.center_description.as_deref(), Some("Total"));
        assert_eq!(chart.id, Some("pie".into()));
    }

    #[test]
    fn pie_hit_test_includes_active_offset_and_excludes_padding() {
        let chart = PieChart::new([1., 1.])
            .value(|value| *value)
            .inner_radius(40.)
            .outer_radius(80.)
            .pad_angle(0.2)
            .active_index(Some(0))
            .active_offset(10.);
        let bounds = Bounds::from_corners(point(px(0.), px(0.)), point(px(200.), px(200.)));

        assert_eq!(
            chart.hovered_index(point(px(185.), px(100.)), bounds),
            Some(0)
        );
        assert_eq!(chart.hovered_index(point(px(100.), px(180.)), bounds), None);
    }
}
