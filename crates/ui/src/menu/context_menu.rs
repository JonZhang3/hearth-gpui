use std::{cell::RefCell, rc::Rc};

use gpui::{
    Anchor, AnyElement, App, Context, DismissEvent, Element, ElementId, Entity, Focusable,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Pixels, Point, StyleRefinement, Styled,
    Subscription, Window, anchored, deferred, div, prelude::FluentBuilder, px,
};

use crate::{
    ActiveTheme as _,
    animation::{OverlayLifecycle, OverlayPhase, Transition, effective_motion_duration},
    menu::PopupMenu,
};

type OpenChangeCallback = Rc<dyn Fn(&bool, &mut Window, &mut App)>;
type MeasureCallback = Box<dyn FnOnce(&mut Window, &mut App)>;

/// A extension trait for adding a context menu to an element.
pub trait ContextMenuExt: InteractiveElement + ParentElement + Styled {
    /// Add a context menu to the element.
    ///
    /// This will changed the element to be `relative` positioned, and add a child `ContextMenu` element.
    /// Because the `ContextMenu` element is positioned `absolute`, it will not affect the layout of the parent element.
    #[track_caller]
    fn context_menu(
        mut self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> ContextMenu<Self>
    where
        Self: Sized,
    {
        // The ID must be stable across renders, otherwise the element state
        // (open menu) is lost on every re-render.
        let caller = std::panic::Location::caller();
        let id = self
            .interactivity()
            .element_id
            .clone()
            .map(|id| ElementId::Name(format!("context-menu-{:?}", id).into()))
            .unwrap_or_else(|| ElementId::CodeLocation(*caller));
        ContextMenu::new(id, self).menu(f)
    }
}

impl<E: InteractiveElement + ParentElement + Styled> ContextMenuExt for E {}

/// A context menu that can be shown on right-click.
pub struct ContextMenu<E: ParentElement + Styled + Sized> {
    id: ElementId,
    element: Option<E>,
    menu: Option<Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>>,
    // This is not in use, just for style refinement forwarding.
    _ignore_style: StyleRefinement,
    anchor: Anchor,
    on_open_change: Option<OpenChangeCallback>,
}

impl<E: ParentElement + Styled> ContextMenu<E> {
    /// Create a new context menu with the given ID.
    pub fn new(id: impl Into<ElementId>, element: E) -> Self {
        Self {
            id: id.into(),
            element: Some(element),
            menu: None,
            anchor: Anchor::TopLeft,
            on_open_change: None,
            _ignore_style: StyleRefinement::default(),
        }
    }

    /// Build the context menu using the given builder function.
    #[must_use]
    fn menu<F>(mut self, builder: F) -> Self
    where
        F: Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    {
        self.menu = Some(Rc::new(builder));
        self
    }

    /// Sets a callback that receives each effective open-state change.
    pub fn on_open_change(
        mut self,
        callback: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    fn with_element_state<R>(
        &mut self,
        id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut ContextMenuState, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<ContextMenuState, _>(
            Some(id),
            |element_state, window| {
                let mut element_state = element_state.unwrap().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
    }
}

impl<E: ParentElement + Styled> ParentElement for ContextMenu<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E: ParentElement + Styled> Styled for ContextMenu<E> {
    fn style(&mut self) -> &mut StyleRefinement {
        if let Some(element) = &mut self.element {
            element.style()
        } else {
            &mut self._ignore_style
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> IntoElement for ContextMenu<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct ContextMenuSharedState {
    menu_view: Option<Entity<PopupMenu>>,
    lifecycle: OverlayLifecycle,
    position: Point<Pixels>,
    request_generation: u64,
    measurement_generation: Option<u64>,
    _subscription: Option<Subscription>,
}

impl ContextMenuSharedState {
    /// Starts an interruptible open or close transition and refreshes the window on completion.
    fn set_open(
        shared_state: &Rc<RefCell<Self>>,
        open: bool,
        on_open_change: Option<&OpenChangeCallback>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let transition = {
            let mut state = shared_state.borrow_mut();
            if open {
                state.lifecycle.begin_open()
            } else {
                state.lifecycle.begin_close()
            }
        };
        let Some(transition) = transition else {
            return;
        };

        if let Some(callback) = on_open_change {
            callback(&open, window, cx);
        }

        window.refresh();
        let duration = effective_motion_duration(cx.theme().style.motion.fast(), cx);
        let shared_state = shared_state.clone();
        window
            .spawn(cx, async move |cx| {
                cx.background_executor().timer(duration).await;
                let completed = {
                    let mut state = shared_state.borrow_mut();
                    if open {
                        state.lifecycle.complete_open(transition)
                    } else {
                        let completed = state.lifecycle.complete_close(transition);
                        if completed {
                            state.menu_view = None;
                            state.measurement_generation = None;
                            state._subscription = None;
                        }
                        completed
                    }
                };
                if completed {
                    cx.update(|window, _| window.refresh()).ok();
                }
            })
            .detach();
    }

    /// Accepts a width measurement only for the latest unopened menu request.
    fn complete_measurement(
        shared_state: &Rc<RefCell<Self>>,
        generation: u64,
        on_open_change: Option<&OpenChangeCallback>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let accepted = {
            let mut state = shared_state.borrow_mut();
            if state.request_generation != generation
                || state.measurement_generation != Some(generation)
                || state.lifecycle.phase() != OverlayPhase::Closed
            {
                false
            } else {
                state.measurement_generation = None;
                true
            }
        };

        if accepted {
            Self::set_open(shared_state, true, on_open_change, window, cx);
        }
    }
}

pub struct ContextMenuState {
    element: Option<AnyElement>,
    shared_state: Rc<RefCell<ContextMenuSharedState>>,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            element: None,
            shared_state: Rc::new(RefCell::new(ContextMenuSharedState {
                menu_view: None,
                lifecycle: OverlayLifecycle::default(),
                position: Default::default(),
                request_generation: 0,
                measurement_generation: None,
                _subscription: None,
            })),
        }
    }
}

/// Measures a menu subtree without prepainting, painting, or exposing it to accessibility APIs.
struct ContextMenuMeasurement {
    child: Option<AnyElement>,
    on_measure: Option<MeasureCallback>,
}

impl ContextMenuMeasurement {
    /// Creates a one-frame intrinsic-width probe for a menu subtree.
    fn new(
        child: impl IntoElement,
        on_measure: impl FnOnce(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            child: Some(child.into_any_element()),
            on_measure: Some(Box::new(on_measure)),
        }
    }
}

impl Element for ContextMenuMeasurement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let mut child = self.child.take().expect("measurement child should exist");
        let layout_id = child.request_layout(window, cx);
        (layout_id, child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(on_measure) = self.on_measure.take() {
            on_measure(window, cx);
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        _: &mut Window,
        _: &mut App,
    ) {
    }
}

impl IntoElement for ContextMenuMeasurement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> Element for ContextMenu<E> {
    type RequestLayoutState = ContextMenuState;
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let anchor = self.anchor;
        let on_open_change = self.on_open_change.clone();

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |this, state: &mut ContextMenuState, window, cx| {
                let (position, phase, animation_key, measurement_generation) = {
                    let shared_state = state.shared_state.borrow();
                    (
                        shared_state.position,
                        shared_state.lifecycle.phase(),
                        shared_state.lifecycle.animation_key(),
                        shared_state.measurement_generation,
                    )
                };
                let menu_view = state.shared_state.borrow().menu_view.clone();
                let mut menu_element = None;
                if let (OverlayPhase::Closed, Some(generation), Some(menu)) =
                    (phase, measurement_generation, menu_view.clone())
                {
                    let shared_state = state.shared_state.clone();
                    let on_open_change = on_open_change.clone();
                    let measurement = ContextMenuMeasurement::new(menu, move |window, cx| {
                        ContextMenuSharedState::complete_measurement(
                            &shared_state,
                            generation,
                            on_open_change.as_ref(),
                            window,
                            cx,
                        );
                    });
                    menu_element = Some(
                        deferred(
                            anchored().child(
                                div()
                                    .w(window.bounds().size.width)
                                    .h(window.bounds().size.height)
                                    .child(
                                        anchored()
                                            .position(position)
                                            .snap_to_window_with_margin(px(8.))
                                            .anchor(anchor)
                                            .child(measurement),
                                    ),
                            ),
                        )
                        .with_priority(1)
                        .into_any(),
                    );
                } else if phase != OverlayPhase::Closed {
                    let has_menu_item = menu_view
                        .as_ref()
                        .map(|menu| !menu.read(cx).is_empty())
                        .unwrap_or(false);

                    if has_menu_item {
                        let closing = phase == OverlayPhase::Closing;
                        let motion = cx.theme().style.motion;
                        let offset = cx.theme().style.overlays.side_offset;
                        let menu_surface = div()
                            .relative()
                            .when_some(menu_view, |this, menu| {
                                // Focus only while the menu accepts input.
                                if !closing && !menu.focus_handle(cx).contains_focused(window, cx) {
                                    menu.focus_handle(cx).focus(window, cx);
                                }

                                this.child(menu.clone())
                            })
                            .when(closing, |this| {
                                this.child(div().absolute().top_0().left_0().size_full().occlude())
                            });
                        let menu_surface = Transition::new(motion.fast())
                            .ease_token(if closing {
                                motion.exit_easing
                            } else {
                                motion.enter_easing
                            })
                            .slide_y(
                                if closing { px(0.) } else { -offset },
                                if closing { -offset } else { px(0.) },
                            )
                            .apply(
                                menu_surface,
                                ElementId::NamedInteger(
                                    "context-menu-motion".into(),
                                    animation_key,
                                ),
                            );
                        menu_element = Some(
                            deferred(
                                anchored().child(
                                    div()
                                        .w(window.bounds().size.width)
                                        .h(window.bounds().size.height)
                                        .on_scroll_wheel(|_, _, cx| {
                                            cx.stop_propagation();
                                        })
                                        .child(
                                            anchored()
                                                .position(position)
                                                .snap_to_window_with_margin(px(8.))
                                                .anchor(anchor)
                                                .child(menu_surface),
                                        ),
                                ),
                            )
                            .with_priority(1)
                            .into_any(),
                        );
                    }
                }

                let mut element = this
                    .element
                    .take()
                    .expect("Element should exists.")
                    .children(menu_element)
                    .into_any_element();

                let layout_id = element.request_layout(window, cx);

                (
                    layout_id,
                    ContextMenuState {
                        element: Some(element),
                        ..Default::default()
                    },
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(element) = &mut request_layout.element {
            element.prepaint(window, cx);
        }
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(element) = &mut request_layout.element {
            element.paint(window, cx);
        }

        // Take the builder before setting up element state to avoid borrow issues
        let builder = self.menu.clone();
        let on_open_change = self.on_open_change.clone();

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |_view, state: &mut ContextMenuState, window, _| {
                let shared_state = state.shared_state.clone();

                let hitbox = hitbox.clone();
                // When right mouse click, to build content menu, and show it at the mouse position.
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase.bubble()
                        && event.button == MouseButton::Right
                        && hitbox.is_hovered(window)
                    {
                        // Capture the focused element to restore focus to on dismiss.
                        // If focus is still on the previous menu, keep its captured focus.
                        let previous_focus_handle = window.focused(cx).and_then(|focused| {
                            let shared_state = shared_state.borrow();
                            match shared_state.menu_view.as_ref() {
                                Some(menu) if menu.read(cx).focus_handle == focused => {
                                    menu.read(cx).previous_focus_handle.clone()
                                }
                                _ => Some(focused),
                            }
                        });

                        {
                            let mut shared_state = shared_state.borrow_mut();
                            shared_state.position = event.position;
                            shared_state.request_generation =
                                shared_state.request_generation.wrapping_add(1);
                        }
                        let request_generation = shared_state.borrow().request_generation;
                        cx.stop_propagation();

                        // Build after event dispatch so entity updates never overlap input capture.
                        window.defer(cx, {
                            let shared_state = shared_state.clone();
                            let builder = builder.clone();
                            let on_open_change = on_open_change.clone();
                            move |window, cx| {
                                let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                                    let Some(build) = &builder else {
                                        return menu;
                                    };
                                    build(menu, window, cx)
                                });

                                if shared_state.borrow().request_generation != request_generation {
                                    return;
                                }

                                if menu.read(cx).is_empty() {
                                    ContextMenuSharedState::set_open(
                                        &shared_state,
                                        false,
                                        on_open_change.as_ref(),
                                        window,
                                        cx,
                                    );
                                    return;
                                }

                                menu.update(cx, |menu, cx| {
                                    menu.set_previous_focus(previous_focus_handle, cx);
                                });

                                // Set up the subscription for dismiss handling
                                let _subscription = window.subscribe(&menu, cx, {
                                    let shared_state = shared_state.clone();
                                    let on_open_change = on_open_change.clone();
                                    move |_, _: &DismissEvent, window, cx| {
                                        window.defer(cx, {
                                            let shared_state = shared_state.clone();
                                            let on_open_change = on_open_change.clone();
                                            move |window, cx| {
                                                if shared_state.borrow().request_generation
                                                    != request_generation
                                                {
                                                    return;
                                                }
                                                ContextMenuSharedState::set_open(
                                                    &shared_state,
                                                    false,
                                                    on_open_change.as_ref(),
                                                    window,
                                                    cx,
                                                );
                                            }
                                        });
                                    }
                                });

                                // A closed menu is measured before it becomes visible. Reopened
                                // menus already have a painted surface and may reverse immediately.
                                let should_measure = {
                                    let mut state = shared_state.borrow_mut();
                                    let should_measure =
                                        state.lifecycle.phase() == OverlayPhase::Closed;
                                    state.menu_view = Some(menu.clone());
                                    state.measurement_generation =
                                        should_measure.then_some(request_generation);
                                    state._subscription = Some(_subscription);
                                    should_measure
                                };
                                window.refresh();

                                if !should_measure {
                                    ContextMenuSharedState::set_open(
                                        &shared_state,
                                        true,
                                        on_open_change.as_ref(),
                                        window,
                                        cx,
                                    );
                                }
                            }
                        });
                    }
                });
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use gpui::{
        AppContext as _, Context, FocusHandle, IntoElement, Render, TestAppContext,
        VisualTestContext, actions, point, px,
    };
    use std::cell::Cell;
    use std::time::Duration;

    actions!(context_menu_test, [RemoveTab]);

    /// The regression shape: the action handler lives on the trigger's
    /// ancestor (like an action bar), which is NOT on the focus path while
    /// focus is in the content area.
    struct TestRoot {
        content_focus: FocusHandle,
        received: Rc<Cell<bool>>,
    }

    impl Render for TestRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let received = self.received.clone();
            div()
                .size_full()
                .child(
                    div()
                        .id("content")
                        .h(px(40.))
                        .track_focus(&self.content_focus),
                )
                .child(
                    div()
                        .id("action-bar")
                        .h(px(60.))
                        .on_action(move |_: &RemoveTab, _, _| received.set(true))
                        .child(
                            div()
                                .id("tab")
                                .size_full()
                                .context_menu(|menu, _, _| menu.menu("Close", Box::new(RemoveTab))),
                        ),
                )
        }
    }

    #[gpui::test]
    fn action_bubbles_from_trigger_and_focus_restores_on_dismiss(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(Theme::default());
            super::super::popup_menu::init(cx);
        });

        let received = Rc::new(Cell::new(false));
        let (root, cx) = cx.add_window_view({
            let received = received.clone();
            move |window, cx| {
                let content_focus = cx.focus_handle();
                content_focus.focus(window, cx);
                TestRoot {
                    content_focus,
                    received,
                }
            }
        });
        let content_focus = root.read_with(cx, |root, _| root.content_focus.clone());
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        // Right-click inside the tab to open the context menu.
        cx.simulate_event(MouseDownEvent {
            button: MouseButton::Right,
            position: point(px(50.), px(70.)),
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        // The menu entity is built in a deferred callback, then rendered
        // (which also focuses it) on the next draw.
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        // Select "Close" and confirm. Keyboard confirm and mouse click share
        // the same `confirm` path in `PopupMenu`.
        cx.simulate_keystrokes("down enter");
        cx.run_until_parked();

        // The action must reach the handler on the trigger's ancestor chain,
        // even though the action bar was never on the focus path.
        assert!(received.get());
        // And dismiss must restore focus to where it was before the menu
        // opened, keeping the dangling-focus fix (#2614).
        cx.update(|window, cx| {
            assert_eq!(window.focused(cx).as_ref(), Some(&content_focus));
        });
    }

    #[gpui::test]
    fn reopen_invalidates_pending_context_menu_close(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::default()));
        let cx = cx.add_empty_window();
        let shared_state = Rc::new(RefCell::new(ContextMenuSharedState {
            menu_view: None,
            lifecycle: OverlayLifecycle::default(),
            position: Point::default(),
            request_generation: 0,
            measurement_generation: None,
            _subscription: None,
        }));

        cx.update(|window, cx| {
            ContextMenuSharedState::set_open(&shared_state, true, None, window, cx);
        });
        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();
        assert_eq!(shared_state.borrow().lifecycle.phase(), OverlayPhase::Open);

        cx.update(|window, cx| {
            ContextMenuSharedState::set_open(&shared_state, false, None, window, cx);
            ContextMenuSharedState::set_open(&shared_state, true, None, window, cx);
        });
        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();

        assert_eq!(shared_state.borrow().lifecycle.phase(), OverlayPhase::Open);
    }

    #[gpui::test]
    fn measurement_opens_only_the_current_context_menu_request(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::default()));
        let cx = cx.add_empty_window();
        let shared_state = Rc::new(RefCell::new(ContextMenuSharedState {
            menu_view: None,
            lifecycle: OverlayLifecycle::default(),
            position: Point::default(),
            request_generation: 2,
            measurement_generation: Some(2),
            _subscription: None,
        }));

        cx.update(|window, cx| {
            ContextMenuSharedState::complete_measurement(&shared_state, 1, None, window, cx);
        });
        assert_eq!(
            shared_state.borrow().lifecycle.phase(),
            OverlayPhase::Closed
        );

        cx.update(|window, cx| {
            ContextMenuSharedState::complete_measurement(&shared_state, 2, None, window, cx);
        });
        let state = shared_state.borrow();
        assert_eq!(state.lifecycle.phase(), OverlayPhase::Opening);
        assert_eq!(state.measurement_generation, None);
    }

    struct LifecycleFixture {
        build_count: Rc<Cell<usize>>,
        open_changes: Rc<RefCell<Vec<bool>>>,
        empty: bool,
    }

    impl Render for LifecycleFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let build_count = self.build_count.clone();
            let open_changes = self.open_changes.clone();
            let empty = self.empty;

            div()
                .id("context-menu-lifecycle-trigger")
                .w(px(400.))
                .h(px(200.))
                .context_menu(move |menu, _, _| {
                    build_count.set(build_count.get() + 1);
                    if empty { menu } else { menu.label("Item") }
                })
                .on_open_change(move |open, _, _| open_changes.borrow_mut().push(*open))
        }
    }

    #[gpui::test]
    fn repeated_right_click_repositions_without_close_reopen_cycle(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let build_count = Rc::new(Cell::new(0));
        let open_changes = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let build_count = build_count.clone();
            let open_changes = open_changes.clone();
            move |_, _| LifecycleFixture {
                build_count,
                open_changes,
                empty: false,
            }
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        for position in [point(px(40.), px(40.)), point(px(300.), px(40.))] {
            cx.simulate_event(MouseDownEvent {
                button: MouseButton::Right,
                position,
                modifiers: Default::default(),
                click_count: 1,
                first_mouse: false,
            });
            cx.run_until_parked();
            cx.update(|window, cx| _ = window.draw(cx));
        }

        assert_eq!(build_count.get(), 2);
        assert_eq!(*open_changes.borrow(), vec![true]);
    }

    #[gpui::test]
    fn empty_context_menu_does_not_enter_open_state(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let build_count = Rc::new(Cell::new(0));
        let open_changes = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let build_count = build_count.clone();
            let open_changes = open_changes.clone();
            move |_, _| LifecycleFixture {
                build_count,
                open_changes,
                empty: true,
            }
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| _ = window.draw(cx));
        cx.simulate_event(MouseDownEvent {
            button: MouseButton::Right,
            position: point(px(40.), px(40.)),
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        cx.run_until_parked();

        assert_eq!(build_count.get(), 1);
        assert!(open_changes.borrow().is_empty());
    }

    #[gpui::test]
    fn completed_close_releases_the_retained_menu(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let menu = cx.update(|_, cx| cx.new(|cx| PopupMenu::new(cx).label("Item")));
        let shared_state = Rc::new(RefCell::new(ContextMenuSharedState {
            menu_view: Some(menu),
            lifecycle: OverlayLifecycle::default(),
            position: Point::default(),
            request_generation: 0,
            measurement_generation: None,
            _subscription: None,
        }));

        cx.update(|window, cx| {
            ContextMenuSharedState::set_open(&shared_state, true, None, window, cx);
        });
        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();
        cx.update(|window, cx| {
            ContextMenuSharedState::set_open(&shared_state, false, None, window, cx);
        });
        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();

        assert_eq!(
            shared_state.borrow().lifecycle.phase(),
            OverlayPhase::Closed
        );
        assert!(shared_state.borrow().menu_view.is_none());
    }
}
