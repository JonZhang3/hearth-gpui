use std::rc::Rc;

use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Bounds, BoxShadow, ClickEvent, Edges, ElementId,
    FocusHandle, InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement, Pixels,
    Point, RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, WindowControlArea, actions, anchored, div, hsla, point, prelude::FluentBuilder,
    px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, FocusTrapElement as _, IconName, Root, Sizable as _, StyledExt,
    TITLE_BAR_HEIGHT, WindowExt as _,
    animation::OverlayPhase,
    button::Button,
    dialog::{DialogContent, DialogTitle, modal_overlay},
    scroll::ScrollableElement as _,
    text::{SelectionScope, SelectionScopeElement as _},
    theme::Density,
    v_flex,
};

const CONTEXT: &str = "Dialog";
const ESCAPE_CONTEXT: &str = "DialogEscape";
const CONFIRM_CONTEXT: &str = "DialogConfirm";
const ALERT_CONTEXT: &str = "AlertDialog";
const DEFAULT_CONFIRM_CONTEXT: &str = "Dialog && !Button";
const CONFIRM_ONLY_DEFAULT_CONTEXT: &str = "DialogConfirm && !Button";

actions!(dialog, [CancelDialog, ConfirmDialog]);

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", CancelDialog, Some(CONTEXT)),
        KeyBinding::new("enter", ConfirmDialog, Some(DEFAULT_CONFIRM_CONTEXT)),
        KeyBinding::new("escape", CancelDialog, Some(ESCAPE_CONTEXT)),
        KeyBinding::new("enter", ConfirmDialog, Some(CONFIRM_ONLY_DEFAULT_CONTEXT)),
        KeyBinding::new("escape", CancelDialog, Some(ALERT_CONTEXT)),
    ]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DialogPresentation {
    Standard,
    Alert,
}

/// Internal callbacks shared by Dialog and AlertDialog.
#[derive(Clone)]
pub(crate) struct DialogCallbacks {
    pub(crate) on_ok: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static>,
    pub(crate) on_cancel: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static>,
    pub(crate) on_close: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
}

impl Default for DialogCallbacks {
    fn default() -> Self {
        Self {
            on_ok: Rc::new(|_, _, _| true),
            on_cancel: Rc::new(|_, _, _| true),
            on_close: Rc::new(|_, _, _| {}),
        }
    }
}

impl DialogCallbacks {
    /// Sets the callback for when the dialog is has been confirmed.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_ok(
        mut self,
        on_ok: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_ok = Rc::new(on_ok);
        self
    }

    /// Sets the callback for when the dialog is has been canceled.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_cancel = Rc::new(on_cancel);
        self
    }
}

type ContentBuilderFn = Rc<dyn Fn(DialogContent, &mut Window, &mut App) -> DialogContent + 'static>;
type SlotBuilderFn = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
struct StandardDialogMetrics {
    default_width: Pixels,
    close_inset: Pixels,
    small_title: bool,
}

impl StandardDialogMetrics {
    /// Resolves Dialog-only geometry from semantic density without extending StylePreset.
    fn for_density(density: Density) -> Self {
        match density {
            Density::Compact => Self {
                default_width: px(384.),
                close_inset: px(8.),
                small_title: false,
            },
            Density::Standard => Self {
                default_width: px(448.),
                close_inset: px(16.),
                small_title: true,
            },
            Density::Comfortable => Self {
                default_width: px(448.),
                close_inset: px(16.),
                small_title: false,
            },
        }
    }
}

#[derive(Clone)]
pub(crate) struct DialogProps {
    width: Option<Pixels>,
    max_width: Option<Pixels>,
    margin_top: Option<Pixels>,
    show_close_button: bool,

    show_overlay: bool,
    dismiss_on_overlay_click: bool,
    pub(crate) overlay_visible: bool,
    dismiss_on_escape: bool,
    confirm_on_enter: bool,
}

impl Default for DialogProps {
    fn default() -> Self {
        Self {
            margin_top: None,
            width: None,
            max_width: None,
            show_overlay: true,
            dismiss_on_escape: true,
            confirm_on_enter: true,
            overlay_visible: false,
            show_close_button: true,
            dismiss_on_overlay_click: true,
        }
    }
}

/// A modal to display content in a dialog box.
#[derive(IntoElement)]
pub struct Dialog {
    pub(crate) style: StyleRefinement,
    children: Vec<AnyElement>,
    trigger: Option<Button>,
    title: Option<SlotBuilderFn>,
    description: Option<SlotBuilderFn>,
    pub(crate) header: Option<AnyElement>,
    pub(crate) footer: Option<AnyElement>,
    footer_builder: Option<SlotBuilderFn>,
    pub(crate) content_builder: Option<ContentBuilderFn>,
    pub(crate) props: DialogProps,
    pub(crate) a11y_role: Role,
    pub(crate) a11y_label: Option<SharedString>,
    pub(crate) a11y_description: Option<SharedString>,
    pub(crate) presentation: DialogPresentation,

    callbacks: DialogCallbacks,

    /// Focus handle owned by Root while this modal remains active.
    pub(crate) focus_handle: FocusHandle,
    pub(crate) layer_ix: usize,
    pub(crate) lifecycle_phase: OverlayPhase,
    initial_focus: Option<FocusHandle>,
}

impl Dialog {
    /// Creates a Dialog with standard dismissal and confirmation behavior.
    pub fn new(cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            style: StyleRefinement::default(),
            trigger: None,
            title: None,
            description: None,
            header: None,
            footer: None,
            footer_builder: None,
            content_builder: None,
            props: DialogProps::default(),
            children: Vec::new(),
            layer_ix: 0,
            lifecycle_phase: OverlayPhase::Open,
            initial_focus: None,
            callbacks: DialogCallbacks::default(),
            a11y_role: Role::Dialog,
            a11y_label: None,
            a11y_description: None,
            presentation: DialogPresentation::Standard,
        }
    }

    /// Sets the Button that opens this Dialog without replacing its click handler.
    pub fn trigger(mut self, trigger: Button) -> Self {
        self.trigger = Some(trigger);
        self
    }

    /// Sets the content of the dialog.
    pub fn content<F>(mut self, builder: F) -> Self
    where
        F: Fn(DialogContent, &mut Window, &mut App) -> DialogContent + 'static,
    {
        self.content_builder = Some(Rc::new(builder));
        self
    }

    /// Sets the title of the dialog.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        let title = title.into();
        self.a11y_label = Some(title.clone());
        self.title = Some(Rc::new(move |_, _| title.clone().into_any_element()));
        self
    }

    /// Sets a custom title renderer. Pair it with [`Self::aria_label`].
    pub fn title_element<E>(mut self, title: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self
    where
        E: IntoElement,
    {
        self.title = Some(Rc::new(move |window, cx| {
            title(window, cx).into_any_element()
        }));
        self
    }

    /// Sets text description and exposes it to assistive technology.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        let description = description.into();
        self.a11y_description = Some(description.clone());
        self.description = Some(Rc::new(move |_, _| description.clone().into_any_element()));
        self
    }

    /// Sets a custom description renderer. Pair it with [`Self::aria_description`].
    pub fn description_element<E>(
        mut self,
        description: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        self.description = Some(Rc::new(move |window, cx| {
            description(window, cx).into_any_element()
        }));
        self
    }

    /// Sets the accessible name announced for the dialog surface.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.a11y_label = Some(label.into());
        self
    }

    /// Sets the accessible description announced for the dialog surface.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.a11y_description = Some(description.into());
        self
    }

    /// Sets the internal header used by AlertDialog's semantic composition.
    pub(crate) fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    /// Sets a repeatable footer renderer, typically containing Dialog actions.
    pub fn footer<E>(mut self, footer: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self
    where
        E: IntoElement,
    {
        self.footer_builder = Some(Rc::new(move |window, cx| {
            footer(window, cx).into_any_element()
        }));
        self
    }

    /// Sets the already-built footer used by AlertDialog's shared renderer.
    pub(crate) fn footer_element(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Sets the internal callback bundle shared with AlertDialog.
    pub(crate) fn callbacks(mut self, callbacks: DialogCallbacks) -> Self {
        self.callbacks = callbacks;
        self
    }

    pub(crate) fn alert_dialog_role(mut self) -> Self {
        self.a11y_role = Role::AlertDialog;
        self.presentation = DialogPresentation::Alert;
        self
    }

    /// Sets the control that receives focus after the modal surface mounts.
    pub fn initial_focus(mut self, focus_handle: FocusHandle) -> Self {
        self.initial_focus = Some(focus_handle);
        self
    }

    /// Runs after confirmation or cancellation accepts a close request.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.callbacks.on_close = Rc::new(on_close);
        self
    }

    /// Handles confirmation and returns whether the Dialog may close.
    pub fn on_ok(
        mut self,
        on_ok: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_ok(on_ok);
        self
    }

    /// Handles cancellation and returns whether the Dialog may close.
    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_cancel(on_cancel);
        self
    }

    /// Sets whether the top-right close control is visible.
    pub fn show_close_button(mut self, show: bool) -> Self {
        self.props.show_close_button = show;
        self
    }

    /// Sets the top offset; the default is one tenth of the viewport height.
    pub fn margin_top(mut self, margin_top: impl Into<Pixels>) -> Self {
        self.props.margin_top = Some(margin_top.into());
        self
    }

    /// Sets the preferred width before applying the viewport safety inset.
    pub fn w(mut self, width: impl Into<Pixels>) -> Self {
        self.props.width = Some(width.into());
        self
    }

    /// Sets the preferred maximum width before applying the viewport safety inset.
    pub fn max_w(mut self, max_width: impl Into<Pixels>) -> Self {
        self.props.max_width = Some(max_width.into());
        self
    }

    /// Sets whether the modal backdrop is painted.
    pub fn show_overlay(mut self, show: bool) -> Self {
        self.props.show_overlay = show;
        self
    }

    /// Sets whether a primary click on the backdrop requests cancellation.
    pub fn dismiss_on_overlay_click(mut self, dismiss: bool) -> Self {
        self.props.dismiss_on_overlay_click = dismiss;
        self
    }

    /// Sets whether Escape may cancel the dialog.
    pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
        self.props.dismiss_on_escape = dismiss;
        self
    }

    /// Sets whether Enter dispatches the standard Dialog confirmation action.
    pub fn confirm_on_enter(mut self, confirm: bool) -> Self {
        self.props.confirm_on_enter = confirm;
        self
    }

    pub(crate) fn has_overlay(&self) -> bool {
        self.props.show_overlay
    }

    pub(crate) fn with_props(mut self, props: DialogProps) -> Self {
        self.props = props;
        self
    }

    fn defer_close_dialog(window: &mut Window, cx: &mut App) {
        Root::update(window, cx, |root, window, cx| {
            root.defer_close_dialog(window, cx);
        });
    }
}

impl ParentElement for Dialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Dialog {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl Dialog {
    fn render_trigger(self, trigger: Button, _: &mut Window, _: &mut App) -> AnyElement {
        let content_builder = self.content_builder.clone();
        let style = self.style.clone();
        let props = self.props.clone();
        let callbacks = self.callbacks.clone();
        let title = self.title.clone();
        let description = self.description.clone();
        let footer_builder = self.footer_builder.clone();
        let initial_focus = self.initial_focus.clone();
        let a11y_role = self.a11y_role;
        let a11y_label = self.a11y_label.clone();
        let a11y_description = self.a11y_description.clone();

        trigger
            .append_on_click(move |_, window, cx| {
                let content_builder = content_builder.clone();
                let style = style.clone();
                let props = props.clone();
                let callbacks = callbacks.clone();
                let title = title.clone();
                let description = description.clone();
                let footer_builder = footer_builder.clone();
                let initial_focus = initial_focus.clone();
                let a11y_label = a11y_label.clone();
                let a11y_description = a11y_description.clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    let mut dialog = dialog
                        .refine_style(&style)
                        .callbacks(callbacks.clone())
                        .with_props(props.clone())
                        .content({
                            let content_builder = content_builder.clone();
                            move |content, window, cx| {
                                if let Some(builder) = content_builder.clone() {
                                    builder(content, window, cx)
                                } else {
                                    content
                                }
                            }
                        });
                    dialog.title = title.clone();
                    dialog.description = description.clone();
                    dialog.footer_builder = footer_builder.clone();
                    dialog.initial_focus = initial_focus.clone();
                    dialog.a11y_role = a11y_role;
                    dialog.a11y_label = a11y_label.clone();
                    dialog.a11y_description = a11y_description.clone();
                    dialog
                });
                cx.stop_propagation();
            })
            .into_any_element()
    }
}

impl RenderOnce for Dialog {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(trigger) = self.trigger.take() {
            return self.render_trigger(trigger, window, cx);
        }

        let layer_ix = self.layer_ix;
        let on_close = self.callbacks.on_close.clone();
        let on_ok = self.callbacks.on_ok.clone();
        let on_cancel = self.callbacks.on_cancel.clone();
        let is_alert = self.presentation == DialogPresentation::Alert;
        let modal_metrics = cx.theme().style.modals;
        let standard_metrics = StandardDialogMetrics::for_density(cx.theme().style.density);

        let window_paddings = crate::window_border::window_paddings(window);
        let view_size = window.viewport_size()
            - gpui::size(
                window_paddings.left + window_paddings.right,
                window_paddings.top + window_paddings.bottom,
            );
        let bounds = Bounds {
            origin: Point::default(),
            size: view_size,
        };
        let safe_width = (view_size.width - px(32.)).max(px(0.));
        let preferred_width = self.props.width.unwrap_or(standard_metrics.default_width);
        let width = self
            .props
            .max_width
            .map_or(preferred_width, |max_width| preferred_width.min(max_width))
            .min(safe_width);
        let offset_top = px(layer_ix as f32 * 16.);
        let y = self.props.margin_top.unwrap_or(view_size.height / 10.) + offset_top;
        let x = bounds.center().x - width / 2.;

        let base_size = window.text_style().font_size;
        let rem_size = window.rem_size();

        let mut paddings = Edges::all(modal_metrics.padding);
        if let Some(pl) = self.style.padding.left {
            paddings.left = pl.to_pixels(base_size, rem_size);
        }
        if let Some(pr) = self.style.padding.right {
            paddings.right = pr.to_pixels(base_size, rem_size);
        }
        if let Some(pt) = self.style.padding.top {
            paddings.top = pt.to_pixels(base_size, rem_size);
        }
        if let Some(pb) = self.style.padding.bottom {
            paddings.bottom = pb.to_pixels(base_size, rem_size);
        }

        let closing = self.lifecycle_phase == OverlayPhase::Closing;
        if !closing {
            let dialog_focus = self.focus_handle.clone();
            let initial_focus = self.initial_focus.clone();
            window.defer(cx, move |window, cx| {
                if !dialog_focus.is_focused(window) {
                    return;
                }
                if let Some(initial_focus) = initial_focus {
                    initial_focus.focus(window, cx);
                    if dialog_focus.contains_focused(window, cx) {
                        return;
                    }
                    dialog_focus.focus(window, cx);
                }

                let starting_focus = window.focused(cx);
                // Bound the global tab-stop scan so a malformed focus graph cannot hang opening.
                const MAX_INITIAL_FOCUS_ATTEMPTS: usize = 100;
                for _ in 0..MAX_INITIAL_FOCUS_ATTEMPTS {
                    window.focus_next(cx);
                    if dialog_focus.contains_focused(window, cx) {
                        return;
                    }
                    if window.focused(cx) == starting_focus {
                        break;
                    }
                }
                dialog_focus.focus(window, cx);
            });
        }
        let motion = cx.theme().style.motion;
        let elevation_enabled = cx.theme().style.elevation.enabled;
        let overlay_animation = Animation::new(if is_alert {
            motion.fast()
        } else {
            motion.emphasis()
        })
        .with_easing(move |delta| {
            if closing {
                motion.exit_easing.sample(delta)
            } else {
                motion.enter_easing.sample(delta)
            }
        });
        // shadcn Dialog content uses a 100 ms zoom transition. Keep this
        // independent from the existing overlay timing.
        let content_animation = Animation::new(motion.fast()).with_easing(move |delta| {
            if closing {
                motion.exit_easing.sample(delta)
            } else {
                motion.enter_easing.sample(delta)
            }
        });

        let overlay_background = self.props.overlay_visible.then(|| {
            if self.props.show_overlay {
                hsla(0., 0., 0., modal_metrics.overlay_opacity)
            } else {
                hsla(0., 0., 0., 0.)
            }
        });
        let dismiss_on_overlay_click = self.props.dismiss_on_overlay_click;
        // Only the topmost open backdrop may receive dismissal input.
        let overlay_accepts_input = self.props.show_overlay && !closing;
        let owns_overlay = (self.layer_ix + 1) == Root::read(window, cx).active_dialogs.len();

        let backdrop = modal_overlay(view_size, overlay_background).when(
            overlay_accepts_input && owns_overlay,
            |this| {
                this.window_control_area(WindowControlArea::Drag)
                    .on_any_mouse_down({
                        let on_cancel = on_cancel.clone();
                        let on_close = on_close.clone();
                        move |event, window, cx| {
                            if event.position.y < TITLE_BAR_HEIGHT {
                                return;
                            }

                            cx.stop_propagation();
                            if dismiss_on_overlay_click && event.button == MouseButton::Left {
                                if on_cancel(&ClickEvent::default(), window, cx) {
                                    window.close_dialog(cx);
                                    on_close(&ClickEvent::default(), window, cx);
                                }
                            }
                        }
                    })
            },
        );

        let title = self.title.take().map(|title| {
            DialogTitle::new()
                .font_medium()
                .when(standard_metrics.small_title && !is_alert, |this| {
                    this.text_sm()
                })
                .when(!standard_metrics.small_title || is_alert, |this| {
                    this.text_base()
                })
                .child(title(window, cx))
                .into_any_element()
        });
        let description = self.description.take().map(|description| {
            crate::dialog::DialogDescription::new()
                .child(description(window, cx))
                .into_any_element()
        });
        let semantic_header = (title.is_some() || description.is_some()).then(|| {
            v_flex()
                .pl(paddings.left)
                .pr(paddings.right)
                .gap(modal_metrics.header_gap)
                .children(title)
                .children(description)
        });

        let content = v_flex()
            .id(layer_ix)
            .role(self.a11y_role)
            .when_some(self.a11y_label, |this, label| this.aria_label(label))
            .when_some(self.a11y_description, |this, description| {
                this.aria_description(description)
            })
            .track_focus(&self.focus_handle)
            .focus_trap(format!("dialog-{}", layer_ix), &self.focus_handle)
            .bg(cx.theme().tokens.popover)
            .text_color(cx.theme().popover_foreground)
            .border_1()
            .border_color(cx.theme().foreground.opacity(modal_metrics.ring_opacity))
            .rounded(cx.theme().style.radii.xl)
            .min_h_24()
            .pt(paddings.top)
            .pb(paddings.bottom)
            .gap(modal_metrics.gap)
            .refine_style(&self.style)
            .px_0()
            .when_some(
                match (
                    is_alert,
                    self.props.dismiss_on_escape,
                    self.props.confirm_on_enter,
                ) {
                    (true, true, _) => Some(ALERT_CONTEXT),
                    (true, false, _) => None,
                    (false, true, true) => Some(CONTEXT),
                    (false, true, false) => Some(ESCAPE_CONTEXT),
                    (false, false, true) => Some(CONFIRM_CONTEXT),
                    (false, false, false) => None,
                },
                |this, context| this.key_context(context),
            )
            .when(!closing, |this| {
                this.on_action({
                    let on_cancel = on_cancel.clone();
                    let on_close = on_close.clone();
                    move |_: &CancelDialog, window, cx| {
                        if on_cancel(&ClickEvent::default(), window, cx) {
                            window.close_dialog(cx);
                            on_close(&ClickEvent::default(), window, cx);
                        }
                    }
                })
                .on_action({
                    let on_ok = on_ok.clone();
                    let on_close = on_close.clone();
                    move |_: &ConfirmDialog, window, cx| {
                        if on_ok(&ClickEvent::default(), window, cx) {
                            Self::defer_close_dialog(window, cx);
                            on_close(&ClickEvent::default(), window, cx);
                        }
                    }
                })
            })
            .occlude()
            .relative()
            .w(width)
            .when(!is_alert, |this| this.absolute().left(x).top(y))
            .child(
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .gap(modal_metrics.gap)
                    .when_some(self.header, |this, header| {
                        this.child(div().pl(paddings.left).pr(paddings.right).child(header))
                    })
                    .children(semantic_header)
                    .when_some(self.content_builder, |this, builder| {
                        this.child(builder(
                            DialogContent::new().pl(paddings.left).pr(paddings.right),
                            window,
                            cx,
                        ))
                    })
                    .when(!self.children.is_empty(), |this| {
                        this.child(
                            div().flex_1().overflow_hidden().child(
                                // Body
                                v_flex()
                                    .size_full()
                                    .overflow_y_scrollbar()
                                    .pl(paddings.left)
                                    .pr(paddings.right)
                                    .children(self.children),
                            ),
                        )
                    }),
            )
            .when_some(self.footer, |this, footer| {
                this.child(div().pl(paddings.left).pr(paddings.right).child(footer))
            })
            .when_some(self.footer_builder, |this, footer| {
                this.child(
                    div()
                        .pl(paddings.left)
                        .pr(paddings.right)
                        .child(footer(window, cx)),
                )
            })
            .children(self.props.show_close_button.then(|| {
                // Keep positioning on a stable wrapper. The Button changes its
                // interaction tree during close and must not own modal geometry.
                div()
                    .absolute()
                    .top(standard_metrics.close_inset)
                    .right(standard_metrics.close_inset)
                    .child(
                        Button::new("close")
                            .aria_label(t!("Common.Close"))
                            .small()
                            .ghost()
                            .icon(IconName::Close)
                            .when(!closing, |this| {
                                this.on_click({
                                    let on_cancel = self.callbacks.on_cancel.clone();
                                    let on_close = self.callbacks.on_close.clone();
                                    move |_, window, cx| {
                                        if on_cancel(&ClickEvent::default(), window, cx) {
                                            window.close_dialog(cx);
                                            on_close(&ClickEvent::default(), window, cx);
                                        }
                                    }
                                })
                            }),
                    )
            }))
            .when(closing, |this| {
                this.child(div().absolute().top_0().left_0().size_full().occlude())
            })
            .with_animation(
                ElementId::NamedInteger("dialog-motion".into(), closing as u64),
                content_animation,
                move |this, delta| {
                    let delta = if closing { 1.0 - delta } else { delta };
                    // This is equivalent to `shadow_xl`. Standard Dialog keeps its
                    // final geometry stable and animates only content opacity.
                    let shadow = (elevation_enabled && !is_alert).then(|| {
                        vec![
                            BoxShadow {
                                color: hsla(0., 0., 0., 0.1),
                                offset: point(px(0.), px(20.)),
                                blur_radius: px(25.),
                                spread_radius: px(-5.),
                                inset: false,
                            },
                            BoxShadow {
                                color: hsla(0., 0., 0., 0.1),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(10.),
                                spread_radius: px(-6.),
                                inset: false,
                            },
                        ]
                    });
                    if is_alert {
                        // GPUI does not expose a layout-independent transform for
                        // arbitrary element trees. Keep the final width stable so
                        // modal motion never reflows or clips semantic content.
                        this.opacity(delta)
                    } else {
                        this.opacity(delta)
                            .left(x)
                            .top(y)
                            .w(width)
                            .when_some(shadow, |this, shadow| this.shadow(shadow))
                    }
                },
            )
            .selection_scope(SelectionScope::Dialog(layer_ix));

        // Backdrop and content remain siblings so overlay opacity never fades the
        // Dialog content. This is required for scale-only content motion.
        let dialog_layer = if is_alert {
            div()
                .id(("alert-dialog-layer", layer_ix))
                .relative()
                .w(view_size.width)
                .h(view_size.height)
                .flex()
                .items_center()
                .justify_center()
                .px_4()
                .child(
                    backdrop
                        .id(("dialog-backdrop", layer_ix))
                        .absolute()
                        .top_0()
                        .left_0()
                        .with_animation(
                            ElementId::NamedInteger(
                                "alert-dialog-overlay-motion".into(),
                                closing as u64,
                            ),
                            overlay_animation,
                            move |this, delta| {
                                this.opacity(if closing { 1.0 - delta } else { delta })
                            },
                        ),
                )
                .child(content)
                .into_any_element()
        } else {
            div()
                .id(("dialog-layer", layer_ix))
                .relative()
                .w(view_size.width)
                .h(view_size.height)
                .child(
                    backdrop
                        .id(("dialog-backdrop", layer_ix))
                        .absolute()
                        .top_0()
                        .left_0()
                        .with_animation(
                            ElementId::NamedInteger("dialog-overlay".into(), closing as u64),
                            overlay_animation,
                            move |this, delta| {
                                this.opacity(if closing { 1.0 - delta } else { delta })
                            },
                        ),
                )
                .child(content)
                .into_any_element()
        };

        anchored()
            .position(point(window_paddings.left, window_paddings.top))
            .snap_to_window()
            .child(dialog_layer)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        AppContext as _, Context, KeyContext, KeyDownEvent, KeyUpEvent, Keystroke, Render,
        TestAppContext, VisualTestContext,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct KeymapFixture;

    struct LayerFixture;

    struct TriggerFixture {
        trigger_clicks: Arc<AtomicUsize>,
        confirmations: Arc<AtomicUsize>,
        body_focus: FocusHandle,
    }

    impl Render for KeymapFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    impl Render for LayerFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().children(Root::render_dialog_layer(window, cx))
        }
    }

    impl Render for TriggerFixture {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let trigger_clicks = self.trigger_clicks.clone();
            let confirmations = self.confirmations.clone();
            let body_focus = self.body_focus.clone();

            div()
                .child(
                    Dialog::new(cx)
                        .trigger(Button::new("dialog-trigger").label("Open dialog").on_click(
                            move |_, _, _| {
                                trigger_clicks.fetch_add(1, Ordering::SeqCst);
                            },
                        ))
                        .title("Accessible title")
                        .description("Accessible description")
                        .initial_focus(body_focus.clone())
                        .content(move |content, _, _| {
                            content.child(div().track_focus(&body_focus).child("Dialog body"))
                        })
                        .on_ok(move |_, _, _| {
                            confirmations.fetch_add(1, Ordering::SeqCst);
                            true
                        }),
                )
                .children(Root::render_dialog_layer(window, cx))
        }
    }

    #[test]
    fn standard_metrics_follow_semantic_density() {
        let compact = StandardDialogMetrics::for_density(Density::Compact);
        let standard = StandardDialogMetrics::for_density(Density::Standard);
        let comfortable = StandardDialogMetrics::for_density(Density::Comfortable);

        assert_eq!(compact.default_width, px(384.));
        assert_eq!(standard.default_width, px(448.));
        assert_eq!(comfortable.default_width, px(448.));
        assert_eq!(compact.close_inset, px(8.));
        assert_eq!(standard.close_inset, px(16.));
        assert!(standard.small_title);
        assert!(!compact.small_title);
        assert!(!comfortable.small_title);
    }

    #[gpui::test]
    fn width_builders_write_dialog_layout_props(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let dialog = Dialog::new(cx).w(px(720.)).max_w(px(640.));

            assert_eq!(dialog.props.width, Some(px(720.)));
            assert_eq!(dialog.props.max_width, Some(px(640.)));
        });
    }

    #[gpui::test]
    fn text_slots_supply_accessibility_metadata(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let dialog = Dialog::new(cx)
                .title("Connection settings")
                .description("Configure the active connection.");

            assert_eq!(dialog.a11y_label.as_deref(), Some("Connection settings"));
            assert_eq!(
                dialog.a11y_description.as_deref(),
                Some("Configure the active connection.")
            );
        });
    }

    #[gpui::test]
    fn trigger_preserves_handler_and_enter_confirms_once(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let trigger_clicks = Arc::new(AtomicUsize::new(0));
        let confirmations = Arc::new(AtomicUsize::new(0));
        let captured_trigger_clicks = trigger_clicks.clone();
        let captured_confirmations = confirmations.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let body_focus = cx.focus_handle();
            let fixture = cx.new(move |_| TriggerFixture {
                trigger_clicks,
                confirmations,
                body_focus,
            });
            Root::new(fixture, window, cx)
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
        cx.simulate_event(KeyUpEvent {
            keystroke: enter.clone(),
        });
        cx.run_until_parked();

        assert!(cx.update(|window, cx| crate::WindowExt::has_active_dialog(window, cx)));
        assert_eq!(captured_trigger_clicks.load(Ordering::SeqCst), 1);

        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.simulate_event(KeyDownEvent {
            keystroke: enter.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke: enter });
        cx.run_until_parked();

        assert_eq!(captured_confirmations.load(Ordering::SeqCst), 1);
    }

    #[gpui::test]
    fn focused_button_enter_activates_button_instead_of_default_confirmation(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let button_clicks = Arc::new(AtomicUsize::new(0));
        let confirmations = Arc::new(AtomicUsize::new(0));
        let captured_button_clicks = button_clicks.clone();
        let captured_confirmations = confirmations.clone();
        let (_, cx) = cx.add_window_view(|window, cx| {
            let fixture = cx.new(|_| LayerFixture);
            Root::new(fixture, window, cx)
        });
        let button_focus = cx.update(|_, cx| cx.focus_handle());
        let captured_focus = button_focus.clone();
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            window.open_dialog(cx, move |dialog, _, _| {
                let button_clicks = button_clicks.clone();
                let confirmations = confirmations.clone();
                dialog
                    .title("Button priority")
                    .show_close_button(false)
                    .initial_focus(button_focus.clone())
                    .child(
                        Button::new("secondary-action")
                            .label("Secondary action")
                            .focus_handle(button_focus.clone())
                            .on_click(move |_, _, _| {
                                button_clicks.fetch_add(1, Ordering::SeqCst);
                            }),
                    )
                    .on_ok(move |_, _, _| {
                        confirmations.fetch_add(1, Ordering::SeqCst);
                        true
                    })
            });
            let _ = window.draw(cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.run_until_parked();
        assert!(cx.update(|window, _| captured_focus.is_focused(window)));

        let enter = Keystroke::parse("enter").expect("enter must be a valid keystroke");
        cx.simulate_event(KeyDownEvent {
            keystroke: enter.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke: enter });
        cx.run_until_parked();

        assert_eq!(captured_button_clicks.load(Ordering::SeqCst), 1);
        assert_eq!(captured_confirmations.load(Ordering::SeqCst), 0);
        assert!(cx.update(|window, cx| crate::WindowExt::has_active_dialog(window, cx)));
    }

    #[gpui::test]
    fn default_focus_moves_to_first_dialog_tab_stop(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let fixture = cx.new(|_| LayerFixture);
            Root::new(fixture, window, cx)
        });
        let target = cx.update(|_, cx| cx.focus_handle());
        let captured = target.clone();

        cx.update(|window, cx| {
            window.open_dialog(cx, move |dialog, _, _| {
                dialog.title("Focus test").show_close_button(false).child(
                    Button::new("focus-target")
                        .label("Target")
                        .focus_handle(target.clone()),
                )
            });
            let _ = window.draw(cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.run_until_parked();

        assert!(cx.update(|window, _| captured.is_focused(window)));
    }

    #[gpui::test]
    fn cancel_veto_keeps_dialog_open(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cancellations = Arc::new(AtomicUsize::new(0));
        let captured = cancellations.clone();
        let (_, cx) = cx.add_window_view(|window, cx| {
            let fixture = cx.new(|_| LayerFixture);
            Root::new(fixture, window, cx)
        });

        cx.update(|window, cx| {
            window.open_dialog(cx, move |dialog, _, _| {
                dialog.title("Veto test").on_cancel({
                    let cancellations = cancellations.clone();
                    move |_, _, _| {
                        cancellations.fetch_add(1, Ordering::SeqCst);
                        false
                    }
                })
            });
            let _ = window.draw(cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            window.dispatch_action(Box::new(CancelDialog), cx);
        });

        assert_eq!(captured.load(Ordering::SeqCst), 1);
        assert!(cx.update(|window, cx| crate::WindowExt::has_active_dialog(window, cx)));
    }

    #[gpui::test]
    fn standard_and_alert_dialogs_use_distinct_keymaps(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| KeymapFixture);
        let (
            standard_confirm,
            escape_confirm,
            confirm_only_confirm,
            alert_confirm,
            standard_cancel,
            escape_cancel,
            confirm_only_cancel,
            alert_cancel,
        ) = cx.update(|window, _| {
            let standard =
                KeyContext::try_from(CONTEXT).expect("standard dialog context must be valid");
            let escape = KeyContext::try_from(ESCAPE_CONTEXT)
                .expect("escape-only dialog context must be valid");
            let confirm_only = KeyContext::try_from(CONFIRM_CONTEXT)
                .expect("confirm-only dialog context must be valid");
            let alert =
                KeyContext::try_from(ALERT_CONTEXT).expect("alert dialog context must be valid");
            (
                window.bindings_for_action_in_context(&ConfirmDialog, standard.clone()),
                window.bindings_for_action_in_context(&ConfirmDialog, escape.clone()),
                window.bindings_for_action_in_context(&ConfirmDialog, confirm_only.clone()),
                window.bindings_for_action_in_context(&ConfirmDialog, alert.clone()),
                window.bindings_for_action_in_context(&CancelDialog, standard),
                window.bindings_for_action_in_context(&CancelDialog, escape),
                window.bindings_for_action_in_context(&CancelDialog, confirm_only),
                window.bindings_for_action_in_context(&CancelDialog, alert),
            )
        });

        assert!(!standard_confirm.is_empty());
        assert!(escape_confirm.is_empty());
        assert!(!confirm_only_confirm.is_empty());
        assert!(alert_confirm.is_empty());
        assert!(!standard_cancel.is_empty());
        assert!(!escape_cancel.is_empty());
        assert!(confirm_only_cancel.is_empty());
        assert!(!alert_cancel.is_empty());
    }
}
