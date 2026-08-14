// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added or exposed behavior through `skeleton_metrics`, `skeleton_opacity`,
//   `resolves_preset_radius_from_semantic_density`, `pulse_is_symmetric_and_reaches_half_opacity`.
// - Reworked Skeleton around interruptible and reduced-motion-aware transitions, semantic Style
//   Preset geometry and density.
use crate::{ActiveTheme, Density, MotionEasing, StylePreset, StyledExt};
use gpui::{
    Animation, AnimationExt, IntoElement, Pixels, RenderOnce, StyleRefinement, Styled, div,
};

/// Geometry resolved from the active Style Preset without checking preset IDs.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SkeletonMetrics {
    radius: Pixels,
}

/// Resolves the pinned shadcn Skeleton radius for each semantic density.
fn skeleton_metrics(style: &StylePreset) -> SkeletonMetrics {
    SkeletonMetrics {
        radius: match style.density {
            Density::Compact | Density::Standard => style.radii.md,
            Density::Comfortable => style.radii.xl,
        },
    }
}

/// Returns the shadcn pulse opacity for a normalized animation position.
fn skeleton_opacity(delta: f32, easing: MotionEasing) -> f32 {
    let pulse = if delta < 0.5 {
        delta * 2.0
    } else {
        (1.0 - delta) * 2.0
    };
    1.0 - easing.sample(pulse) * 0.5
}

/// A skeleton loading placeholder element.
#[derive(IntoElement)]
pub struct Skeleton {
    style: StyleRefinement,
    secondary: bool,
}

impl Skeleton {
    /// Create a new Skeleton element.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            secondary: false,
        }
    }

    /// Use a subtler version of the semantic muted surface.
    pub fn secondary(mut self) -> Self {
        self.secondary = true;
        self
    }
}

impl Styled for Skeleton {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let style = cx.theme().style.as_ref();
        let metrics = skeleton_metrics(style);
        let motion = style.motion;
        let easing = motion.move_easing;

        div()
            .w_full()
            .h_4()
            .rounded(metrics.radius)
            .bg(if self.secondary {
                cx.theme().muted.opacity(0.5).into()
            } else {
                cx.theme().muted
            })
            .refine_style(&self.style)
            .with_animation(
                "skeleton",
                Animation::new(motion.loading()).repeat(),
                move |this, delta| this.opacity(skeleton_opacity(delta, easing)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_preset_radius_from_semantic_density() {
        let vega = StylePreset::vega();
        let nova = StylePreset::nova();
        let maia = StylePreset::maia();

        assert_eq!(skeleton_metrics(&vega).radius, vega.radii.md);
        assert_eq!(skeleton_metrics(&nova).radius, nova.radii.md);
        assert_eq!(skeleton_metrics(&maia).radius, maia.radii.xl);
    }

    #[test]
    fn pulse_is_symmetric_and_reaches_half_opacity() {
        let easing = MotionEasing::EaseInOutCubic;

        assert_eq!(skeleton_opacity(0.0, easing), 1.0);
        assert_eq!(skeleton_opacity(0.5, easing), 0.5);
        assert_eq!(skeleton_opacity(1.0, easing), 1.0);
        assert_eq!(
            skeleton_opacity(0.25, easing),
            skeleton_opacity(0.75, easing)
        );
    }
}
