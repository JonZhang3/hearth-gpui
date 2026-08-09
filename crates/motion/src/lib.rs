//! Renderer-independent motion state and timing primitives.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

/// Named easing curves shared by component state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionEasing {
    Linear,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
}

impl MotionEasing {
    /// Samples the easing curve while clamping input to `0..=1`.
    pub fn sample(self, delta: f32) -> f32 {
        let delta = delta.clamp(0.0, 1.0);
        match self {
            Self::Linear => delta,
            Self::EaseInCubic => delta.powi(3),
            Self::EaseOutCubic => 1.0 - (1.0 - delta).powi(3),
            Self::EaseInOutCubic if delta < 0.5 => 4.0 * delta.powi(3),
            Self::EaseInOutCubic => 1.0 - (-2.0 * delta + 2.0).powi(3) / 2.0,
        }
    }
}

/// Samples a CSS-compatible cubic Bezier easing curve.
///
/// The input is the curve's x coordinate. The implementation solves the curve
/// parameter before evaluating y, matching CSS `cubic-bezier` semantics.
pub fn sample_cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let x1 = x1.clamp(0.0, 1.0);
    let x2 = x2.clamp(0.0, 1.0);
    if x == 0.0 || x == 1.0 {
        return x;
    }

    let coordinate = |t: f32, a: f32, b: f32| {
        let one_minus_t = 1.0 - t;
        3.0 * a * one_minus_t.powi(2) * t + 3.0 * b * one_minus_t * t.powi(2) + t.powi(3)
    };
    let derivative = |t: f32, a: f32, b: f32| {
        3.0 * a * (1.0 - t).powi(2) + 6.0 * (b - a) * (1.0 - t) * t + 3.0 * (1.0 - b) * t.powi(2)
    };

    // Newton-Raphson converges quickly for ordinary easing curves.
    let mut parameter = x;
    for _ in 0..8 {
        let error = coordinate(parameter, x1, x2) - x;
        let slope = derivative(parameter, x1, x2);
        if error.abs() < 1e-6 || slope.abs() < 1e-6 {
            break;
        }
        parameter = (parameter - error / slope).clamp(0.0, 1.0);
    }

    // Degenerate curves can make Newton's method stall. Bisection guarantees
    // a stable solution because valid CSS x control points are monotonic.
    if (coordinate(parameter, x1, x2) - x).abs() >= 1e-5 {
        let mut lower = 0.0;
        let mut upper = 1.0;
        for _ in 0..16 {
            parameter = (lower + upper) * 0.5;
            if coordinate(parameter, x1, x2) < x {
                lower = parameter;
            } else {
                upper = parameter;
            }
        }
    }

    coordinate(parameter, y1, y2).clamp(0.0, 1.0)
}

/// Easing sampler used by a tween segment.
#[derive(Clone)]
enum TweenEasing {
    Named(MotionEasing),
    Custom(Rc<dyn Fn(f32) -> f32>),
}

impl TweenEasing {
    fn sample(&self, delta: f32) -> f32 {
        match self {
            Self::Named(easing) => easing.sample(delta),
            Self::Custom(easing) => easing(delta.clamp(0.0, 1.0)).clamp(0.0, 1.0),
        }
    }
}

/// Describes one renderer-independent tween segment.
#[derive(Clone)]
pub struct TweenSpec {
    pub duration: Duration,
    easing: TweenEasing,
}

impl TweenSpec {
    /// Creates a tween with the given duration and easing curve.
    pub fn new(duration: Duration, easing: MotionEasing) -> Self {
        Self {
            duration,
            easing: TweenEasing::Named(easing),
        }
    }

    /// Creates a tween from a custom normalized easing function.
    pub fn with_easing_fn(duration: Duration, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        Self {
            duration,
            easing: TweenEasing::Custom(Rc::new(easing)),
        }
    }
}

/// Runtime motion preference resolved by the renderer adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionPreference {
    Full,
    Reduced,
}

/// Stable identity for one active animation request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionGeneration(u64);

impl MotionGeneration {
    /// Returns the numeric generation for diagnostics and adapter keys.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Current playback state returned with every sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionStatus {
    Idle,
    Running,
}

/// One sampled visual value and its completion signal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSample {
    pub value: f32,
    pub status: MotionStatus,
    pub completed_generation: Option<MotionGeneration>,
}

#[derive(Clone)]
struct ActiveTween {
    from: f32,
    target: f32,
    started_at: Instant,
    duration: Duration,
    easing: TweenEasing,
    generation: MotionGeneration,
}

/// An interruptible scalar value driven by explicit monotonic timestamps.
#[derive(Clone)]
pub struct MotionValue {
    value: f32,
    generation: u64,
    active: Option<ActiveTween>,
    pending_completion: Option<MotionGeneration>,
}

impl MotionValue {
    /// Creates a stable value without scheduling animation work.
    pub const fn new(value: f32) -> Self {
        Self {
            value,
            generation: 0,
            active: None,
            pending_completion: None,
        }
    }

    /// Returns the most recently sampled visual value.
    pub const fn value(&self) -> f32 {
        self.value
    }

    /// Returns the current target, or the stable value when idle.
    pub fn target(&self) -> f32 {
        self.active
            .as_ref()
            .map_or(self.value, |active| active.target)
    }

    /// Immediately sets the value and cancels active or pending completion.
    pub fn jump(&mut self, value: f32) {
        self.value = value;
        self.active = None;
        self.pending_completion = None;
        self.generation = self.generation.wrapping_add(1);
    }

    /// Animates to a target, resuming from the current sampled value.
    ///
    /// Reversing to the previous segment origin shortens the duration by the
    /// remaining distance so direction changes preserve perceived speed.
    pub fn animate_to(
        &mut self,
        target: f32,
        spec: TweenSpec,
        now: Instant,
        preference: MotionPreference,
    ) -> MotionGeneration {
        if let Some(active) = self.active.as_ref()
            && active.target == target
        {
            return active.generation;
        }

        let previous_active = self.active.clone();
        let _ = self.sample(now);
        self.pending_completion = None;
        self.generation = self.generation.wrapping_add(1);
        let generation = MotionGeneration(self.generation);

        let duration = previous_active
            .filter(|active| active.from == target)
            .map(|active| {
                let full_distance = (active.target - active.from).abs();
                if full_distance <= f32::EPSILON {
                    Duration::ZERO
                } else {
                    spec.duration
                        .mul_f32(((target - self.value).abs() / full_distance).clamp(0.0, 1.0))
                }
            })
            .unwrap_or(spec.duration);

        if preference == MotionPreference::Reduced
            || duration.is_zero()
            || (self.value - target).abs() <= f32::EPSILON
        {
            self.value = target;
            self.active = None;
            self.pending_completion = Some(generation);
            return generation;
        }

        self.active = Some(ActiveTween {
            from: self.value,
            target,
            started_at: now,
            duration,
            easing: spec.easing,
            generation,
        });
        generation
    }

    /// Samples the current value and returns completion exactly once.
    pub fn sample(&mut self, now: Instant) -> MotionSample {
        if let Some(generation) = self.pending_completion.take() {
            return MotionSample {
                value: self.value,
                status: MotionStatus::Idle,
                completed_generation: Some(generation),
            };
        }

        let Some(active) = self.active.clone() else {
            return MotionSample {
                value: self.value,
                status: MotionStatus::Idle,
                completed_generation: None,
            };
        };
        let elapsed = now.saturating_duration_since(active.started_at);
        let linear_delta = if active.duration.is_zero() {
            1.0
        } else {
            elapsed.as_secs_f32() / active.duration.as_secs_f32()
        };
        self.value =
            active.from + (active.target - active.from) * active.easing.sample(linear_delta);

        if linear_delta >= 1.0 {
            self.value = active.target;
            self.active = None;
            MotionSample {
                value: self.value,
                status: MotionStatus::Idle,
                completed_generation: Some(active.generation),
            }
        } else {
            MotionSample {
                value: self.value,
                status: MotionStatus::Running,
                completed_generation: None,
            }
        }
    }

    /// Stops at the current sampled value without emitting completion.
    pub fn stop(&mut self, now: Instant) -> f32 {
        let value = self.sample(now).value;
        self.active = None;
        self.pending_completion = None;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> TweenSpec {
        TweenSpec::new(Duration::from_millis(100), MotionEasing::Linear)
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
    }

    #[test]
    fn samples_start_middle_and_completion_once() {
        let start = Instant::now();
        let mut value = MotionValue::new(0.0);
        let generation = value.animate_to(1.0, spec(), start, MotionPreference::Full);

        assert_eq!(value.sample(start).value, 0.0);
        assert_eq!(
            value.sample(start + Duration::from_millis(50)),
            MotionSample {
                value: 0.5,
                status: MotionStatus::Running,
                completed_generation: None,
            }
        );
        assert_eq!(
            value
                .sample(start + Duration::from_millis(100))
                .completed_generation,
            Some(generation)
        );
        assert_eq!(
            value
                .sample(start + Duration::from_millis(120))
                .completed_generation,
            None
        );
    }

    #[test]
    fn reversal_continues_from_current_value_and_scales_duration() {
        let start = Instant::now();
        let mut value = MotionValue::new(0.0);
        value.animate_to(1.0, spec(), start, MotionPreference::Full);
        assert_close(value.sample(start + Duration::from_millis(40)).value, 0.4);

        let reverse_at = start + Duration::from_millis(40);
        let generation = value.animate_to(0.0, spec(), reverse_at, MotionPreference::Full);
        assert_close(value.sample(reverse_at).value, 0.4);
        assert_close(
            value.sample(reverse_at + Duration::from_millis(20)).value,
            0.2,
        );
        assert_eq!(
            value
                .sample(reverse_at + Duration::from_millis(40))
                .completed_generation,
            Some(generation)
        );
    }

    #[test]
    fn stale_target_does_not_restart_on_rerender() {
        let start = Instant::now();
        let mut value = MotionValue::new(0.0);
        let generation = value.animate_to(1.0, spec(), start, MotionPreference::Full);
        assert_eq!(
            value.animate_to(
                1.0,
                spec(),
                start + Duration::from_millis(30),
                MotionPreference::Full,
            ),
            generation
        );
        assert_eq!(value.sample(start + Duration::from_millis(50)).value, 0.5);
    }

    #[test]
    fn reduced_motion_completes_immediately_once() {
        let start = Instant::now();
        let mut value = MotionValue::new(0.0);
        let generation = value.animate_to(1.0, spec(), start, MotionPreference::Reduced);
        let sample = value.sample(start);
        assert_eq!(sample.value, 1.0);
        assert_eq!(sample.completed_generation, Some(generation));
        assert_eq!(value.sample(start).completed_generation, None);
    }

    #[test]
    fn cubic_bezier_solves_x_before_sampling_y() {
        assert!((sample_cubic_bezier(0.0, 0.0, 1.0, 1.0, 0.25) - 0.25).abs() < 1e-4);
        let eased = sample_cubic_bezier(0.33, 1.0, 0.68, 1.0, 0.5);
        assert!(eased > 0.8 && eased < 0.9);
    }
}
