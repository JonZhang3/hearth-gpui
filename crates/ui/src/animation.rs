// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public types: `OverlayPhase`, `OverlayTransition`, `OverlayLifecycle`, `MotionElement`.
// - Added public methods: `effective_motion_duration`, `opened`, `phase`, `animation_key`,
//   `active_transition`, `is_mounted`, `accepts_input`, `begin_open` and 6 more.
// - Added or exposed behavior through `effective_motion_duration`, `opened`, `phase`,
//   `animation_key`, `active_transition`, `is_mounted`, `accepts_input`, `begin_open` and 23 more.
// - Reworked Animation around interruptible and reduced-motion-aware transitions, invalid and
//   validation state handling.
use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, App, Element, ElementId, GlobalElementId, Hsla, InspectorElementId, IntoElement,
    ParentElement, Pixels, Point, Styled, Window, point, prelude::FluentBuilder, px, relative,
};
use gpui_component_motion::{
    MotionPreference, MotionStatus, MotionValue, TweenSpec, sample_cubic_bezier,
};
use smallvec::SmallVec;

use crate::theme::MotionEasing;

/// A cubic bezier function like CSS `cubic-bezier`.
///
/// Builder:
///
/// https://cubic-bezier.com
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> impl Fn(f32) -> f32 {
    move |t: f32| sample_cubic_bezier(x1, y1, x2, y2, t)
}

// ── Easing presets ──────────────────────────────────────────────────────────

/// Cubic ease-out — fast start, slow end. Good for enter animations.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Cubic ease-in — slow start, fast end. Good for exit animations.
pub fn ease_in_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t
}

/// Cubic ease-in-out — slow start and end. Good for position transitions.
pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Resolves a duration for lifecycle timers that are not rendered through
/// `AnimationExt`.
///
/// GPUI animation elements already honor `App::reduce_motion` automatically.
/// Exit timers must use this helper so reduced motion also removes delayed
/// unmounting and focus restoration.
pub fn effective_motion_duration(duration: Duration, cx: &App) -> Duration {
    if cx.reduce_motion() {
        Duration::ZERO
    } else {
        duration
    }
}

/// Explicit lifecycle shared by transient overlay components.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPhase {
    #[default]
    Closed,
    Opening,
    Open,
    Closing,
}

/// Transition generation used to reject completion from cancelled tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayTransition {
    generation: u64,
}

/// Small state machine for interruptible overlay enter and exit transitions.
///
/// A completion method returns `true` exactly once for the active generation.
/// Components use that signal to emit dismissal callbacks and restore focus.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OverlayLifecycle {
    phase: OverlayPhase,
    generation: u64,
}

impl OverlayLifecycle {
    /// Creates a lifecycle for content that is mounted in the open state.
    pub fn opened() -> Self {
        Self {
            phase: OverlayPhase::Open,
            generation: 0,
        }
    }

    /// Returns the current lifecycle phase.
    pub fn phase(&self) -> OverlayPhase {
        self.phase
    }

    /// Returns the identity of the active enter or exit animation.
    ///
    /// Completing an enter transition preserves this value, while every close
    /// or interrupted reopen receives a new value and therefore a fresh GPUI
    /// animation state.
    pub fn animation_key(&self) -> u64 {
        self.generation
    }

    /// Returns the generation token owned by the active enter or exit motion.
    pub fn active_transition(&self) -> Option<OverlayTransition> {
        matches!(self.phase, OverlayPhase::Opening | OverlayPhase::Closing).then_some(
            OverlayTransition {
                generation: self.generation,
            },
        )
    }

    /// Returns whether content must remain mounted for enter or exit motion.
    pub fn is_mounted(&self) -> bool {
        self.phase != OverlayPhase::Closed
    }

    /// Returns whether the mounted content may accept user actions.
    pub fn accepts_input(&self) -> bool {
        matches!(self.phase, OverlayPhase::Opening | OverlayPhase::Open)
    }

    /// Starts opening or reverses an in-progress close.
    pub fn begin_open(&mut self) -> Option<OverlayTransition> {
        if matches!(self.phase, OverlayPhase::Opening | OverlayPhase::Open) {
            return None;
        }

        self.generation = self.generation.wrapping_add(1);
        self.phase = OverlayPhase::Opening;
        Some(OverlayTransition {
            generation: self.generation,
        })
    }

    /// Completes opening only when the transition is still current.
    pub fn complete_open(&mut self, transition: OverlayTransition) -> bool {
        if self.phase != OverlayPhase::Opening || transition.generation != self.generation {
            return false;
        }

        self.phase = OverlayPhase::Open;
        true
    }

    /// Starts closing and returns the generation that owns close completion.
    pub fn begin_close(&mut self) -> Option<OverlayTransition> {
        if matches!(self.phase, OverlayPhase::Closed | OverlayPhase::Closing) {
            return None;
        }

        self.generation = self.generation.wrapping_add(1);
        self.phase = OverlayPhase::Closing;
        Some(OverlayTransition {
            generation: self.generation,
        })
    }

    /// Completes closing once; stale or duplicate completions are rejected.
    pub fn complete_close(&mut self, transition: OverlayTransition) -> bool {
        if self.phase != OverlayPhase::Closing || transition.generation != self.generation {
            return false;
        }

        self.phase = OverlayPhase::Closed;
        true
    }
}

// ── Lerp trait ──────────────────────────────────────────────────────────────

/// Trait for types that support linear interpolation.
pub trait Lerp: Clone {
    fn lerp(&self, target: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        self + (target - self) * t
    }
}

impl Lerp for Pixels {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        let a: f32 = (*self).into();
        let b: f32 = (*target).into();
        px(a + (b - a) * t)
    }
}

impl Lerp for Point<Pixels> {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        point(
            Lerp::lerp(&self.x, &target.x, t),
            Lerp::lerp(&self.y, &target.y, t),
        )
    }
}

impl Lerp for Hsla {
    /// Interpolate each channel linearly. Intended for transitions between
    /// near-grayscale UI colors (e.g. text colors), where hue interpolation is
    /// irrelevant.
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Hsla {
            h: self.h.lerp(&target.h, t),
            s: self.s.lerp(&target.s, t),
            l: self.l.lerp(&target.l, t),
            a: self.a.lerp(&target.a, t),
        }
    }
}

// ── Transition combinator ───────────────────────────────────────────────────

/// A composable transition that describes animated style changes.
///
/// # Example
///
/// ```ignore
/// Transition::new(Duration::from_millis(150))
///     .ease(ease_out_cubic)
///     .slide_y(px(-4.), px(0.))
///     .fade(0.0, 1.0)
///     .apply(element, "enter-anim")
/// ```
#[derive(Clone)]
pub struct Transition {
    pub duration: Duration,
    easing: TransitionEasing,
    effects: SmallVec<[TransitionEffect; 2]>,
    on_complete: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

#[derive(Clone)]
enum TransitionEasing {
    Token(MotionEasing),
    Custom(Rc<dyn Fn(f32) -> f32>),
}

#[derive(Clone, Copy)]
enum TransitionEffect {
    SlideY(Pixels, Pixels),
    SlideX(Pixels, Pixels),
    Fade(f32, f32),
    Width(Pixels, Pixels),
    RelativeWidth(f32, f32),
    Height(Pixels, Pixels),
}

impl Transition {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            easing: TransitionEasing::Token(MotionEasing::EaseOutCubic),
            effects: SmallVec::new(),
            on_complete: None,
        }
    }

    /// Set the easing function.
    pub fn ease(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.easing = TransitionEasing::Custom(Rc::new(easing));
        self
    }

    /// Sets a semantic easing curve resolved from the active Style Preset.
    pub fn ease_token(mut self, easing: MotionEasing) -> Self {
        self.easing = TransitionEasing::Token(easing);
        self
    }

    /// Animate vertical offset from `from` to `to`.
    pub fn slide_y(mut self, from: Pixels, to: Pixels) -> Self {
        self.effects.push(TransitionEffect::SlideY(from, to));
        self
    }

    /// Animate horizontal offset from `from` to `to`.
    pub fn slide_x(mut self, from: Pixels, to: Pixels) -> Self {
        self.effects.push(TransitionEffect::SlideX(from, to));
        self
    }

    /// Animate opacity from `from` to `to`.
    pub fn fade(mut self, from: f32, to: f32) -> Self {
        self.effects.push(TransitionEffect::Fade(from, to));
        self
    }

    /// Animate width from `from` to `to`.
    pub fn width(mut self, from: Pixels, to: Pixels) -> Self {
        self.effects.push(TransitionEffect::Width(from, to));
        self
    }

    /// Animate width as a fraction of the containing block.
    pub fn relative_width(mut self, from: f32, to: f32) -> Self {
        self.effects.push(TransitionEffect::RelativeWidth(
            from.clamp(0., 1.),
            to.clamp(0., 1.),
        ));
        self
    }

    /// Animate height from `from` to `to`.
    pub fn height(mut self, from: Pixels, to: Pixels) -> Self {
        self.effects.push(TransitionEffect::Height(from, to));
        self
    }

    /// Runs once after the active target reaches its final sampled value.
    pub fn on_complete(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_complete = Some(Rc::new(handler));
        self
    }

    /// Applies this transition using persistent GPUI element state.
    pub fn apply<E: IntoElement + Styled + 'static>(
        self,
        element: E,
        id: impl Into<ElementId>,
    ) -> MotionElement<E> {
        MotionElement {
            id: id.into(),
            element: Some(element),
            descriptor: TransitionDescriptor::from_effects(&self.effects),
            duration: self.duration,
            easing: self.easing,
            on_complete: self.on_complete,
        }
    }
}

impl FluentBuilder for Transition {}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct TransitionValues {
    opacity: f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct TransitionMask {
    opacity: bool,
    x: bool,
    y: bool,
    width: bool,
    relative_width: bool,
    height: bool,
}

#[derive(Debug, Clone, Copy)]
struct TransitionDescriptor {
    initial: TransitionValues,
    target: TransitionValues,
    mask: TransitionMask,
}

impl TransitionDescriptor {
    fn from_effects(effects: &[TransitionEffect]) -> Self {
        let mut descriptor = Self {
            initial: TransitionValues {
                opacity: 1.0,
                ..Default::default()
            },
            target: TransitionValues {
                opacity: 1.0,
                ..Default::default()
            },
            mask: TransitionMask::default(),
        };
        for effect in effects {
            match effect {
                TransitionEffect::SlideY(from, to) => {
                    descriptor.initial.y = from.as_f32();
                    descriptor.target.y = to.as_f32();
                    descriptor.mask.y = true;
                }
                TransitionEffect::SlideX(from, to) => {
                    descriptor.initial.x = from.as_f32();
                    descriptor.target.x = to.as_f32();
                    descriptor.mask.x = true;
                }
                TransitionEffect::Fade(from, to) => {
                    descriptor.initial.opacity = *from;
                    descriptor.target.opacity = *to;
                    descriptor.mask.opacity = true;
                }
                TransitionEffect::Width(from, to) => {
                    descriptor.initial.width = from.as_f32();
                    descriptor.target.width = to.as_f32();
                    descriptor.mask.width = true;
                    descriptor.mask.relative_width = false;
                }
                TransitionEffect::RelativeWidth(from, to) => {
                    descriptor.initial.width = *from;
                    descriptor.target.width = *to;
                    descriptor.mask.width = false;
                    descriptor.mask.relative_width = true;
                }
                TransitionEffect::Height(from, to) => {
                    descriptor.initial.height = from.as_f32();
                    descriptor.target.height = to.as_f32();
                    descriptor.mask.height = true;
                }
            }
        }
        descriptor
    }
}

struct MotionElementState {
    opacity: MotionValue,
    x: MotionValue,
    y: MotionValue,
    width: MotionValue,
    height: MotionValue,
    target: TransitionValues,
    mask: TransitionMask,
    epoch: u64,
    completed_epoch: u64,
}

impl MotionElementState {
    fn new(initial: TransitionValues) -> Self {
        Self {
            opacity: MotionValue::new(initial.opacity),
            x: MotionValue::new(initial.x),
            y: MotionValue::new(initial.y),
            width: MotionValue::new(initial.width),
            height: MotionValue::new(initial.height),
            target: initial,
            mask: TransitionMask::default(),
            epoch: 0,
            completed_epoch: 0,
        }
    }

    fn retarget(
        &mut self,
        descriptor: TransitionDescriptor,
        duration: Duration,
        easing: &TransitionEasing,
        now: Instant,
        preference: MotionPreference,
    ) {
        let first_target = self.epoch == 0;
        let target_changed = self.target != descriptor.target;
        if !first_target && !target_changed {
            return;
        }

        self.epoch = self.epoch.wrapping_add(1);
        self.target = descriptor.target;
        // Translation remains owned by the adapter after entry so an interrupted
        // close can settle the current offset instead of jumping to zero.
        self.mask.opacity |= descriptor.mask.opacity;
        self.mask.x |= descriptor.mask.x;
        self.mask.y |= descriptor.mask.y;
        self.mask.width = descriptor.mask.width;
        self.mask.relative_width = descriptor.mask.relative_width;
        self.mask.height = descriptor.mask.height;

        let spec = || match easing {
            TransitionEasing::Token(easing) => TweenSpec::new(duration, *easing),
            TransitionEasing::Custom(easing) => {
                let easing = easing.clone();
                TweenSpec::with_easing_fn(duration, move |delta| easing(delta))
            }
        };
        self.opacity
            .animate_to(descriptor.target.opacity, spec(), now, preference);
        self.x
            .animate_to(descriptor.target.x, spec(), now, preference);
        self.y
            .animate_to(descriptor.target.y, spec(), now, preference);
        if descriptor.mask.width || descriptor.mask.relative_width {
            self.width
                .animate_to(descriptor.target.width, spec(), now, preference);
        }
        if descriptor.mask.height {
            self.height
                .animate_to(descriptor.target.height, spec(), now, preference);
        }
    }

    fn sample(&mut self, now: Instant) -> (TransitionValues, bool, bool) {
        let opacity = self.opacity.sample(now);
        let x = self.x.sample(now);
        let y = self.y.sample(now);
        let width = self.width.sample(now);
        let height = self.height.sample(now);
        let running = [
            opacity.status,
            x.status,
            y.status,
            width.status,
            height.status,
        ]
        .into_iter()
        .any(|status| status == MotionStatus::Running);
        let completed = !running && self.completed_epoch != self.epoch;
        if completed {
            self.completed_epoch = self.epoch;
        }
        (
            TransitionValues {
                opacity: opacity.value,
                x: x.value,
                y: y.value,
                width: width.value,
                height: height.value,
            },
            running,
            completed,
        )
    }
}

/// GPUI adapter that applies an interruptible transition to a styled element.
pub struct MotionElement<E> {
    id: ElementId,
    element: Option<E>,
    descriptor: TransitionDescriptor,
    duration: Duration,
    easing: TransitionEasing,
    on_complete: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl<E: ParentElement> ParentElement for MotionElement<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E: IntoElement + Styled + 'static> IntoElement for MotionElement<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E: IntoElement + Styled + 'static> Element for MotionElement<E> {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let descriptor = self.descriptor;
        let duration = self.duration;
        let easing = self.easing.clone();
        let preference = if cx.reduce_motion() {
            MotionPreference::Reduced
        } else {
            MotionPreference::Full
        };
        let mut element = self
            .element
            .take()
            .expect("motion element is requested once");
        let ((layout_id, element), completed) = window.with_element_state(
            global_id.expect("motion element requires a stable id"),
            |state: Option<MotionElementState>, window| {
                let now = Instant::now();
                let mut state =
                    state.unwrap_or_else(|| MotionElementState::new(descriptor.initial));
                state.retarget(descriptor, duration, &easing, now, preference);
                let (values, running, completed) = state.sample(now);

                if state.mask.opacity {
                    element = element.opacity(values.opacity);
                }
                if state.mask.x {
                    element = element.left(px(values.x));
                }
                if state.mask.y {
                    element = element.top(px(values.y));
                }
                if state.mask.relative_width {
                    element = element.w(relative(values.width));
                } else if state.mask.width {
                    element = element.w(px(values.width));
                }
                if state.mask.height {
                    element = element.h(px(values.height));
                }
                let mut element = element.into_any_element();
                let layout_id = element.request_layout(window, cx);
                if running {
                    window.request_animation_frame();
                }
                (((layout_id, element), completed), state)
            },
        );

        if completed && let Some(on_complete) = self.on_complete.clone() {
            window.defer(cx, move |window, cx| on_complete(window, cx));
        }
        (layout_id, element)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[test]
    fn overlay_close_completion_is_single_and_generation_safe() {
        let mut lifecycle = OverlayLifecycle::default();
        let opening = lifecycle.begin_open().unwrap();
        assert!(lifecycle.complete_open(opening));

        let closing = lifecycle.begin_close().unwrap();
        assert!(!lifecycle.accepts_input());
        assert!(lifecycle.is_mounted());
        assert!(lifecycle.complete_close(closing));
        assert!(!lifecycle.complete_close(closing));
        assert!(!lifecycle.is_mounted());
    }

    #[test]
    fn reopening_invalidates_pending_close_completion() {
        let mut lifecycle = OverlayLifecycle::default();
        let opening = lifecycle.begin_open().unwrap();
        assert!(lifecycle.complete_open(opening));

        let closing = lifecycle.begin_close().unwrap();
        let reopening = lifecycle.begin_open().unwrap();
        assert!(!lifecycle.complete_close(closing));
        assert!(lifecycle.complete_open(reopening));
        assert_eq!(lifecycle.phase(), OverlayPhase::Open);
    }

    #[test]
    fn opening_completion_preserves_animation_identity() {
        let mut lifecycle = OverlayLifecycle::default();
        let opening = lifecycle.begin_open().unwrap();
        let opening_key = lifecycle.animation_key();
        assert!(lifecycle.complete_open(opening));
        assert_eq!(lifecycle.animation_key(), opening_key);

        let closing = lifecycle.begin_close().unwrap();
        let closing_key = lifecycle.animation_key();
        assert_ne!(closing_key, opening_key);

        let reopening = lifecycle.begin_open().unwrap();
        assert_ne!(lifecycle.animation_key(), closing_key);
        assert!(!lifecycle.complete_close(closing));
        assert!(lifecycle.complete_open(reopening));
    }

    #[test]
    fn adapter_retargets_opacity_and_settles_entry_translation_without_a_jump() {
        let opening = TransitionDescriptor::from_effects(&[
            TransitionEffect::SlideY(px(-8.), px(0.)),
            TransitionEffect::Fade(0., 1.),
        ]);
        let closing = TransitionDescriptor::from_effects(&[TransitionEffect::Fade(1., 0.)]);
        let start = Instant::now();
        let mut state = MotionElementState::new(opening.initial);
        let easing = TransitionEasing::Token(MotionEasing::Linear);
        state.retarget(
            opening,
            Duration::from_millis(100),
            &easing,
            start,
            MotionPreference::Full,
        );
        let (before_reverse, _, _) = state.sample(start + Duration::from_millis(40));

        let reverse_at = start + Duration::from_millis(40);
        state.retarget(
            closing,
            Duration::from_millis(100),
            &easing,
            reverse_at,
            MotionPreference::Full,
        );
        let (after_reverse, _, _) = state.sample(reverse_at);
        assert!((after_reverse.opacity - before_reverse.opacity).abs() < 1e-5);
        assert!((after_reverse.y - before_reverse.y).abs() < 1e-5);

        let (finished, running, completed) = state.sample(reverse_at + Duration::from_millis(100));
        assert_eq!(finished.opacity, 0.);
        assert_eq!(finished.y, 0.);
        assert!(!running);
        assert!(completed);
    }

    #[test]
    fn adapter_reduced_motion_completes_at_the_target_once() {
        let descriptor = TransitionDescriptor::from_effects(&[TransitionEffect::Fade(0., 1.)]);
        let start = Instant::now();
        let mut state = MotionElementState::new(descriptor.initial);
        state.retarget(
            descriptor,
            Duration::from_millis(100),
            &TransitionEasing::Token(MotionEasing::EaseOutCubic),
            start,
            MotionPreference::Reduced,
        );

        let (values, running, completed) = state.sample(start);
        assert_eq!(values.opacity, 1.);
        assert!(!running);
        assert!(completed);
        assert!(!state.sample(start).2);
    }

    #[test]
    fn relative_width_retargets_from_the_current_sample() {
        let start = Instant::now();
        let opening =
            TransitionDescriptor::from_effects(&[TransitionEffect::RelativeWidth(0.25, 0.75)]);
        let closing =
            TransitionDescriptor::from_effects(&[TransitionEffect::RelativeWidth(0.75, 0.1)]);
        let easing = TransitionEasing::Token(MotionEasing::Linear);
        let duration = Duration::from_millis(100);
        let mut state = MotionElementState::new(opening.initial);

        state.retarget(opening, duration, &easing, start, MotionPreference::Full);
        let reverse_at = start + Duration::from_millis(50);
        let (before_reverse, _, _) = state.sample(reverse_at);
        state.retarget(
            closing,
            duration,
            &easing,
            reverse_at,
            MotionPreference::Full,
        );
        let (after_reverse, _, _) = state.sample(reverse_at);

        assert!(state.mask.relative_width);
        assert!(!state.mask.width);
        assert!((after_reverse.width - before_reverse.width).abs() < 1e-5);
        let (finished, running, _) = state.sample(reverse_at + duration);
        assert_eq!(finished.width, 0.1);
        assert!(!running);
    }

    #[gpui::test]
    fn reduced_motion_removes_lifecycle_delay(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let duration = Duration::from_millis(150);
            assert_eq!(effective_motion_duration(duration, cx), duration);

            cx.set_reduce_motion(true);
            assert_eq!(effective_motion_duration(duration, cx), Duration::ZERO);
        });
    }
}
