use gpui::{
    Anchor, AnyElement, App, Bounds, Context, ElementId, InteractiveElement as _, IntoElement,
    ParentElement, Pixels, Render, RenderOnce, StatefulInteractiveElement, StyleRefinement, Styled,
    Task, Window, div, prelude::FluentBuilder as _, px,
};
use instant::Duration;
use std::{cell::Cell, rc::Rc};

use crate::{
    ActiveTheme as _, ElementExt, StyledExt as _,
    animation::{OverlayLifecycle, OverlayPhase, Transition, effective_motion_duration},
    popover::Popover,
    theme::OverlayPlacement,
};

/// A hover card element that displays content when hovering over a trigger element.
///
/// Similar to Popover but triggered by mouse hover instead of click, with configurable delays
/// for showing and hiding the content.
#[derive(IntoElement)]
pub struct HoverCard {
    id: ElementId,
    style: StyleRefinement,
    anchor: Anchor,
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
    /// Create a new HoverCard.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            anchor: Anchor::TopCenter,
            trigger: None,
            content: None,
            children: vec![],
            open_delay: Duration::from_secs_f64(0.6),
            close_delay: Duration::from_secs_f64(0.3),
            appearance: true,
            on_open_change: None,
        }
    }

    /// Set the anchor corner of the hover card, default is [`Anchor::TopCenter`].
    ///
    /// Imagine the card has a pointer tip (like a speech bubble's tail). The anchor
    /// is where that tip sits relative to the trigger — e.g. `Anchor::TopCenter`
    /// places it at the trigger's top center. The card then hangs off that point.
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Set the trigger element of the hover card.
    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: IntoElement + 'static,
    {
        self.trigger = Some(Box::new(|_, _| trigger.into_any_element()));
        self
    }

    /// Set the content builder of the hover card.
    ///
    /// The builder function receives the HoverCardState, Window, and Context as parameters.
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

    /// Set the delay before showing the hover card in milliseconds, default is 600ms.
    pub fn open_delay(mut self, duration: Duration) -> Self {
        self.open_delay = duration;
        self
    }

    /// Set the delay before hiding the hover card in milliseconds, default is 300ms.
    pub fn close_delay(mut self, duration: Duration) -> Self {
        self.close_delay = duration;
        self
    }

    /// Set whether to apply default appearance styles, default is `true`.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Set a callback to be called when the open state changes.
    pub fn on_open_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_open_change = Some(Rc::new(callback));
        self
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

/// State management for HoverCard component.
pub struct HoverCardState {
    lifecycle: OverlayLifecycle,
    trigger_bounds: Bounds<Pixels>,
    open_delay: Duration,
    close_delay: Duration,

    // Timer management
    open_task: Option<Task<()>>,
    close_task: Option<Task<()>>,
    epoch: usize, // Used to cancel stale timers

    // Hover state tracking
    is_hovering_trigger: bool,
    is_hovering_content: bool,

    // Callbacks
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl HoverCardState {
    fn new(open_delay: Duration, close_delay: Duration) -> Self {
        Self {
            lifecycle: OverlayLifecycle::default(),
            trigger_bounds: Bounds::default(),
            open_delay,
            close_delay,
            open_task: None,
            close_task: None,
            epoch: 0,
            is_hovering_trigger: false,
            is_hovering_content: false,
            on_open_change: None,
        }
    }

    /// Check if the hover card is open.
    pub fn is_open(&self) -> bool {
        self.lifecycle.accepts_input()
    }

    /// Schedule opening the hover card after the configured delay.
    fn schedule_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_tasks();
        let epoch = self.next_epoch();
        let delay = self.open_delay;

        self.open_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;

            let _ = this.update_in(cx, |state, window, cx| {
                if state.epoch == epoch {
                    state.set_open(true, window, cx);
                }
            });
        }));
    }

    /// Schedule closing the hover card after the configured delay.
    fn schedule_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_tasks();
        let epoch = self.next_epoch();
        let delay = self.close_delay;

        self.close_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;

            let _ = this.update_in(cx, |state, window, cx| {
                if state.epoch == epoch && !state.is_hovering_trigger && !state.is_hovering_content
                {
                    state.set_open(false, window, cx);
                }
            });
        }));
    }

    fn cancel_tasks(&mut self) {
        self.epoch += 1; // Invalidate all pending timers
        self.open_task = None;
        self.close_task = None;
    }

    fn next_epoch(&mut self) -> usize {
        self.epoch += 1;
        self.epoch
    }

    fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        let transition = if open {
            self.lifecycle.begin_open()
        } else {
            self.lifecycle.begin_close()
        };
        let Some(transition) = transition else {
            return;
        };

        if let Some(callback) = self.on_open_change.as_ref() {
            callback(&open, window, cx);
        }
        cx.notify();

        let duration = effective_motion_duration(cx.theme().style.motion.fast(), cx);
        cx.spawn_in(window, async move |state, cx| {
            cx.background_executor().timer(duration).await;
            let _ = state.update(cx, |state, cx| {
                let completed = if open {
                    state.lifecycle.complete_open(transition)
                } else {
                    state.lifecycle.complete_close(transition)
                };
                if completed {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Handle hover state change on the trigger element.
    fn on_trigger_hover(&mut self, hovering: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.is_hovering_trigger = hovering;

        if hovering {
            self.schedule_open(window, cx);
        } else {
            // Only close if not hovering content
            if !self.is_hovering_content {
                self.schedule_close(window, cx);
            }
        }
    }

    /// Handle hover state change on the content element.
    fn on_content_hover(&mut self, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.is_hovering_content = hovered;

        if hovered {
            self.cancel_tasks();
        } else {
            // Only close if not hovering trigger
            if !self.is_hovering_trigger {
                self.schedule_close(window, cx);
            }
        }
    }
}

impl Render for HoverCardState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div() // Empty render
    }
}

impl RenderOnce for HoverCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| {
            HoverCardState::new(self.open_delay, self.close_delay)
        });

        state.update(cx, |state, _| {
            state.open_delay = self.open_delay;
            state.close_delay = self.close_delay;
            state.on_open_change = self.on_open_change.clone();
        });

        let phase = state.read(cx).lifecycle.phase();
        let animation_key = state.read(cx).lifecycle.animation_key();
        let mounted = state.read(cx).lifecycle.is_mounted();
        let closing = phase == OverlayPhase::Closing;
        let trigger_bounds = state.read(cx).trigger_bounds;

        let Some(trigger) = self.trigger else {
            return div().id("empty");
        };

        let anchor = self.anchor;
        let position = Rc::new(Cell::new(Popover::resolved_corner(anchor, trigger_bounds)));

        let root = div().id(self.id).child(
            div()
                .id("trigger")
                .child((trigger)(window, cx))
                .on_hover(window.listener_for(&state, |state, hovered, window, cx| {
                    state.on_trigger_hover(*hovered, window, cx);
                }))
                .on_prepaint({
                    let state = state.clone();
                    let position = position.clone();
                    move |bounds, _, cx| {
                        position.set(Popover::resolved_corner(anchor, bounds));
                        state.update(cx, |state, _| {
                            state.trigger_bounds = bounds;
                        });
                    }
                }),
        );

        if !mounted {
            return root;
        }

        let popover_content =
            Popover::render_popover_content(self.anchor, self.appearance, window, cx)
                .overflow_hidden()
                .when(!closing, |this| {
                    this.on_hover(window.listener_for(&state, |state, hovered, window, cx| {
                        state.on_content_hover(*hovered, window, cx);
                    }))
                })
                .when_some(self.content, |this, content| {
                    this.child(state.update(cx, |state, cx| (content)(state, window, cx)))
                })
                .children(self.children)
                .when(closing, |this| {
                    this.child(div().absolute().top_0().left_0().size_full().occlude())
                })
                .refine_style(&self.style);

        let placement = match self.anchor {
            Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => OverlayPlacement::Top,
            Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => {
                OverlayPlacement::Bottom
            }
            Anchor::LeftCenter => OverlayPlacement::Left,
            Anchor::RightCenter => OverlayPlacement::Right,
        };
        let offset = cx.theme().style.overlays.enter_offset(placement);
        let motion = cx.theme().style.motion;
        let popover_content = Transition::new(motion.fast())
            .ease_token(if closing {
                motion.exit_easing
            } else {
                motion.enter_easing
            })
            .slide_x(
                if closing { px(0.) } else { offset.x },
                if closing { offset.x } else { px(0.) },
            )
            .slide_y(
                if closing { px(0.) } else { offset.y },
                if closing { offset.y } else { px(0.) },
            )
            .fade(
                if closing { 1.0 } else { 0.0 },
                if closing { 0.0 } else { 1.0 },
            )
            .apply(
                popover_content,
                ElementId::NamedInteger("hover-card-motion".into(), animation_key),
            );

        root.child(Popover::render_popover(
            self.anchor,
            position,
            popover_content,
            window,
            cx,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn reopen_during_hover_card_exit_keeps_latest_generation(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (state, cx) =
            cx.add_window_view(|_, _| HoverCardState::new(Duration::ZERO, Duration::ZERO));

        state.update_in(cx, |state, window, cx| {
            state.set_open(true, window, cx);
            state.set_open(false, window, cx);
            state.set_open(true, window, cx);
        });
        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();

        assert_eq!(
            state.read_with(cx, |state, _| state.lifecycle.phase()),
            OverlayPhase::Open
        );
    }
}
