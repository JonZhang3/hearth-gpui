use crate::{ActiveTheme, StyledExt};
use gpui::{Animation, AnimationExt, IntoElement, RenderOnce, StyleRefinement, Styled, div};

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

    /// Set use secondary color.
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
        let motion = cx.theme().style.motion;
        let easing = motion.move_easing;

        div()
            .w_full()
            .h_4()
            .bg(if self.secondary {
                cx.theme().skeleton.opacity(0.5).into()
            } else {
                cx.theme().skeleton
            })
            .refine_style(&self.style)
            .with_animation(
                "skeleton",
                Animation::new(motion.loading())
                    .repeat()
                    .with_easing(move |delta| {
                        let pulse = if delta < 0.5 {
                            delta * 2.0
                        } else {
                            (1.0 - delta) * 2.0
                        };
                        easing.sample(pulse)
                    }),
                move |this, delta| {
                    let v = 1.0 - delta * 0.4;
                    this.opacity(v)
                },
            )
    }
}
