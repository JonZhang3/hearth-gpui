use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    ActiveTheme, Density, Disableable, Side, Sizable, Size, StylePreset, StyledExt,
    animation::Lerp, h_flex, text::Text, theme::MotionEasing, tooltip::ComponentTooltip,
};
use gpui::{
    Animation, AnimationExt as _, App, Background, Div, ElementId, Hsla, InteractiveElement,
    IntoElement, ParentElement as _, Pixels, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Toggled, Window, div,
    prelude::FluentBuilder as _, px,
};

/// Geometry and elevation resolved from the active Style Preset without preset ID checks.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SwitchMetrics {
    track_width: Pixels,
    track_height: Pixels,
    thumb_edge: Pixels,
    thumb_travel: Pixels,
    shadow: bool,
}

/// Resolves the two shadcn sizes and proportional GPUI-specific size extensions.
fn switch_metrics(size: Size, style: &StylePreset) -> SwitchMetrics {
    let (track_width, track_height, thumb_edge) = match size {
        Size::XSmall => (px(20.), px(12.), px(10.)),
        Size::Small => (px(24.), px(14.), px(12.)),
        Size::Medium => (px(32.), px(18.4), px(16.)),
        Size::Large => (px(40.), px(22.), px(20.)),
        Size::Size(height) => {
            let height = height.max(px(4.));
            (height * 1.75, height, (height - px(2.)).max(px(2.)))
        }
    };

    SwitchMetrics {
        track_width,
        track_height,
        thumb_edge,
        thumb_travel: (track_width - thumb_edge - px(2.)).max(px(0.)),
        shadow: style.elevation.enabled && style.density == Density::Standard,
    }
}

/// Renderable Switch colors captured before a state transition begins.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SwitchPaintState {
    background: Background,
    border: Hsla,
    ring: Hsla,
    opacity: f32,
    thumb_x: Pixels,
}

/// Active color transition retained across rerenders for interruption-safe reversal.
#[derive(Debug, Clone, Copy)]
struct ActiveSwitchTransition {
    from: SwitchPaintState,
    target: SwitchPaintState,
    started_at: Instant,
    duration: Duration,
    easing: MotionEasing,
}

/// A renderable transition segment beginning at the currently sampled paint value.
#[derive(Debug, Clone, Copy)]
struct SwitchTransition {
    from: SwitchPaintState,
    to: SwitchPaintState,
    duration: Duration,
    epoch: u64,
}

/// Persistent paint state used to avoid jumps when a controlled Switch reverses rapidly.
#[derive(Debug, Clone, Copy)]
struct SwitchMotionState {
    target: SwitchPaintState,
    active: Option<ActiveSwitchTransition>,
    epoch: u64,
}

impl SwitchMotionState {
    /// Creates stable motion state without animating the first render.
    fn new(target: SwitchPaintState) -> Self {
        Self {
            target,
            active: None,
            epoch: 0,
        }
    }

    /// Samples the visible paint value and clears a completed transition.
    fn current(&mut self, now: Instant) -> SwitchPaintState {
        let Some(active) = self.active else {
            return self.target;
        };
        let elapsed = now.saturating_duration_since(active.started_at);
        let linear_delta = if active.duration.is_zero() {
            1.
        } else {
            elapsed.as_secs_f32() / active.duration.as_secs_f32()
        };
        let current = interpolate_switch_paint(
            active.from,
            active.target,
            active.easing.sample(linear_delta),
        );
        if linear_delta >= 1. {
            self.active = None;
            active.target
        } else {
            current
        }
    }

    /// Retargets from the current sampled value and scales reverse duration to distance traveled.
    fn transition_to(
        &mut self,
        target: SwitchPaintState,
        now: Instant,
        duration: Duration,
        easing: MotionEasing,
    ) -> Option<SwitchTransition> {
        let previous_active = self.active;
        let target_unchanged = self.target == target;
        let current = self.current(now);
        if target_unchanged && self.active.is_none() {
            return None;
        }

        self.target = target;
        self.epoch = self.epoch.wrapping_add(1);
        let duration = previous_active
            .map(|active| {
                let elapsed = now
                    .saturating_duration_since(active.started_at)
                    .min(active.duration);
                if target_unchanged {
                    active.duration.saturating_sub(elapsed)
                } else if target == active.from {
                    elapsed
                } else {
                    duration
                }
            })
            .unwrap_or(duration);
        if duration.is_zero() || current == target {
            self.active = None;
            return None;
        }

        self.active = Some(ActiveSwitchTransition {
            from: current,
            target,
            started_at: now,
            duration,
            easing,
        });
        Some(SwitchTransition {
            from: current,
            to: target,
            duration,
            epoch: self.epoch,
        })
    }
}

/// Interpolates solid paint values while preserving arbitrary renderable backgrounds.
fn interpolate_switch_paint(
    from: SwitchPaintState,
    to: SwitchPaintState,
    delta: f32,
) -> SwitchPaintState {
    let background = match (from.background.as_solid(), to.background.as_solid()) {
        (Some(from), Some(to)) => Lerp::lerp(&from, &to, delta).into(),
        _ if delta >= 1. => to.background,
        _ => from.background,
    };
    SwitchPaintState {
        background,
        border: Lerp::lerp(&from.border, &to.border, delta),
        ring: Lerp::lerp(&from.ring, &to.ring, delta),
        opacity: from.opacity + (to.opacity - from.opacity) * delta,
        thumb_x: Lerp::lerp(&from.thumb_x, &to.thumb_x, delta),
    }
}

/// Paints the Track and Thumb from one sampled state so their motion stays frame-synchronous.
#[allow(clippy::too_many_arguments)]
fn paint_switch_track(
    track: Div,
    paint: SwitchPaintState,
    metrics: SwitchMetrics,
    thumb_background: Background,
    show_ring: bool,
    ring_width: Pixels,
    ring_inset: Pixels,
) -> Div {
    track
        .bg(paint.background)
        .border_color(paint.border)
        .opacity(paint.opacity)
        .when(show_ring, |this| {
            this.child(
                div()
                    .absolute()
                    .top(-ring_inset)
                    .right(-ring_inset)
                    .bottom(-ring_inset)
                    .left(-ring_inset)
                    .border(ring_width)
                    .border_color(paint.ring)
                    .rounded(metrics.track_height * 0.5 + ring_width),
            )
        })
        .child(
            div()
                .size(metrics.thumb_edge)
                .rounded(metrics.thumb_edge * 0.5)
                .bg(thumb_background)
                .left(paint.thumb_x),
        )
}

/// Derives an internal Switch element ID without flattening structural caller IDs.
fn switch_child_id(id: &ElementId, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), name.into())
}

/// Mirrors shadcn's `focus-visible` policy for pointer and keyboard activation.
fn switch_focus_visible(focused: bool, last_input_was_keyboard: bool) -> bool {
    focused && last_input_was_keyboard
}

/// A binary control aligned with shadcn Switch semantics.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    style: StyleRefinement,
    checked: bool,
    disabled: bool,
    invalid: bool,
    label: Option<Text>,
    aria_label: Option<SharedString>,
    label_side: Side,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    size: Size,
    tab_stop: bool,
    tab_index: isize,
    color: Option<Hsla>,
    tooltip: ComponentTooltip,
}

impl Switch {
    /// Creates a Switch with the given stable element ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            checked: false,
            disabled: false,
            invalid: false,
            label: None,
            aria_label: None,
            on_click: None,
            label_side: Side::Right,
            size: Size::Medium,
            tab_stop: true,
            tab_index: 0,
            color: None,
            tooltip: ComponentTooltip::default(),
        }
    }

    /// Sets the controlled checked state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Sets the visible convenience label.
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the accessible name independently from the visible label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Places the convenience label on the left or right side of the track.
    pub fn label_side(mut self, side: Side) -> Self {
        self.label_side = side;
        self
    }

    /// Sets whether the Switch is invalid.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets whether the Switch participates in sequential keyboard focus.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Sets the keyboard tab index.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Sets the activation handler, receiving the requested next checked state.
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Overrides the checked Track background with a solid semantic color.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets tooltip text for the Switch.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip.text = Some((tooltip.into(), None));
        self
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Switch {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Switch {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let disabled = self.disabled;
        let focus_handle = window
            .use_keyed_state(switch_child_id(&self.id, "focus"), cx, |_, cx| {
                cx.focus_handle()
            })
            .read(cx)
            .clone();
        let focus_visible = switch_focus_visible(
            focus_handle.is_focused(window),
            window.last_input_was_keyboard(),
        );
        let metrics = switch_metrics(self.size, &cx.theme().style);
        let ring_width = cx.theme().style.focus.ring_width;
        let ring_inset = ring_width + cx.theme().style.focus.ring_offset;
        let invalid_color = cx
            .theme()
            .danger
            .opacity(if cx.theme().is_dark() { 0.5 } else { 1. });
        let border = if self.invalid {
            invalid_color
        } else if focus_visible {
            cx.theme().ring
        } else {
            cx.theme().transparent
        };
        let ring_color = if self.invalid {
            cx.theme()
                .danger
                .opacity(if cx.theme().is_dark() { 0.4 } else { 0.2 })
        } else {
            cx.theme().ring.opacity(0.5)
        };
        let ring_visible = self.invalid || focus_visible;
        let checked_background = self
            .color
            .map(Background::from)
            .unwrap_or(cx.theme().tokens.primary.background);
        let background = if checked {
            checked_background
        } else {
            cx.theme().tokens.switch.background
        };
        let thumb_x = if checked {
            metrics.thumb_travel
        } else {
            px(0.)
        };
        let paint = SwitchPaintState {
            background,
            border,
            ring: if ring_visible {
                ring_color
            } else {
                ring_color.opacity(0.)
            },
            opacity: if disabled { 0.5 } else { 1. },
            thumb_x,
        };
        let duration = if cx.reduce_motion() {
            Duration::ZERO
        } else {
            cx.theme().style.motion.normal()
        };
        let easing = cx.theme().style.motion.move_easing;
        let motion_state =
            window.use_keyed_state(switch_child_id(&self.id, "paint-motion"), cx, |_, _| {
                SwitchMotionState::new(paint)
            });
        let paint_transition = motion_state.update(cx, |state, _| {
            state.transition_to(paint, Instant::now(), duration, easing)
        });

        let configured_thumb = cx.theme().tokens.switch_thumb;
        let thumb_background = if configured_thumb != cx.theme().tokens.background {
            configured_thumb.background
        } else if cx.theme().is_dark() {
            if checked {
                cx.theme().primary_foreground.into()
            } else {
                cx.theme().foreground.into()
            }
        } else {
            configured_thumb.background
        };
        let show_ring = ring_visible
            || paint_transition
                .is_some_and(|transition| transition.from.ring.a > 0. || transition.to.ring.a > 0.);
        let track = div()
            .relative()
            .flex_none()
            .flex()
            .items_center()
            .w(metrics.track_width)
            .h(metrics.track_height)
            .rounded(metrics.track_height * 0.5)
            .border_1()
            .when(metrics.shadow, |this| this.shadow_xs());
        let track = if let Some(transition) = paint_transition {
            track
                .with_animation(
                    switch_child_id(&self.id, format!("paint-{}", transition.epoch)),
                    Animation::new(transition.duration)
                        .with_easing(move |delta| easing.sample(delta)),
                    move |this, delta| {
                        let paint = interpolate_switch_paint(transition.from, transition.to, delta);
                        paint_switch_track(
                            this,
                            paint,
                            metrics,
                            thumb_background,
                            show_ring,
                            ring_width,
                            ring_inset,
                        )
                    },
                )
                .into_any_element()
        } else {
            paint_switch_track(
                track,
                paint,
                metrics,
                thumb_background,
                show_ring,
                ring_width,
                ring_inset,
            )
            .into_any_element()
        };

        let accessible_label = self
            .aria_label
            .or_else(|| self.label.as_ref().map(|label| label.get_text(cx)));
        let on_click = self.on_click.clone();
        let interactive = !disabled && on_click.is_some();
        let element = h_flex()
            .id(self.id.clone())
            .role(Role::Switch)
            .aria_toggled(if checked {
                Toggled::True
            } else {
                Toggled::False
            })
            .when_some(accessible_label, |this, label| this.aria_label(label))
            .when(interactive, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_stop(self.tab_stop)
                        .tab_index(self.tab_index),
                )
            })
            .gap_2()
            .items_center()
            .when(self.label_side.is_left(), |this| this.flex_row_reverse())
            .child(track)
            .when_some(self.label, |this, label| {
                this.child(
                    div()
                        .line_height(metrics.track_height)
                        .map(|this| match self.size {
                            Size::XSmall => this.text_xs(),
                            Size::Large => this.text_base(),
                            Size::Small | Size::Medium | Size::Size(_) => this.text_sm(),
                        })
                        .child(label),
                )
            })
            .refine_style(&self.style)
            .when(interactive, |this| {
                this.on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                    // Pointer activation must not create a keyboard focus-visible ring.
                    window.prevent_default();
                })
            })
            .when(interactive, |this| {
                this.on_click(move |_, window, cx| {
                    window.prevent_default();
                    if let Some(on_click) = on_click.as_ref() {
                        on_click(&!checked, window, cx);
                    }
                })
            })
            .map(|this| self.tooltip.apply(&self.id, this));

        crate::accessibility::accessibility_state(element, self.invalid, false, disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementExt as _;
    use gpui::{
        AppContext as _, Element as _, KeyDownEvent, KeyUpEvent, Keystroke, Render, TestAppContext,
        VisualTestContext, accesskit, div,
    };
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    #[test]
    fn metrics_match_builtin_shadcn_presets() {
        for style in [
            StylePreset::vega(),
            StylePreset::nova(),
            StylePreset::maia(),
        ] {
            let default = switch_metrics(Size::Medium, &style);
            assert_eq!(default.track_width, px(32.));
            assert_eq!(default.track_height, px(18.4));
            assert_eq!(default.thumb_edge, px(16.));
            assert_eq!(default.thumb_travel, px(14.));

            let small = switch_metrics(Size::Small, &style);
            assert_eq!(small.track_width, px(24.));
            assert_eq!(small.track_height, px(14.));
            assert_eq!(small.thumb_edge, px(12.));
            assert_eq!(small.thumb_travel, px(10.));
        }
        assert!(switch_metrics(Size::Medium, &StylePreset::vega()).shadow);
        assert!(!switch_metrics(Size::Medium, &StylePreset::nova()).shadow);
        assert!(!switch_metrics(Size::Medium, &StylePreset::maia()).shadow);
    }

    #[test]
    fn builder_preserves_public_configuration() {
        let switch = Switch::new("builder")
            .checked(true)
            .label("Visible")
            .aria_label("Accessible")
            .label_side(Side::Left)
            .invalid(true)
            .disabled(true)
            .small()
            .tab_stop(false)
            .tab_index(2)
            .color(Hsla::red())
            .tooltip("Details")
            .on_click(|_, _, _| {});

        assert!(switch.checked);
        assert!(switch.label.is_some());
        assert_eq!(switch.aria_label.as_deref(), Some("Accessible"));
        assert!(switch.label_side.is_left());
        assert!(switch.invalid);
        assert!(switch.disabled);
        assert_eq!(switch.size, Size::Small);
        assert!(!switch.tab_stop);
        assert_eq!(switch.tab_index, 2);
        assert!(switch.color.is_some());
        assert!(switch.tooltip.text.is_some());
        assert!(switch.on_click.is_some());
    }

    #[test]
    fn motion_state_skips_initial_render_and_reverses_from_current_value() {
        let off = SwitchPaintState {
            background: Hsla::white().into(),
            border: Hsla::transparent_black(),
            ring: Hsla::transparent_black(),
            opacity: 1.,
            thumb_x: px(0.),
        };
        let on = SwitchPaintState {
            background: Hsla::black().into(),
            border: Hsla::red(),
            ring: Hsla::red(),
            opacity: 0.5,
            thumb_x: px(14.),
        };
        let now = Instant::now();
        let duration = Duration::from_millis(150);
        let mut state = SwitchMotionState::new(off);

        assert!(
            state
                .transition_to(off, now, duration, MotionEasing::Linear)
                .is_none()
        );
        let forward = state
            .transition_to(on, now, duration, MotionEasing::Linear)
            .unwrap();
        assert_eq!(forward.from, off);
        assert_eq!(forward.to, on);

        let halfway = now + Duration::from_millis(75);
        let reverse = state
            .transition_to(off, halfway, duration, MotionEasing::Linear)
            .unwrap();
        assert_eq!(reverse.duration, Duration::from_millis(75));
        assert_eq!(reverse.from.opacity, 0.75);
        assert_eq!(reverse.from.thumb_x, px(7.));
        assert_eq!(reverse.to, off);
    }

    #[test]
    fn reduced_motion_reaches_the_target_without_an_active_transition() {
        let off = SwitchPaintState {
            background: Hsla::white().into(),
            border: Hsla::transparent_black(),
            ring: Hsla::transparent_black(),
            opacity: 1.,
            thumb_x: px(0.),
        };
        let on = SwitchPaintState {
            background: Hsla::black().into(),
            border: Hsla::red(),
            ring: Hsla::red(),
            opacity: 0.5,
            thumb_x: px(14.),
        };
        let mut state = SwitchMotionState::new(off);

        assert!(
            state
                .transition_to(on, Instant::now(), Duration::ZERO, MotionEasing::Linear)
                .is_none()
        );
        assert_eq!(state.target, on);
        assert!(state.active.is_none());
    }

    #[test]
    fn focus_ring_is_limited_to_keyboard_focus() {
        assert!(switch_focus_visible(true, true));
        assert!(!switch_focus_visible(true, false));
        assert!(!switch_focus_visible(false, true));
        assert!(!switch_focus_visible(false, false));
    }

    #[test]
    fn internal_ids_preserve_structural_identity() {
        let structured = ElementId::NamedInteger("switch".into(), 1);
        let textual = ElementId::Name("switch-1".into());

        assert_eq!(structured.to_string(), textual.to_string());
        assert_ne!(
            switch_child_id(&structured, "paint-motion"),
            switch_child_id(&textual, "paint-motion")
        );
    }

    #[derive(Debug, PartialEq)]
    struct SwitchAccessibility {
        role: Role,
        label: Option<String>,
        toggled: Option<Toggled>,
        invalid: bool,
        disabled: bool,
    }

    struct AccessibilityProbe {
        metadata: Arc<Mutex<Option<SwitchAccessibility>>>,
    }

    impl Render for AccessibilityProbe {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let metadata = self.metadata.clone();
            div().on_prepaint(move |_, window, cx| {
                let mut node = accesskit::Node::new(Role::Switch);
                let switch = Switch::new("accessible-switch")
                    .aria_label("Airplane mode")
                    .checked(true)
                    .invalid(true)
                    .disabled(true)
                    .render(window, cx)
                    .into_element();
                let role = switch
                    .a11y_role()
                    .expect("switch must expose its accessibility role");
                switch.write_a11y_info(&mut node);
                *metadata.lock().unwrap() = Some(SwitchAccessibility {
                    role,
                    label: node.label().map(ToOwned::to_owned),
                    toggled: node.toggled(),
                    invalid: node.invalid() == Some(accesskit::Invalid::True),
                    disabled: node.is_disabled(),
                });
            })
        }
    }

    #[gpui::test]
    fn exposes_name_checked_invalid_and_disabled(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let metadata = Arc::new(Mutex::new(None));
        let captured = metadata.clone();
        let (_, cx) = cx.add_window_view(move |_, _| AccessibilityProbe { metadata });
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        assert_eq!(
            *captured.lock().unwrap(),
            Some(SwitchAccessibility {
                role: Role::Switch,
                label: Some("Airplane mode".into()),
                toggled: Some(Toggled::True),
                invalid: true,
                disabled: true,
            })
        );
    }

    struct KeyboardFixture {
        calls: Arc<AtomicUsize>,
        checked: Arc<AtomicBool>,
    }

    impl Render for KeyboardFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            let checked = self.checked.clone();
            div()
                .child(Switch::new("read-only-switch").checked(true))
                .child(Switch::new("disabled-switch").disabled(true))
                .child(
                    Switch::new("keyboard-switch")
                        .aria_label("Airplane mode")
                        .on_click(move |value, _, _| {
                            calls.fetch_add(1, Ordering::SeqCst);
                            checked.store(*value, Ordering::SeqCst);
                        }),
                )
        }
    }

    #[gpui::test]
    fn space_activates_once_and_ignores_key_repeat(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let checked = Arc::new(AtomicBool::new(false));
        let captured_calls = calls.clone();
        let captured_checked = checked.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| KeyboardFixture { calls, checked });
            crate::Root::new(fixture, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
        });
        let space = Keystroke::parse("space").expect("space must be valid");
        cx.simulate_event(KeyDownEvent {
            keystroke: space.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyDownEvent {
            keystroke: space.clone(),
            is_held: true,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke: space });
        cx.run_until_parked();

        assert_eq!(captured_calls.load(Ordering::SeqCst), 1);
        assert!(captured_checked.load(Ordering::SeqCst));
    }
}
