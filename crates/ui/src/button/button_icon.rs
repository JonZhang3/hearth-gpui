use crate::{Icon, Sizable, Size, progress::ProgressCircle, spinner::Spinner};
use gpui::{App, IntoElement, RenderOnce, Window};

/// Button icon which can be an Icon, Spinner, or Progress use for `icon` method of Button.
#[doc(hidden)]
#[derive(IntoElement)]
pub struct ButtonIcon {
    icon: ButtonIconVariant,
    size: Size,
}

impl<T> From<T> for ButtonIcon
where
    T: Into<ButtonIconVariant>,
{
    fn from(icon: T) -> Self {
        ButtonIcon::new(icon)
    }
}

impl ButtonIcon {
    /// Creates a new ButtonIcon with the given icon.
    pub fn new(icon: impl Into<ButtonIconVariant>) -> Self {
        Self {
            icon: icon.into(),
            size: Size::Medium,
        }
    }
}

impl Sizable for ButtonIcon {
    fn with_size(mut self, size: impl Into<crate::Size>) -> Self {
        self.size = size.into();
        self
    }
}

/// Button icon which can be an Icon, Spinner, Progress, or ProgressCircle use for `icon` method of Button.
#[doc(hidden)]
#[derive(IntoElement)]
pub enum ButtonIconVariant {
    Icon(Icon),
    Spinner(Spinner),
    Progress(ProgressCircle),
}

impl<T> From<T> for ButtonIconVariant
where
    T: Into<Icon>,
{
    fn from(icon: T) -> Self {
        Self::Icon(icon.into())
    }
}

impl From<Spinner> for ButtonIconVariant {
    fn from(spinner: Spinner) -> Self {
        Self::Spinner(spinner)
    }
}

impl From<ProgressCircle> for ButtonIconVariant {
    fn from(progress: ProgressCircle) -> Self {
        Self::Progress(progress)
    }
}

impl ButtonIconVariant {
    /// Returns true if the ButtonIconKind is an Icon.
    #[inline]
    #[cfg(test)]
    pub(crate) fn is_spinner(&self) -> bool {
        matches!(self, Self::Spinner(_))
    }

    /// Returns true if the ButtonIconKind is a Progress or ProgressCircle.
    #[inline]
    #[cfg(test)]
    pub(crate) fn is_progress(&self) -> bool {
        matches!(self, Self::Progress(_))
    }
}

impl Sizable for ButtonIconVariant {
    fn with_size(self, size: impl Into<crate::Size>) -> Self {
        match self {
            Self::Icon(icon) => Self::Icon(icon.with_size(size)),
            Self::Spinner(spinner) => Self::Spinner(spinner.with_size(size)),
            Self::Progress(progress) => Self::Progress(progress.with_size(size)),
        }
    }
}

impl RenderOnce for ButtonIconVariant {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        match self {
            Self::Icon(icon) => icon.into_any_element(),
            Self::Spinner(spinner) => spinner.into_any_element(),
            Self::Progress(progress) => progress.into_any_element(),
        }
    }
}

impl RenderOnce for ButtonIcon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.icon.with_size(self.size).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IconName;

    #[gpui::test]
    fn test_button_icon_builder(_cx: &mut gpui::TestAppContext) {
        let icon = ButtonIcon::new(IconName::Plus).large();

        assert_eq!(icon.size, Size::Large);
    }

    #[gpui::test]
    fn test_button_icon_variant_types(_cx: &mut gpui::TestAppContext) {
        // Test Icon variant
        let icon_variant = ButtonIconVariant::Icon(Icon::new(IconName::Plus));
        assert!(!icon_variant.is_spinner());
        assert!(!icon_variant.is_progress());

        // Test Spinner variant
        let spinner_variant = ButtonIconVariant::Spinner(Spinner::new());
        assert!(spinner_variant.is_spinner());
        assert!(!spinner_variant.is_progress());

        // Test Progress variant
        let progress_variant = ButtonIconVariant::Progress(ProgressCircle::new(75));
        assert!(!progress_variant.is_spinner());
        assert!(progress_variant.is_progress());
    }
}
