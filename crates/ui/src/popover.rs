use gpui::{
    Anchor, AnyElement, App, Bounds, Context, Deferred, DismissEvent, Div, ElementId, EventEmitter,
    FocusHandle, Focusable, InteractiveElement as _, IntoElement, KeyBinding, MouseButton,
    ParentElement, Pixels, Point, Render, RenderOnce, Role, SharedString, Stateful,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription, Window, anchored,
    deferred, div, point, prelude::FluentBuilder as _, px,
};
use std::{cell::Cell, rc::Rc};

use crate::{
    ActiveTheme as _, Density, ElementExt, Selectable, StyledExt as _,
    actions::Cancel,
    animation::{OverlayLifecycle, OverlayPhase, Transition, effective_motion_duration},
    button::Button,
    global_state::GlobalState,
    v_flex,
};

const CONTEXT: &str = "Popover";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))])
}

/// Physical side on which Popover content is displayed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PopoverSide {
    Top,
    Right,
    #[default]
    Bottom,
    Left,
}

/// Cross-axis alignment of Popover content relative to its trigger.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PopoverAlign {
    Start,
    #[default]
    Center,
    End,
}

/// Trigger capability required to expose Popover expanded state on its own accessibility node.
pub trait PopoverTrigger: Selectable + IntoElement {
    /// Applies the Popover expanded state to the rendered trigger node.
    fn popover_expanded(self, expanded: bool) -> Self;
}

impl PopoverTrigger for Button {
    fn popover_expanded(self, expanded: bool) -> Self {
        self.aria_expanded(expanded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PopoverShadow {
    Medium,
    ExtraLarge,
}

/// Popover-only presentation derived from semantic Style Preset values.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PopoverMetrics {
    width: Pixels,
    padding: Pixels,
    gap: Pixels,
    radius: Pixels,
    ring_opacity: f32,
    header_gap: Pixels,
    title_size: Pixels,
    shadow: PopoverShadow,
}

impl PopoverMetrics {
    /// Resolves the pinned shadcn geometry without branching on preset identifiers.
    fn resolve(cx: &App) -> Self {
        let style = &cx.theme().style;
        match style.density {
            Density::Standard => Self {
                width: px(288.),
                padding: px(16.),
                gap: px(16.),
                radius: style.radii.md,
                ring_opacity: 0.1,
                header_gap: px(4.),
                title_size: px(14.),
                shadow: PopoverShadow::Medium,
            },
            Density::Compact => Self {
                width: px(288.),
                padding: px(10.),
                gap: px(10.),
                radius: style.radii.lg,
                ring_opacity: 0.1,
                header_gap: px(2.),
                title_size: px(14.),
                shadow: PopoverShadow::Medium,
            },
            Density::Comfortable => Self {
                width: px(288.),
                padding: px(16.),
                gap: px(16.),
                radius: style.radii.xl,
                ring_opacity: 0.05,
                header_gap: px(4.),
                title_size: px(16.),
                shadow: PopoverShadow::ExtraLarge,
            },
        }
    }
}

/// A popover element that can be triggered by a button or any other element.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    style: StyleRefinement,
    side: PopoverSide,
    align: PopoverAlign,
    default_open: bool,
    open: Option<bool>,
    tracked_focus_handle: Option<FocusHandle>,
    trigger: Option<Box<dyn FnOnce(bool, &Window, &App) -> AnyElement + 'static>>,
    content: Option<
        Rc<
            dyn Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> AnyElement
                + 'static,
        >,
    >,
    children: Vec<AnyElement>,
    /// Style for trigger element.
    /// This is used for hotfix the trigger element style to support w_full.
    trigger_style: Option<StyleRefinement>,
    mouse_button: MouseButton,
    appearance: bool,
    overlay_closable: bool,
    side_offset: Option<Pixels>,
    align_offset: Pixels,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    on_close_complete: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl Popover {
    /// Create a new Popover with `view` mode.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            side: PopoverSide::Bottom,
            align: PopoverAlign::Center,
            trigger: None,
            trigger_style: None,
            content: None,
            tracked_focus_handle: None,
            children: vec![],
            mouse_button: MouseButton::Left,
            appearance: true,
            overlay_closable: true,
            side_offset: None,
            align_offset: px(0.),
            aria_label: None,
            aria_description: None,
            default_open: false,
            open: None,
            on_open_change: None,
            on_close_complete: None,
        }
    }

    /// Maps the legacy anchor API to the side/alignment placement model.
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        (self.side, self.align) = Self::placement_from_anchor(anchor.into());
        self
    }

    /// Sets the physical side on which the content is displayed.
    pub fn side(mut self, side: PopoverSide) -> Self {
        self.side = side;
        self
    }

    /// Sets content alignment along the trigger's cross axis.
    pub fn align(mut self, align: PopoverAlign) -> Self {
        self.align = align;
        self
    }

    /// Set the distance between the popover surface and its trigger.
    pub fn side_offset(mut self, offset: Pixels) -> Self {
        self.side_offset = Some(offset);
        self
    }

    /// Offsets the content along its alignment axis.
    pub fn align_offset(mut self, offset: Pixels) -> Self {
        self.align_offset = offset;
        self
    }

    /// Sets the accessible name of the standard Popover dialog surface.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets the accessible description of the standard Popover dialog surface.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Set the mouse button to trigger the popover, default is `MouseButton::Left`.
    pub fn mouse_button(mut self, mouse_button: MouseButton) -> Self {
        self.mouse_button = mouse_button;
        self
    }

    /// Set the trigger element of the popover.
    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: PopoverTrigger + 'static,
    {
        self.trigger = Some(Box::new(|is_open, _, _| {
            let selected = trigger.is_selected();
            trigger
                .popover_expanded(is_open)
                .selected(selected || is_open)
                .into_any_element()
        }));
        self
    }

    /// Sets a component-owned trigger builder that receives the resolved open state.
    ///
    /// Composite controls use this path when their trigger semantics cannot be expressed by
    /// [`PopoverTrigger`] alone, while retaining Popover's placement and lifecycle ownership.
    pub(crate) fn trigger_builder(
        mut self,
        trigger: impl FnOnce(bool, &Window, &App) -> AnyElement + 'static,
    ) -> Self {
        self.trigger = Some(Box::new(trigger));
        self
    }

    /// Set the default open state of the popover, default is `false`.
    ///
    /// This is only used to initialize the open state of the popover.
    ///
    /// And please note that if you use the `open` method, this value will be ignored.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Force set the open state of the popover.
    ///
    /// If this is set, the popover will be controlled by this value.
    ///
    /// NOTE: You must be used in conjunction with `on_open_change` to handle state changes.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Add a callback to be called when the open state changes.
    ///
    /// The first `&bool` parameter is the **new open state**.
    ///
    /// This is useful when using the `open` method to control the popover state.
    pub fn on_open_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    /// Registers an internal callback that runs after the final close transition unmounts.
    ///
    /// Overlay-owning components use this hook to retain their content throughout exit motion and
    /// release cached entities only after the lifecycle accepts the matching close generation.
    pub(crate) fn on_close_complete(
        mut self,
        callback: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close_complete = Some(Rc::new(callback));
        self
    }

    /// Set the style for the trigger element.
    pub fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.trigger_style = Some(style);
        self
    }

    /// Set whether clicking outside the popover will dismiss it, default is `true`.
    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }

    /// Set the content builder for content of the Popover.
    ///
    /// This callback will called every time on render the popover.
    /// So, you should avoid creating new elements or entities in the content closure.
    pub fn content<F, E>(mut self, content: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> E + 'static,
    {
        self.content = Some(Rc::new(move |state, window, cx| {
            content(state, window, cx).into_any_element()
        }));
        self
    }

    /// Set whether the popover no style, default is `false`.
    ///
    /// If no style:
    ///
    /// - The popover will not have a bg, border, shadow, or padding.
    /// - The click out of the popover will not dismiss it.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Bind the focus handle to receive focus when the popover is opened.
    /// If you not set this, a new focus handle will be created for the popover to
    ///
    /// If popover is opened, the focus will be moved to the focus handle.
    pub fn track_focus(mut self, handle: &FocusHandle) -> Self {
        self.tracked_focus_handle = Some(handle.clone());
        self
    }

    /// Converts the legacy anchor API into independent side and alignment values.
    fn placement_from_anchor(anchor: Anchor) -> (PopoverSide, PopoverAlign) {
        match anchor {
            Anchor::TopLeft => (PopoverSide::Bottom, PopoverAlign::Start),
            Anchor::TopCenter => (PopoverSide::Bottom, PopoverAlign::Center),
            Anchor::TopRight => (PopoverSide::Bottom, PopoverAlign::End),
            Anchor::BottomLeft => (PopoverSide::Top, PopoverAlign::Start),
            Anchor::BottomCenter => (PopoverSide::Top, PopoverAlign::Center),
            Anchor::BottomRight => (PopoverSide::Top, PopoverAlign::End),
            Anchor::LeftCenter => (PopoverSide::Right, PopoverAlign::Center),
            Anchor::RightCenter => (PopoverSide::Left, PopoverAlign::Center),
        }
    }

    /// Resolves the content anchor and matching trigger-edge point.
    fn anchor_and_position(
        side: PopoverSide,
        align: PopoverAlign,
        bounds: Bounds<Pixels>,
    ) -> (Anchor, Point<Pixels>) {
        match (side, align) {
            (PopoverSide::Bottom, PopoverAlign::Start) => (Anchor::TopLeft, bounds.bottom_left()),
            (PopoverSide::Bottom, PopoverAlign::Center) => {
                (Anchor::TopCenter, bounds.bottom_center())
            }
            (PopoverSide::Bottom, PopoverAlign::End) => (Anchor::TopRight, bounds.bottom_right()),
            (PopoverSide::Top, PopoverAlign::Start) => (Anchor::BottomLeft, bounds.origin),
            (PopoverSide::Top, PopoverAlign::Center) => (Anchor::BottomCenter, bounds.top_center()),
            (PopoverSide::Top, PopoverAlign::End) => (Anchor::BottomRight, bounds.top_right()),
            (PopoverSide::Right, PopoverAlign::Start) => (Anchor::TopLeft, bounds.top_right()),
            (PopoverSide::Right, PopoverAlign::Center) => {
                (Anchor::LeftCenter, bounds.right_center())
            }
            (PopoverSide::Right, PopoverAlign::End) => (Anchor::BottomLeft, bounds.bottom_right()),
            (PopoverSide::Left, PopoverAlign::Start) => (Anchor::TopRight, bounds.origin),
            (PopoverSide::Left, PopoverAlign::Center) => {
                (Anchor::RightCenter, bounds.left_center())
            }
            (PopoverSide::Left, PopoverAlign::End) => (Anchor::BottomRight, bounds.bottom_left()),
        }
    }

    /// Resolves side and alignment offsets into GPUI window coordinates.
    fn placement_offset(&self) -> Point<Pixels> {
        let side_offset = self.side_offset.unwrap_or(px(4.));
        match self.side {
            PopoverSide::Top => point(self.align_offset, -side_offset),
            PopoverSide::Right => point(side_offset, self.align_offset),
            PopoverSide::Bottom => point(self.align_offset, side_offset),
            PopoverSide::Left => point(-side_offset, self.align_offset),
        }
    }

    /// Returns mirrored directional translations for enter and exit motion.
    fn motion_translation(side: PopoverSide, closing: bool) -> (Point<Pixels>, Point<Pixels>) {
        let offset = match side {
            PopoverSide::Top => point(px(0.), px(8.)),
            PopoverSide::Right => point(px(-8.), px(0.)),
            PopoverSide::Bottom => point(px(0.), px(-8.)),
            PopoverSide::Left => point(px(8.), px(0.)),
        };
        let resting = point(px(0.), px(0.));
        if closing {
            (resting, offset)
        } else {
            (offset, resting)
        }
    }
}

/// A vertical heading section for Popover content.
#[derive(IntoElement)]
pub struct PopoverHeader {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl PopoverHeader {
    /// Creates an empty Popover header.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for PopoverHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for PopoverHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for PopoverHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for PopoverHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .gap(PopoverMetrics::resolve(cx).header_gap)
            .text_sm()
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// The primary heading content of a [`PopoverHeader`].
#[derive(IntoElement)]
pub struct PopoverTitle {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl PopoverTitle {
    /// Creates an empty Popover title.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for PopoverTitle {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for PopoverTitle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for PopoverTitle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for PopoverTitle {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_size(PopoverMetrics::resolve(cx).title_size)
            .font_medium()
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// Supporting text displayed below a [`PopoverTitle`].
#[derive(IntoElement)]
pub struct PopoverDescription {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl PopoverDescription {
    /// Creates an empty Popover description.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for PopoverDescription {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for PopoverDescription {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for PopoverDescription {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for PopoverDescription {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .children(self.children)
    }
}

impl ParentElement for Popover {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

pub struct PopoverState {
    focus_handle: FocusHandle,
    pub(crate) tracked_focus_handle: Option<FocusHandle>,
    previous_focus_handle: Option<FocusHandle>,
    trigger_bounds: Bounds<Pixels>,
    trigger_bounds_captured: bool,
    lifecycle: OverlayLifecycle,
    initialized: bool,
    overlay_closable: bool,
    focus_observed: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    on_close_complete: Option<Rc<dyn Fn(&mut Window, &mut App)>>,

    _dismiss_subscription: Option<Subscription>,
}

impl PopoverState {
    pub fn new(default_open: bool, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tracked_focus_handle: None,
            previous_focus_handle: None,
            trigger_bounds: Bounds::default(),
            trigger_bounds_captured: false,
            lifecycle: if default_open {
                OverlayLifecycle::opened()
            } else {
                OverlayLifecycle::default()
            },
            initialized: false,
            overlay_closable: true,
            focus_observed: false,
            on_open_change: None,
            on_close_complete: None,
            _dismiss_subscription: None,
        }
    }

    /// Check if the popover is open.
    pub fn is_open(&self) -> bool {
        self.lifecycle.accepts_input()
    }

    /// Returns the latest painted Trigger bounds for owner-managed dismissal policies.
    pub(crate) fn trigger_bounds(&self) -> Bounds<Pixels> {
        self.trigger_bounds
    }

    /// Dismiss the popover if it is open.
    pub fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.begin_close(window, cx);
    }

    /// Open the popover if it is closed.
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.begin_open(window, cx);
    }

    fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        if open {
            self.begin_open(window, cx);
        } else {
            self.begin_close(window, cx);
        }
    }

    /// Applies the initial uncontrolled or controlled state through the full overlay lifecycle.
    fn initialize(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        if open {
            self.begin_open_with_notification(false, window, cx);
        }
    }

    fn toggle_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.lifecycle.accepts_input() {
            self.begin_close(window, cx);
        } else {
            self.begin_open(window, cx);
        }
    }

    /// Starts or reverses the enter lifecycle and invalidates stale close work.
    fn begin_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.begin_open_with_notification(true, window, cx);
    }

    /// Starts or reverses opening and optionally reports a user-requested state change.
    fn begin_open_with_notification(
        &mut self,
        notify_callback: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let was_closed = self.lifecycle.phase() == OverlayPhase::Closed;
        let Some(transition) = self.lifecycle.begin_open() else {
            return;
        };

        if was_closed {
            self.previous_focus_handle = window.focused(cx);
        }
        GlobalState::global_mut(cx).register_deferred_popover(&self.focus_handle);
        if self._dismiss_subscription.is_none() {
            let state = cx.entity();
            let focus_handle = if let Some(tracked_focus_handle) = self.tracked_focus_handle.clone()
            {
                tracked_focus_handle
            } else {
                self.focus_handle.clone()
            };
            focus_handle.focus(window, cx);

            self._dismiss_subscription =
                Some(
                    window.subscribe(&cx.entity(), cx, move |_, _: &DismissEvent, window, cx| {
                        state.update(cx, |state, cx| {
                            state.dismiss(window, cx);
                        });
                        window.refresh();
                    }),
                );
        }

        if notify_callback && let Some(callback) = self.on_open_change.as_ref() {
            callback(&true, window, cx);
        }
        cx.notify();

        let duration = effective_motion_duration(cx.theme().style.motion.fast(), cx);
        cx.spawn_in(window, async move |state, cx| {
            cx.background_executor().timer(duration).await;
            let _ = state.update_in(cx, |state, _, cx| {
                if state.lifecycle.complete_open(transition) {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Starts exit and restores focus only after content is unmounted.
    fn begin_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(transition) = self.lifecycle.begin_close() else {
            return;
        };

        if let Some(callback) = self.on_open_change.as_ref() {
            callback(&false, window, cx);
        }
        cx.notify();

        let duration = effective_motion_duration(cx.theme().style.motion.fast(), cx);
        cx.spawn_in(window, async move |state, cx| {
            cx.background_executor().timer(duration).await;
            let _ = state.update_in(cx, |state, window, cx| {
                if !state.lifecycle.complete_close(transition) {
                    return;
                }

                GlobalState::global_mut(cx).unregister_deferred_popover(&state.focus_handle);
                state._dismiss_subscription = None;
                if let Some(previous) = state.previous_focus_handle.take()
                    && state.focus_handle.contains_focused(window, cx)
                {
                    previous.focus(window, cx);
                }
                if let Some(callback) = state.on_close_complete.clone() {
                    callback(window, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn on_action_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss(window, cx);
    }

    /// Dismisses a closable Popover when keyboard focus leaves its content subtree.
    fn on_focus_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlay_closable {
            self.dismiss(window, cx);
        }
    }
}

impl Focusable for PopoverState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopoverState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl EventEmitter<DismissEvent> for PopoverState {}

impl Popover {
    pub(crate) fn render_popover<E>(
        anchor: Anchor,
        position: Rc<Cell<Point<Pixels>>>,
        placement_offset: Point<Pixels>,
        content: E,
        _: &mut Window,
        _: &mut App,
    ) -> Deferred
    where
        E: IntoElement + 'static,
    {
        deferred(
            anchored()
                .snap_to_window_with_margin(px(8.))
                .anchor(anchor)
                .position(position.get())
                .offset(placement_offset)
                .child(div().relative().child(content)),
        )
        .with_priority(1)
    }

    pub(crate) fn render_popover_content(
        appearance: bool,
        _: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        let metrics = PopoverMetrics::resolve(cx);
        v_flex()
            .id("content")
            .relative()
            .occlude()
            .tab_group()
            .when(appearance, |this| {
                this.w(metrics.width)
                    .p(metrics.padding)
                    .gap(metrics.gap)
                    .text_sm()
                    .rounded(metrics.radius)
                    .bg(cx.theme().popover)
                    .text_color(cx.theme().popover_foreground)
                    .border_1()
                    .border_color(cx.theme().foreground.opacity(metrics.ring_opacity))
                    .when(cx.theme().style.elevation.enabled, |this| {
                        match metrics.shadow {
                            PopoverShadow::Medium => this.shadow_md(),
                            PopoverShadow::ExtraLarge => this.shadow_2xl(),
                        }
                    })
            })
    }
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let force_open = self.open;
        let default_open = self.default_open;
        let tracked_focus_handle = self.tracked_focus_handle.clone();
        let state = window.use_keyed_state(self.id.clone(), cx, |window, cx| {
            let state = PopoverState::new(false, cx);
            cx.on_release_in(window, |state: &mut PopoverState, _, cx| {
                GlobalState::global_mut(cx).unregister_deferred_popover(&state.focus_handle);
            })
            .detach();
            state
        });

        state.update(cx, |state, cx| {
            if let Some(tracked_focus_handle) = tracked_focus_handle {
                state.tracked_focus_handle = Some(tracked_focus_handle);
            }
            state.on_open_change = self.on_open_change.clone();
            state.on_close_complete = self.on_close_complete.clone();
            state.overlay_closable = self.overlay_closable;
            state.initialize(force_open.unwrap_or(default_open), window, cx);
            if !state.focus_observed {
                cx.on_focus_out(&state.focus_handle, window, |state, _, window, cx| {
                    state.on_focus_out(window, cx);
                })
                .detach();
                state.focus_observed = true;
            }
            if let Some(force_open) = force_open {
                state.set_open(force_open, window, cx);
            }
        });

        let phase = state.read(cx).lifecycle.phase();
        let open = state.read(cx).lifecycle.accepts_input();
        let mounted = state.read(cx).lifecycle.is_mounted();
        let closing = phase == OverlayPhase::Closing;
        let focus_handle = state.read(cx).focus_handle.clone();
        let trigger_bounds = state.read(cx).trigger_bounds;
        let trigger_bounds_captured = state.read(cx).trigger_bounds_captured;
        let placement_offset = self.placement_offset();
        let trigger_style = self.trigger_style.clone();

        let Some(trigger) = self.trigger else {
            return div().id("empty");
        };

        let parent_view_id = window.current_view();

        // Shared cell so the deferred Anchored element can read the real trigger bounds at
        // prepaint time (after trigger's on_prepaint has already fired with the correct bounds).
        let (content_anchor, initial_position) =
            Self::anchor_and_position(self.side, self.align, trigger_bounds);
        let position = Rc::new(Cell::new(initial_position));

        let trigger_el = div()
            .id("trigger")
            .when_some(trigger_style, |this, style| this.refine_style(&style))
            .when(self.mouse_button == MouseButton::Left, |this| {
                this.on_click({
                    let state = state.clone();
                    move |_, window, cx| {
                        cx.stop_propagation();
                        state.update(cx, |state, cx| state.toggle_open(window, cx));
                        cx.notify(parent_view_id);
                    }
                })
            })
            .when(self.mouse_button != MouseButton::Left, |this| {
                this.on_mouse_down(self.mouse_button, {
                    let state = state.clone();
                    move |_, window, cx| {
                        cx.stop_propagation();
                        state.update(cx, |state, cx| {
                            state.toggle_open(window, cx);
                        });
                        cx.notify(parent_view_id);
                    }
                })
            })
            .on_prepaint({
                let state = state.clone();
                let position = position.clone();
                let side = self.side;
                let align = self.align;
                move |bounds, window, cx| {
                    position.set(Self::anchor_and_position(side, align, bounds).1);
                    let first_capture = state.update(cx, |state, _| {
                        let first = !state.trigger_bounds_captured;
                        state.trigger_bounds = bounds;
                        state.trigger_bounds_captured = true;
                        first
                    });
                    // On the very first bounds capture, request a new frame so the popover
                    // renders at the correct position (outside the current paint cycle).
                    if first_capture {
                        window.request_animation_frame();
                    }
                }
            })
            // ElementExt::on_prepaint appends an absolute measurement canvas. Keep it
            // before the visible trigger so its static position matches the trigger origin.
            .child((trigger)(open, window, cx));

        // Keep activation handlers on the trigger subtree so pointer events from
        // deferred content cannot bubble into a second open-state toggle.
        let el = div()
            .id(self.id)
            .flex_none()
            .w_auto()
            .h_auto()
            .child(trigger_el);

        if !mounted || !trigger_bounds_captured {
            return el;
        }

        let popover_content = Self::render_popover_content(self.appearance, window, cx)
            .track_focus(&focus_handle)
            .key_context(CONTEXT)
            .when(!closing, |this| {
                this.on_action(window.listener_for(&state, PopoverState::on_action_cancel))
            })
            .when_some(self.content, |this, content| {
                this.child(state.update(cx, |state, cx| (content)(state, window, cx)))
            })
            .children(self.children)
            .when(self.appearance, |this| {
                this.role(Role::Dialog)
                    .when_some(self.aria_label, |this, label| this.aria_label(label))
                    .when_some(self.aria_description, |this, description| {
                        this.aria_description(description)
                    })
            })
            .when(self.overlay_closable && !closing, |this| {
                this.on_mouse_down_out({
                    let state = state.clone();
                    move |event, window, cx| {
                        if state.read(cx).trigger_bounds.contains(&event.position) {
                            return;
                        }
                        state.update(cx, |state, cx| {
                            state.dismiss(window, cx);
                        });
                        cx.notify(parent_view_id);
                    }
                })
            })
            .when(closing, |this| {
                this.child(div().absolute().top_0().left_0().size_full().occlude())
            })
            .refine_style(&self.style);

        let motion = cx.theme().style.motion;
        let (motion_from, motion_to) = Self::motion_translation(self.side, closing);
        let popover_content = Transition::new(motion.fast())
            .ease_token(if closing {
                motion.exit_easing
            } else {
                motion.enter_easing
            })
            .slide_x(motion_from.x, motion_to.x)
            .slide_y(motion_from.y, motion_to.y)
            .apply(popover_content, "popover-motion");

        el.child(Self::render_popover(
            content_anchor,
            position,
            placement_offset,
            popover_content,
            window,
            cx,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{MouseButton, TestAppContext};
    use std::{cell::Cell, rc::Rc, time::Duration};

    #[test]
    fn test_popover_builder_chaining() {
        let popover = Popover::new("test")
            .anchor(Anchor::BottomCenter)
            .mouse_button(MouseButton::Right)
            .default_open(true)
            .appearance(false)
            .overlay_closable(false)
            .side_offset(px(2.))
            .align_offset(px(3.));

        assert_eq!(popover.side, PopoverSide::Top);
        assert_eq!(popover.align, PopoverAlign::Center);
        assert_eq!(popover.mouse_button, MouseButton::Right);
        assert!(popover.default_open);
        assert!(!popover.appearance);
        assert!(!popover.overlay_closable);
        assert_eq!(popover.side_offset, Some(px(2.)));
        assert_eq!(popover.align_offset, px(3.));
    }

    #[test]
    fn placement_resolves_trigger_edge_and_content_anchor() {
        let bounds = Bounds {
            origin: Point {
                x: px(100.),
                y: px(100.),
            },
            size: gpui::Size {
                width: px(200.),
                height: px(50.),
            },
        };

        assert_eq!(
            Popover::anchor_and_position(PopoverSide::Bottom, PopoverAlign::Center, bounds),
            (Anchor::TopCenter, point(px(200.), px(150.)))
        );
        assert_eq!(
            Popover::anchor_and_position(PopoverSide::Top, PopoverAlign::End, bounds),
            (Anchor::BottomRight, point(px(300.), px(100.)))
        );
        assert_eq!(
            Popover::anchor_and_position(PopoverSide::Right, PopoverAlign::Start, bounds),
            (Anchor::TopLeft, point(px(300.), px(100.)))
        );
    }

    #[test]
    fn side_and_alignment_offsets_move_away_from_the_trigger() {
        assert_eq!(
            Popover::new("bottom")
                .side(PopoverSide::Bottom)
                .side_offset(px(4.))
                .align_offset(px(2.))
                .placement_offset(),
            point(px(2.), px(4.))
        );
        assert_eq!(
            Popover::new("left")
                .side(PopoverSide::Left)
                .side_offset(px(4.))
                .align_offset(px(2.))
                .placement_offset(),
            point(px(-4.), px(2.))
        );
    }

    #[gpui::test]
    fn lifecycle_reopen_invalidates_pending_popover_close(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (state, cx) = cx.add_window_view(|_, cx| PopoverState::new(false, cx));

        state.update_in(cx, |state, window, cx| {
            state.show(window, cx);
            state.dismiss(window, cx);
            state.show(window, cx);
        });
        assert_eq!(
            state.read_with(cx, |state, _| state.lifecycle.phase()),
            OverlayPhase::Opening
        );

        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();
        assert_eq!(
            state.read_with(cx, |state, _| state.lifecycle.phase()),
            OverlayPhase::Open
        );

        state.update_in(cx, |state, window, cx| state.dismiss(window, cx));
        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();
        assert_eq!(
            state.read_with(cx, |state, _| state.lifecycle.phase()),
            OverlayPhase::Closed
        );
    }

    #[gpui::test]
    fn close_complete_runs_only_for_the_final_close_generation(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let completions = Rc::new(Cell::new(0));
        let (state, cx) = cx.add_window_view({
            let completions = completions.clone();
            move |_, cx| {
                let mut state = PopoverState::new(false, cx);
                state.on_close_complete = Some(Rc::new(move |_, _| {
                    completions.set(completions.get() + 1);
                }));
                state
            }
        });

        state.update_in(cx, |state, window, cx| {
            state.show(window, cx);
            state.dismiss(window, cx);
            state.show(window, cx);
        });
        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();
        assert_eq!(completions.get(), 0);

        state.update_in(cx, |state, window, cx| state.dismiss(window, cx));
        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();
        assert_eq!(completions.get(), 1);
    }
}
