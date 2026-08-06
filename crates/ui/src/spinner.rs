use crate::{ActiveTheme as _, Icon, IconName, Sizable, Size};
use gpui::{
    Animation, AnimationExt as _, App, Hsla, IntoElement, ParentElement, RenderOnce, Styled as _,
    Transformation, Window, div, percentage, prelude::FluentBuilder as _,
};

/// A cycling loading spinner.
#[derive(IntoElement)]
pub struct Spinner {
    size: Size,
    icon: Icon,
    easing: Option<Box<dyn Fn(f32) -> f32>>,
    color: Option<Hsla>,
}

impl Spinner {
    /// Create a new loading spinner.
    pub fn new() -> Self {
        Self {
            size: Size::Medium,
            easing: None,
            icon: Icon::new(IconName::Loader),
            color: None,
        }
    }

    /// Set specified icon for the spinner.
    ///
    /// Default is [`IconName::Loader`].
    ///
    /// Please ensure the icon used is suitable for a loading spinner.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Set the icon color.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the easing function.
    pub fn ease(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.easing = Some(Box::new(easing));
        self
    }
}

impl Sizable for Spinner {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let motion = cx.theme().style.motion;
        let easing = self.easing.unwrap_or_else(|| {
            let easing = motion.move_easing;
            Box::new(move |delta| easing.sample(delta))
        });

        div()
            .child(
                self.icon
                    .with_size(self.size)
                    .when_some(self.color, |this, color| this.text_color(color))
                    .with_animation(
                        "circle",
                        Animation::new(motion.loading())
                            .repeat()
                            .with_easing(easing),
                        |this, delta| this.transform(Transformation::rotate(percentage(delta))),
                    ),
            )
            .into_element()
    }
}
