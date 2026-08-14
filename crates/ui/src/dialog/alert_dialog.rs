// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public types: `AlertDialogSize`, `AlertDialogAction`, `AlertDialogCancel`,
//   `AlertDialogContent`.
// - Added public methods: `variant`, `disabled`, `size`, `media`, `media_element`, `title_element`,
//   `description_element`, `aria_label` and 5 more.
// - Removed public methods: `confirm`, `footer`, `icon`, `button_props`, `width`, `show_cancel`,
//   `overlay_closable`, `close_button` and 2 more.
// - Removed or replaced `confirm`, `footer`, `debug_assert_no_trigger`, `icon`, `button_props`,
//   `width`, `show_cancel`, `overlay_closable` and 3 more.
// - Reworked Alert Dialog around accessibility semantics and ARIA state, semantic Style Preset
//   geometry and density, keyboard navigation and activation behavior, focus-visible and focus
//   restoration behavior.
use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, FocusHandle, IntoElement, ParentElement as _,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use crate::{
    ActiveTheme as _, Disableable as _, Icon, Root, Sizable as _, Size, StyledExt as _,
    button::{Button, ButtonVariant, ButtonVariants as _},
    dialog::{ConfirmDialog, Dialog, DialogCallbacks},
    h_flex, v_flex,
};

/// Semantic content widths supported by an alert dialog.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AlertDialogSize {
    #[default]
    Default,
    Small,
}

enum AlertDialogMedia {
    Icon(Box<Icon>),
    Element(AnyElement),
}

/// The affirmative action rendered in an alert dialog footer.
pub struct AlertDialogAction {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    disabled: bool,
}

impl AlertDialogAction {
    /// Creates an alert action with a stable element identity and visible label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Default,
            disabled: false,
        }
    }

    /// Sets the Button visual variant used for the action.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets whether the action is unavailable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn render(self, full_width: bool, focus_handle: Option<FocusHandle>) -> AnyElement {
        let button = Button::new(self.id)
            .label(self.label)
            .with_variant(self.variant)
            .disabled(self.disabled)
            .when_some(focus_handle, |this, focus_handle| {
                this.focus_handle(focus_handle)
            })
            .when(full_width, |this| this.w_full())
            .on_click(move |_, window, cx| window.dispatch_action(Box::new(ConfirmDialog), cx))
            .into_any_element();

        button
    }
}

/// The cancellation action rendered in an alert dialog footer.
pub struct AlertDialogCancel {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    disabled: bool,
}

impl AlertDialogCancel {
    /// Creates a cancel action with a stable element identity and visible label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Outline,
            disabled: false,
        }
    }

    /// Sets the Button visual variant used for cancellation.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets whether cancellation is unavailable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn render(self, full_width: bool, focus_handle: Option<FocusHandle>) -> AnyElement {
        let button = Button::new(self.id)
            .label(self.label)
            .with_variant(self.variant)
            .disabled(self.disabled)
            .when_some(focus_handle, |this, focus_handle| {
                this.focus_handle(focus_handle)
            })
            .when(full_width, |this| this.w_full())
            .on_click(move |_, window, cx| {
                window.dispatch_action(Box::new(super::CancelDialog), cx)
            })
            .into_any_element();

        button
    }
}

/// Semantic slots rendered inside an [`AlertDialog`].
pub struct AlertDialogContent {
    size: AlertDialogSize,
    media: Option<AlertDialogMedia>,
    title: Option<AnyElement>,
    title_text: Option<SharedString>,
    description: Option<AnyElement>,
    description_text: Option<SharedString>,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
    action: Option<AlertDialogAction>,
    cancel: Option<AlertDialogCancel>,
    children: Vec<AnyElement>,
}

impl AlertDialogContent {
    /// Creates empty semantic alert dialog content.
    pub fn new() -> Self {
        Self {
            size: AlertDialogSize::Default,
            media: None,
            title: None,
            title_text: None,
            description: None,
            description_text: None,
            aria_label: None,
            aria_description: None,
            action: None,
            cancel: None,
            children: Vec::new(),
        }
    }

    /// Sets the semantic content width and layout.
    pub fn size(mut self, size: AlertDialogSize) -> Self {
        self.size = size;
        self
    }

    /// Adds an icon to the header using the active modal metrics.
    pub fn media(mut self, media: impl Into<Icon>) -> Self {
        self.media = Some(AlertDialogMedia::Icon(Box::new(media.into())));
        self
    }

    /// Adds custom visual media. The caller owns its internal geometry.
    pub fn media_element(mut self, media: impl IntoElement) -> Self {
        self.media = Some(AlertDialogMedia::Element(media.into_any_element()));
        self
    }

    /// Sets a text title and uses it as the default accessible name.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        let title = title.into();
        self.title = Some(title.clone().into_any_element());
        self.title_text = Some(title);
        self
    }

    /// Sets a custom title element. Pair this with [`Self::aria_label`].
    pub fn title_element(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self.title_text = None;
        self
    }

    /// Sets text description and exposes it to assistive technology.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        let description = description.into();
        self.description = Some(description.clone().into_any_element());
        self.description_text = Some(description);
        self
    }

    /// Sets a custom description element. Pair this with [`Self::aria_description`].
    pub fn description_element(mut self, description: impl IntoElement) -> Self {
        self.description = Some(description.into_any_element());
        self.description_text = None;
        self
    }

    /// Overrides the accessible name derived from a text title.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Overrides the accessible description derived from text content.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Sets the affirmative action.
    pub fn action(mut self, action: AlertDialogAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Sets the cancellation action.
    pub fn cancel(mut self, cancel: AlertDialogCancel) -> Self {
        self.cancel = Some(cancel);
        self
    }

    fn into_dialog(
        self,
        mut dialog: Dialog,
        callbacks: DialogCallbacks,
        window: &mut Window,
        cx: &mut App,
    ) -> Dialog {
        let metrics = cx.theme().style.modals;
        let compact =
            self.size == AlertDialogSize::Small || window.viewport_size().width < gpui::px(640.);
        let width = match self.size {
            AlertDialogSize::Default => metrics.default_width,
            AlertDialogSize::Small => metrics.small_width,
        };
        let full_width_actions = self.size == AlertDialogSize::Small;
        // Disabled buttons do not participate in GPUI focus tracking, so only
        // create candidates that can safely receive initial modal focus.
        let cancel_focus = self
            .cancel
            .as_ref()
            .filter(|cancel| !cancel.disabled)
            .map(|cancel| {
                window
                    .use_keyed_state(cancel.id.clone(), cx, |_, cx| cx.focus_handle())
                    .read(cx)
                    .clone()
            });
        let action_focus = self
            .action
            .as_ref()
            .filter(|action| !action.disabled)
            .map(|action| {
                window
                    .use_keyed_state(action.id.clone(), cx, |_, cx| cx.focus_handle())
                    .read(cx)
                    .clone()
            });
        let initial_focus = cancel_focus.clone().or_else(|| action_focus.clone());

        let title = self.title;
        let description = self.description;
        let media = self.media.map(|media| match media {
            AlertDialogMedia::Icon(icon) => icon
                .with_size(Size::Size(metrics.media_icon_size))
                .into_any_element(),
            AlertDialogMedia::Element(element) => element,
        });
        let children = self.children;

        // Give text slots a definite width so GPUI measures long content against
        // the padded dialog column instead of its unconstrained intrinsic width.
        let text = v_flex()
            .w_full()
            .min_w_0()
            .gap(metrics.header_gap)
            .when(compact, |this| this.items_center().text_center())
            .when_some(title, |this, title| {
                this.child(
                    div()
                        .w_full()
                        .min_w_0()
                        .whitespace_normal()
                        .text_size(metrics.title_font_size)
                        .font_medium()
                        .child(title),
                )
            })
            .when_some(description, |this, description| {
                this.child(
                    div()
                        .w_full()
                        .min_w_0()
                        .whitespace_normal()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(description),
                )
            });

        let header = if let Some(media) = media {
            let media = div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .size(metrics.media_size)
                .bg(cx.theme().muted)
                .rounded(if metrics.media_round {
                    metrics.media_size / 2.
                } else {
                    cx.theme().style.radii.md
                })
                .when(compact, |this| this.mb_2())
                .child(media);

            if compact {
                v_flex()
                    .w_full()
                    .min_w_0()
                    .items_center()
                    .gap(metrics.header_gap)
                    .child(media)
                    .child(text)
                    .into_any_element()
            } else {
                h_flex()
                    .w_full()
                    .min_w_0()
                    .items_start()
                    .gap(metrics.gap)
                    .child(media)
                    .child(text.flex_1())
                    .into_any_element()
            }
        } else {
            text.into_any_element()
        };

        let footer = if full_width_actions {
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .when_some(self.cancel, |this, cancel| {
                    this.child(cancel.render(true, cancel_focus.clone()))
                })
                .when_some(self.action, |this, action| {
                    this.child(action.render(true, action_focus.clone()))
                })
        } else {
            h_flex()
                .justify_end()
                .gap_2()
                .when_some(self.cancel, |this, cancel| {
                    this.child(cancel.render(false, cancel_focus.clone()))
                })
                .when_some(self.action, |this, action| {
                    this.child(action.render(false, action_focus.clone()))
                })
        }
        .when(metrics.footer_separated, |this| {
            this.ml(-metrics.padding)
                .mr(-metrics.padding)
                .mb(-metrics.padding)
                .p(metrics.footer_padding)
                .border_t_1()
                .border_color(cx.theme().border)
        })
        .when(metrics.footer_tinted, |this| {
            this.bg(cx.theme().muted.opacity(0.5))
                .rounded_b(cx.theme().style.radii.xl)
        });

        dialog = dialog
            .alert_dialog_role()
            .w(width)
            .show_close_button(false)
            .dismiss_on_overlay_click(false)
            .callbacks(callbacks)
            .when_some(initial_focus, |this, focus_handle| {
                this.initial_focus(focus_handle)
            })
            .header(header)
            .footer_element(footer)
            .children(children);

        if let Some(label) = self.aria_label.or(self.title_text) {
            dialog = dialog.aria_label(label);
        }
        if let Some(description) = self.aria_description.or(self.description_text) {
            dialog = dialog.aria_description(description);
        }

        dialog
    }
}

impl Default for AlertDialogContent {
    fn default() -> Self {
        Self::new()
    }
}

impl gpui::ParentElement for AlertDialogContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

type ContentBuilder =
    Rc<dyn Fn(AlertDialogContent, &mut Window, &mut App) -> AlertDialogContent + 'static>;
type TriggerHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type TriggerBuilder = Box<dyn FnOnce(TriggerHandler) -> AnyElement + 'static>;

/// A modal confirmation surface that requires an explicit response.
#[derive(IntoElement)]
pub struct AlertDialog {
    base: Dialog,
    trigger: Option<TriggerBuilder>,
    content_builder: Option<ContentBuilder>,
    callbacks: DialogCallbacks,
}

impl AlertDialog {
    /// Creates an alert dialog with non-dismissible overlay and no close button.
    pub fn new(cx: &mut App) -> Self {
        Self {
            base: Dialog::new(cx)
                .dismiss_on_overlay_click(false)
                .show_close_button(false)
                .confirm_on_enter(false),
            trigger: None,
            content_builder: None,
            callbacks: DialogCallbacks::default(),
        }
    }

    /// Sets the interactive element that opens this dialog.
    ///
    /// The open handler is attached to the trigger itself so pointer clicks and
    /// Enter/Space activation follow the same GPUI interaction path.
    pub fn trigger(mut self, trigger: Button) -> Self {
        self.trigger = Some(Box::new(move |handler| {
            trigger
                .append_on_click(move |event, window, cx| handler(event, window, cx))
                .into_any_element()
        }));
        self
    }

    /// Defines the semantic content rendered each time the modal is displayed.
    pub fn content<F>(mut self, builder: F) -> Self
    where
        F: Fn(AlertDialogContent, &mut Window, &mut App) -> AlertDialogContent + 'static,
    {
        self.content_builder = Some(Rc::new(builder));
        self
    }

    /// Sets whether Escape may cancel the dialog. The default is `true`.
    pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
        self.base = self.base.dismiss_on_escape(dismiss);
        self
    }

    /// Runs the affirmative action and closes when it returns `true`.
    pub fn on_action(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_ok(handler);
        self
    }

    /// Runs cancellation and closes when it returns `true`.
    pub fn on_cancel(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_cancel(handler);
        self
    }

    /// Runs once after the dialog has accepted a close operation.
    pub fn on_close(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.callbacks.on_close = Rc::new(handler);
        self
    }

    /// Converts the semantic alert content into the shared modal renderer.
    pub(crate) fn into_dialog(self, window: &mut Window, cx: &mut App) -> Dialog {
        let content = self
            .content_builder
            .map(|builder| builder(AlertDialogContent::new(), window, cx))
            .unwrap_or_default();

        content.into_dialog(self.base, self.callbacks, window, cx)
    }

    fn render_trigger(self, trigger: TriggerBuilder) -> AnyElement {
        let style = self.base.style.clone();
        let props = self.base.props.clone();
        let content_builder = self.content_builder.clone();
        let callbacks = self.callbacks.clone();

        let open: TriggerHandler = Rc::new(move |_, window, cx| {
            let style = style.clone();
            let props = props.clone();
            let content_builder = content_builder.clone();
            let callbacks = callbacks.clone();
            Root::update(window, cx, move |root, window, cx| {
                let style = style.clone();
                let props = props.clone();
                let content_builder = content_builder.clone();
                let callbacks = callbacks.clone();
                root.open_dialog_with_presentation(
                    super::DialogPresentation::Alert,
                    move |dialog, window, cx| {
                        let content = content_builder
                            .as_ref()
                            .map(|builder| builder(AlertDialogContent::new(), window, cx))
                            .unwrap_or_default();
                        content.into_dialog(
                            dialog.refine_style(&style).with_props(props.clone()),
                            callbacks.clone(),
                            window,
                            cx,
                        )
                    },
                    window,
                    cx,
                );
            });
            cx.stop_propagation();
        });

        trigger(open)
    }
}

impl Styled for AlertDialog {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.base.style
    }
}

impl RenderOnce for AlertDialog {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(trigger) = self.trigger.take() {
            self.render_trigger(trigger)
        } else {
            self.into_dialog(window, cx).into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ElementExt as _, WindowExt as _};
    use gpui::{
        AppContext as _, Bounds, Context, InteractiveElement as _, KeyDownEvent, KeyUpEvent,
        Keystroke, Pixels, Render, TestAppContext, VisualTestContext, px,
    };
    use std::{
        cell::RefCell,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    struct TriggerFixture {
        clicks: Arc<AtomicUsize>,
    }

    struct BackgroundFixture {
        focus_handle: FocusHandle,
    }

    struct DescriptionLayoutFixture {
        bounds: Arc<Mutex<Option<Bounds<Pixels>>>>,
    }

    impl Render for TriggerFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let clicks = self.clicks.clone();
            let dialog_layer = crate::Root::render_dialog_layer(window, cx);
            div()
                .child(
                    AlertDialog::new(cx)
                        .trigger(
                            Button::new("keyboard-alert-trigger")
                                .label("Open alert")
                                .on_click(move |_, _, _| {
                                    clicks.fetch_add(1, Ordering::SeqCst);
                                }),
                        )
                        .content(|content, _, _| {
                            content
                                .title("Confirm action")
                                .cancel(AlertDialogCancel::new("keyboard-cancel", "Cancel"))
                                .action(AlertDialogAction::new("keyboard-action", "Continue"))
                        }),
                )
                .children(dialog_layer)
        }
    }

    impl Render for BackgroundFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let dialog_layer = crate::Root::render_dialog_layer(window, cx);
            div()
                .track_focus(&self.focus_handle)
                .child("Background")
                .children(dialog_layer)
        }
    }

    impl Render for DescriptionLayoutFixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.bounds.clone();

            AlertDialog::new(cx).content(move |content, _, _| {
                let bounds = bounds.clone();
                content
                    .size(AlertDialogSize::Small)
                    .title("Delete chat?")
                    .description_element(
                        div()
                            .on_prepaint(move |description_bounds, _, _| {
                                *bounds.lock().unwrap() = Some(description_bounds);
                            })
                            .child("This will permanently delete this chat conversation. Review Settings before continuing."),
                    )
                    .aria_description("This will permanently delete this chat conversation.")
                    .cancel(AlertDialogCancel::new("layout-cancel", "Cancel"))
                    .action(AlertDialogAction::new("layout-action", "Delete"))
            })
        }
    }

    #[test]
    fn text_slots_supply_accessibility_fallbacks() {
        let content = AlertDialogContent::new()
            .title("Delete chat?")
            .description("This action cannot be undone.")
            .size(AlertDialogSize::Small);

        assert_eq!(content.title_text.as_deref(), Some("Delete chat?"));
        assert_eq!(
            content.description_text.as_deref(),
            Some("This action cannot be undone.")
        );
        assert_eq!(content.size, AlertDialogSize::Small);
    }

    #[test]
    fn alert_actions_use_safe_default_variants() {
        let action = AlertDialogAction::new("action", "Continue");
        let cancel = AlertDialogCancel::new("cancel", "Cancel");

        assert_eq!(action.variant, ButtonVariant::Default);
        assert_eq!(cancel.variant, ButtonVariant::Outline);
        assert!(!action.disabled);
        assert!(!cancel.disabled);
    }

    #[gpui::test]
    fn trigger_opens_alert_dialog_from_keyboard(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let clicks = Arc::new(AtomicUsize::new(0));
        let captured = clicks.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| TriggerFixture { clicks });
            crate::Root::new(fixture, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            window.focus_next(cx);
            let _ = window.draw(cx);
        });
        let enter = Keystroke::parse("enter").expect("enter must be a valid keystroke");
        cx.simulate_event(KeyDownEvent {
            keystroke: enter.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke: enter });
        cx.run_until_parked();

        assert!(cx.update(|window, cx| crate::WindowExt::has_active_dialog(window, cx)));
        assert_eq!(captured.load(Ordering::SeqCst), 1);
    }

    #[gpui::test]
    fn disabled_cancel_is_skipped_by_initial_focus(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let background_focus = Rc::new(RefCell::new(None));
        let captured = background_focus.clone();
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let focus_handle = cx.focus_handle();
            *background_focus.borrow_mut() = Some(focus_handle.clone());
            let fixture = cx.new(|_| BackgroundFixture { focus_handle });
            crate::Root::new(fixture, window, cx)
        });
        let fixture = root.read_with(cx, |root, _| {
            root.view()
                .clone()
                .downcast::<BackgroundFixture>()
                .expect("background fixture should be mounted")
        });

        cx.update(|window, cx| {
            window.open_alert_dialog(cx, |dialog, _, _| {
                dialog.content(|content, _, _| {
                    content
                        .title("Confirm action")
                        .cancel(AlertDialogCancel::new("disabled-cancel", "Cancel").disabled(true))
                        .action(AlertDialogAction::new("enabled-action", "Continue"))
                })
            });
            fixture.update(cx, |_, cx| cx.notify());
            let _ = window.draw(cx);
            let _ = window.draw(cx);
            window.focus_next(cx);
            let _ = window.draw(cx);
        });

        let background_focus = captured
            .borrow()
            .clone()
            .expect("background focus handle should exist");
        assert!(!cx.update(|window, _| background_focus.is_focused(window)));
    }

    #[gpui::test]
    fn description_wraps_inside_small_dialog_padding(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let bounds = Arc::new(Mutex::new(None));
        let captured = bounds.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| DescriptionLayoutFixture { bounds });
            crate::Root::new(fixture, window, cx)
        });

        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let description = captured
            .lock()
            .unwrap()
            .expect("description should be laid out");
        assert!(description.size.width <= px(272.));
        assert!(description.size.height > px(20.));
    }
}
