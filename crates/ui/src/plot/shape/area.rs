// @reference: https://d3js.org/d3-shape/area

use gpui::{Background, Bounds, Path, PathBuilder, Pixels, Point, Window, px};

use crate::plot::{StrokeStyle, origin_point};

fn step_after_points(points: &[Point<Pixels>]) -> Vec<Point<Pixels>> {
    let mut result = Vec::with_capacity(points.len().saturating_mul(2));
    if let Some(first) = points.first().copied() {
        result.push(first);
    }
    for pair in points.windows(2) {
        result.push(Point::new(pair[1].x, pair[0].y));
        result.push(pair[1]);
    }
    result
}

fn reversed_step_after_points(points: &[Point<Pixels>]) -> Vec<Point<Pixels>> {
    let mut result = step_after_points(points);
    result.reverse();
    result
}

fn append_curve(
    builder: &mut PathBuilder,
    points: &[Point<Pixels>],
    style: StrokeStyle,
    connect: bool,
) {
    let Some(first) = points.first().copied() else {
        return;
    };
    if connect {
        builder.line_to(first);
    } else {
        builder.move_to(first);
    }

    match style {
        StrokeStyle::Natural => {
            for index in 0..points.len().saturating_sub(1) {
                let p0 = if index == 0 {
                    points[0]
                } else {
                    points[index - 1]
                };
                let p1 = points[index];
                let p2 = points[index + 1];
                let p3 = if index + 2 < points.len() {
                    points[index + 2]
                } else {
                    points[points.len() - 1]
                };
                let c1 = Point::new(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0);
                let c2 = Point::new(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0);
                builder.cubic_bezier_to(p2, c1, c2);
            }
        }
        StrokeStyle::Linear => {
            for point in &points[1..] {
                builder.line_to(*point);
            }
        }
        StrokeStyle::StepAfter => {
            for point in &step_after_points(points)[1..] {
                builder.line_to(*point);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub struct Area<T> {
    data: Vec<T>,
    x: Box<dyn Fn(&T) -> Option<f32>>,
    y0: Box<dyn Fn(&T) -> Option<f32>>,
    y1: Box<dyn Fn(&T) -> Option<f32>>,
    fill: Background,
    stroke: Background,
    stroke_style: StrokeStyle,
}

impl<T> Default for Area<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            x: Box::new(|_| None),
            y0: Box::new(|_| None),
            y1: Box::new(|_| None),
            fill: Default::default(),
            stroke: Default::default(),
            stroke_style: Default::default(),
        }
    }
}

impl<T> Area<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the data of the Area.
    pub fn data<I>(mut self, data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        self.data = data.into_iter().collect();
        self
    }

    /// Set the x of the Area.
    pub fn x<F>(mut self, x: F) -> Self
    where
        F: Fn(&T) -> Option<f32> + 'static,
    {
        self.x = Box::new(x);
        self
    }

    /// Set the y0 of the Area.
    pub fn y0(mut self, y0: f32) -> Self {
        self.y0 = Box::new(move |_| Some(y0));
        self
    }

    /// Sets a datum-dependent lower boundary for stacked areas.
    pub fn y0_fn<F>(mut self, y0: F) -> Self
    where
        F: Fn(&T) -> Option<f32> + 'static,
    {
        self.y0 = Box::new(y0);
        self
    }

    /// Set the y1 of the Area.
    pub fn y1<F>(mut self, y1: F) -> Self
    where
        F: Fn(&T) -> Option<f32> + 'static,
    {
        self.y1 = Box::new(y1);
        self
    }

    /// Set the fill color of the Area.
    pub fn fill(mut self, fill: impl Into<Background>) -> Self {
        self.fill = fill.into();
        self
    }

    /// Set the stroke color of the Area.
    pub fn stroke(mut self, stroke: impl Into<Background>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Set the stroke style of the Area.
    pub fn stroke_style(mut self, stroke_style: StrokeStyle) -> Self {
        self.stroke_style = stroke_style;
        self
    }

    fn path(&self, bounds: &Bounds<Pixels>) -> (Option<Path<Pixels>>, Option<Path<Pixels>>) {
        let origin = bounds.origin;
        let mut area_builder = PathBuilder::fill();
        let mut line_builder = PathBuilder::stroke(px(1.));

        let mut points = vec![];

        let mut baseline_points = vec![];
        for v in &self.data {
            let x_tick = (self.x)(v);
            let y_tick = (self.y1)(v);

            if let (Some(x), Some(y)) = (x_tick, y_tick) {
                let pos = origin_point(px(x), px(y), origin);

                points.push(pos);
                if let Some(y0) = (self.y0)(v) {
                    baseline_points.push(origin_point(px(x), px(y0), origin));
                }
            }
        }

        if points.is_empty() {
            return (None, None);
        }

        if points.len() == 1 {
            area_builder.move_to(points[0]);
            line_builder.move_to(points[0]);
            return (area_builder.build().ok(), line_builder.build().ok());
        }

        append_curve(&mut area_builder, &points, self.stroke_style, false);
        append_curve(&mut line_builder, &points, self.stroke_style, false);

        // Close path
        if baseline_points.len() == points.len() {
            if matches!(self.stroke_style, StrokeStyle::StepAfter) {
                let baseline = reversed_step_after_points(&baseline_points);
                append_curve(&mut area_builder, &baseline, StrokeStyle::Linear, true);
            } else {
                baseline_points.reverse();
                append_curve(&mut area_builder, &baseline_points, self.stroke_style, true);
            }
            area_builder.close();
        }

        (area_builder.build().ok(), line_builder.build().ok())
    }

    /// Paint the Area.
    pub fn paint(&self, bounds: &Bounds<Pixels>, window: &mut Window) {
        let (area, line) = self.path(bounds);

        if let Some(area) = area {
            window.paint_path(area, self.fill);
        }
        if let Some(line) = line {
            window.paint_path(line, self.stroke);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_after_geometry_can_be_replayed_in_exact_reverse() {
        let points = vec![
            Point::new(px(0.), px(10.)),
            Point::new(px(20.), px(30.)),
            Point::new(px(40.), px(15.)),
        ];
        let forward = step_after_points(&points);
        let reverse = reversed_step_after_points(&points);

        assert_eq!(forward[1], Point::new(px(20.), px(10.)));
        assert_eq!(forward[2], points[1]);
        assert_eq!(reverse.first(), forward.last());
        assert_eq!(reverse.last(), forward.first());
    }
}
