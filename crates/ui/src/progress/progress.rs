// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `aria_label`, `aria_value`.
// - Added or exposed behavior through `progress_child_id`, `resolve`, `aria_label`, `aria_value`,
//   `preset_density_resolves_pinned_default_height`, `value_clamps_invalid_and_out_of_range_input`,
//   `builder_preserves_accessibility_metadata`, `internal_ids_preserve_structural_identity`.
// - Reworked Progress around accessibility semantics and ARIA state, interruptible and
//   reduced-motion-aware transitions, semantic Style Preset geometry and density, invalid and
//   validation state handling.
use std::sync::Arc;

use crate::{ActiveTheme, Sizable, Size, StyledExt, animation::Transition, theme::Density};
use gpui::{
    Animation, AnimationExt as _, App, Background, ElementId, Hsla, InteractiveElement as _,
    IntoElement, ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px, relative,
};

/// Creates stable internal state IDs without flattening structured caller IDs.
fn progress_child_id(id: &ElementId, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), name.into())
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ProgressMetrics {
    height: gpui::Pixels,
    radius: gpui::Pixels,
}

impl ProgressMetrics {
    /// Resolves shadcn geometry from semantic density and the public size API.
    fn resolve(size: Size, density: Density) -> Self {
        let height = match (density, size) {
            (_, Size::Size(height)) => height.max(px(0.)),
            (Density::Compact, Size::XSmall) => px(2.),
            (Density::Compact, Size::Small) => px(3.),
            (Density::Compact, Size::Medium) => px(4.),
            (Density::Compact, Size::Large) => px(6.),
            (Density::Standard, Size::XSmall) => px(4.),
            (Density::Standard, Size::Small) => px(5.),
            (Density::Standard, Size::Medium) => px(6.),
            (Density::Standard, Size::Large) => px(8.),
            (Density::Comfortable, Size::XSmall) => px(6.),
            (Density::Comfortable, Size::Small) => px(8.),
            (Density::Comfortable, Size::Medium) => px(12.),
            (Density::Comfortable, Size::Large) => px(16.),
        };
        Self {
            height,
            radius: height / 2.,
        }
    }
}

/// A linear horizontal progress bar element.
#[derive(IntoElement)]
pub struct Progress {
    id: ElementId,
    style: StyleRefinement,
    color: Option<Hsla>,
    value: f32,
    size: Size,
    loading: bool,
    aria_label: Option<SharedString>,
    aria_value: Option<SharedString>,
}

impl Progress {
    /// Create a new Progress bar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: Default::default(),
            color: None,
            style: StyleRefinement::default(),
            size: Size::default(),
            loading: false,
            aria_label: None,
            aria_value: None,
        }
    }

    /// Enable indeterminate loading animation.
    ///
    /// When `loading` is `true`, the `value` is ignored and an infinite
    /// sliding animation is shown instead.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set the color of the progress bar.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the accessible name announced for the progress indicator.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set a human-readable accessible value such as "3 of 5 files".
    pub fn aria_value(mut self, value: impl Into<SharedString>) -> Self {
        self.aria_value = Some(value.into());
        self
    }

    /// Set the percentage value of the progress bar.
    ///
    /// The value should be between 0.0 and 100.0.
    pub fn value(mut self, value: f32) -> Self {
        self.value = if value.is_finite() {
            value.clamp(0., 100.)
        } else {
            0.
        };
        self
    }
}

impl Styled for Progress {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Progress {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Progress {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let indicator_bg = self
            .color
            .map(Background::from)
            .unwrap_or(cx.theme().tokens.progress_bar.into());
        let track_bg: Background = cx.theme().tokens.muted.into();
        let value = self.value;
        let loading = self.loading;
        let motion = cx.theme().style.motion;
        let metrics = ProgressMetrics::resolve(self.size, cx.theme().style.density);
        let progress_id = self.id.clone();

        let radius = self.style.corner_radii.clone();
        let mut inner_style = StyleRefinement::default();
        inner_style.corner_radii = radius;

        let root = div()
            .id(self.id)
            .role(Role::ProgressIndicator)
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .when_some(self.aria_value, |this, value| this.aria_value(value))
            .when(!loading, |this| {
                this.aria_numeric_value(value as f64)
                    .aria_min_numeric_value(0.0)
                    .aria_max_numeric_value(100.0)
            })
            .w_full()
            .relative()
            .h(metrics.height)
            .rounded(metrics.radius)
            .overflow_hidden()
            .bg(track_bg)
            .refine_style(&self.style);

        let indicator = div()
            .absolute()
            .top_0()
            .left_0()
            .h_full()
            .bg(indicator_bg)
            .rounded(metrics.radius)
            .refine_style(&inner_style)
            .map(|this| match value {
                v if v >= 100. || loading => this,
                _ => this.rounded_r_none(),
            });

        root.child(if loading {
            if cx.reduce_motion() {
                indicator
                    .left(relative(0.25))
                    .right(relative(0.25))
                    .into_any_element()
            } else {
                let easing = motion.move_easing;
                indicator
                    .with_animation(
                        progress_child_id(&progress_id, "loading-motion"),
                        Animation::new(motion.loading()).repeat(),
                        move |this, delta| {
                            let start =
                                relative(easing.sample(((delta - 0.5) / 0.5).clamp(0., 1.)));
                            let end = relative(easing.sample(1.0 - delta));
                            this.when(delta > 0.5, |this| this.left(start)).right(end)
                        },
                    )
                    .into_any_element()
            }
        } else {
            let fraction = (value / 100.).clamp(0., 1.);
            Transition::new(motion.normal())
                .ease_token(motion.move_easing)
                // Matching initial and target widths keeps first paint stable;
                // retained MotionValue state handles later interruptions.
                .relative_width(fraction, fraction)
                .apply(indicator, progress_child_id(&progress_id, "value-motion"))
                .into_any_element()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_density_resolves_pinned_default_height() {
        assert_eq!(
            ProgressMetrics::resolve(Size::Medium, Density::Compact).height,
            px(4.)
        );
        assert_eq!(
            ProgressMetrics::resolve(Size::Medium, Density::Standard).height,
            px(6.)
        );
        assert_eq!(
            ProgressMetrics::resolve(Size::Medium, Density::Comfortable).height,
            px(12.)
        );
    }

    #[test]
    fn value_clamps_invalid_and_out_of_range_input() {
        assert_eq!(Progress::new("nan").value(f32::NAN).value, 0.);
        assert_eq!(Progress::new("low").value(-10.).value, 0.);
        assert_eq!(Progress::new("high").value(120.).value, 100.);
    }

    #[test]
    fn builder_preserves_accessibility_metadata() {
        let progress = Progress::new("upload")
            .aria_label("Upload progress")
            .aria_value("3 of 5 files")
            .loading(true);

        assert_eq!(progress.aria_label, Some("Upload progress".into()));
        assert_eq!(progress.aria_value, Some("3 of 5 files".into()));
        assert!(progress.loading);
    }

    #[test]
    fn internal_ids_preserve_structural_identity() {
        let structured = ElementId::NamedInteger("progress".into(), 1);
        let textual = ElementId::Name("progress-1".into());
        assert_eq!(structured.to_string(), textual.to_string());
        assert_ne!(
            progress_child_id(&structured, "value-motion"),
            progress_child_id(&textual, "value-motion")
        );
    }
}
