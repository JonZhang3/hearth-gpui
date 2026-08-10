use crate::{
    ActiveTheme, Placement, StyledExt,
    animation::{OverlayLifecycle, effective_motion_duration},
    dialog::{Dialog, DialogPresentation},
    focus_trap::FocusTrapManager,
    input::{Copy, InputState},
    native_menu::FallbackMenuOverlay,
    notification::{Notification, NotificationList},
    sheet::Sheet,
    text::{SelectionScope, TextSelectionController, TextViewState, WindowTextSelection},
    tooltip::TooltipOverlay,
    window_border,
};
use gpui::{
    Anchor, AnyView, App, AppContext, Bounds, ClipboardItem, Context, ElementId, Entity, EntityId,
    FocusHandle, Hitbox, InteractiveElement, IntoElement, KeyBinding, ParentElement as _, Pixels,
    Render, StyleRefinement, Styled, WeakEntity, WeakFocusHandle, Window, actions, div,
    prelude::FluentBuilder as _,
};
use std::{any::TypeId, collections::HashMap, rc::Rc};

actions!(root, [Tab, TabPrev]);

const CONTEXT: &str = "Root";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
    ]);
}

/// Root is a view for the App window for as the top level view (Must be the first view in the window).
///
/// It is used to manage the Sheet, Dialog, and Notification.
pub struct Root {
    style: StyleRefinement,
    view: AnyView,
    pub(crate) active_sheet: Option<ActiveSheet>,
    pub(crate) active_dialogs: Vec<ActiveDialog>,
    pub(super) focused_input: Option<Entity<InputState>>,
    pub notification: Entity<NotificationList>,
    pub(crate) tooltip_overlay: Entity<TooltipOverlay>,
    pub(crate) native_menu_overlay: Entity<FallbackMenuOverlay>,
    sheet_size: Option<Pixels>,
    window_shadow_size: Pixels,
    /// Render the Linux CSD `window_border` wrapper.
    bordered: bool,
    /// The focus handle that will be restored after a dialog is closed with animation.
    /// Used to handle rapid dialog opening/closing to maintain correct focus chain.
    pending_focus_restore: Option<WeakFocusHandle>,
    /// Monotonic identity for interruptible modal lifecycle tasks.
    next_overlay_id: u64,
    /// Window-level text selection state. See `text::window_selection`.
    pub(crate) text_selection: WindowTextSelection,
    /// Selectable TextViews registered this frame, keyed by entity id.
    pub(crate) selectable_text_views:
        HashMap<EntityId, (WeakEntity<TextViewState>, Hitbox, SelectionScope)>,
    /// Inline text bounds for selectable TextViews, keyed by parent TextView id.
    pub(crate) selectable_text_inlines: HashMap<EntityId, Vec<Bounds<Pixels>>>,
}

#[derive(Clone)]
pub(crate) struct ActiveSheet {
    id: u64,
    focus_handle: FocusHandle,
    /// The previous focused handle before opening the Sheet.
    previous_focused_handle: Option<WeakFocusHandle>,
    placement: Placement,
    builder: Rc<dyn Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static>,
    lifecycle: OverlayLifecycle,
}

#[derive(Clone)]
pub(crate) struct ActiveDialog {
    id: u64,
    focus_handle: FocusHandle,
    /// The previous focused handle before opening the Dialog.
    previous_focused_handle: Option<WeakFocusHandle>,
    builder: Rc<dyn Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static>,
    presentation: DialogPresentation,
    lifecycle: OverlayLifecycle,
}

impl ActiveDialog {
    pub(crate) fn new(
        id: u64,
        focus_handle: FocusHandle,
        previous_focused_handle: Option<WeakFocusHandle>,
        presentation: DialogPresentation,
        builder: impl Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    ) -> Self {
        Self {
            id,
            focus_handle,
            previous_focused_handle,
            builder: Rc::new(builder),
            presentation,
            lifecycle: OverlayLifecycle::opened(),
        }
    }
}

impl Root {
    /// Create a new Root view.
    pub fn new(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        #[cfg(all(target_os = "macos", not(test)))]
        crate::macos_accessibility::install_window_hit_test_forwarder(window);

        Self {
            style: StyleRefinement::default(),
            view: view.into(),
            active_sheet: None,
            active_dialogs: Vec::new(),
            focused_input: None,
            notification: cx.new(|cx| NotificationList::new(window, cx)),
            tooltip_overlay: cx.new(|_| TooltipOverlay::new()),
            native_menu_overlay: cx.new(|_| FallbackMenuOverlay::new()),
            sheet_size: None,
            window_shadow_size: window_border::SHADOW_SIZE,
            bordered: true,
            pending_focus_restore: None,
            next_overlay_id: 0,
            text_selection: WindowTextSelection::default(),
            selectable_text_views: HashMap::new(),
            selectable_text_inlines: HashMap::new(),
        }
    }

    /// Enable or disable the Linux client-side window border wrapper.
    ///
    /// Defaults to `true`. Use `bordered(false)` for layer-shell fullscreen windows
    /// or other surfaces that should not render GPUI Component's window border.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set the window border shadow size for Linux client-side decorations.
    ///
    /// Default: [`window_border::SHADOW_SIZE`]
    pub fn window_shadow_size(mut self, size: impl Into<Pixels>) -> Self {
        self.window_shadow_size = size.into();
        self
    }

    pub fn update<F, R>(window: &mut Window, cx: &mut App, f: F) -> R
    where
        F: FnOnce(&mut Self, &mut Window, &mut Context<Self>) -> R,
    {
        let root = window
            .root::<Root>()
            .flatten()
            .expect("BUG: window first layer should be a gpui_component::Root.");

        root.update(cx, |root, cx| f(root, window, cx))
    }

    pub fn read<'a>(window: &'a Window, cx: &'a App) -> &'a Self {
        &window
            .root::<Root>()
            .expect("The window root view should be of type `ui::Root`.")
            .unwrap()
            .read(cx)
    }

    // Render Notification layer.
    pub fn render_notification_layer(
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement + use<>> {
        let root = window.root::<Root>()??;

        let active_sheet_placement = root.read(cx).active_sheet.clone().map(|d| d.placement);

        let sheet_size = root.read(cx).sheet_size;
        let (mt, mr, mb, ml) = match active_sheet_placement {
            Some(Placement::Top) => (sheet_size, None, None, None),
            Some(Placement::Right) => (None, sheet_size, None, None),
            Some(Placement::Bottom) => (None, None, sheet_size, None),
            Some(Placement::Left) => (None, None, None, sheet_size),
            _ => (None, None, None, None),
        };

        let placement = cx.theme().notification.placement;

        Some(
            div()
                .absolute()
                .when(matches!(placement, Anchor::TopRight), |this| {
                    this.top_0().right_0()
                })
                .when(matches!(placement, Anchor::TopLeft), |this| {
                    this.top_0().left_0()
                })
                .when(matches!(placement, Anchor::TopCenter), |this| {
                    this.top_0().mx_auto()
                })
                .when(matches!(placement, Anchor::BottomRight), |this| {
                    this.bottom_0().right_0()
                })
                .when(matches!(placement, Anchor::BottomLeft), |this| {
                    this.bottom_0().left_0()
                })
                .when(matches!(placement, Anchor::BottomCenter), |this| {
                    this.bottom_0().mx_auto()
                })
                .when_some(mt, |this, offset| this.mt(offset))
                .when_some(mr, |this, offset| this.mr(offset))
                .when_some(mb, |this, offset| this.mb(offset))
                .when_some(ml, |this, offset| this.ml(offset))
                .child(root.read(cx).notification.clone()),
        )
    }

    /// Render the Sheet layer.
    pub fn render_sheet_layer(
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement + use<>> {
        let root = window.root::<Root>()??;

        if let Some(active_sheet) = root.read(cx).active_sheet.clone() {
            let mut sheet = Sheet::new(window, cx);
            sheet = (active_sheet.builder)(sheet, window, cx);
            sheet.focus_handle = active_sheet.focus_handle.clone();
            sheet.placement = active_sheet.placement;
            sheet.instance_id = active_sheet.id;
            sheet.lifecycle_phase = active_sheet.lifecycle.phase();
            sheet.measured_size = root.read(cx).sheet_size;
            let root_for_size = root.clone();
            sheet.observe_size = Some(Rc::new(move |size, cx| {
                root_for_size.update(cx, |root, cx| {
                    if root.sheet_size != Some(size) {
                        root.sheet_size = Some(size);
                        cx.notify();
                    }
                });
            }));

            return Some(div().relative().child(sheet));
        }

        None
    }

    /// Render the Dialog layer.
    pub fn render_dialog_layer(
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement + use<>> {
        let root = window.root::<Root>()??;

        let active_dialogs = root.read(cx).active_dialogs.clone();

        if active_dialogs.is_empty() {
            return None;
        }

        let mut show_overlay_ix = None;

        let mut dialogs = active_dialogs
            .iter()
            .enumerate()
            .map(|(i, active_dialog)| {
                let mut dialog = Dialog::new(cx);

                dialog = (active_dialog.builder)(dialog, window, cx);

                // Give the dialog the focus handle, because `dialog` is a temporary value, is not possible to
                // keep the focus handle in the dialog.
                //
                // So we keep the focus handle in the `active_dialog`, this is owned by the `Root`.
                dialog.focus_handle = active_dialog.focus_handle.clone();
                dialog.lifecycle_phase = active_dialog.lifecycle.phase();
                dialog.presentation = active_dialog.presentation;

                dialog.layer_ix = i;
                // Find the dialog which one needs to show overlay.
                if dialog.has_overlay() {
                    show_overlay_ix = Some(i);
                }

                dialog
            })
            .collect::<Vec<_>>();

        if let Some(ix) = show_overlay_ix {
            if let Some(dialog) = dialogs.get_mut(ix) {
                dialog.props.overlay_visible = true;
            }
        }

        Some(div().children(dialogs))
    }

    pub fn open_dialog<F>(&mut self, build: F, window: &mut Window, cx: &mut Context<'_, Root>)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        self.open_dialog_with_presentation(DialogPresentation::Standard, build, window, cx);
    }

    /// Opens a modal using an explicit internal surface presentation.
    pub(crate) fn open_dialog_with_presentation<F>(
        &mut self,
        presentation: DialogPresentation,
        build: F,
        window: &mut Window,
        cx: &mut Context<'_, Root>,
    ) where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        let mut previous_focused_handle = window.focused(cx).map(|h| h.downgrade());

        // Use pending focus restore if available to maintain correct focus chain
        // when a new dialog is opened immediately after closing another dialog.
        if let Some(pending_handle) = self.pending_focus_restore.take() {
            previous_focused_handle = Some(pending_handle);
        }

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        self.next_overlay_id = self.next_overlay_id.wrapping_add(1);
        self.active_dialogs.push(ActiveDialog::new(
            self.next_overlay_id,
            focus_handle,
            previous_focused_handle,
            presentation,
            build,
        ));
        // Opening a modal confines selection to it; drop any background
        // selection so it cannot linger (or be copied) under the modal.
        self.clear_text_selection(cx);
        cx.notify();
    }

    pub fn close_dialog(&mut self, window: &mut Window, cx: &mut Context<'_, Root>) {
        self.begin_close_dialog(window, cx);
    }

    pub(crate) fn defer_close_dialog(&mut self, window: &mut Window, cx: &mut Context<'_, Root>) {
        self.begin_close_dialog(window, cx);
    }

    /// Starts one close lifecycle for the topmost dialog and removes it only
    /// after the shared exit duration has completed.
    fn begin_close_dialog(&mut self, window: &mut Window, cx: &mut Context<'_, Root>) {
        self.focused_input = None;
        let Some(active_dialog) = self.active_dialogs.last_mut() else {
            return;
        };
        let Some(transition) = active_dialog.lifecycle.begin_close() else {
            return;
        };

        let dialog_id = active_dialog.id;
        let previous_handle = active_dialog
            .previous_focused_handle
            .as_ref()
            .and_then(|handle| handle.upgrade());
        if let Some(handle) = previous_handle.as_ref() {
            self.pending_focus_restore = Some(handle.downgrade());
        }

        let duration = effective_motion_duration(
            if active_dialog.presentation == DialogPresentation::Alert {
                cx.theme().style.motion.fast()
            } else {
                cx.theme().style.motion.emphasis()
            },
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(duration).await;
            let _ = this.update_in(cx, |this, window, cx| {
                let Some(index) = this
                    .active_dialogs
                    .iter()
                    .position(|dialog| dialog.id == dialog_id)
                else {
                    return;
                };
                if !this.active_dialogs[index]
                    .lifecycle
                    .complete_close(transition)
                {
                    return;
                }

                this.active_dialogs.remove(index);
                let newer_dialog_is_open = this
                    .active_dialogs
                    .iter()
                    .skip(index)
                    .any(|dialog| dialog.lifecycle.accepts_input());
                if !newer_dialog_is_open && let Some(handle) = previous_handle {
                    window.focus(&handle, cx);
                }
                this.pending_focus_restore = None;
                cx.notify();
            });
        })
        .detach();

        self.clear_text_selection(cx);
        cx.notify();
    }

    pub fn close_all_dialogs(&mut self, window: &mut Window, cx: &mut Context<'_, Root>) {
        self.focused_input = None;
        let previous_handle = self
            .active_dialogs
            .first()
            .and_then(|dialog| dialog.previous_focused_handle.as_ref())
            .and_then(|handle| handle.upgrade());
        let transitions = self
            .active_dialogs
            .iter_mut()
            .filter_map(|dialog| {
                dialog
                    .lifecycle
                    .begin_close()
                    .map(|transition| (dialog.id, transition, dialog.presentation))
            })
            .collect::<Vec<_>>();
        if transitions.is_empty() {
            return;
        }

        let duration = transitions
            .iter()
            .map(|(_, _, presentation)| {
                if *presentation == DialogPresentation::Alert {
                    cx.theme().style.motion.fast()
                } else {
                    cx.theme().style.motion.emphasis()
                }
            })
            .max()
            .unwrap_or_default();
        let duration = effective_motion_duration(duration, cx);
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(duration).await;
            let _ = this.update_in(cx, |this, window, cx| {
                for (dialog_id, transition, _) in transitions {
                    let Some(index) = this
                        .active_dialogs
                        .iter()
                        .position(|dialog| dialog.id == dialog_id)
                    else {
                        continue;
                    };
                    if this.active_dialogs[index]
                        .lifecycle
                        .complete_close(transition)
                    {
                        this.active_dialogs.remove(index);
                    }
                }

                if !this
                    .active_dialogs
                    .iter()
                    .any(|dialog| dialog.lifecycle.accepts_input())
                    && let Some(handle) = previous_handle
                {
                    window.focus(&handle, cx);
                }
                this.pending_focus_restore = None;
                cx.notify();
            });
        })
        .detach();

        self.clear_text_selection(cx);
        cx.notify();
    }

    pub fn open_sheet_at<F>(
        &mut self,
        placement: Placement,
        build: F,
        window: &mut Window,
        cx: &mut Context<'_, Root>,
    ) where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        let previous_focused_handle = self
            .active_sheet
            .take()
            .and_then(|s| s.previous_focused_handle)
            .or_else(|| window.focused(cx).map(|h| h.downgrade()));

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);
        self.next_overlay_id = self.next_overlay_id.wrapping_add(1);
        self.active_sheet = Some(ActiveSheet {
            id: self.next_overlay_id,
            focus_handle,
            previous_focused_handle,
            placement,
            builder: Rc::new(build),
            lifecycle: OverlayLifecycle::opened(),
        });
        self.sheet_size = None;
        // Opening a modal confines selection to it; drop any background
        // selection so it cannot linger (or be copied) under the modal.
        self.clear_text_selection(cx);
        cx.notify();
    }

    pub fn close_sheet(&mut self, window: &mut Window, cx: &mut Context<'_, Root>) {
        self.focused_input = None;
        let Some(active_sheet) = self.active_sheet.as_mut() else {
            return;
        };
        let Some(transition) = active_sheet.lifecycle.begin_close() else {
            return;
        };
        let sheet_id = active_sheet.id;
        let previous_handle = active_sheet
            .previous_focused_handle
            .as_ref()
            .and_then(|handle| handle.upgrade());
        let duration = effective_motion_duration(cx.theme().style.motion.slow(), cx);
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(duration).await;
            let _ = this.update_in(cx, |this, window, cx| {
                let completed = this
                    .active_sheet
                    .as_mut()
                    .filter(|sheet| sheet.id == sheet_id)
                    .is_some_and(|sheet| sheet.lifecycle.complete_close(transition));
                if !completed {
                    return;
                }

                this.active_sheet = None;
                this.sheet_size = None;
                if let Some(handle) = previous_handle {
                    window.focus(&handle, cx);
                }
                cx.notify();
            });
        })
        .detach();
        self.clear_text_selection(cx);
        cx.notify();
    }

    pub fn push_notification(
        &mut self,
        note: impl Into<Notification>,
        window: &mut Window,
        cx: &mut Context<'_, Root>,
    ) {
        self.notification
            .update(cx, |view, cx| view.push(note, window, cx));
        cx.notify();
    }

    /// Removes all notifications whose id matches `T`, including ones registered with
    /// either [`Notification::id`] or [`Notification::id1`] (any key).
    pub fn remove_notification<T: Sized + 'static>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Root>,
    ) {
        self.notification.update(cx, |view, cx| {
            view.close_by_type(TypeId::of::<T>(), window, cx);
        });
        cx.notify();
    }

    /// Removes the notification matching the given type and element id (paired with [`Notification::id1`]).
    pub fn remove_notification1<T: Sized + 'static>(
        &mut self,
        key: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut Context<'_, Root>,
    ) {
        let key = key.into();
        self.notification.update(cx, |view, cx| {
            view.close((TypeId::of::<T>(), key), window, cx);
        });
        cx.notify();
    }

    pub fn clear_notifications(&mut self, window: &mut Window, cx: &mut Context<'_, Root>) {
        self.notification
            .update(cx, |view, cx| view.clear(window, cx));
        cx.notify();
    }

    /// Get the tooltip overlay entity for this window.
    pub(crate) fn tooltip_overlay(window: &Window, cx: &App) -> Option<Entity<TooltipOverlay>> {
        let root = window.root::<Root>()??;
        Some(root.read(cx).tooltip_overlay.clone())
    }

    /// Get the fallback native-menu overlay entity for this window.
    pub(crate) fn native_menu_overlay(
        window: &Window,
        cx: &App,
    ) -> Option<Entity<FallbackMenuOverlay>> {
        let root = window.root::<Root>()??;
        Some(root.read(cx).native_menu_overlay.clone())
    }

    /// Return the root view of the Root.
    pub fn view(&self) -> &AnyView {
        &self.view
    }

    fn on_action_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        // Check if we're inside a focus trap
        if let Some(container_focus_handle) = FocusTrapManager::find_active_trap(window, cx) {
            // We're in a focus trap - try to focus next, then check if we're still inside
            let before_focus = window.focused(cx);

            // Try normal focus navigation
            window.focus_next(cx);

            // Check if we're still in the trap
            if !container_focus_handle.contains_focused(window, cx) {
                // We jumped out of the trap - need to cycle back to the beginning
                // Find the first focusable element in the trap by continuing to focus_next
                let mut attempts = 0;
                const MAX_ATTEMPTS: usize = 100; // Prevent infinite loop

                while !container_focus_handle.contains_focused(window, cx)
                    && attempts < MAX_ATTEMPTS
                {
                    window.focus_next(cx);
                    attempts += 1;

                    // If we cycled back to where we started, restore original focus
                    if window.focused(cx) == before_focus {
                        break;
                    }
                }
            }
            return;
        }

        // Normal tab navigation
        window.focus_next(cx);
    }

    fn on_action_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        // Check if we're inside a focus trap
        if let Some(container_focus_handle) = FocusTrapManager::find_active_trap(window, cx) {
            // We're in a focus trap - try to focus previous, then check if we're still inside
            let before_focus = window.focused(cx);

            // Try normal focus navigation
            window.focus_prev(cx);

            // Check if we're still in the trap
            if !container_focus_handle.contains_focused(window, cx) {
                // We jumped out of the trap - need to cycle back to the end
                // Find the last focusable element in the trap by continuing to focus_prev
                let mut attempts = 0;
                const MAX_ATTEMPTS: usize = 100; // Prevent infinite loop

                while !container_focus_handle.contains_focused(window, cx)
                    && attempts < MAX_ATTEMPTS
                {
                    window.focus_prev(cx);
                    attempts += 1;

                    // If we cycled back to where we started, restore original focus
                    if window.focused(cx) == before_focus {
                        break;
                    }
                }
            }
            return;
        }

        // Normal tab navigation
        window.focus_prev(cx);
    }

    fn on_action_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.window_selected_text(cx).trim().to_string();
        if text.is_empty() {
            cx.propagate();
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
}

impl Styled for Root {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(cx.theme().font_size);

        let inner = div()
            .id("root")
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_tab))
            .on_action(cx.listener(Self::on_action_tab_prev))
            .on_action(cx.listener(Self::on_action_copy))
            .relative()
            .size_full()
            .font_family(cx.theme().font_family.clone())
            .bg(cx.theme().tokens.background)
            .text_color(cx.theme().foreground)
            .refine_style(&self.style)
            .child(TextSelectionController)
            .child(self.view.clone())
            .child(self.tooltip_overlay.clone())
            .child(self.native_menu_overlay.clone());

        if self.bordered {
            window_border()
                .shadow_size(self.window_shadow_size)
                .child(inner)
                .into_any_element()
        } else {
            inner.into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::OverlayPhase;
    use gpui::TestAppContext;
    use std::time::Duration;

    struct TestView;

    impl Render for TestView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    #[gpui::test]
    fn bordered_builder_toggles_window_border(cx: &mut TestAppContext) {
        cx.update(crate::init);

        let (default_root, _) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| TestView);
            Root::new(view, window, cx)
        });
        assert!(default_root.read_with(cx, |root, _| root.bordered));

        let (root, _) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| TestView);
            Root::new(view, window, cx).bordered(false)
        });
        assert!(!root.read_with(cx, |root, _| root.bordered));

        let (root, _) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| TestView);
            Root::new(view, window, cx).bordered(false).bordered(true)
        });
        assert!(root.read_with(cx, |root, _| root.bordered));
    }

    #[gpui::test]
    fn dialog_close_keeps_content_mounted_and_rejects_duplicate_close(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| TestView);
            Root::new(view, window, cx)
        });

        root.update_in(cx, |root, window, cx| {
            root.open_dialog(|dialog, _, _| dialog, window, cx);
            root.close_dialog(window, cx);
            root.close_dialog(window, cx);
        });
        assert_eq!(root.read_with(cx, |root, _| root.active_dialogs.len()), 1);
        assert_eq!(
            root.read_with(cx, |root, _| root.active_dialogs[0].lifecycle.phase()),
            OverlayPhase::Closing
        );

        cx.background_executor
            .advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        assert!(root.read_with(cx, |root, _| root.active_dialogs.is_empty()));
    }

    #[gpui::test]
    fn alert_dialog_close_uses_fast_modal_duration(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| TestView);
            Root::new(view, window, cx)
        });

        root.update_in(cx, |root, window, cx| {
            root.open_dialog_with_presentation(
                DialogPresentation::Alert,
                |dialog, _, _| dialog.alert_dialog_role(),
                window,
                cx,
            );
            root.close_dialog(window, cx);
        });

        cx.background_executor
            .advance_clock(Duration::from_millis(99));
        cx.run_until_parked();
        assert_eq!(root.read_with(cx, |root, _| root.active_dialogs.len()), 1);

        cx.background_executor
            .advance_clock(Duration::from_millis(1));
        cx.run_until_parked();
        assert!(root.read_with(cx, |root, _| root.active_dialogs.is_empty()));
    }

    #[gpui::test]
    fn reduced_motion_dialog_close_unmounts_without_delay(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
            cx.set_reduce_motion(true);
        });
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| TestView);
            Root::new(view, window, cx)
        });

        root.update_in(cx, |root, window, cx| {
            root.open_dialog(|dialog, _, _| dialog, window, cx);
            root.close_dialog(window, cx);
        });
        cx.run_until_parked();

        assert!(root.read_with(cx, |root, _| root.active_dialogs.is_empty()));
    }

    #[gpui::test]
    fn sheet_close_keeps_content_mounted_and_rejects_duplicate_close(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| TestView);
            Root::new(view, window, cx)
        });

        root.update_in(cx, |root, window, cx| {
            root.open_sheet_at(Placement::Right, |sheet, _, _| sheet, window, cx);
            root.close_sheet(window, cx);
            root.close_sheet(window, cx);
        });
        assert_eq!(
            root.read_with(cx, |root, _| root
                .active_sheet
                .as_ref()
                .unwrap()
                .lifecycle
                .phase()),
            OverlayPhase::Closing
        );

        cx.background_executor
            .advance_clock(Duration::from_millis(250));
        cx.run_until_parked();
        assert!(root.read_with(cx, |root, _| root.active_sheet.is_none()));
    }

    #[gpui::test]
    fn reduced_motion_sheet_close_unmounts_without_delay(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
            cx.set_reduce_motion(true);
        });
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| TestView);
            Root::new(view, window, cx)
        });

        root.update_in(cx, |root, window, cx| {
            root.open_sheet_at(Placement::Right, |sheet, _, _| sheet, window, cx);
            root.close_sheet(window, cx);
        });
        cx.run_until_parked();

        assert!(root.read_with(cx, |root, _| root.active_sheet.is_none()));
    }

    #[gpui::test]
    fn close_all_dialogs_uses_one_exit_lifecycle_for_nested_dialogs(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| TestView);
            Root::new(view, window, cx)
        });

        root.update_in(cx, |root, window, cx| {
            root.open_dialog(|dialog, _, _| dialog, window, cx);
            root.open_dialog(|dialog, _, _| dialog, window, cx);
            root.close_all_dialogs(window, cx);
            root.close_all_dialogs(window, cx);
        });
        assert_eq!(root.read_with(cx, |root, _| root.active_dialogs.len()), 2);
        assert!(root.read_with(cx, |root, _| {
            root.active_dialogs
                .iter()
                .all(|dialog| dialog.lifecycle.phase() == OverlayPhase::Closing)
        }));

        cx.background_executor
            .advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        assert!(root.read_with(cx, |root, _| root.active_dialogs.is_empty()));
    }

    #[gpui::test]
    fn opening_dialog_during_exit_preserves_the_new_dialog(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| TestView);
            Root::new(view, window, cx)
        });

        root.update_in(cx, |root, window, cx| {
            root.open_dialog(|dialog, _, _| dialog.title("First"), window, cx);
            root.close_dialog(window, cx);
            root.open_dialog(|dialog, _, _| dialog.title("Second"), window, cx);
        });
        cx.background_executor
            .advance_clock(Duration::from_millis(300));
        cx.run_until_parked();

        assert_eq!(root.read_with(cx, |root, _| root.active_dialogs.len()), 1);
        assert_eq!(
            root.read_with(cx, |root, _| root.active_dialogs[0].lifecycle.phase()),
            OverlayPhase::Open
        );
    }

    #[gpui::test]
    fn reopening_sheet_during_exit_invalidates_the_old_close_task(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| TestView);
            Root::new(view, window, cx)
        });

        root.update_in(cx, |root, window, cx| {
            root.open_sheet_at(Placement::Left, |sheet, _, _| sheet, window, cx);
            root.close_sheet(window, cx);
            root.open_sheet_at(Placement::Right, |sheet, _, _| sheet, window, cx);
        });
        cx.background_executor
            .advance_clock(Duration::from_millis(250));
        cx.run_until_parked();

        let (placement, phase) = root.read_with(cx, |root, _| {
            let sheet = root.active_sheet.as_ref().unwrap();
            (sheet.placement, sheet.lifecycle.phase())
        });
        assert_eq!(placement, Placement::Right);
        assert_eq!(phase, OverlayPhase::Open);
    }
}
