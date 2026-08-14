// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public types: `SpinnerVariant`, `SpinnerAnimation`.
// - Added public methods: `variant`, `id`, `aria_label`, `animation`.
// - Added or exposed behavior through `defaults`, `variant`, `id`, `aria_label`, `animation`,
//   `builder_preserves_aligned_defaults_and_overrides`,
//   `explicit_icon_and_animation_are_independent_from_variant_order`,
//   `exposes_accessible_loading_status`.
// - Reworked Spinner around accessibility semantics and ARIA state, interruptible and
//   reduced-motion-aware transitions.
use std::panic::Location;

use crate::{ActiveTheme as _, Icon, IconName, Sizable, Size};
use gpui::{
    Animation, AnimationExt as _, App, ElementId, Hsla, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement as _, Styled as _,
    Transformation, Window, div, linear, percentage, prelude::FluentBuilder as _,
};

/// Built-in icon and motion combinations for [`Spinner`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerVariant {
    /// The shadcn-aligned circular loader with continuous rotation.
    #[default]
    Circular,
    /// The original segmented loader with semantic eased rotation.
    Classic,
}

impl SpinnerVariant {
    /// Resolve the default icon and animation without overriding explicit choices.
    fn defaults(self) -> (IconName, SpinnerAnimation) {
        match self {
            Self::Circular => (IconName::LoaderCircle, SpinnerAnimation::LinearSpin),
            Self::Classic => (IconName::Loader, SpinnerAnimation::SemanticSpin),
        }
    }
}

/// Motion treatments supported by [`Spinner`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerAnimation {
    /// Rotate one full turn with linear easing, matching shadcn.
    #[default]
    LinearSpin,
    /// Rotate one full turn using the active Style Preset's move easing.
    SemanticSpin,
}

/// A cycling loading spinner.
#[derive(IntoElement)]
pub struct Spinner {
    id: ElementId,
    size: Size,
    variant: SpinnerVariant,
    icon: Option<Icon>,
    animation: Option<SpinnerAnimation>,
    easing: Option<Box<dyn Fn(f32) -> f32>>,
    color: Option<Hsla>,
    aria_label: SharedString,
}

impl Spinner {
    /// Create a new loading spinner.
    ///
    /// The caller location provides a stable accessibility and animation ID.
    /// Use [`Spinner::id`] when creating multiple spinners from the same code
    /// location, such as inside an iterator.
    #[track_caller]
    pub fn new() -> Self {
        Self {
            id: ElementId::CodeLocation(*Location::caller()),
            size: Size::Medium,
            variant: SpinnerVariant::default(),
            animation: None,
            easing: None,
            icon: None,
            color: None,
            aria_label: "Loading".into(),
        }
    }

    /// Set a built-in icon and motion combination.
    ///
    /// Explicit [`Spinner::icon`] and [`Spinner::animation`] overrides remain
    /// authoritative regardless of builder call order.
    pub fn variant(mut self, variant: SpinnerVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set a stable element ID.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the accessible loading status name.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = label.into();
        self
    }

    /// Set specified icon for the spinner.
    ///
    /// Default is [`IconName::LoaderCircle`].
    ///
    /// Please ensure the icon used is suitable for a loading spinner.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Override the motion selected by the active variant.
    pub fn animation(mut self, animation: SpinnerAnimation) -> Self {
        self.animation = Some(animation);
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
        let (default_icon, default_animation) = self.variant.defaults();
        let animation = self.animation.unwrap_or(default_animation);
        let easing = self.easing.unwrap_or_else(|| match animation {
            SpinnerAnimation::LinearSpin => Box::new(linear),
            SpinnerAnimation::SemanticSpin => {
                let easing = motion.move_easing;
                Box::new(move |delta| easing.sample(delta))
            }
        });
        let icon = self
            .icon
            .unwrap_or_else(|| Icon::new(default_icon))
            .with_size(self.size)
            .when_some(self.color, |this, color| this.text_color(color));

        div()
            .id(self.id)
            // AccessKit does not expose ARIA's Status role. An indeterminate
            // ProgressIndicator preserves the loading-status semantics.
            .role(Role::ProgressIndicator)
            .aria_label(self.aria_label)
            .child(if cx.reduce_motion() {
                icon.into_any_element()
            } else {
                icon.with_animation(
                    "spin",
                    Animation::new(motion.loading())
                        .repeat()
                        .with_easing(easing),
                    |this, delta| this.transform(Transformation::rotate(percentage(delta))),
                )
                .into_any_element()
            })
            .into_element()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use gpui::{Element as _, Render, TestAppContext};

    use crate::ElementExt as _;

    use super::*;

    #[test]
    fn builder_preserves_aligned_defaults_and_overrides() {
        let spinner = Spinner::new()
            .id(("spinner", 1_u64))
            .aria_label("Loading projects")
            .small()
            .color(gpui::black());

        assert_eq!(spinner.id, ElementId::NamedInteger("spinner".into(), 1));
        assert_eq!(spinner.aria_label, "Loading projects");
        assert_eq!(spinner.size, Size::Small);
        assert_eq!(spinner.color, Some(gpui::black()));
        assert_eq!(spinner.variant, SpinnerVariant::Circular);
        assert_eq!(spinner.animation, None);
        assert!(spinner.icon.is_none());
        let (default_icon, default_animation) = spinner.variant.defaults();
        assert_eq!(default_animation, SpinnerAnimation::LinearSpin);
        assert!(
            Icon::new(default_icon)
                .path_ref()
                .ends_with("loader-circle.svg")
        );
    }

    #[test]
    fn explicit_icon_and_animation_are_independent_from_variant_order() {
        let (classic_icon, classic_animation) = SpinnerVariant::Classic.defaults();
        assert_eq!(classic_animation, SpinnerAnimation::SemanticSpin);
        assert!(Icon::new(classic_icon).path_ref().ends_with("loader.svg"));

        let spinner = Spinner::new()
            .icon(IconName::LoaderCircle)
            .animation(SpinnerAnimation::LinearSpin)
            .variant(SpinnerVariant::Classic);

        assert_eq!(spinner.variant, SpinnerVariant::Classic);
        assert_eq!(spinner.animation, Some(SpinnerAnimation::LinearSpin));
        assert!(
            spinner
                .icon
                .as_ref()
                .unwrap()
                .path_ref()
                .ends_with("loader-circle.svg")
        );
    }

    struct AccessibilityProbe {
        metadata: Arc<Mutex<Option<(Role, Option<String>)>>>,
    }

    impl Render for AccessibilityProbe {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let metadata = self.metadata.clone();
            div().on_prepaint(move |_, window, cx| {
                let spinner = Spinner::new()
                    .id("accessible-spinner")
                    .aria_label("Loading messages")
                    .render(window, cx)
                    .into_element();
                let role = spinner
                    .a11y_role()
                    .expect("spinner must expose a loading status role");
                let mut node = gpui::accesskit::Node::new(role);
                spinner.write_a11y_info(&mut node);
                *metadata.lock().unwrap() = Some((role, node.label().map(ToOwned::to_owned)));
            })
        }
    }

    #[gpui::test]
    fn exposes_accessible_loading_status(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let metadata = Arc::new(Mutex::new(None));
        let captured = metadata.clone();
        let (_, cx) = cx.add_window_view(move |_, _| AccessibilityProbe { metadata });
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        assert_eq!(
            *captured.lock().unwrap(),
            Some((Role::ProgressIndicator, Some("Loading messages".into())))
        );
    }
}
