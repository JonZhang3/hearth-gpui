use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div,
};
use smallvec::SmallVec;

use crate::StyledExt as _;

/// A layout container that preserves a width-to-height ratio.
///
/// The parent must constrain at least one axis. By default, the container fills
/// the available width and derives its height from `ratio`.
#[derive(IntoElement)]
pub struct AspectRatio {
    ratio: f32,
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 1]>,
}

impl AspectRatio {
    /// Creates an aspect-ratio container using `width / height`.
    ///
    /// Invalid ratios fall back to `1.0` so non-finite layout values never
    /// reach GPUI's layout engine.
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: normalize_ratio(ratio),
            style: StyleRefinement::default(),
            children: SmallVec::new(),
        }
    }

    /// Replaces the width-to-height ratio.
    ///
    /// Invalid ratios fall back to a square ratio.
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = normalize_ratio(ratio);
        self
    }
}

impl ParentElement for AspectRatio {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for AspectRatio {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AspectRatio {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .relative()
            .w_full()
            .aspect_ratio(self.ratio)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// Keeps invalid ratios out of Taffy while preserving a deterministic layout.
fn normalize_ratio(ratio: f32) -> f32 {
    if ratio.is_finite() && ratio > 0.0 {
        ratio
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::{AspectRatio, normalize_ratio};
    use gpui::ParentElement as _;

    #[test]
    fn aspect_ratio_builder_preserves_ratio_and_children() {
        let aspect_ratio = AspectRatio::new(16.0 / 9.0).child("media").ratio(4.0 / 3.0);

        assert_eq!(aspect_ratio.ratio, 4.0 / 3.0);
        assert_eq!(aspect_ratio.children.len(), 1);
    }

    #[test]
    fn invalid_ratios_fall_back_to_square() {
        assert_eq!(normalize_ratio(0.0), 1.0);
        assert_eq!(normalize_ratio(-1.0), 1.0);
        assert_eq!(normalize_ratio(f32::NAN), 1.0);
        assert_eq!(normalize_ratio(f32::INFINITY), 1.0);
    }
}
