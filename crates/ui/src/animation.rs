use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, App, ElementId, Hsla, IntoElement, Pixels, Point, Styled, point,
    prelude::FluentBuilder, px,
};
use smallvec::SmallVec;

use crate::theme::MotionEasing;

/// A cubic bezier function like CSS `cubic-bezier`.
///
/// Builder:
///
/// https://cubic-bezier.com
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> impl Fn(f32) -> f32 {
    move |t: f32| {
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        let t2 = t * t;
        let t3 = t2 * t;

        // The Bezier curve function for x and y, where x0 = 0, y0 = 0, x3 = 1, y3 = 1
        let _x = 3.0 * x1 * one_t2 * t + 3.0 * x2 * one_t * t2 + t3;
        let y = 3.0 * y1 * one_t2 * t + 3.0 * y2 * one_t * t2 + t3;

        y
    }
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
    easing: Rc<dyn Fn(f32) -> f32>,
    effects: SmallVec<[TransitionEffect; 2]>,
}

#[derive(Clone, Copy)]
enum TransitionEffect {
    SlideY(Pixels, Pixels),
    SlideX(Pixels, Pixels),
    Fade(f32, f32),
    Width(Pixels, Pixels),
    Height(Pixels, Pixels),
}

impl Transition {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            easing: Rc::new(ease_out_cubic),
            effects: SmallVec::new(),
        }
    }

    /// Set the easing function.
    pub fn ease(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.easing = Rc::new(easing);
        self
    }

    /// Sets a semantic easing curve resolved from the active Style Preset.
    pub fn ease_token(self, easing: MotionEasing) -> Self {
        self.ease(move |delta| easing.sample(delta))
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

    /// Animate height from `from` to `to`.
    pub fn height(mut self, from: Pixels, to: Pixels) -> Self {
        self.effects.push(TransitionEffect::Height(from, to));
        self
    }

    /// Apply this transition to a Styled element, returning an AnimationElement.
    pub fn apply<E: IntoElement + Styled + 'static>(
        self,
        element: E,
        id: impl Into<ElementId>,
    ) -> gpui::AnimationElement<E> {
        let animation = Animation::new(self.duration).with_easing({
            let easing = self.easing.clone();
            move |t| easing(t)
        });
        let effects = self.effects;
        element.with_animation(id, animation, move |el, delta| {
            let mut el = el;
            for effect in &effects {
                match effect {
                    TransitionEffect::SlideY(from, to) => {
                        el = el.top(Lerp::lerp(from, to, delta));
                    }
                    TransitionEffect::SlideX(from, to) => {
                        el = el.left(Lerp::lerp(from, to, delta));
                    }
                    TransitionEffect::Fade(from, to) => {
                        el = el.opacity(Lerp::lerp(from, to, delta));
                    }
                    TransitionEffect::Width(from, to) => {
                        el = el.w(Lerp::lerp(from, to, delta));
                    }
                    TransitionEffect::Height(from, to) => {
                        el = el.h(Lerp::lerp(from, to, delta));
                    }
                }
            }
            el
        })
    }
}

impl FluentBuilder for Transition {}

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
