use std::{
    ops::Range,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    ActiveTheme, AxisExt, Density, ElementExt, StylePreset, StyledExt,
    accessibility::accessibility_state, animation::Lerp, h_flex,
};
use gpui::{
    AccessibleAction, Along, Animation, AnimationExt, AnyElement, App, AppContext as _, Axis,
    Background, Bounds, Context, Corners, DefiniteLength, DragMoveEvent, ElementId, Empty, Entity,
    EntityId, EventEmitter, Fill, FocusHandle, Hsla, InteractiveElement, IntoElement, IsZero,
    KeyDownEvent, MouseButton, MouseDownEvent, Orientation, ParentElement as _, Pixels, Point,
    Render, RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _, px, relative,
};
use rust_i18n::t;

/// The shadcn transition family used by a Slider Style Preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SliderMotionKind {
    Ring,
    Colors,
}

/// Component-local geometry resolved from semantic Style Preset density.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SliderMetrics {
    track_edge: Pixels,
    thumb_edge: Pixels,
    ring_width: Pixels,
    hit_edge: Pixels,
    min_vertical_length: Pixels,
    shadow: bool,
    thumb_border_uses_ring: bool,
    motion_kind: SliderMotionKind,
}

impl SliderMetrics {
    /// Resolves pinned Vega, Nova, and Maia geometry without preset ID checks.
    fn resolve(style: &StylePreset) -> Self {
        match style.density {
            Density::Compact => Self {
                track_edge: px(4.),
                thumb_edge: px(12.),
                ring_width: px(3.),
                hit_edge: px(28.),
                min_vertical_length: px(160.),
                shadow: false,
                thumb_border_uses_ring: true,
                motion_kind: SliderMotionKind::Ring,
            },
            Density::Standard => Self {
                track_edge: px(6.),
                thumb_edge: px(16.),
                ring_width: px(4.),
                hit_edge: px(24.),
                min_vertical_length: px(160.),
                shadow: style.elevation.enabled,
                thumb_border_uses_ring: false,
                motion_kind: SliderMotionKind::Ring,
            },
            Density::Comfortable => Self {
                track_edge: px(12.),
                thumb_edge: px(16.),
                ring_width: px(4.),
                hit_edge: px(24.),
                min_vertical_length: px(160.),
                shadow: style.elevation.enabled,
                thumb_border_uses_ring: false,
                motion_kind: SliderMotionKind::Colors,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ActiveRingTransition {
    from: Hsla,
    target: Hsla,
    started_at: Instant,
    duration: Duration,
    easing: crate::MotionEasing,
}

#[derive(Debug, Clone, Copy)]
struct RingTransition {
    from: Hsla,
    to: Hsla,
    duration: Duration,
    epoch: u64,
}

/// Persistent Thumb ring motion that retargets from the currently visible color.
#[derive(Debug, Clone, Copy)]
struct SliderRingMotionState {
    target: Hsla,
    active: Option<ActiveRingTransition>,
    epoch: u64,
}

struct SliderThumbSpec {
    id: ElementId,
    position: DefiniteLength,
    is_start: bool,
    value: f64,
    min: f64,
    max: f64,
    label: SharedString,
    description: Option<SharedString>,
    focus_handle: FocusHandle,
    hovered: bool,
    pressed: bool,
    thumb_background: Background,
    thumb_border: Hsla,
    radius: Corners<Pixels>,
    metrics: SliderMetrics,
}

impl SliderRingMotionState {
    fn new(target: Hsla) -> Self {
        Self {
            target,
            active: None,
            epoch: 0,
        }
    }

    fn current(&mut self, now: Instant) -> Hsla {
        let Some(active) = self.active else {
            return self.target;
        };
        let elapsed = now.saturating_duration_since(active.started_at);
        let linear_delta = if active.duration.is_zero() {
            1.
        } else {
            elapsed.as_secs_f32() / active.duration.as_secs_f32()
        };
        let current = Lerp::lerp(
            &active.from,
            &active.target,
            active.easing.sample(linear_delta),
        );
        if linear_delta >= 1. {
            self.active = None;
            active.target
        } else {
            current
        }
    }

    fn transition_to(
        &mut self,
        target: Hsla,
        now: Instant,
        duration: Duration,
        easing: crate::MotionEasing,
    ) -> Option<RingTransition> {
        let previous = self.active;
        let target_unchanged = self.target == target;
        let current = self.current(now);
        if target_unchanged && self.active.is_none() {
            return None;
        }

        self.target = target;
        self.epoch = self.epoch.wrapping_add(1);
        let duration = previous
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

        self.active = Some(ActiveRingTransition {
            from: current,
            target,
            started_at: now,
            duration,
            easing,
        });
        Some(RingTransition {
            from: current,
            to: target,
            duration,
            epoch: self.epoch,
        })
    }
}

fn slider_child_id(id: &ElementId, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), name.into())
}

#[derive(Clone)]
struct DragThumb((EntityId, bool));

impl Render for DragThumb {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone)]
struct DragSlider(EntityId);

impl Render for DragSlider {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// Events emitted by the [`SliderState`].
pub enum SliderEvent {
    /// Emitted continuously while the slider value is being changed by the user.
    Change(SliderValue),
    /// Emitted once when the user releases the slider after a drag or click.
    Release(SliderValue),
}

/// The value of the slider, can be a single value or a range of values.
///
/// - Can from a f32 value, which will be treated as a single value.
/// - Or from a (f32, f32) tuple, which will be treated as a range of values.
///
/// The default value is `SliderValue::Single(0.0)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SliderValue {
    Single(f32),
    Range(f32, f32),
}

impl std::fmt::Display for SliderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SliderValue::Single(value) => write!(f, "{}", value),
            SliderValue::Range(start, end) => write!(f, "{}..{}", start, end),
        }
    }
}

impl From<f32> for SliderValue {
    fn from(value: f32) -> Self {
        SliderValue::Single(value)
    }
}

impl From<(f32, f32)> for SliderValue {
    fn from(value: (f32, f32)) -> Self {
        SliderValue::Range(value.0, value.1)
    }
}

impl From<Range<f32>> for SliderValue {
    fn from(value: Range<f32>) -> Self {
        SliderValue::Range(value.start, value.end)
    }
}

impl Default for SliderValue {
    fn default() -> Self {
        SliderValue::Single(0.)
    }
}

impl SliderValue {
    /// Clamp the value to the given range.
    pub fn clamp(self, min: f32, max: f32) -> Self {
        match self {
            SliderValue::Single(value) => SliderValue::Single(value.clamp(min, max)),
            SliderValue::Range(start, end) => {
                SliderValue::Range(start.clamp(min, max), end.clamp(min, max))
            }
        }
    }

    /// Check if the value is a single value.
    #[inline]
    pub fn is_single(&self) -> bool {
        matches!(self, SliderValue::Single(_))
    }

    /// Check if the value is a range of values.
    #[inline]
    pub fn is_range(&self) -> bool {
        matches!(self, SliderValue::Range(_, _))
    }

    /// Get the start value.
    pub fn start(&self) -> f32 {
        match self {
            SliderValue::Single(value) => *value,
            SliderValue::Range(start, _) => *start,
        }
    }

    /// Get the end value.
    pub fn end(&self) -> f32 {
        match self {
            SliderValue::Single(value) => *value,
            SliderValue::Range(_, end) => *end,
        }
    }

    fn set_start(&mut self, value: f32) {
        if let SliderValue::Range(_, end) = self {
            *self = SliderValue::Range(value.min(*end), *end);
        } else {
            *self = SliderValue::Single(value);
        }
    }

    fn set_end(&mut self, value: f32) {
        if let SliderValue::Range(start, _) = self {
            *self = SliderValue::Range(*start, value.max(*start));
        } else {
            *self = SliderValue::Single(value);
        }
    }
}

/// The scale mode of the slider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SliderScale {
    /// Linear scale where values change uniformly across the slider range.
    /// This is the default mode.
    #[default]
    Linear,
    /// Logarithmic scale where the distance between values increases exponentially.
    ///
    /// This is useful for parameters that have a large range of values where smaller
    /// changes are more significant at lower values. Common examples include:
    ///
    /// - Volume controls (human hearing perception is logarithmic)
    /// - Frequency controls (musical notes follow a logarithmic scale)
    /// - Zoom levels
    /// - Any parameter where you want finer control at lower values
    ///
    /// # For example
    ///
    /// ```
    /// use gpui_component::slider::{SliderState, SliderScale};
    ///
    /// let slider = SliderState::new()
    ///     .min(1.0)    // Must be > 0 for logarithmic scale
    ///     .max(1000.0)
    ///     .scale(SliderScale::Logarithmic);
    /// ```
    ///
    /// - Moving the slider 1/3 of the way will yield ~10
    /// - Moving it 2/3 of the way will yield ~100
    /// - The full range covers 3 orders of magnitude evenly
    Logarithmic,
}

impl SliderScale {
    #[inline]
    pub fn is_linear(&self) -> bool {
        matches!(self, SliderScale::Linear)
    }

    #[inline]
    pub fn is_logarithmic(&self) -> bool {
        matches!(self, SliderScale::Logarithmic)
    }
}

/// State of the [`Slider`].
pub struct SliderState {
    min: f32,
    max: f32,
    step: f32,
    value: SliderValue,
    /// When is single value mode, only `end` is used, the start is always 0.0.
    percentage: Range<f32>,
    /// The bounds of the slider after rendered.
    bounds: Bounds<Pixels>,
    scale: SliderScale,
    /// Tracks whether the user is currently interacting with the slider so we
    /// only emit [`SliderEvent::Release`] after a real press/drag.
    dragging: bool,
    /// The range thumb currently controlled by keyboard and accessibility actions.
    active_start_thumb: bool,
    hovered_start_thumb: bool,
    hovered_end_thumb: bool,
}

impl SliderState {
    /// Create a new [`SliderState`].
    pub fn new() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            value: SliderValue::default(),
            percentage: (0.0..0.0),
            bounds: Bounds::default(),
            scale: SliderScale::default(),
            dragging: false,
            active_start_thumb: false,
            hovered_start_thumb: false,
            hovered_end_thumb: false,
        }
    }

    /// Set the minimum value of the slider, default: 0.0
    pub fn min(mut self, min: f32) -> Self {
        if self.scale.is_logarithmic() {
            assert!(
                min > 0.0,
                "`min` must be greater than 0 for SliderScale::Logarithmic"
            );
            assert!(
                min < self.max,
                "`min` must be less than `max` for Logarithmic scale"
            );
        }
        self.min = min;
        self.value = self.normalize_value(self.value);
        self.update_thumb_pos();
        self
    }

    /// Set the maximum value of the slider, default: 100.0
    pub fn max(mut self, max: f32) -> Self {
        if self.scale.is_logarithmic() {
            assert!(
                max > self.min,
                "`max` must be greater than `min` for Logarithmic scale"
            );
        }
        self.max = max;
        self.value = self.normalize_value(self.value);
        self.update_thumb_pos();
        self
    }

    /// Set the step value of the slider, default: 1.0
    pub fn step(mut self, step: f32) -> Self {
        assert!(
            step.is_finite() && step > 0.0,
            "`step` must be finite and greater than 0"
        );
        self.step = step;
        self
    }

    /// Set the scale of the slider, default: [`SliderScale::Linear`].
    pub fn scale(mut self, scale: SliderScale) -> Self {
        if scale.is_logarithmic() {
            assert!(
                self.min > 0.0,
                "`min` must be greater than 0 for Logarithmic scale"
            );
            assert!(
                self.max > self.min,
                "`max` must be greater than `min` for Logarithmic scale"
            );
        }
        self.scale = scale;
        self.update_thumb_pos();
        self
    }

    /// Set the default value of the slider, default: 0.0
    pub fn default_value(mut self, value: impl Into<SliderValue>) -> Self {
        self.value = self.normalize_value(value.into());
        self.update_thumb_pos();
        self
    }

    /// Set the value of the slider.
    pub fn set_value(
        &mut self,
        value: impl Into<SliderValue>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.value = self.normalize_value(value.into());
        self.update_thumb_pos();
        cx.notify();
    }

    /// Get the value of the slider.
    pub fn value(&self) -> SliderValue {
        self.value
    }

    /// Get the minimum value.
    pub fn min_value(&self) -> f32 {
        self.min
    }

    /// Get the maximum value.
    pub fn max_value(&self) -> f32 {
        self.max
    }

    /// Get the step value.
    pub fn step_value(&self) -> f32 {
        self.step
    }

    /// Converts a value between 0.0 and 1.0 to a value between the minimum and maximum value,
    /// depending on the chosen scale.
    fn percentage_to_value(&self, percentage: f32) -> f32 {
        match self.scale {
            SliderScale::Linear => self.min + (self.max - self.min) * percentage,
            SliderScale::Logarithmic => {
                // when percentage is 0, this simplifies to (max/min)^0 * min = 1 * min = min
                // when percentage is 1, this simplifies to (max/min)^1 * min = (max*min)/min = max
                // we clamp just to make sure we don't have issue with floating point precision
                let base = self.max / self.min;
                (base.powf(percentage) * self.min).clamp(self.min, self.max)
            }
        }
    }

    /// Converts a value between the minimum and maximum value to a value between 0.0 and 1.0,
    /// depending on the chosen scale.
    fn value_to_percentage(&self, value: f32) -> f32 {
        match self.scale {
            SliderScale::Linear => {
                let range = self.max - self.min;
                if range <= 0.0 {
                    0.0
                } else {
                    (value - self.min) / range
                }
            }
            SliderScale::Logarithmic => {
                let base = self.max / self.min;
                (value / self.min).log(base).clamp(0.0, 1.0)
            }
        }
    }

    fn update_thumb_pos(&mut self) {
        match self.value {
            SliderValue::Single(value) => {
                let percentage = self.value_to_percentage(value.clamp(self.min, self.max));
                self.percentage = 0.0..percentage;
            }
            SliderValue::Range(start, end) => {
                let clamped_start = start.clamp(self.min, self.max);
                let clamped_end = end.clamp(self.min, self.max);
                self.percentage =
                    self.value_to_percentage(clamped_start)..self.value_to_percentage(clamped_end);
            }
        }
    }

    /// Clamps and orders values so visual, keyboard, and accessibility state agree.
    fn normalize_value(&self, value: SliderValue) -> SliderValue {
        if self.min > self.max {
            return value;
        }
        match value.clamp(self.min, self.max) {
            SliderValue::Single(value) => SliderValue::Single(value),
            SliderValue::Range(start, end) if start <= end => SliderValue::Range(start, end),
            SliderValue::Range(start, end) => SliderValue::Range(end, start),
        }
    }

    /// Snaps a raw value to the configured step relative to the minimum value.
    fn snap_value(&self, value: f32) -> f32 {
        if self.min >= self.max {
            return self.min;
        }
        let steps = ((value - self.min) / self.step).round();
        (self.min + steps * self.step).clamp(self.min, self.max)
    }

    /// Updates one thumb and emits the same change/commit events for keyboard and a11y input.
    fn set_thumb_value(
        &mut self,
        is_start: bool,
        value: f32,
        commit: bool,
        cx: &mut Context<Self>,
    ) {
        let previous = self.value;
        let value = self.snap_value(value);
        if is_start {
            self.value.set_start(value);
        } else {
            self.value.set_end(value);
        }
        self.active_start_thumb = is_start;
        self.update_thumb_pos();
        if self.value == previous {
            return;
        }
        cx.emit(SliderEvent::Change(self.value));
        if commit {
            cx.emit(SliderEvent::Release(self.value));
        }
        cx.notify();
    }

    /// Handles the native Slider keyboard contract for one thumb.
    fn handle_key(&mut self, key: &str, is_start: bool, cx: &mut Context<Self>) -> bool {
        let current = if is_start {
            self.value.start()
        } else {
            self.value.end()
        };
        let value = match key {
            "left" | "down" => current - self.step,
            "right" | "up" => current + self.step,
            "home" => self.min,
            "end" => self.max,
            _ => return false,
        };
        self.set_thumb_value(is_start, value, true, cx);
        true
    }

    fn set_thumb_hovered(&mut self, is_start: bool, hovered: bool, cx: &mut Context<Self>) {
        let target = if is_start {
            &mut self.hovered_start_thumb
        } else {
            &mut self.hovered_end_thumb
        };
        if *target == hovered {
            return;
        }
        *target = hovered;
        cx.notify();
    }

    /// Update value by mouse position
    fn update_value_by_position(
        &mut self,
        axis: Axis,
        position: Point<Pixels>,
        is_start: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dragging = true;
        let bounds = self.bounds;

        let inner_pos = if axis.is_horizontal() {
            position.x - bounds.left()
        } else {
            bounds.bottom() - position.y
        };
        let total_size = bounds.size.along(axis);
        if total_size <= px(0.) {
            return;
        }
        let percentage = inner_pos.clamp(px(0.), total_size) / total_size;

        let percentage = if is_start {
            percentage.clamp(0.0, self.percentage.end)
        } else {
            percentage.clamp(self.percentage.start, 1.0)
        };

        let value = self.snap_value(self.percentage_to_value(percentage));

        if is_start {
            self.value.set_start(value);
        } else {
            self.value.set_end(value);
        }
        self.active_start_thumb = is_start;
        // Recompute the visual position from the snapped value so the Thumb,
        // accessibility value, and emitted value always describe one state.
        self.update_thumb_pos();
        cx.emit(SliderEvent::Change(self.value));
        cx.notify();
    }

    /// Emit [`SliderEvent::Release`] if the user was actively interacting
    /// with the slider. Called on mouse-up both inside and outside the slider.
    fn handle_release(&mut self, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        self.dragging = false;
        cx.emit(SliderEvent::Release(self.value));
    }
}

impl EventEmitter<SliderEvent> for SliderState {}

/// A Slider element.
#[derive(IntoElement)]
pub struct Slider {
    state: Entity<SliderState>,
    axis: Axis,
    style: StyleRefinement,
    disabled: bool,
    reverse: bool,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
}

impl Slider {
    /// Create a new [`Slider`] element bind to the [`SliderState`].
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            axis: Axis::Horizontal,
            state: state.clone(),
            style: StyleRefinement::default(),
            disabled: false,
            reverse: false,
            aria_label: None,
            aria_description: None,
        }
    }

    /// As a horizontal slider.
    pub fn horizontal(mut self) -> Self {
        self.axis = Axis::Horizontal;
        self
    }

    /// As a vertical slider.
    pub fn vertical(mut self) -> Self {
        self.axis = Axis::Vertical;
        self
    }

    /// Set the disabled state of the slider, default: false
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the accessible name announced for the Slider thumbs.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets additional accessible help text for the Slider thumbs.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Reverse the filled (highlighted) side of the track, default: false.
    ///
    /// By default the track is filled from the min end to the thumb. With
    /// `reverse`, the fill goes from the thumb to the max end instead — useful
    /// when the slider represents a remaining amount (e.g. time left).
    ///
    /// This only changes the visual fill; values, events and interactions are
    /// unaffected. It applies to single-value sliders and is ignored for
    /// range sliders.
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    fn render_thumb(&self, spec: SliderThumbSpec, window: &mut Window, cx: &mut App) -> AnyElement {
        let entity_id = self.state.entity_id();
        let axis = self.axis;
        let is_start = spec.is_start;
        let focus_visible =
            spec.focus_handle.is_focused(window) && window.last_input_was_keyboard();
        let ring_visible = !self.disabled && (spec.hovered || spec.pressed || focus_visible);
        let ring_color = cx.theme().ring.opacity(if ring_visible { 0.5 } else { 0. });
        let duration = if cx.reduce_motion() || spec.metrics.motion_kind != SliderMotionKind::Ring {
            Duration::ZERO
        } else {
            cx.theme().style.motion.normal()
        };
        let easing = cx.theme().style.motion.move_easing;
        let motion_id = slider_child_id(&spec.id, "ring-motion");
        let motion_state =
            window.use_keyed_state(motion_id, cx, |_, _| SliderRingMotionState::new(ring_color));
        let transition = motion_state.update(cx, |state, _| {
            state.transition_to(ring_color, Instant::now(), duration, easing)
        });
        let ring_edge = spec.metrics.thumb_edge + spec.metrics.ring_width * 2.;
        let thumb = div()
            .size(spec.metrics.thumb_edge)
            .flex_shrink_0()
            .border_1()
            .border_color(spec.thumb_border)
            .corner_radii(spec.radius)
            .bg(spec.thumb_background)
            .when(spec.metrics.shadow, |this| this.shadow_sm());
        let ring_radius = spec.radius.map(|radius| {
            if radius.is_zero() {
                px(0.)
            } else {
                *radius + spec.metrics.ring_width
            }
        });
        let ring = div()
            .size(ring_edge)
            .flex()
            .items_center()
            .justify_center()
            .corner_radii(ring_radius)
            .bg(ring_color)
            .child(thumb);
        let ring = if let Some(transition) = transition {
            let animation_id = slider_child_id(&spec.id, format!("ring-{}", transition.epoch));
            ring.with_animation(
                animation_id,
                Animation::new(transition.duration).with_easing(move |delta| easing.sample(delta)),
                move |this, delta| this.bg(Lerp::lerp(&transition.from, &transition.to, delta)),
            )
            .into_any_element()
        } else {
            ring.into_any_element()
        };
        let state_for_increment = self.state.clone();
        let state_for_decrement = self.state.clone();
        let state_for_set_value = self.state.clone();
        let state_for_keyboard = self.state.clone();
        let state_for_hover = self.state.clone();
        let focus_on_press = spec.focus_handle.clone();

        let thumb = div()
            .id(spec.id)
            .role(Role::Slider)
            .aria_numeric_value(spec.value)
            .aria_min_numeric_value(spec.min)
            .aria_max_numeric_value(spec.max)
            .aria_numeric_value_step(self.state.read(cx).step_value() as f64)
            .aria_orientation(if axis.is_vertical() {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            })
            .aria_label(spec.label)
            .when_some(spec.description, |this, description| {
                this.aria_description(description)
            })
            .absolute()
            .when(axis.is_horizontal(), |this| {
                this.top((spec.metrics.track_edge - spec.metrics.hit_edge) * 0.5)
                    .left(spec.position)
                    .ml(spec.metrics.hit_edge * -0.5)
            })
            .when(axis.is_vertical(), |this| {
                this.bottom(spec.position)
                    .left((spec.metrics.track_edge - spec.metrics.hit_edge) * 0.5)
                    .mb(spec.metrics.hit_edge * -0.5)
            })
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0()
            .size(spec.metrics.hit_edge)
            .child(ring)
            .when(!self.disabled, |this| {
                this.track_focus(&spec.focus_handle.tab_stop(true))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        window.prevent_default();
                        focus_on_press.focus(window, cx);
                        cx.stop_propagation();
                    })
                    .on_hover(move |hovered, _, cx| {
                        state_for_hover.update(cx, |state, cx| {
                            state.set_thumb_hovered(is_start, *hovered, cx)
                        });
                    })
                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                        if event.keystroke.modifiers.modified() {
                            return;
                        }
                        state_for_keyboard.update(cx, |state, cx| {
                            if state.handle_key(&event.keystroke.key, is_start, cx) {
                                window.prevent_default();
                                cx.stop_propagation();
                            }
                        });
                    })
                    .on_a11y_action(AccessibleAction::Increment, move |_, _, cx| {
                        state_for_increment.update(cx, |state, cx| {
                            let current = if is_start {
                                state.value().start()
                            } else {
                                state.value().end()
                            };
                            state.set_thumb_value(is_start, current + state.step_value(), true, cx);
                        });
                    })
                    .on_a11y_action(AccessibleAction::Decrement, move |_, _, cx| {
                        state_for_decrement.update(cx, |state, cx| {
                            let current = if is_start {
                                state.value().start()
                            } else {
                                state.value().end()
                            };
                            state.set_thumb_value(is_start, current - state.step_value(), true, cx);
                        });
                    })
                    .on_a11y_action(AccessibleAction::SetValue, move |data, _, cx| {
                        let Some(gpui::accesskit::ActionData::Value(value)) = data else {
                            return;
                        };
                        let Ok(value) = value.parse::<f32>() else {
                            return;
                        };
                        state_for_set_value.update(cx, |state, cx| {
                            state.set_thumb_value(is_start, value, true, cx);
                        });
                    })
                    .on_drag(DragThumb((entity_id, is_start)), |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
                    .on_drag_move(window.listener_for(
                        &self.state,
                        move |view, e: &DragMoveEvent<DragThumb>, window, cx| match e.drag(cx) {
                            DragThumb((id, is_start)) => {
                                if *id != entity_id {
                                    return;
                                }

                                view.update_value_by_position(
                                    axis,
                                    e.event.position,
                                    *is_start,
                                    window,
                                    cx,
                                )
                            }
                        },
                    ))
            });

        accessibility_state(thumb, false, false, self.disabled).into_any_element()
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let axis = self.axis;
        let entity_id = self.state.entity_id();
        let root_id: ElementId = ("slider", entity_id).into();
        let state = self.state.read(cx);
        let value = state.value();
        let is_range = value.is_range();
        let percentage = state.percentage.clone();
        let hovered_start_thumb = state.hovered_start_thumb;
        let hovered_end_thumb = state.hovered_end_thumb;
        let dragging = state.dragging;
        let active_start_thumb = state.active_start_thumb;
        let slider_min = state.min_value() as f64;
        let slider_max = state.max_value() as f64;
        let (bar_start, bar_end) = if self.reverse && !is_range {
            // Fill from the thumb to the max end (remaining side).
            (relative(percentage.end), relative(0.))
        } else {
            (relative(percentage.start), relative(1. - percentage.end))
        };
        let rem_size = window.rem_size();
        let metrics = SliderMetrics::resolve(&cx.theme().style);

        let custom_range_background = self.style.background.clone();
        let range_background: Fill = custom_range_background
            .clone()
            .unwrap_or_else(|| cx.theme().primary.into());
        let range_color = range_background
            .color()
            .and_then(|background| background.as_solid())
            .unwrap_or(cx.theme().primary);
        let track_background: Fill = custom_range_background
            .and_then(|fill| fill.color())
            .map(|background| Fill::from(background.opacity(0.2)))
            .unwrap_or_else(|| cx.theme().muted.into());
        let thumb_bg: Background = self
            .style
            .text
            .color
            .map(Into::into)
            .unwrap_or_else(|| cx.theme().tokens.slider_thumb.into());
        let corner_radii = self.style.corner_radii.clone();
        let default_radius = px(999.);
        let mut radius = Corners {
            top_left: corner_radii
                .top_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            top_right: corner_radii
                .top_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            bottom_left: corner_radii
                .bottom_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            bottom_right: corner_radii
                .bottom_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
        };
        if cx.theme().style.radii.md.is_zero() {
            radius.top_left = px(0.);
            radius.top_right = px(0.);
            radius.bottom_left = px(0.);
            radius.bottom_right = px(0.);
        }
        let thumb_border = if metrics.thumb_border_uses_ring {
            cx.theme().ring
        } else {
            range_color
        };
        let base_label = self
            .aria_label
            .clone()
            .unwrap_or_else(|| t!("Slider.label").into());
        let start_label = if is_range {
            format!("{}, {}", base_label, t!("Slider.start")).into()
        } else {
            base_label.clone()
        };
        let end_label = if is_range {
            format!("{}, {}", base_label, t!("Slider.end")).into()
        } else {
            base_label
        };
        let start_thumb_id = slider_child_id(&root_id, "start-thumb");
        let end_thumb_id = slider_child_id(&root_id, "end-thumb");
        let start_focus = window
            .use_keyed_state(slider_child_id(&root_id, "start-focus"), cx, |_, cx| {
                cx.focus_handle()
            })
            .read(cx)
            .clone();
        let end_focus = window
            .use_keyed_state(slider_child_id(&root_id, "end-focus"), cx, |_, cx| {
                cx.focus_handle()
            })
            .read(cx)
            .clone();
        let track_start_focus = start_focus.clone();
        let track_end_focus = end_focus.clone();

        div()
            .id(root_id)
            .flex()
            .flex_1()
            .items_center()
            .justify_center()
            .when(axis.is_vertical(), |this| {
                this.min_h(metrics.min_vertical_length)
            })
            .when(axis.is_horizontal(), |this| this.w_full())
            .refine_style(&self.style)
            .bg(cx.theme().transparent)
            .text_color(cx.theme().foreground)
            .when(self.disabled, |this| this.opacity(0.5))
            .when(!self.disabled, |this| {
                this.on_mouse_up(
                    MouseButton::Left,
                    window.listener_for(&self.state, |state, _, _, cx| {
                        state.handle_release(cx);
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    window.listener_for(&self.state, |state, _, _, cx| {
                        state.handle_release(cx);
                    }),
                )
            })
            .child(
                h_flex()
                    .id("slider-bar-container")
                    .when(!self.disabled, |this| {
                        this.on_mouse_down(
                            MouseButton::Left,
                            window.listener_for(
                                &self.state,
                                move |state, e: &MouseDownEvent, window, cx| {
                                    let mut is_start = false;
                                    if is_range {
                                        let bar_size = state.bounds.size.along(axis);
                                        let inner_pos = if axis.is_horizontal() {
                                            e.position.x - state.bounds.left()
                                        } else {
                                            state.bounds.bottom() - e.position.y
                                        };
                                        let center = ((percentage.end - percentage.start) / 2.0
                                            + percentage.start)
                                            * bar_size;
                                        is_start = inner_pos < center;
                                    }

                                    if is_start {
                                        track_start_focus.focus(window, cx);
                                    } else {
                                        track_end_focus.focus(window, cx);
                                    }

                                    state.update_value_by_position(
                                        axis, e.position, is_start, window, cx,
                                    )
                                },
                            ),
                        )
                    })
                    .when(!self.disabled && !is_range, |this| {
                        this.on_drag(DragSlider(entity_id), |drag, _, _, cx| {
                            cx.stop_propagation();
                            cx.new(|_| drag.clone())
                        })
                        .on_drag_move(window.listener_for(
                            &self.state,
                            move |view, e: &DragMoveEvent<DragSlider>, window, cx| match e.drag(cx)
                            {
                                DragSlider(id) => {
                                    if *id != entity_id {
                                        return;
                                    }

                                    view.update_value_by_position(
                                        axis,
                                        e.event.position,
                                        false,
                                        window,
                                        cx,
                                    )
                                }
                            },
                        ))
                    })
                    .when(axis.is_horizontal(), |this| {
                        this.items_center().h(metrics.hit_edge).w_full()
                    })
                    .when(axis.is_vertical(), |this| {
                        this.justify_center().w(metrics.hit_edge).h_full()
                    })
                    .flex_shrink_0()
                    .child(
                        div()
                            .id("slider-bar")
                            .relative()
                            .when(axis.is_horizontal(), |this| {
                                this.w_full().h(metrics.track_edge)
                            })
                            .when(axis.is_vertical(), |this| {
                                this.h_full().w(metrics.track_edge)
                            })
                            .bg(track_background)
                            .corner_radii(radius)
                            .child(
                                div()
                                    .absolute()
                                    .when(axis.is_horizontal(), |this| {
                                        this.h_full().left(bar_start).right(bar_end)
                                    })
                                    .when(axis.is_vertical(), |this| {
                                        this.w_full().bottom(bar_start).top(bar_end)
                                    })
                                    .bg(range_background)
                                    .when(!cx.theme().style.radii.md.is_zero(), |this| {
                                        this.rounded_full()
                                    }),
                            )
                            .when(is_range, |this| {
                                this.child(self.render_thumb(
                                    SliderThumbSpec {
                                        id: start_thumb_id,
                                        position: relative(percentage.start),
                                        is_start: true,
                                        value: value.start() as f64,
                                        min: slider_min,
                                        max: value.end() as f64,
                                        label: start_label,
                                        description: self.aria_description.clone(),
                                        focus_handle: start_focus,
                                        hovered: hovered_start_thumb,
                                        pressed: dragging && active_start_thumb,
                                        thumb_background: thumb_bg,
                                        thumb_border,
                                        radius,
                                        metrics,
                                    },
                                    window,
                                    cx,
                                ))
                            })
                            .child(self.render_thumb(
                                SliderThumbSpec {
                                    id: end_thumb_id,
                                    position: relative(percentage.end),
                                    is_start: false,
                                    value: value.end() as f64,
                                    min: if is_range {
                                        value.start() as f64
                                    } else {
                                        slider_min
                                    },
                                    max: slider_max,
                                    label: end_label,
                                    description: self.aria_description.clone(),
                                    focus_handle: end_focus,
                                    hovered: hovered_end_thumb,
                                    pressed: dragging && !active_start_thumb,
                                    thumb_background: thumb_bg,
                                    thumb_border,
                                    radius,
                                    metrics,
                                },
                                window,
                                cx,
                            ))
                            .on_prepaint({
                                let state = self.state.clone();
                                move |bounds, _, cx| state.update(cx, |r, _| r.bounds = bounds)
                            }),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[test]
    fn slider_metrics_match_pinned_style_presets() {
        let vega = SliderMetrics::resolve(&StylePreset::vega());
        assert_eq!(vega.track_edge, px(6.));
        assert_eq!(vega.thumb_edge, px(16.));
        assert_eq!(vega.ring_width, px(4.));
        assert!(vega.shadow);
        assert_eq!(vega.motion_kind, SliderMotionKind::Ring);

        let nova = SliderMetrics::resolve(&StylePreset::nova());
        assert_eq!(nova.track_edge, px(4.));
        assert_eq!(nova.thumb_edge, px(12.));
        assert_eq!(nova.ring_width, px(3.));
        assert!(!nova.shadow);
        assert!(nova.thumb_border_uses_ring);

        let maia = SliderMetrics::resolve(&StylePreset::maia());
        assert_eq!(maia.track_edge, px(12.));
        assert_eq!(maia.thumb_edge, px(16.));
        assert_eq!(maia.ring_width, px(4.));
        assert_eq!(maia.motion_kind, SliderMotionKind::Colors);
    }

    #[test]
    fn slider_normalizes_values_and_snaps_relative_to_minimum() {
        let slider = SliderState::new()
            .min(-5.)
            .max(5.)
            .step(2.)
            .default_value((8., -8.));

        assert_eq!(slider.value(), SliderValue::Range(-5., 5.));
        assert_eq!(slider.snap_value(-2.2), -3.);
        assert_eq!(slider.snap_value(4.9), 5.);
    }

    #[test]
    fn slider_internal_ids_preserve_structural_identity() {
        let structured = ElementId::NamedInteger("slider".into(), 1);
        let textual = ElementId::Name("slider-1".into());

        assert_eq!(structured.to_string(), textual.to_string());
        assert_ne!(
            slider_child_id(&structured, "end-thumb"),
            slider_child_id(&textual, "end-thumb")
        );
    }

    #[test]
    fn ring_motion_retargets_from_the_current_interpolated_color() {
        let hidden = Hsla::transparent_black();
        let visible = Hsla::red().opacity(0.5);
        let duration = Duration::from_millis(100);
        let easing = crate::MotionEasing::Linear;
        let now = Instant::now();
        let mut state = SliderRingMotionState::new(hidden);

        assert!(state.transition_to(hidden, now, duration, easing).is_none());
        assert!(
            state
                .transition_to(visible, now, duration, easing)
                .is_some()
        );
        let reversed = state
            .transition_to(hidden, now + Duration::from_millis(50), duration, easing)
            .expect("a mid-transition reversal must animate from the visible value");

        assert!(reversed.from.a > hidden.a);
        assert!(reversed.from.a < visible.a);
        assert_eq!(reversed.to, hidden);
    }

    #[test]
    #[should_panic(expected = "`step` must be finite and greater than 0")]
    fn slider_rejects_non_positive_steps() {
        _ = SliderState::new().step(0.);
    }

    #[gpui::test]
    fn keyboard_updates_only_the_focused_range_thumb(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let slider = cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(10.)
                    .step(2.)
                    .default_value(2.0..8.0)
            });

            slider.update(cx, |state, cx| {
                assert!(state.handle_key("right", true, cx));
                assert_eq!(state.value(), SliderValue::Range(4., 8.));
                assert!(state.handle_key("home", false, cx));
                assert_eq!(state.value(), SliderValue::Range(4., 4.));
                assert!(!state.handle_key("space", false, cx));
            });
        });
    }
}
