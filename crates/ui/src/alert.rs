use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt, button::Button, theme::Density,
    v_flex,
};

/// The visual variant of an [`Alert`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AlertVariant {
    #[default]
    Default,
    Destructive,
}

impl AlertVariant {
    /// Returns the inherited foreground used by the icon and title.
    fn foreground(self, cx: &App) -> Hsla {
        match self {
            Self::Default => cx.theme().foreground,
            Self::Destructive => cx.theme().danger,
        }
    }

    /// Returns the foreground used by the description slot.
    fn description_foreground(self, cx: &App) -> Hsla {
        match self {
            Self::Default => cx.theme().muted_foreground,
            Self::Destructive => cx.theme().danger.opacity(0.9),
        }
    }
}

/// Geometry that differs between the compact and regular shadcn Alert styles.
#[derive(Debug, Clone, Copy, PartialEq)]
struct AlertMetrics {
    padding_x: gpui::Pixels,
    padding_y: gpui::Pixels,
    icon_gap: gpui::Pixels,
    action_gap: gpui::Pixels,
    action_offset_x: gpui::Pixels,
    action_offset_y: gpui::Pixels,
}

impl AlertMetrics {
    /// Resolves Alert geometry from the active Style Preset density.
    fn for_density(density: Density) -> Self {
        match density {
            Density::Compact => Self {
                padding_x: px(10.),
                padding_y: px(8.),
                icon_gap: px(8.),
                action_gap: px(8.),
                action_offset_x: px(-2.),
                action_offset_y: px(0.),
            },
            Density::Standard | Density::Comfortable => Self {
                padding_x: px(16.),
                padding_y: px(12.),
                icon_gap: px(10.),
                action_gap: px(10.),
                action_offset_x: px(-4.),
                action_offset_y: px(-2.),
            },
        }
    }
}

/// A visual content slot with optional text retained for accessibility metadata.
enum AlertSlot {
    Text(SharedString),
    Element(AnyElement),
}

impl AlertSlot {
    /// Returns text that can be exposed on the parent Alert node.
    fn accessibility_text(&self) -> Option<SharedString> {
        match self {
            Self::Text(text) => Some(text.clone()),
            Self::Element(_) => None,
        }
    }

    /// Converts the slot into its visual element.
    fn into_element(self) -> AnyElement {
        match self {
            Self::Text(text) => text.into_any_element(),
            Self::Element(element) => element,
        }
    }
}

/// A callout that draws attention to important content or status.
#[derive(IntoElement)]
pub struct Alert {
    id: ElementId,
    style: StyleRefinement,
    variant: AlertVariant,
    icon: Option<Icon>,
    title: Option<AlertSlot>,
    description: Option<AlertSlot>,
    action: Option<AnyElement>,
    aria_label: Option<SharedString>,
    banner: bool,
    on_close: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    visible: bool,
}

impl Alert {
    /// Creates an empty Alert. Add title, description, icon, and action slots as needed.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            variant: AlertVariant::Default,
            icon: None,
            title: None,
            description: None,
            action: None,
            aria_label: None,
            banner: false,
            on_close: None,
            visible: true,
        }
    }

    /// Sets the visual variant without adding an icon or changing the content slots.
    pub fn with_variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Applies the destructive visual variant.
    pub fn destructive(self) -> Self {
        self.with_variant(AlertVariant::Destructive)
    }

    /// Sets the optional leading icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets a text title that is also exposed as the accessible Alert name.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(AlertSlot::Text(title.into()));
        self
    }

    /// Sets a custom title element.
    ///
    /// Custom elements do not expose text automatically. Pair this with
    /// [`Alert::aria_label`] when the Alert has no text title.
    pub fn title_element(mut self, title: impl IntoElement) -> Self {
        self.title = Some(AlertSlot::Element(title.into_any_element()));
        self
    }

    /// Sets text description content and exposes it to assistive technology.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(AlertSlot::Text(description.into()));
        self
    }

    /// Sets a custom description element.
    ///
    /// Custom elements do not expose text automatically. Pair this with
    /// [`Alert::aria_label`] when no text title or description is available.
    pub fn description_element(mut self, description: impl IntoElement) -> Self {
        self.description = Some(AlertSlot::Element(description.into_any_element()));
        self
    }

    /// Sets the optional action displayed in the top-right corner.
    ///
    /// This replaces a previously configured close action.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self.on_close = None;
        self
    }

    /// Sets the accessible name announced for the Alert surface.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Uses the full-width borderless Banner appearance without hiding content slots.
    pub fn banner(mut self) -> Self {
        self.banner = true;
        self
    }

    /// Controls whether the Alert is rendered.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Adds an accessible close button in the action slot.
    ///
    /// This replaces a previously configured custom action. The callback owns the
    /// visibility state and must request a redraw when it changes.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Rc::new(on_close));
        self.action = None;
        self
    }
}

impl Styled for Alert {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Alert {
    /// Builds the concrete root element so state precedence can be tested directly.
    fn render_root(self, cx: &mut App) -> gpui::Stateful<gpui::Div> {
        let Self {
            id,
            style,
            variant,
            icon,
            title,
            description,
            action,
            aria_label,
            banner,
            on_close,
            visible: _,
        } = self;
        let metrics = AlertMetrics::for_density(cx.theme().style.density);
        let foreground = variant.foreground(cx);
        let description_foreground = variant.description_foreground(cx);
        let title_text = title.as_ref().and_then(AlertSlot::accessibility_text);
        let description_text = description.as_ref().and_then(AlertSlot::accessibility_text);
        let accessibility_label = aria_label
            .clone()
            .or_else(|| title_text.clone())
            .or_else(|| description_text.clone());
        let accessibility_description =
            description_text.filter(|_| aria_label.is_some() || title_text.is_some());
        let title = title.map(AlertSlot::into_element);
        let description = description.map(AlertSlot::into_element);
        let trailing = action.or_else(|| {
            on_close.map(|on_close| {
                Button::new("alert-close")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .aria_label(t!("Common.Close"))
                    .on_click(move |event, window, cx| {
                        on_close(event, window, cx);
                    })
                    .into_any_element()
            })
        });

        let content = v_flex()
            .flex_1()
            .min_w_0()
            .gap(px(2.))
            .when_some(title, |this, title| {
                this.child(div().w_full().font_medium().child(title))
            })
            .when_some(description, |this, description| {
                this.child(
                    div()
                        .w_full()
                        .text_color(description_foreground)
                        .child(description),
                )
            });

        v_flex()
            .id(id)
            .role(Role::Alert)
            .when_some(accessibility_label, |this, label| this.aria_label(label))
            .when_some(accessibility_description, |this, description| {
                this.aria_description(description)
            })
            .relative()
            .w_full()
            .text_sm()
            .text_color(foreground)
            .bg(cx.theme().tokens.background)
            .px(metrics.padding_x)
            .py(metrics.padding_y)
            .when(!banner, |this| {
                this.border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().style.radii.lg)
            })
            .refine_style(&style)
            .child(
                div()
                    .flex()
                    .items_start()
                    .min_w_0()
                    .when_some(icon, |this, icon| {
                        this.gap(metrics.icon_gap).child(
                            div()
                                .mt(px(2.))
                                .flex_none()
                                .child(icon.with_size(crate::Size::Medium)),
                        )
                    })
                    .child(content)
                    .when_some(trailing, |this, trailing| {
                        this.child(
                            div()
                                .flex_none()
                                .ml(metrics.action_gap)
                                .mr(metrics.action_offset_x)
                                .mt(metrics.action_offset_y)
                                .child(trailing),
                        )
                    }),
            )
    }
}

impl RenderOnce for Alert {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // Returning Empty prevents hidden alerts from registering an element ID
        // or an AccessKit node before GPUI evaluates display styles.
        if !self.visible {
            return gpui::Empty.into_any_element();
        }

        self.render_root(cx).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_slots_are_optional_and_trailing_configuration_is_last_write_wins() {
        let empty = Alert::new("empty");
        assert!(empty.icon.is_none());
        assert!(empty.title.is_none());
        assert!(empty.description.is_none());
        assert!(empty.action.is_none());
        assert!(empty.on_close.is_none());

        let action_then_close = Alert::new("action-then-close")
            .action(Button::new("action").label("Action"))
            .on_close(|_, _, _| {});
        assert!(action_then_close.action.is_none());
        assert!(action_then_close.on_close.is_some());

        let close_then_action = Alert::new("close-then-action")
            .on_close(|_, _, _| {})
            .action(Button::new("action").label("Action"));
        assert!(close_then_action.action.is_some());
        assert!(close_then_action.on_close.is_none());
    }

    #[test]
    fn alert_metrics_match_shadcn_presets() {
        let compact = AlertMetrics::for_density(Density::Compact);
        assert_eq!(compact.padding_x, px(10.));
        assert_eq!(compact.padding_y, px(8.));
        assert_eq!(compact.icon_gap, px(8.));
        assert_eq!(compact.action_gap, px(8.));
        assert_eq!(compact.action_offset_x, px(-2.));
        assert_eq!(compact.action_offset_y, px(0.));

        for density in [Density::Standard, Density::Comfortable] {
            let regular = AlertMetrics::for_density(density);
            assert_eq!(regular.padding_x, px(16.));
            assert_eq!(regular.padding_y, px(12.));
            assert_eq!(regular.icon_gap, px(10.));
            assert_eq!(regular.action_gap, px(10.));
            assert_eq!(regular.action_offset_x, px(-4.));
            assert_eq!(regular.action_offset_y, px(-2.));
        }
    }

    #[gpui::test]
    fn hidden_alert_has_no_element_identity(cx: &mut gpui::TestAppContext) {
        use gpui::Element as _;

        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let alert = Alert::new("hidden-alert")
                .description("Hidden content")
                .visible(false)
                .flex()
                .render(window, cx)
                .into_element();
            assert!(alert.id().is_none());
        });
    }

    #[gpui::test]
    fn alert_variants_keep_neutral_surface_and_semantic_text(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|_, cx| {
            assert_eq!(AlertVariant::Default.foreground(cx), cx.theme().foreground);
            assert_eq!(
                AlertVariant::Default.description_foreground(cx),
                cx.theme().muted_foreground
            );
            assert_eq!(AlertVariant::Destructive.foreground(cx), cx.theme().danger);
            assert_eq!(
                AlertVariant::Destructive.description_foreground(cx),
                cx.theme().danger.opacity(0.9)
            );
        });
    }

    #[gpui::test]
    fn alert_derives_accessibility_metadata_from_text_slots(cx: &mut gpui::TestAppContext) {
        use crate::ElementExt as _;
        use gpui::{Element as _, Render};
        use std::sync::{Arc, Mutex};

        type AccessibilityMetadata = (Option<String>, Option<String>);

        struct AlertA11yProbe {
            metadata: Arc<Mutex<Vec<AccessibilityMetadata>>>,
        }

        impl Render for AlertA11yProbe {
            fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
                let metadata = self.metadata.clone();
                div().on_prepaint(move |_, _window, cx| {
                    let mut metadata_for = |alert: Alert| {
                        let mut node = gpui::accesskit::Node::new(Role::Alert);
                        alert.render_root(cx).write_a11y_info(&mut node);
                        (
                            node.label().map(ToOwned::to_owned),
                            node.description().map(ToOwned::to_owned),
                        )
                    };

                    *metadata.lock().unwrap() = vec![
                        metadata_for(
                            Alert::new("title-and-description")
                                .title("Synchronization failed")
                                .description("Try again later."),
                        ),
                        metadata_for(
                            Alert::new("description-only").description("Connection restored."),
                        ),
                        metadata_for(
                            Alert::new("explicit-name")
                                .title("Visible title")
                                .description("Visible description")
                                .aria_label("Explicit accessible name"),
                        ),
                    ];
                })
            }
        }

        cx.update(crate::init);
        let metadata = Arc::new(Mutex::new(Vec::new()));
        let captured = metadata.clone();
        let (_, cx) = cx.add_window_view(move |_, _| AlertA11yProbe { metadata });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        assert_eq!(
            *captured.lock().unwrap(),
            vec![
                (
                    Some("Synchronization failed".into()),
                    Some("Try again later.".into())
                ),
                (Some("Connection restored.".into()), None),
                (
                    Some("Explicit accessible name".into()),
                    Some("Visible description".into())
                ),
            ]
        );
    }

    #[gpui::test]
    fn wide_action_participates_in_layout_without_overlapping_content(
        cx: &mut gpui::TestAppContext,
    ) {
        use crate::ElementExt as _;
        use gpui::{Bounds, Pixels, Render};
        use std::sync::{Arc, Mutex};

        type CapturedBounds = Arc<Mutex<(Option<Bounds<Pixels>>, Option<Bounds<Pixels>>)>>;

        struct AlertLayoutProbe {
            bounds: CapturedBounds,
        }

        impl Render for AlertLayoutProbe {
            fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
                let content_bounds = self.bounds.clone();
                let action_bounds = self.bounds.clone();

                Alert::new("wide-action-alert")
                    .w(px(320.))
                    .aria_label("Update available")
                    .description_element(
                        div()
                            .w_full()
                            .on_prepaint(move |bounds, _, _| {
                                content_bounds.lock().unwrap().0 = Some(bounds);
                            })
                            .child("Install the update when convenient."),
                    )
                    .action(
                        div()
                            .w(px(140.))
                            .h(px(24.))
                            .on_prepaint(move |bounds, _, _| {
                                action_bounds.lock().unwrap().1 = Some(bounds);
                            })
                            .child("Install and restart"),
                    )
            }
        }

        cx.update(crate::init);
        let bounds: CapturedBounds = Arc::new(Mutex::new((None, None)));
        let captured = bounds.clone();
        let (_, cx) = cx.add_window_view(move |_, _| AlertLayoutProbe { bounds });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let (content, action) = *captured.lock().unwrap();
        let content = content.expect("content should be laid out");
        let action = action.expect("action should be laid out");
        assert!(content.right() <= action.left());
    }
}
