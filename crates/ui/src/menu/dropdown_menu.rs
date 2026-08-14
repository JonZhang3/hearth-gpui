// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `side_offset`.
// - Added or exposed behavior through `dropdown_expanded`, `side_offset`,
//   `repeated_trigger_click_closes_without_reopening`.
// - Reworked Dropdown Menu around accessibility semantics and ARIA state, focus-visible and focus
//   restoration behavior.
use std::rc::Rc;

use gpui::{
    Anchor, Context, DismissEvent, ElementId, Entity, Focusable, InteractiveElement, IntoElement,
    Pixels, RenderOnce, SharedString, StyleRefinement, Styled, Window, prelude::FluentBuilder as _,
};

use crate::{Selectable, button::Button, menu::PopupMenu, popover::Popover};

/// A dropdown menu trait for buttons and other interactive elements
pub trait DropdownMenu: Styled + Selectable + InteractiveElement + IntoElement + 'static {
    /// Applies the resolved open state to the concrete trigger.
    ///
    /// The default keeps existing composite triggers selected while open. Triggers that own an
    /// exposed accessibility node can override this method to publish `aria-expanded` as well.
    fn dropdown_expanded(self, expanded: bool) -> Self {
        let selected = self.is_selected();
        self.selected(selected || expanded)
    }

    /// Create a dropdown menu with the given items, anchored to the TopLeft corner
    fn dropdown_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> DropdownMenuPopover<Self> {
        self.dropdown_menu_with_anchor(Anchor::TopLeft, f)
    }

    /// Create a dropdown menu with the given items, anchored to the given corner
    fn dropdown_menu_with_anchor(
        mut self,
        anchor: impl Into<Anchor>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> DropdownMenuPopover<Self> {
        let style = self.style().clone();
        let id = self.interactivity().element_id.clone();

        DropdownMenuPopover::new(id.unwrap_or(0.into()), anchor, self, f).trigger_style(style)
    }
}

impl DropdownMenu for Button {
    fn dropdown_expanded(self, expanded: bool) -> Self {
        let selected = self.is_selected();
        self.aria_expanded(expanded).selected(selected || expanded)
    }
}

#[derive(IntoElement)]
pub struct DropdownMenuPopover<T: DropdownMenu> {
    id: ElementId,
    style: StyleRefinement,
    anchor: Anchor,
    side_offset: Option<Pixels>,
    trigger: T,
    builder: Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>,
}

impl<T> DropdownMenuPopover<T>
where
    T: DropdownMenu,
{
    fn new(
        id: ElementId,
        anchor: impl Into<Anchor>,
        trigger: T,
        builder: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        Self {
            id: SharedString::from(format!("dropdown-menu:{:?}", id)).into(),
            style: StyleRefinement::default(),
            anchor: anchor.into(),
            side_offset: None,
            trigger,
            builder: Rc::new(builder),
        }
    }

    /// Set the anchor corner for the dropdown menu popover.
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Set the distance between the dropdown menu and its trigger.
    pub fn side_offset(mut self, offset: Pixels) -> Self {
        self.side_offset = Some(offset);
        self
    }

    /// Set the style refinement for the dropdown menu trigger.
    fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.style = style;
        self
    }
}

#[derive(Default)]
struct DropdownMenuState {
    menu: Option<Entity<PopupMenu>>,
    open: bool,
}

impl<T> RenderOnce for DropdownMenuPopover<T>
where
    T: DropdownMenu,
{
    fn render(self, window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let builder = self.builder.clone();
        let menu_state =
            window.use_keyed_state(self.id.clone(), cx, |_, _| DropdownMenuState::default());

        let open_state = menu_state.clone();
        let close_state = menu_state.clone();

        Popover::new(SharedString::from(format!("popover:{}", self.id)))
            .appearance(false)
            .overlay_closable(false)
            .on_open_change(move |open, _, cx| {
                if !open {
                    open_state.update(cx, |state, _| state.open = false);
                }
            })
            .on_close_complete(move |_, cx| {
                close_state.update(cx, |state, _| {
                    state.menu = None;
                    state.open = false;
                });
            })
            .trigger_builder(move |expanded, _, _| {
                self.trigger.dropdown_expanded(expanded).into_any_element()
            })
            .trigger_style(self.style)
            .anchor(self.anchor)
            .when_some(self.side_offset, |this, offset| this.side_offset(offset))
            .content(move |popover_state, window, cx| {
                // Here is special logic to only create the PopupMenu once and reuse it.
                // Because this `content` will called in every time render, so we need to store the menu
                // in state to avoid recreating at every render.
                //
                // And we also need to rebuild the menu when it is dismissed, to rebuild menu items
                // dynamically for support `dropdown_menu` method, so we listen for DismissEvent below.
                let menu = match menu_state.read(cx).menu.clone() {
                    Some(menu) => menu,
                    None => {
                        let builder = builder.clone();
                        let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                            builder(menu, window, cx)
                        });
                        menu_state.update(cx, |state, _| {
                            state.menu = Some(menu.clone());
                        });
                        // Listen for dismiss events from the PopupMenu to close the popover.
                        let popover_state = cx.entity();
                        window
                            .subscribe(&menu, cx, move |_, _: &DismissEvent, window, cx| {
                                popover_state.update(cx, |state, cx| {
                                    state.dismiss(window, cx);
                                });
                            })
                            .detach();

                        menu.clone()
                    }
                };

                let becoming_open = popover_state.is_open() && !menu_state.read(cx).open;
                let trigger_bounds = popover_state.trigger_bounds();
                menu.update(cx, |menu, cx| {
                    menu.set_dismiss_exclusion_bounds(trigger_bounds, cx);
                    if becoming_open {
                        menu.prepare_for_open(cx);
                    }
                });

                if becoming_open {
                    menu.focus_handle(cx).focus(window, cx);
                    menu_state.update(cx, |state, _| state.open = true);
                }

                menu.clone()
            })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;

    use super::*;
    use gpui::{
        AppContext as _, Context, Modifiers, ParentElement as _, Render, TestAppContext,
        VisualTestContext, div,
    };

    struct DropdownFixture {
        build_count: Arc<AtomicUsize>,
    }

    impl Render for DropdownFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let build_count = self.build_count.clone();

            div().debug_selector(|| "dropdown-trigger".into()).child(
                Button::new("dropdown-button")
                    .label("Menu")
                    .dropdown_menu(move |menu, _, _| {
                        build_count.fetch_add(1, Ordering::SeqCst);
                        menu.label("Item")
                    }),
            )
        }
    }

    #[gpui::test]
    fn repeated_trigger_click_closes_without_reopening(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let build_count = Arc::new(AtomicUsize::new(0));
        let (_, cx) = cx.add_window_view({
            let build_count = build_count.clone();
            move |window, cx| {
                let fixture = cx.new(|_| DropdownFixture { build_count });
                crate::Root::new(fixture, window, cx)
            }
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        let trigger_bounds = cx
            .debug_bounds("dropdown-trigger")
            .expect("Dropdown trigger bounds should be available after drawing");

        cx.simulate_click(trigger_bounds.center(), Modifiers::default());
        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        assert_eq!(build_count.load(Ordering::SeqCst), 1);

        cx.simulate_click(trigger_bounds.center(), Modifiers::default());
        cx.run_until_parked();
        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        assert_eq!(build_count.load(Ordering::SeqCst), 1);

        cx.simulate_click(trigger_bounds.center(), Modifiers::default());
        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        assert_eq!(build_count.load(Ordering::SeqCst), 2);
    }
}
