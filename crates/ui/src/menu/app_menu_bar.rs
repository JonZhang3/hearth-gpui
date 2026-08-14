// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added or exposed behavior through `adjacent_enabled_index`, `owned_menu`,
//   `keyboard_navigation_skips_disabled_menus`,
//   `keyboard_navigation_stops_when_every_menu_is_disabled`,
//   `rapid_trigger_click_does_not_reach_title_bar_double_click`.
// - Reworked App Menu Bar around accessibility semantics and ARIA state, semantic Style Preset
//   geometry and density, keyboard navigation and activation behavior, focus-visible and focus
//   restoration behavior.
use crate::{
    ActiveTheme, Disableable, Selectable, Sizable,
    actions::{Cancel, SelectLeft, SelectRight},
    button::Button,
    global_state::GlobalState,
    h_flex,
    menu::PopupMenu,
};
use gpui::{
    App, AppContext as _, ClickEvent, Context, DismissEvent, Entity, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, OwnedMenu, ParentElement,
    Render, Role, SharedString, StatefulInteractiveElement, Styled, Subscription, Window, anchored,
    deferred, div, prelude::FluentBuilder, px,
};

const CONTEXT: &str = "AppMenuBar";
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
    ]);
}

/// The application menu bar, for Windows and Linux.
pub struct AppMenuBar {
    menus: Vec<Entity<AppMenu>>,
    selected_index: Option<usize>,
    action_context: Option<FocusHandle>,
}

impl AppMenuBar {
    /// Create a new app menu bar.
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let mut this = Self {
                selected_index: None,
                action_context: None,
                menus: Vec::new(),
            };
            this.reload(cx);
            this
        })
    }

    /// Reload the menus from the app.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let menu_bar = cx.entity();
        let menus: Vec<OwnedMenu> = GlobalState::global(cx)
            .app_menus()
            .iter()
            .cloned()
            .collect();
        self.menus = menus
            .iter()
            .enumerate()
            .map(|(ix, menu)| AppMenu::new(ix, menu, menu_bar.clone(), cx))
            .collect();
        self.selected_index = None;
        self.action_context = None;
        cx.notify();
    }

    fn on_move_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected_index) = self.selected_index else {
            return;
        };

        if let Some(new_ix) = self.adjacent_enabled_index(selected_index, -1, cx) {
            self.set_selected_index(Some(new_ix), window, cx);
        }
    }

    fn on_move_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected_index) = self.selected_index else {
            return;
        };

        if let Some(new_ix) = self.adjacent_enabled_index(selected_index, 1, cx) {
            self.set_selected_index(Some(new_ix), window, cx);
        }
    }

    fn on_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.set_selected_index(None, window, cx);
    }

    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_index.is_none() && ix.is_some() {
            self.action_context = window.focused(cx);
        } else if ix.is_none() {
            if let Some(action_context) = self.action_context.as_ref() {
                action_context.focus(window, cx);
            }
            self.action_context = None;
        }

        self.selected_index = ix;
        cx.notify();
    }

    /// Returns the next enabled top-level menu in the requested direction.
    ///
    /// The search wraps once and remains bounded when every menu is disabled.
    fn adjacent_enabled_index(
        &self,
        selected_index: usize,
        direction: isize,
        cx: &App,
    ) -> Option<usize> {
        let menu_count = self.menus.len();
        if menu_count == 0 {
            return None;
        }

        (1..=menu_count)
            .map(|distance| {
                (selected_index as isize + direction * distance as isize)
                    .rem_euclid(menu_count as isize) as usize
            })
            .find(|index| !self.menus[*index].read(cx).menu.disabled)
    }

    #[inline]
    fn has_activated_menu(&self) -> bool {
        self.selected_index.is_some()
    }
}

impl Render for AppMenuBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let metrics = cx.theme().style.controls.sm;

        h_flex()
            .id("app-menu-bar")
            .role(Role::MenuBar)
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_move_left))
            .on_action(cx.listener(Self::on_move_right))
            .on_action(cx.listener(Self::on_cancel))
            .h(metrics.height)
            .w_auto()
            .max_w_full()
            .flex_none()
            .gap(metrics.gap)
            .overflow_x_scroll()
            .restrict_scroll_to_axis()
            .children(self.menus.clone())
    }
}

/// A menu in the menu bar.
pub(super) struct AppMenu {
    menu_bar: Entity<AppMenuBar>,
    ix: usize,
    name: SharedString,
    menu: OwnedMenu,
    popup_menu: Option<Entity<PopupMenu>>,

    _subscription: Option<Subscription>,
}

impl AppMenu {
    pub(super) fn new(
        ix: usize,
        menu: &OwnedMenu,
        menu_bar: Entity<AppMenuBar>,
        cx: &mut App,
    ) -> Entity<Self> {
        let name = menu.name.clone();
        cx.new(|_| Self {
            ix,
            menu_bar,
            name,
            menu: menu.clone(),
            popup_menu: None,
            _subscription: None,
        })
    }

    fn is_selected(&self, cx: &App) -> bool {
        self.menu_bar.read(cx).selected_index == Some(self.ix)
    }

    fn build_popup_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PopupMenu> {
        let action_context = self.menu_bar.read(cx).action_context.clone();
        let popup_menu = match self.popup_menu.as_ref() {
            None => {
                let items = self.menu.items.clone();
                let popup_menu = PopupMenu::build(window, cx, |menu, window, cx| {
                    menu.with_menu_items(items, window, cx)
                });
                popup_menu.update(cx, |menu, cx| {
                    menu.set_action_context(action_context.clone(), cx);
                });
                self._subscription =
                    Some(cx.subscribe_in(&popup_menu, window, Self::handle_dismiss));
                self.popup_menu = Some(popup_menu.clone());

                popup_menu
            }
            Some(menu) => {
                menu.update(cx, |menu, cx| {
                    menu.set_action_context(action_context.clone(), cx);
                });
                menu.clone()
            }
        };

        let focus_handle = popup_menu.read(cx).focus_handle(cx);
        if !focus_handle.contains_focused(window, cx) {
            focus_handle.focus(window, cx);
        }

        popup_menu
    }

    fn handle_dismiss(
        &mut self,
        _: &Entity<PopupMenu>,
        _: &DismissEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._subscription.take();
        self.popup_menu.take();
        self.menu_bar.update(cx, |state, cx| {
            state.on_cancel(&Cancel, window, cx);
        });
    }

    fn handle_trigger_click(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A menu trigger lives inside the draggable title bar. Keep its click,
        // including a rapid second click, from reaching TitleBar's double-click
        // handler and maximizing or restoring the window.
        cx.stop_propagation();

        if matches!(event, ClickEvent::Mouse(_)) {
            return;
        }

        self.toggle(window, cx);
    }

    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.disabled {
            return;
        }

        let is_selected = self.is_selected(cx);
        _ = self.menu_bar.update(cx, |state, cx| {
            let new_ix = if is_selected { None } else { Some(self.ix) };
            state.set_selected_index(new_ix, window, cx);
        });
    }

    fn handle_hover(&mut self, hovered: &bool, window: &mut Window, cx: &mut Context<Self>) {
        if !*hovered || self.menu.disabled {
            return;
        }

        let has_activated_menu = self.menu_bar.read(cx).has_activated_menu();
        if !has_activated_menu {
            return;
        }

        _ = self.menu_bar.update(cx, |state, cx| {
            state.set_selected_index(Some(self.ix), window, cx);
        });
    }
}

impl Render for AppMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_selected = self.is_selected(cx);
        let is_disabled = self.menu.disabled;
        let side_offset = cx.theme().style.overlays.side_offset;
        let trigger_radius = cx.theme().style.radii.md;
        let trigger_hover = cx.theme().tokens.muted.background.opacity(0.65);
        let trigger_active = cx.theme().tokens.muted.background;
        let trigger_foreground = cx.theme().foreground;

        div()
            .id(self.ix)
            .relative()
            .rounded(trigger_radius)
            .when(!is_disabled && !is_selected, |this| {
                this.hover(|this| this.bg(trigger_hover))
            })
            .when(is_selected, |this| this.bg(trigger_active))
            .child(
                Button::new("menu")
                    .small()
                    .ghost()
                    .label(self.name.clone())
                    .selected(is_selected)
                    .disabled(is_disabled)
                    .aria_expanded(is_selected)
                    .accessibility_role(Role::MenuItem)
                    .pressed_offset(false)
                    // The menu item owns hover/open backgrounds so those
                    // states remain distinct from the generic Ghost variant.
                    .bg(cx.theme().transparent)
                    .when(is_selected, |this| this.text_color(trigger_foreground))
                    .when(!is_disabled, |this| {
                        this.on_mouse_down(
                            MouseButton::Left,
                            window.listener_for(&cx.entity(), move |this, _, window, cx| {
                                // Stop propagation to avoid dragging the window.
                                window.prevent_default();
                                cx.stop_propagation();
                                this.toggle(window, cx);
                            }),
                        )
                    })
                    .on_click(cx.listener(Self::handle_trigger_click)),
            )
            .on_hover(cx.listener(Self::handle_hover))
            .when(is_selected, |this| {
                this.child(deferred(
                    anchored()
                        .anchor(gpui::Anchor::TopLeft)
                        .snap_to_window_with_margin(px(8.))
                        .child(
                            div()
                                .size_full()
                                .occlude()
                                .top(side_offset)
                                .child(self.build_popup_menu(window, cx)),
                        ),
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InteractiveElementExt as _;

    use std::{cell::Cell, rc::Rc};

    use gpui::{Modifiers, MouseDownEvent, MouseUpEvent, TestAppContext, VisualTestContext, point};

    /// Creates a minimal top-level menu for keyboard-navigation tests.
    fn owned_menu(name: &str, disabled: bool) -> OwnedMenu {
        OwnedMenu {
            name: name.into(),
            items: Vec::new(),
            disabled,
        }
    }

    struct TestRoot {
        menu_bar: Entity<AppMenuBar>,
        first_focus: FocusHandle,
        second_focus: FocusHandle,
    }

    struct DoubleClickRoot {
        menu_bar: Entity<AppMenuBar>,
        parent_double_clicks: Rc<Cell<usize>>,
    }

    impl Render for DoubleClickRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let parent_double_clicks = self.parent_double_clicks.clone();
            div()
                .id("double-click-root")
                .size_full()
                .on_double_click(move |_, _, _| {
                    parent_double_clicks.set(parent_double_clicks.get() + 1)
                })
                .child(self.menu_bar.clone())
        }
    }

    impl Render for TestRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .child(div().id("first").track_focus(&self.first_focus))
                .child(div().id("second").track_focus(&self.second_focus))
                .child(self.menu_bar.clone())
        }
    }

    #[gpui::test]
    fn preserves_action_context_while_switching_menus(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(crate::Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| {
            let first_focus = cx.focus_handle();
            let second_focus = cx.focus_handle();
            first_focus.focus(window, cx);

            TestRoot {
                menu_bar: cx.new(|_| AppMenuBar {
                    menus: Vec::new(),
                    selected_index: None,
                    action_context: None,
                }),
                first_focus,
                second_focus,
            }
        });

        let (menu_bar, first_focus, second_focus) = root.read_with(cx, |root, _| {
            (
                root.menu_bar.clone(),
                root.first_focus.clone(),
                root.second_focus.clone(),
            )
        });

        menu_bar.update_in(cx, |menu_bar, window, cx| {
            menu_bar.set_selected_index(Some(0), window, cx);
            assert_eq!(menu_bar.action_context.as_ref(), Some(&first_focus));

            second_focus.focus(window, cx);
            menu_bar.set_selected_index(Some(1), window, cx);
            assert_eq!(menu_bar.action_context.as_ref(), Some(&first_focus));

            menu_bar.set_selected_index(None, window, cx);
            assert!(menu_bar.action_context.is_none());
            assert_eq!(window.focused(cx).as_ref(), Some(&first_focus));

            second_focus.focus(window, cx);
            menu_bar.set_selected_index(Some(0), window, cx);
            assert_eq!(menu_bar.action_context.as_ref(), Some(&second_focus));
        });
    }

    #[gpui::test]
    fn keyboard_navigation_skips_disabled_menus(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(crate::Theme::default()));
        let (root, cx) = cx.add_window_view(|_, cx| {
            let menu_bar = cx.new(|_| AppMenuBar {
                menus: Vec::new(),
                selected_index: None,
                action_context: None,
            });
            let source_menus = [
                owned_menu("File", false),
                owned_menu("Edit", true),
                owned_menu("View", false),
            ];
            let menus = source_menus
                .iter()
                .enumerate()
                .map(|(index, menu)| AppMenu::new(index, menu, menu_bar.clone(), cx))
                .collect();
            menu_bar.update(cx, |menu_bar, _| menu_bar.menus = menus);

            TestRoot {
                menu_bar,
                first_focus: cx.focus_handle(),
                second_focus: cx.focus_handle(),
            }
        });

        let menu_bar = root.read_with(cx, |root, _| root.menu_bar.clone());
        menu_bar.update_in(cx, |menu_bar, _, cx| {
            assert_eq!(menu_bar.adjacent_enabled_index(0, 1, cx), Some(2));
            assert_eq!(menu_bar.adjacent_enabled_index(2, -1, cx), Some(0));
        });
    }

    #[gpui::test]
    fn keyboard_navigation_stops_when_every_menu_is_disabled(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(crate::Theme::default()));
        let (root, cx) = cx.add_window_view(|_, cx| {
            let menu_bar = cx.new(|_| AppMenuBar {
                menus: Vec::new(),
                selected_index: None,
                action_context: None,
            });
            let source_menus = [owned_menu("File", true), owned_menu("Edit", true)];
            let menus = source_menus
                .iter()
                .enumerate()
                .map(|(index, menu)| AppMenu::new(index, menu, menu_bar.clone(), cx))
                .collect();
            menu_bar.update(cx, |menu_bar, _| menu_bar.menus = menus);

            TestRoot {
                menu_bar,
                first_focus: cx.focus_handle(),
                second_focus: cx.focus_handle(),
            }
        });

        let menu_bar = root.read_with(cx, |root, _| root.menu_bar.clone());
        menu_bar.update_in(cx, |menu_bar, _, cx| {
            assert_eq!(menu_bar.adjacent_enabled_index(0, 1, cx), None);
        });
    }

    #[gpui::test]
    fn rapid_trigger_click_does_not_reach_title_bar_double_click(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(crate::Theme::default());
            cx.set_global(GlobalState::new());
        });
        let parent_double_clicks = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let parent_double_clicks = parent_double_clicks.clone();
            move |_, cx| {
                let menu_bar = cx.new(|_| AppMenuBar {
                    menus: Vec::new(),
                    selected_index: None,
                    action_context: None,
                });
                let source_menu = owned_menu("File", false);
                let menus = vec![AppMenu::new(0, &source_menu, menu_bar.clone(), cx)];
                menu_bar.update(cx, |menu_bar, _| menu_bar.menus = menus);

                DoubleClickRoot {
                    menu_bar,
                    parent_double_clicks,
                }
            }
        });
        let cx: &mut VisualTestContext = cx;
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        let position = point(px(10.), px(10.));
        cx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position,
            modifiers: Modifiers::default(),
            click_count: 2,
            first_mouse: false,
        });
        // AppMenu opens on mouse down, so force the same tree change that
        // occurs between the native down and up events in the Story window.
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        cx.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position,
            modifiers: Modifiers::default(),
            click_count: 2,
        });

        assert_eq!(parent_double_clicks.get(), 0);
    }
}
