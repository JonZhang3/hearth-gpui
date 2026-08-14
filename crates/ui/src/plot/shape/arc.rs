// @reference: https://d3js.org/d3-shape/arc

use std::{f32::consts::PI, fmt::Debug};

use gpui::{Bounds, Hsla, Path, PathBuilder, Pixels, Point, Window, point, px};

const EPSILON: f32 = 1e-12;
const HALF_PI: f32 = PI / 2.;

pub struct ArcData<'a, T> {
    pub data: &'a T,
    pub index: usize,
    pub value: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    pub pad_angle: f32,
}

impl<T> Debug for ArcData<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ArcData {{ index: {}, value: {}, start_angle: {}, end_angle: {}, pad_angle: {} }}",
            self.index, self.value, self.start_angle, self.end_angle, self.pad_angle
        )
    }
}

pub struct Arc {
    inner_radius: f32,
    outer_radius: f32,
}

#[derive(Clone, Copy, Debug)]
struct ArcGeometry {
    start_angle: f32,
    end_angle: f32,
    pad_angle: f32,
    inner_radius: f32,
    outer_radius: f32,
}

impl Default for Arc {
    fn default() -> Self {
        Self {
            inner_radius: 0.,
            outer_radius: 0.,
        }
    }
}

impl Arc {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the inner radius of the Arc.
    pub fn inner_radius(mut self, inner_radius: f32) -> Self {
        self.inner_radius = inner_radius;
        self
    }

    /// Set the outer radius of the Arc.
    pub fn outer_radius(mut self, outer_radius: f32) -> Self {
        self.outer_radius = outer_radius;
        self
    }

    /// Get the centroid of the Arc.
    pub fn centroid<T>(&self, arc: &ArcData<T>) -> Point<f32> {
        let start_angle = arc.start_angle - HALF_PI;
        let end_angle = arc.end_angle - HALF_PI;
        let r = (self.inner_radius + self.outer_radius) / 2.;
        let a = (start_angle + end_angle) / 2.;

        point(r * a.cos(), r * a.sin())
    }

    fn geometry<T>(
        &self,
        arc: &ArcData<T>,
        inner_radius: Option<f32>,
        outer_radius: Option<f32>,
    ) -> Option<ArcGeometry> {
        let start_angle = arc.start_angle - HALF_PI;
        let end_angle = arc.end_angle - HALF_PI;
        let sweep = end_angle - start_angle;
        let pad_angle = if sweep >= PI { 0.0001 } else { arc.pad_angle };
        let inner_radius = inner_radius.unwrap_or(self.inner_radius).max(0.);
        let outer_radius = outer_radius.unwrap_or(self.outer_radius).max(0.);
        if outer_radius < EPSILON || sweep.abs() < EPSILON {
            return None;
        }
        Some(ArcGeometry {
            start_angle,
            end_angle,
            pad_angle,
            inner_radius,
            outer_radius,
        })
    }

    /// Returns whether a point lies inside the same padded annular segment painted by this Arc.
    pub fn contains<T>(
        &self,
        arc: &ArcData<T>,
        position: Point<Pixels>,
        bounds: &Bounds<Pixels>,
        inner_radius: Option<f32>,
        outer_radius: Option<f32>,
    ) -> bool {
        let Some(geometry) = self.geometry(arc, inner_radius, outer_radius) else {
            return false;
        };
        let center = point(
            bounds.size.width.as_f32() / 2.,
            bounds.size.height.as_f32() / 2.,
        );
        let dx = position.x.as_f32() - center.x;
        let dy = position.y.as_f32() - center.y;
        let radius = dx.hypot(dy);
        if radius < geometry.inner_radius || radius > geometry.outer_radius {
            return false;
        }

        let sweep = geometry.end_angle - geometry.start_angle;
        if sweep <= 0. {
            return false;
        }
        let angle = dy.atan2(dx);
        let relative_angle = (angle - geometry.start_angle).rem_euclid(2. * PI);
        if relative_angle > sweep {
            return false;
        }

        let effective_pad = if geometry.inner_radius > EPSILON && geometry.pad_angle > 0. {
            let pad_width = geometry.outer_radius * geometry.pad_angle;
            (pad_width / radius.max(EPSILON)).min(sweep * 0.8)
        } else {
            geometry.pad_angle
        };
        relative_angle >= effective_pad * 0.5 && relative_angle <= sweep - effective_pad * 0.5
    }

    fn path<T>(
        &self,
        arc: &ArcData<T>,
        inner_radius: Option<f32>,
        outer_radius: Option<f32>,
        bounds: &Bounds<Pixels>,
    ) -> Option<Path<Pixels>> {
        let geometry = self.geometry(arc, inner_radius, outer_radius)?;
        let start_angle = geometry.start_angle;
        let end_angle = geometry.end_angle;
        let da = end_angle - start_angle;
        let pad_angle = geometry.pad_angle;
        let r0 = geometry.inner_radius;
        let r1 = geometry.outer_radius;

        // Calculate the center point.
        let center_x = bounds.origin.x.as_f32() + bounds.size.width.as_f32() / 2.;
        let center_y = bounds.origin.y.as_f32() + bounds.size.height.as_f32() / 2.;

        // Angle difference.
        // Handle pad angle.
        let (a0_outer, a1_outer, a0_inner, a1_inner) = if r0 > EPSILON && pad_angle > 0.0 {
            let pad_width = r1 * pad_angle;
            let pad_angle_outer = pad_width / r1;
            let mut pad_angle_inner = pad_width / r0;
            let max_inner_pad = da * 0.8;
            if pad_angle_inner > max_inner_pad {
                pad_angle_inner = max_inner_pad;
            }
            (
                start_angle + pad_angle_outer * 0.5,
                end_angle - pad_angle_outer * 0.5,
                start_angle + pad_angle_inner * 0.5,
                end_angle - pad_angle_inner * 0.5,
            )
        } else {
            let pad = pad_angle * 0.5;
            (
                start_angle + pad,
                end_angle - pad,
                start_angle + pad,
                end_angle - pad,
            )
        };

        let da_outer = a1_outer - a0_outer;
        if da_outer <= 0. {
            return None;
        }

        // Calculate the start and end points of the outer arc.
        let x01 = center_x + r1 * a0_outer.cos();
        let y01 = center_y + r1 * a0_outer.sin();
        let x11 = center_x + r1 * a1_outer.cos();
        let y11 = center_y + r1 * a1_outer.sin();

        let mut builder = PathBuilder::fill();

        // Move to the start point of the outer arc.
        builder.move_to(point(px(x01), px(y01)));

        // Draw the outer arc.
        let large_arc = (a1_outer - a0_outer).abs() > PI;
        builder.arc_to(
            point(px(r1), px(r1)),
            px(0.),
            large_arc,
            true,
            point(px(x11), px(y11)),
        );

        if r0 > EPSILON {
            // End point of the inner arc.
            let x10 = center_x + r0 * a1_inner.cos();
            let y10 = center_y + r0 * a1_inner.sin();
            builder.line_to(point(px(x10), px(y10)));

            // Draw the inner arc.
            let x00 = center_x + r0 * a0_inner.cos();
            let y00 = center_y + r0 * a0_inner.sin();
            let large_arc_inner = (a1_inner - a0_inner).abs() > PI;
            builder.arc_to(
                point(px(r0), px(r0)),
                px(0.),
                large_arc_inner,
                false,
                point(px(x00), px(y00)),
            );
        } else {
            // If there is no inner radius, draw a line to the center.
            builder.line_to(point(px(center_x), px(center_y)));
        }

        builder.build().ok()
    }

    /// Paint the Arc.
    pub fn paint<T>(
        &self,
        arc: &ArcData<T>,
        color: impl Into<Hsla>,
        inner_radius: Option<f32>,
        outer_radius: Option<f32>,
        bounds: &Bounds<Pixels>,
        window: &mut Window,
    ) {
        let path = self.path(arc, inner_radius, outer_radius, bounds);
        if let Some(path) = path {
            window.paint_path(path, color.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_default() {
        let arc = Arc::default();
        assert_eq!(arc.inner_radius, 0.);
        assert_eq!(arc.outer_radius, 0.);
    }

    #[test]
    fn test_arc_builder() {
        let arc = Arc::new().inner_radius(10.).outer_radius(20.);

        assert_eq!(arc.inner_radius, 10.);
        assert_eq!(arc.outer_radius, 20.);
    }

    #[test]
    fn test_arc_centroid() {
        let arc = Arc::new().inner_radius(10.).outer_radius(20.);

        let arc_data = ArcData {
            data: &(),
            index: 0,
            value: 1.,
            start_angle: 0.,
            end_angle: PI,
            pad_angle: 0.,
        };

        let centroid = arc.centroid(&arc_data);
        let expected_radius = (10. + 20.) / 2.;
        let expected_angle = (0. + PI - 2. * HALF_PI) / 2.;

        assert_eq!(centroid.x, expected_radius * expected_angle.cos());
        assert_eq!(centroid.y, expected_radius * expected_angle.sin());
    }

    #[test]
    fn arc_contains_respects_annular_bounds_and_padding() {
        let shape = Arc::new().inner_radius(40.).outer_radius(80.);
        let segment = ArcData {
            data: &(),
            index: 0,
            value: 1.,
            start_angle: 0.,
            end_angle: PI,
            pad_angle: 0.2,
        };
        let bounds = Bounds::from_corners(point(px(0.), px(0.)), point(px(200.), px(200.)));

        assert!(shape.contains(&segment, point(px(160.), px(100.)), &bounds, None, None));
        assert!(!shape.contains(&segment, point(px(100.), px(100.)), &bounds, None, None));
        assert!(!shape.contains(&segment, point(px(100.), px(30.)), &bounds, None, None));
    }
}
