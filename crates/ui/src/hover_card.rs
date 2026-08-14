use gpui::{
    Anchor, AnyElement, App, Bounds, Context, DispatchPhase, ElementId, FocusHandle,
    InteractiveElement as _, IntoElement, MouseMoveEvent, ParentElement, Pixels, Point, Render,
    RenderOnce, StatefulInteractiveElement, StyleRefinement, Styled, Task, Window, anchored,
    canvas, deferred, div, point, prelude::FluentBuilder as _, px,
};
use instant::Duration;
use std::rc::Rc;

use crate::{
    ActiveTheme as _, ElementExt, StyledExt as _,
    animation::{OverlayLifecycle, OverlayPhase, OverlayTransition, Transition},
    theme::Density,
    v_flex,
};

/// Physical side on which the HoverCard content is displayed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HoverCardSide {
    Top,
    Right,
    #[default]
    Bottom,
    Left,
}

/// Cross-axis alignment of HoverCard content relative to its trigger.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HoverCardAlign {
    Start,
    #[default]
    Center,
    End,
}

/// Component-local presentation derived from semantic Style Preset metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
struct HoverCardMetrics {
    width: Pixels,
    padding: Pixels,
    radius: Pixels,
    ring_opacity: f32,
    shadow: HoverCardShadow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoverCardShadow {
    Medium,
    ExtraLarge,
}

impl HoverCardMetrics {
    /// Resolves the pinned shadcn geometry without branching on preset identifiers.
    fn resolve(cx: &App) -> Self {
        let style = &cx.theme().style;
        match style.density {
            Density::Compact => Self {
                width: px(256.),
                padding: px(10.),
                radius: style.radii.lg,
                ring_opacity: 0.1,
                shadow: HoverCardShadow::Medium,
            },
            Density::Standard => Self {
                width: px(256.),
                padding: px(16.),
                radius: style.radii.lg,
                ring_opacity: 0.1,
                shadow: HoverCardShadow::Medium,
            },
            Density::Comfortable => Self {
                width: px(288.),
                padding: px(16.),
                radius: style.radii.xl,
                ring_opacity: 0.05,
                shadow: HoverCardShadow::ExtraLarge,
            },
        }
    }
}

/// A non-modal preview card opened by pointer hover or keyboard focus.
#[derive(IntoElement)]
pub struct HoverCard {
    id: ElementId,
    style: StyleRefinement,
    side: HoverCardSide,
    align: HoverCardAlign,
    side_offset: Pixels,
    align_offset: Pixels,
    default_open: bool,
    open: Option<bool>,
    trigger: Option<Box<dyn FnOnce(&mut Window, &App) -> AnyElement + 'static>>,
    content: Option<
        Rc<
            dyn Fn(&mut HoverCardState, &mut Window, &mut Context<HoverCardState>) -> AnyElement
                + 'static,
        >,
    >,
    children: Vec<AnyElement>,
    open_delay: Duration,
    close_delay: Duration,
    appearance: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl HoverCard {
    /// Creates a HoverCard with shadcn-compatible placement and timing defaults.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            side: HoverCardSide::Bottom,
            align: HoverCardAlign::Center,
            side_offset: px(4.),
            align_offset: px(0.),
            default_open: false,
            open: None,
            trigger: None,
            content: None,
            children: vec![],
            open_delay: Duration::from_millis(700),
            close_delay: Duration::from_millis(300),
            appearance: true,
            on_open_change: None,
        }
    }

    /// Maps the legacy GPUI anchor vocabulary onto side and alignment.
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        (self.side, self.align) = Self::placement_from_anchor(anchor.into());
        self
    }

    /// Sets the physical side on which the card is displayed.
    pub fn side(mut self, side: HoverCardSide) -> Self {
        self.side = side;
        self
    }

    /// Sets the cross-axis alignment relative to the trigger.
    pub fn align(mut self, align: HoverCardAlign) -> Self {
        self.align = align;
        self
    }

    /// Sets the distance between the trigger and content surface.
    pub fn side_offset(mut self, offset: Pixels) -> Self {
        self.side_offset = offset;
        self
    }

    /// Offsets the content along its alignment axis.
    pub fn align_offset(mut self, offset: Pixels) -> Self {
        self.align_offset = offset;
        self
    }

    /// Sets the uncontrolled initial open state.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Sets the controlled open state.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Sets the trigger element. The trigger retains its own keyboard and activation behavior.
    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: IntoElement + 'static,
    {
        self.trigger = Some(Box::new(|_, _| trigger.into_any_element()));
        self
    }

    /// Sets content that is built only while the HoverCard surface is mounted.
    pub fn content<F, E>(mut self, content: F) -> Self
    where
        F: Fn(&mut HoverCardState, &mut Window, &mut Context<HoverCardState>) -> E + 'static,
        E: IntoElement + 'static,
    {
        self.content = Some(Rc::new(move |state, window, cx| {
            content(state, window, cx).into_any_element()
        }));
        self
    }

    /// Sets the delay before opening. The shadcn-compatible default is 700ms.
    pub fn open_delay(mut self, duration: Duration) -> Self {
        self.open_delay = duration;
        self
    }

    /// Sets the delay before closing. The shadcn-compatible default is 300ms.
    pub fn close_delay(mut self, duration: Duration) -> Self {
        self.close_delay = duration;
        self
    }

    /// Enables or disables the default HoverCard surface appearance.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Registers a callback for requested open-state changes.
    pub fn on_open_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    /// Converts the legacy anchor API into the independent side/alignment model.
    fn placement_from_anchor(anchor: Anchor) -> (HoverCardSide, HoverCardAlign) {
        match anchor {
            Anchor::TopLeft => (HoverCardSide::Bottom, HoverCardAlign::Start),
            Anchor::TopCenter => (HoverCardSide::Bottom, HoverCardAlign::Center),
            Anchor::TopRight => (HoverCardSide::Bottom, HoverCardAlign::End),
            Anchor::BottomLeft => (HoverCardSide::Top, HoverCardAlign::Start),
            Anchor::BottomCenter => (HoverCardSide::Top, HoverCardAlign::Center),
            Anchor::BottomRight => (HoverCardSide::Top, HoverCardAlign::End),
            Anchor::LeftCenter => (HoverCardSide::Right, HoverCardAlign::Center),
            Anchor::RightCenter => (HoverCardSide::Left, HoverCardAlign::Center),
        }
    }

    /// Resolves the content anchor and matching trigger-edge point.
    fn anchor_and_position(
        side: HoverCardSide,
        align: HoverCardAlign,
        bounds: Bounds<Pixels>,
    ) -> (Anchor, Point<Pixels>) {
        match (side, align) {
            (HoverCardSide::Bottom, HoverCardAlign::Start) => {
                (Anchor::TopLeft, bounds.bottom_left())
            }
            (HoverCardSide::Bottom, HoverCardAlign::Center) => {
                (Anchor::TopCenter, bounds.bottom_center())
            }
            (HoverCardSide::Bottom, HoverCardAlign::End) => {
                (Anchor::TopRight, bounds.bottom_right())
            }
            (HoverCardSide::Top, HoverCardAlign::Start) => (Anchor::BottomLeft, bounds.origin),
            (HoverCardSide::Top, HoverCardAlign::Center) => {
                (Anchor::BottomCenter, bounds.top_center())
            }
            (HoverCardSide::Top, HoverCardAlign::End) => (Anchor::BottomRight, bounds.top_right()),
            (HoverCardSide::Right, HoverCardAlign::Start) => (Anchor::TopLeft, bounds.top_right()),
            (HoverCardSide::Right, HoverCardAlign::Center) => {
                (Anchor::LeftCenter, bounds.right_center())
            }
            (HoverCardSide::Right, HoverCardAlign::End) => {
                (Anchor::BottomLeft, bounds.bottom_right())
            }
            (HoverCardSide::Left, HoverCardAlign::Start) => (Anchor::TopRight, bounds.origin),
            (HoverCardSide::Left, HoverCardAlign::Center) => {
                (Anchor::RightCenter, bounds.left_center())
            }
            (HoverCardSide::Left, HoverCardAlign::End) => {
                (Anchor::BottomRight, bounds.bottom_left())
            }
        }
    }

    /// Resolves side and alignment offsets into GPUI window coordinates.
    fn placement_offset(&self) -> Point<Pixels> {
        match self.side {
            HoverCardSide::Top => point(self.align_offset, -self.side_offset),
            HoverCardSide::Right => point(self.side_offset, self.align_offset),
            HoverCardSide::Bottom => point(self.align_offset, self.side_offset),
            HoverCardSide::Left => point(-self.side_offset, self.align_offset),
        }
    }

    /// Returns mirrored directional translations for enter and exit motion.
    fn motion_translation(
        side: HoverCardSide,
        closing: bool,
    ) -> Option<(Point<Pixels>, Point<Pixels>)> {
        let offset = match side {
            HoverCardSide::Top => point(px(0.), px(8.)),
            HoverCardSide::Right => point(px(-8.), px(0.)),
            HoverCardSide::Bottom => point(px(0.), px(-8.)),
            HoverCardSide::Left => point(px(8.), px(0.)),
        };
        let resting = point(px(0.), px(0.));
        if closing {
            Some((resting, offset))
        } else {
            Some((offset, resting))
        }
    }
}

impl Styled for HoverCard {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for HoverCard {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// Runtime state shared by pointer, focus, controlled state, and exit motion.
pub struct HoverCardState {
    lifecycle: OverlayLifecycle,
    focus_handle: FocusHandle,
    focus_observed: bool,
    controlled_open: Option<bool>,
    trigger_bounds: Bounds<Pixels>,
    trigger_bounds_captured: bool,
    content_bounds: Bounds<Pixels>,
    open_delay: Duration,
    close_delay: Duration,
    open_task: Option<Task<()>>,
    close_task: Option<Task<()>>,
    epoch: usize,
    is_hovering_trigger: bool,
    is_hovering_content: bool,
    is_focus_within: bool,
    pointer_exit: Option<Point<Pixels>>,
    transfer_active: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl HoverCardState {
    fn new(default_open: bool, open_delay: Duration, close_delay: Duration, cx: &mut App) -> Self {
        Self {
            lifecycle: if default_open {
                OverlayLifecycle::opened()
            } else {
                OverlayLifecycle::default()
            },
            focus_handle: cx.focus_handle(),
            focus_observed: false,
            controlled_open: None,
            trigger_bounds: Bounds::default(),
            trigger_bounds_captured: false,
            content_bounds: Bounds::default(),
            open_delay,
            close_delay,
            open_task: None,
            close_task: None,
            epoch: 0,
            is_hovering_trigger: false,
            is_hovering_content: false,
            is_focus_within: false,
            pointer_exit: None,
            transfer_active: false,
            on_open_change: None,
        }
    }

    /// Returns whether the content accepts pointer input.
    pub fn is_open(&self) -> bool {
        self.lifecycle.accepts_input()
    }

    /// Starts an interruptible delayed-open request.
    fn schedule_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.lifecycle.accepts_input() {
            self.cancel_tasks();
            return;
        }
        self.cancel_tasks();
        let epoch = self.next_epoch();
        let delay = self.open_delay;
        self.open_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update_in(cx, |state, window, cx| {
                if state.epoch == epoch && (state.is_hovering_trigger || state.is_focus_within) {
                    state.request_open(true, window, cx);
                }
            });
        }));
    }

    /// Starts an interruptible delayed-close request.
    fn schedule_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_tasks();
        let epoch = self.next_epoch();
        let delay = self.close_delay;
        self.close_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update_in(cx, |state, window, cx| {
                if state.epoch == epoch
                    && !state.is_hovering_trigger
                    && !state.is_hovering_content
                    && !state.is_focus_within
                {
                    state.transfer_active = false;
                    state.request_open(false, window, cx);
                }
            });
        }));
    }

    /// Cancels open and close timers and invalidates their generation.
    fn cancel_tasks(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.open_task = None;
        self.close_task = None;
    }

    /// Creates a new timer generation.
    fn next_epoch(&mut self) -> usize {
        self.epoch = self.epoch.wrapping_add(1);
        self.epoch
    }

    /// Requests a state change while preserving controlled-state ownership.
    fn request_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(controlled_open) = self.controlled_open {
            if controlled_open != open {
                if let Some(callback) = self.on_open_change.as_ref() {
                    callback(&open, window, cx);
                }
            }
            return;
        }
        self.set_open(open, true, window, cx);
    }

    /// Applies an internal lifecycle transition.
    ///
    /// Opening and closing remain mounted until their directional translations complete.
    fn set_open(&mut self, open: bool, notify: bool, window: &mut Window, cx: &mut Context<Self>) {
        let transition = if open {
            self.lifecycle.begin_open()
        } else {
            self.lifecycle.begin_close()
        };
        if transition.is_none() {
            return;
        }

        if notify {
            if let Some(callback) = self.on_open_change.as_ref() {
                callback(&open, window, cx);
            }
        }
        cx.notify();
    }

    /// Completes the active enter or exit after its final translation frame is sampled.
    fn complete_motion(
        &mut self,
        opening: bool,
        transition: OverlayTransition,
        cx: &mut Context<Self>,
    ) {
        let completed = if opening {
            self.lifecycle.complete_open(transition)
        } else {
            self.lifecycle.complete_close(transition)
        };
        if completed {
            cx.notify();
        }
    }

    /// Updates trigger hover state and starts pointer transfer when leaving.
    fn on_trigger_hover(&mut self, hovering: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.is_hovering_trigger = hovering;
        if hovering {
            self.transfer_active = false;
            self.pointer_exit = None;
            self.schedule_open(window, cx);
        } else if !self.is_hovering_content && !self.is_focus_within {
            self.pointer_exit = Some(window.mouse_position());
            self.transfer_active = self.lifecycle.is_mounted();
            self.schedule_close(window, cx);
            cx.notify();
        }
    }

    /// Updates content hover state and cancels closing after a successful transfer.
    fn on_content_hover(&mut self, hovering: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.is_hovering_content = hovering;
        if hovering {
            self.transfer_active = false;
            self.pointer_exit = None;
            self.cancel_tasks();
            cx.notify();
        } else if !self.is_hovering_trigger && !self.is_focus_within {
            self.schedule_close(window, cx);
        }
    }

    /// Opens when the trigger subtree receives keyboard focus.
    fn on_focus_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_focus_within = true;
        self.schedule_open(window, cx);
    }

    /// Schedules closing after focus leaves the trigger subtree.
    fn on_focus_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_focus_within = false;
        if !self.is_hovering_trigger && !self.is_hovering_content {
            self.schedule_close(window, cx);
        }
    }

    /// Returns whether a pointer remains inside the placement-aware transfer corridor.
    fn point_in_safe_corridor(&self, cursor: Point<Pixels>, side: HoverCardSide) -> bool {
        let Some(exit) = self.pointer_exit else {
            return false;
        };
        if self.content_bounds.contains(&cursor) {
            return true;
        }

        let padding = px(6.);
        let (a, b) = match side {
            HoverCardSide::Bottom => (
                point(
                    self.content_bounds.left() - padding,
                    self.content_bounds.top() + padding,
                ),
                point(
                    self.content_bounds.right() + padding,
                    self.content_bounds.top() + padding,
                ),
            ),
            HoverCardSide::Top => (
                point(
                    self.content_bounds.left() - padding,
                    self.content_bounds.bottom() - padding,
                ),
                point(
                    self.content_bounds.right() + padding,
                    self.content_bounds.bottom() - padding,
                ),
            ),
            HoverCardSide::Right => (
                point(
                    self.content_bounds.left() + padding,
                    self.content_bounds.top() - padding,
                ),
                point(
                    self.content_bounds.left() + padding,
                    self.content_bounds.bottom() + padding,
                ),
            ),
            HoverCardSide::Left => (
                point(
                    self.content_bounds.right() - padding,
                    self.content_bounds.top() - padding,
                ),
                point(
                    self.content_bounds.right() - padding,
                    self.content_bounds.bottom() + padding,
                ),
            ),
        };
        point_in_triangle(cursor, exit, a, b)
    }
}

/// Tests a point against a triangle using consistent signed edge areas.
fn point_in_triangle(
    point: Point<Pixels>,
    a: Point<Pixels>,
    b: Point<Pixels>,
    c: Point<Pixels>,
) -> bool {
    let sign = |p1: Point<Pixels>, p2: Point<Pixels>, p3: Point<Pixels>| {
        (p1.x.as_f32() - p3.x.as_f32()) * (p2.y.as_f32() - p3.y.as_f32())
            - (p2.x.as_f32() - p3.x.as_f32()) * (p1.y.as_f32() - p3.y.as_f32())
    };
    let d1 = sign(point, a, b);
    let d2 = sign(point, b, c);
    let d3 = sign(point, c, a);
    let has_negative = d1 < 0. || d2 < 0. || d3 < 0.;
    let has_positive = d1 > 0. || d2 > 0. || d3 > 0.;
    !(has_negative && has_positive)
}

impl Render for HoverCardState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl RenderOnce for HoverCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| {
            HoverCardState::new(self.default_open, self.open_delay, self.close_delay, cx)
        });

        state.update(cx, |state, cx| {
            state.open_delay = self.open_delay;
            state.close_delay = self.close_delay;
            state.on_open_change = self.on_open_change.clone();
            state.controlled_open = self.open;
            if !state.focus_observed {
                cx.on_focus_in(&state.focus_handle, window, HoverCardState::on_focus_in)
                    .detach();
                cx.on_focus_out(&state.focus_handle, window, |state, _, window, cx| {
                    state.on_focus_out(window, cx);
                })
                .detach();
                state.focus_observed = true;
            }
            if let Some(open) = self.open {
                state.set_open(open, false, window, cx);
            }
        });

        let phase = state.read(cx).lifecycle.phase();
        let closing = phase == OverlayPhase::Closing;
        let active_transition = state.read(cx).lifecycle.active_transition();
        let mounted = state.read(cx).lifecycle.is_mounted();
        let trigger_bounds = state.read(cx).trigger_bounds;
        let trigger_bounds_captured = state.read(cx).trigger_bounds_captured;
        let focus_handle = state.read(cx).focus_handle.clone();
        let transfer_active = state.read(cx).transfer_active;
        let placement_offset = self.placement_offset();
        let Some(trigger) = self.trigger else {
            return div().id("empty");
        };

        let side = self.side;
        let align = self.align;
        let root = div()
            .id(self.id)
            // HoverCard is an inline wrapper. Its bounds must follow the visible trigger;
            // otherwise a flex parent can stretch the measurement box and displace content.
            .flex_none()
            .w_auto()
            .h_auto()
            .track_focus(&focus_handle)
            .child(
                div()
                    .id("trigger")
                    .flex_none()
                    .w_auto()
                    .h_auto()
                    .on_hover(window.listener_for(&state, |state, hovered, window, cx| {
                        state.on_trigger_hover(*hovered, window, cx);
                    }))
                    .on_prepaint({
                        let state = state.clone();
                        move |bounds, window, cx| {
                            let first_capture = state.update(cx, |state, _| {
                                let first = !state.trigger_bounds_captured;
                                state.trigger_bounds = bounds;
                                state.trigger_bounds_captured = true;
                                first
                            });
                            if first_capture {
                                window.request_animation_frame();
                            }
                        }
                    })
                    // Keep the measurement canvas before the trigger child. ElementExt
                    // implements on_prepaint as an absolute canvas at its static position;
                    // appending it after the child would measure one trigger-height too low.
                    .child((trigger)(window, cx)),
            )
            .when(transfer_active, |this| {
                let state_for_move = state.clone();
                this.child(
                    canvas(
                        |_, _, _| {},
                        move |_, _, window, _| {
                            let state_for_event = state_for_move.clone();
                            window.on_mouse_event(
                                move |event: &MouseMoveEvent, phase, window, cx| {
                                    if phase != DispatchPhase::Bubble {
                                        return;
                                    }
                                    state_for_event.update(cx, |state, cx| {
                                        if state.transfer_active
                                            && !state.point_in_safe_corridor(event.position, side)
                                        {
                                            state.transfer_active = false;
                                            if !state.is_hovering_trigger
                                                && !state.is_hovering_content
                                                && !state.is_focus_within
                                            {
                                                state.schedule_close(window, cx);
                                            }
                                        }
                                    });
                                },
                            );
                        },
                    )
                    .absolute()
                    .w_0()
                    .h_0(),
                )
            });

        if !mounted || !trigger_bounds_captured {
            return root;
        }

        let metrics = HoverCardMetrics::resolve(cx);
        let content = v_flex()
            .id("content")
            .relative()
            .occlude()
            .when(self.appearance, |this| {
                this.w(metrics.width)
                    .p(metrics.padding)
                    .text_sm()
                    .rounded(metrics.radius)
                    .bg(cx.theme().popover)
                    .text_color(cx.theme().popover_foreground)
                    .border_1()
                    .border_color(cx.theme().foreground.opacity(metrics.ring_opacity))
                    .when(cx.theme().style.elevation.enabled, |this| {
                        match metrics.shadow {
                            HoverCardShadow::Medium => this.shadow_md(),
                            HoverCardShadow::ExtraLarge => this.shadow_2xl(),
                        }
                    })
            })
            .when(!closing, |this| {
                this.on_hover(window.listener_for(&state, |state, hovered, window, cx| {
                    state.on_content_hover(*hovered, window, cx);
                }))
            })
            .when_some(self.content, |this, content| {
                this.child(state.update(cx, |state, cx| (content)(state, window, cx)))
            })
            .children(self.children)
            // Keep the exiting subtree visible while preventing pointer interaction.
            .when(closing, |this| {
                this.child(div().absolute().top_0().left_0().size_full().occlude())
            })
            .refine_style(&self.style);

        // Measure a padding-free wrapper so positioning and the safe corridor use the
        // surface edge rather than the inner content box.
        let content = div()
            .flex_none()
            .w_auto()
            .h_auto()
            .relative()
            .on_prepaint({
                let state = state.clone();
                move |bounds, _, cx| {
                    state.update(cx, |state, _| state.content_bounds = bounds);
                }
            })
            .child(content);

        let motion = cx.theme().style.motion;
        let translation = match phase {
            OverlayPhase::Opening => Self::motion_translation(self.side, false),
            OverlayPhase::Closing => Self::motion_translation(self.side, true),
            OverlayPhase::Closed | OverlayPhase::Open => None,
        };
        let opening = phase == OverlayPhase::Opening;
        let state_for_completion = state.clone();
        let content = Transition::new(motion.fast())
            .ease_token(if closing {
                motion.exit_easing
            } else {
                motion.enter_easing
            })
            // Enter and exit use mirrored directional translation. Opacity and
            // the unavailable subtree scale are intentionally not animated.
            .when_some(translation, |this, (from, to)| {
                this.slide_x(from.x, to.x).slide_y(from.y, to.y)
            })
            .when_some(active_transition, |this, transition| {
                this.on_complete(move |_, cx| {
                    state_for_completion.update(cx, |state, cx| {
                        state.complete_motion(opening, transition, cx);
                    });
                })
            })
            .apply(content, "hover-card-motion")
            .into_any_element();

        let (anchor, position) = Self::anchor_and_position(side, align, trigger_bounds);
        let overlay = deferred(
            anchored()
                .snap_to_window_with_margin(px(8.))
                .anchor(anchor)
                .position(position)
                .offset(placement_offset)
                .child(div().relative().child(content)),
        )
        .with_priority(1);

        // Keep the deferred surface outside the inline trigger wrapper's layout flow.
        root.child(div().absolute().w_0().h_0().child(overlay))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Modifiers, TestAppContext, VisualTestContext, bounds, size};
    use std::{
        cell::Cell,
        sync::{Arc, Mutex},
    };

    #[test]
    fn defaults_match_the_shadcn_contract() {
        let card = HoverCard::new("defaults");
        assert_eq!(card.side, HoverCardSide::Bottom);
        assert_eq!(card.align, HoverCardAlign::Center);
        assert_eq!(card.side_offset, px(4.));
        assert_eq!(card.align_offset, px(0.));
        assert_eq!(card.open_delay, Duration::from_millis(700));
        assert_eq!(card.close_delay, Duration::from_millis(300));
    }

    #[gpui::test]
    fn reopening_during_exit_rejects_stale_close_completion(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (state, cx) = cx.add_window_view(|_, cx| {
            HoverCardState::new(false, Duration::ZERO, Duration::ZERO, cx)
        });

        let opening = state.update_in(cx, |state, window, cx| {
            state.set_open(true, false, window, cx);
            state.lifecycle.active_transition().unwrap()
        });

        state.update(cx, |state, cx| {
            state.complete_motion(true, opening, cx);
        });

        let (closing, reopening) = state.update_in(cx, |state, window, cx| {
            state.set_open(false, false, window, cx);
            let closing = state.lifecycle.active_transition().unwrap();
            state.set_open(true, false, window, cx);
            let reopening = state.lifecycle.active_transition().unwrap();
            (closing, reopening)
        });

        state.update(cx, |state, cx| {
            state.complete_motion(false, closing, cx);
        });
        assert_eq!(
            state.read_with(cx, |state, _| state.lifecycle.phase()),
            OverlayPhase::Opening
        );

        state.update(cx, |state, cx| {
            state.complete_motion(true, reopening, cx);
        });
        assert_eq!(
            state.read_with(cx, |state, _| state.lifecycle.phase()),
            OverlayPhase::Open
        );
    }

    #[test]
    fn legacy_anchors_map_to_side_and_alignment() {
        assert_eq!(
            HoverCard::placement_from_anchor(Anchor::TopRight),
            (HoverCardSide::Bottom, HoverCardAlign::End)
        );
        assert_eq!(
            HoverCard::placement_from_anchor(Anchor::BottomCenter),
            (HoverCardSide::Top, HoverCardAlign::Center)
        );
        assert_eq!(
            HoverCard::placement_from_anchor(Anchor::LeftCenter),
            (HoverCardSide::Right, HoverCardAlign::Center)
        );
    }

    #[test]
    fn every_side_uses_the_expected_trigger_edge() {
        let trigger = bounds(point(px(10.), px(20.)), size(px(100.), px(40.)));
        assert_eq!(
            HoverCard::anchor_and_position(HoverCardSide::Bottom, HoverCardAlign::Center, trigger),
            (Anchor::TopCenter, point(px(60.), px(60.)))
        );
        assert_eq!(
            HoverCard::anchor_and_position(HoverCardSide::Right, HoverCardAlign::End, trigger),
            (Anchor::BottomLeft, point(px(110.), px(60.)))
        );
    }

    #[test]
    fn exit_translation_is_the_inverse_of_entry() {
        assert_eq!(
            HoverCard::motion_translation(HoverCardSide::Bottom, false),
            Some((point(px(0.), px(-8.)), point(px(0.), px(0.))))
        );
        assert_eq!(
            HoverCard::motion_translation(HoverCardSide::Left, false),
            Some((point(px(8.), px(0.)), point(px(0.), px(0.))))
        );
        assert_eq!(
            HoverCard::motion_translation(HoverCardSide::Bottom, true),
            Some((point(px(0.), px(0.)), point(px(0.), px(-8.))))
        );
    }

    #[gpui::test]
    fn safe_corridor_keeps_diagonal_pointer_transfer(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (state, cx) = cx.add_window_view(|_, cx| {
            HoverCardState::new(false, Duration::ZERO, Duration::ZERO, cx)
        });
        state.update(cx, |state, _| {
            state.pointer_exit = Some(point(px(50.), px(40.)));
            state.content_bounds = bounds(point(px(20.), px(60.)), size(px(100.), px(80.)));
        });

        assert!(state.read_with(cx, |state, _| {
            state.point_in_safe_corridor(point(px(70.), px(55.)), HoverCardSide::Bottom)
        }));
        assert!(!state.read_with(cx, |state, _| {
            state.point_in_safe_corridor(point(px(150.), px(45.)), HoverCardSide::Bottom)
        }));
    }

    #[gpui::test]
    fn controlled_state_requests_change_without_mutating_lifecycle(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let requests = Rc::new(Cell::new(0));
        let requests_for_callback = requests.clone();
        let (state, cx) = cx.add_window_view(|_, cx| {
            HoverCardState::new(false, Duration::ZERO, Duration::ZERO, cx)
        });
        state.update(cx, |state, _| {
            state.controlled_open = Some(false);
            state.on_open_change = Some(Rc::new(move |open, _, _| {
                assert!(*open);
                requests_for_callback.set(requests_for_callback.get() + 1);
            }));
        });

        state.update_in(cx, |state, window, cx| {
            state.request_open(true, window, cx);
        });

        assert_eq!(requests.get(), 1);
        assert_eq!(
            state.read_with(cx, |state, _| state.lifecycle.phase()),
            OverlayPhase::Closed
        );
    }

    struct SafeTransferPaintFixture;

    impl Render for SafeTransferPaintFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(400.)).h(px(400.)).child(
                HoverCard::new("paint-phase-transfer")
                    .open_delay(Duration::ZERO)
                    .close_delay(Duration::from_secs(1))
                    .trigger(div().w(px(100.)).h(px(40.)).child("Trigger"))
                    .child("Preview"),
            )
        }
    }

    #[gpui::test]
    fn safe_transfer_listener_is_registered_during_paint(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| SafeTransferPaintFixture);
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.simulate_mouse_move(point(px(10.), px(10.)), None, Modifiers::default());
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.simulate_mouse_move(point(px(250.), px(250.)), None, Modifiers::default());
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
    }

    struct PlacementProbeFixture {
        bounds: Arc<Mutex<Option<(Bounds<Pixels>, Bounds<Pixels>)>>>,
        visible_trigger_bounds: Arc<Mutex<Option<Bounds<Pixels>>>>,
    }

    impl Render for PlacementProbeFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.bounds.clone();
            let visible_trigger_bounds = self.visible_trigger_bounds.clone();
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(400.))
                .h(px(400.))
                .child(
                    HoverCard::new("placement-probe")
                        .default_open(true)
                        .trigger(
                            div()
                                .w(px(100.))
                                .h(px(40.))
                                .on_prepaint(move |visible_bounds, _, _| {
                                    *visible_trigger_bounds.lock().unwrap() = Some(visible_bounds);
                                })
                                .child("Trigger"),
                        )
                        .content(move |state, _, _| {
                            *bounds.lock().unwrap() =
                                Some((state.trigger_bounds, state.content_bounds));
                            div().h(px(100.)).child("Preview")
                        }),
                )
        }
    }

    #[gpui::test]
    fn default_surface_uses_four_pixel_side_gap(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let bounds = Arc::new(Mutex::new(None));
        let captured = bounds.clone();
        let visible_trigger_bounds = Arc::new(Mutex::new(None));
        let visible_trigger_captured = visible_trigger_bounds.clone();
        let (view, cx) = cx.add_window_view(move |_, _| PlacementProbeFixture {
            bounds,
            visible_trigger_bounds,
        });

        for _ in 0..3 {
            view.update(cx, |_, cx| cx.notify());
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            cx.run_until_parked();
        }
        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();
        view.update(cx, |_, cx| cx.notify());
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let (trigger, content) = captured.lock().unwrap().expect("bounds must be captured");
        assert_eq!(
            trigger,
            visible_trigger_captured
                .lock()
                .unwrap()
                .expect("visible trigger bounds must be captured")
        );
        assert_eq!(content.top() - trigger.bottom(), px(4.));
    }
}
