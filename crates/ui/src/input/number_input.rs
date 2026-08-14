// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `invalid`, `read_only`, `aria_label`, `aria_description`, `increment`,
//   `decrement`.
// - Removed public methods: `placeholder`.
// - Added or exposed behavior through `invalid`, `read_only`, `aria_label`, `aria_description`,
//   `increment`, `decrement`, `number_input_builder_preserves_composite_states`.
// - Removed or replaced `placeholder`, `on_increment`, `on_decrement`.
// - Reworked Number Input around accessibility semantics and ARIA state, interruptible and
//   reduced-motion-aware transitions, semantic Style Preset geometry and density, keyboard
//   navigation and activation behavior, focus-visible and focus restoration behavior, invalid and
//   validation state handling.
use std::{rc::Rc, time::Instant};

use gpui::{Animation, AnimationExt as _, Corners, Edges, ElementId, Window, div, px};
use gpui::{AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable};
use gpui::{
    InteractiveElement, IntoElement, KeyBinding, ParentElement, RenderOnce, Role, SharedString,
    StyleRefinement, Styled, TextAlign, actions, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::animation::Lerp;
use crate::{
    ActiveTheme, Disableable, IconName, Sizable, Size, StyledExt as _, button::Button, h_flex,
};

use super::input::{
    InputMotionKind, InputMotionState, InputPaintState, input_child_id, input_focus_visible,
    input_metrics, input_motion_timing, input_uses_semantic_color_motion,
};
use super::{Input, InputState, MaskPattern};

actions!(number_input, [Increment, Decrement]);

const CONTEXT: &str = "NumberInput";
pub fn init(cx: &mut App) {
    cx.bind_keys(vec![
        KeyBinding::new("up", Increment, Some(CONTEXT)),
        KeyBinding::new("down", Decrement, Some(CONTEXT)),
    ]);
}

/// A number input element with increment and decrement buttons.
#[derive(IntoElement)]
pub struct NumberInput {
    state: Entity<InputState>,
    size: Size,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    appearance: bool,
    disabled: bool,
    read_only: bool,
    invalid: bool,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
    style: StyleRefinement,
}

impl NumberInput {
    /// Create a new [`NumberInput`] element bind to the [`InputState`].
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::default(),
            prefix: None,
            suffix: None,
            appearance: true,
            disabled: false,
            read_only: false,
            invalid: false,
            aria_label: None,
            aria_description: None,
            style: StyleRefinement::default(),
        }
    }

    /// Set the prefix element of the number input.
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set the suffix element of the number input.
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set the appearance of the number input, if false will no border and background.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Sets whether the numeric value is invalid.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets the input to read-only while retaining focus, selection, and copy behavior.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Sets the accessible name announced for the spin button.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets supporting text announced for the spin button.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Increments the value through the same path used by pointer and keyboard input.
    pub fn increment(state: &Entity<InputState>, window: &mut Window, cx: &mut App) {
        state.update(cx, |state, cx| {
            if state.disabled || state.read_only {
                return;
            }
            state.focus(window, cx);
            state.on_action_increment(&Increment, window, cx);
        })
    }

    /// Decrements the value through the same path used by pointer and keyboard input.
    pub fn decrement(state: &Entity<InputState>, window: &mut Window, cx: &mut App) {
        state.update(cx, |state, cx| {
            if state.disabled || state.read_only {
                return;
            }
            state.focus(window, cx);
            state.on_action_decrement(&Decrement, window, cx);
        })
    }
}

impl Disableable for NumberInput {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl InputState {
    fn on_action_increment(&mut self, _: &Increment, window: &mut Window, cx: &mut Context<Self>) {
        self.on_number_input_step(StepAction::Increment, window, cx);
    }

    fn on_action_decrement(&mut self, _: &Decrement, window: &mut Window, cx: &mut Context<Self>) {
        self.on_number_input_step(StepAction::Decrement, window, cx);
    }

    pub(super) fn on_number_input_step(
        &mut self,
        action: StepAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || self.read_only {
            return;
        }

        // By default NumberInput steps the value internally with step 1.
        // To opt out and emit `NumberInputEvent::Step` instead (the caller
        // updates the value), call `state.set_step(None, window, cx)`.
        if let Some(step) = self.number_step.clone() {
            let value = self.unmask_value();
            let current = value.trim().parse::<f64>().unwrap_or(0.);
            let step = step.value(current, action, cx);
            if let Some(new_value) =
                step_value(&value, action, step, self.number_min, self.number_max)
            {
                // The stepped value must pass the `pattern`/`validate` check,
                // otherwise fall back to emit the event to let the caller handle it.
                if self.is_valid_input(&new_value, cx) {
                    let range = self.range_to_utf16(&(0..self.text.len()));
                    self.replace_text_in_range_silent(Some(range), &new_value, window, cx);
                    return;
                }
            } else {
                // Stepping cannot move the value in this direction (e.g.
                // Decrement on a below-min value), do nothing.
                return;
            }
        }

        cx.emit(NumberInputEvent::Step(action));
    }
}

/// The step strategy of the [`NumberInput`] for increment/decrement.
///
/// See also [`InputState::step`] and [`InputState::step_by`].
#[derive(Clone)]
pub enum NumberStep {
    /// A fixed step value.
    Fixed(f64),
    /// Calculate the step value from the current value and direction.
    ByValue(Rc<dyn Fn(f64, StepAction, &mut Context<InputState>) -> f64>),
}

impl NumberStep {
    /// Create a step that calculates the step value from the current value
    /// and direction on stepping.
    ///
    /// The current value is the value before stepping; an empty or invalid
    /// value is treated as 0. The [`StepAction`] tells whether the value is
    /// being incremented or decremented, useful when the step differs by
    /// direction at a range boundary.
    ///
    /// The closure receives a [`Context<InputState>`] to read or update other
    /// entities while computing the step, but must not re-enter the owning
    /// [`InputState`] (it is mutably borrowed during stepping).
    pub fn by_value(
        f: impl Fn(f64, StepAction, &mut Context<InputState>) -> f64 + 'static,
    ) -> Self {
        Self::ByValue(Rc::new(f))
    }

    /// Return the step value for the given current value and direction.
    pub(super) fn value(
        &self,
        current: f64,
        action: StepAction,
        cx: &mut Context<InputState>,
    ) -> f64 {
        match self {
            Self::Fixed(step) => *step,
            Self::ByValue(f) => f(current, action, cx),
        }
    }
}

impl From<f64> for NumberStep {
    fn from(step: f64) -> Self {
        Self::Fixed(step)
    }
}

/// Step the `value` by `step` and clamp the result to the `min`/`max` range.
///
/// Returns `None` if stepping cannot move the value in the given direction
/// (e.g. the value is already at the boundary).
///
/// The result keeps the max fraction digits of the current value and the step,
/// to avoid float precision issue, e.g. `0.1 + 0.2 -> 0.3`.
fn step_value(
    value: &str,
    action: StepAction,
    step: f64,
    min: Option<f64>,
    max: Option<f64>,
) -> Option<String> {
    fn fraction_digits(value: &str) -> usize {
        value.split('.').nth(1).map_or(0, |frac| frac.len())
    }

    // A numeric step must move monotonically and remain representable.
    if !step.is_finite() || step <= 0. {
        return None;
    }

    let min = min.filter(|value| value.is_finite());
    let max = max.filter(|value| value.is_finite());
    let current = value.trim().parse::<f64>().ok();
    let mut new_value = match action {
        StepAction::Increment => current.unwrap_or(0.) + step,
        StepAction::Decrement => current.unwrap_or(0.) - step,
    };
    let mut digits = fraction_digits(value).max(fraction_digits(&step.to_string()));
    if let Some(min) = min {
        if new_value < min {
            new_value = min;
            digits = digits.max(fraction_digits(&min.to_string()));
        }
    }
    if let Some(max) = max {
        if new_value > max {
            new_value = max;
            digits = digits.max(fraction_digits(&max.to_string()));
        }
    }

    // Web behavior: stepping must move the value in the pressed direction, so
    // a Decrement below min does nothing rather than clamping up. An empty or
    // invalid value always steps into the range.
    if let Some(current) = current {
        let moved = match action {
            StepAction::Increment => new_value > current,
            StepAction::Decrement => new_value < current,
        };
        if !moved {
            return None;
        }
    }

    Some(format!("{:.*}", digits, new_value))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StepAction {
    Decrement,
    Increment,
}
pub enum NumberInputEvent {
    Step(StepAction),
}
impl EventEmitter<NumberInputEvent> for InputState {}

impl Focusable for NumberInput {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Sizable for NumberInput {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for NumberInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for NumberInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Default to use `MaskPattern::Number` to limit the input to a valid
        // number (optional leading sign, digits and a single dot), and to
        // normalize full-width number characters, e.g. `12。5` -> `12.5`.
        //
        // Only when the user has not set a `mask_pattern` explicitly, so that
        // `set_mask_pattern(MaskPattern::None)` can be used to opt out.
        if !self.state.read(cx).mask_pattern_set {
            self.state.update(cx, |state, _| {
                state.mask_pattern = MaskPattern::Number {
                    separator: None,
                    fraction: None,
                };
            });
        }

        let (numeric_value, numeric_step, numeric_min, numeric_max, focused) = {
            let state = self.state.read(cx);
            let numeric_step = match state.number_step.as_ref() {
                Some(NumberStep::Fixed(step)) if step.is_finite() && *step > 0. => Some(*step),
                _ => None,
            };
            (
                state.unmask_value().trim().parse::<f64>().ok(),
                numeric_step,
                state.number_min.filter(|value| value.is_finite()),
                state.number_max.filter(|value| value.is_finite()),
                state.focus_handle.is_focused(window) && !self.disabled,
            )
        };

        let metrics = input_metrics(&cx.theme().style);
        let control_metrics = cx.theme().style.controls.for_size(self.size);
        let focus_visible = input_focus_visible(focused);
        let disabled = self.disabled;
        let read_only = self.read_only;
        let invalid = self.invalid;
        let appearance = self.appearance;
        let disabled_opacity = if disabled { 0.5 } else { 1. };
        let invalid_border = cx
            .theme()
            .danger
            .opacity(if cx.theme().is_dark() { 0.5 } else { 1. });
        let border = if invalid {
            invalid_border
        } else if focus_visible {
            cx.theme().ring
        } else {
            cx.theme().input
        }
        .opacity(disabled_opacity);
        let ring_visible = appearance && (invalid || focus_visible);
        let ring_color = if invalid {
            cx.theme()
                .danger
                .opacity(if cx.theme().is_dark() { 0.4 } else { 0.2 })
        } else {
            cx.theme().ring.opacity(0.5)
        }
        .opacity(disabled_opacity);
        let paint = InputPaintState {
            background: Input::surface_background(metrics, disabled, cx).opacity(disabled_opacity),
            border,
            ring: if ring_visible {
                ring_color
            } else {
                ring_color.opacity(0.)
            },
        };
        let uses_semantic_color_motion = input_uses_semantic_color_motion(&self.style);
        let root_id: ElementId = ("number-input", self.state.entity_id()).into();
        let motion_state =
            window.use_keyed_state(input_child_id(&root_id, "motion-state"), cx, |_, _| {
                InputMotionState::new(paint)
            });
        let (motion_duration, easing) = input_motion_timing(ring_visible, cx);
        let transition = motion_state.update(cx, |motion, _| {
            motion.transition_to(
                paint,
                Instant::now(),
                motion_duration,
                easing,
                InputMotionKind::ColorsAndShadow,
            )
        });
        let surface_transition = transition.filter(|transition| {
            appearance
                && uses_semantic_color_motion
                && transition.from.background != transition.to.background
        });
        let border_transition = transition.filter(|transition| {
            appearance
                && uses_semantic_color_motion
                && transition.from.border != transition.to.border
        });

        let mut element = h_flex()
            .id(root_id.clone())
            .key_context(CONTEXT)
            .on_action(window.listener_for(&self.state, InputState::on_action_increment))
            .on_action(window.listener_for(&self.state, InputState::on_action_decrement))
            .relative()
            .flex_1()
            .min_w_0()
            .h(control_metrics.height)
            .items_center()
            .when(appearance, |this| {
                this.rounded(metrics.radius)
                    .border_1()
                    .border_color(if uses_semantic_color_motion {
                        cx.theme().transparent
                    } else {
                        paint.border
                    })
                    .bg(if uses_semantic_color_motion {
                        cx.theme().transparent
                    } else {
                        paint.background
                    })
                    .when(metrics.shadow, |this| this.shadow_xs())
            })
            .refine_style(&self.style);

        if appearance && uses_semantic_color_motion {
            let mut surface_style = StyleRefinement::default();
            surface_style.corner_radii = element.style().corner_radii.clone();
            let surface = div()
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .left_0()
                .bg(paint.background)
                .refine_style(&surface_style)
                .into_any_element();
            let surface = if let Some(transition) = surface_transition {
                let from = transition.from;
                let to = transition.to;
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .left_0()
                    .bg(from.background)
                    .refine_style(&surface_style)
                    .with_animation(
                        input_child_id(&root_id, format!("surface-{}", transition.epoch)),
                        Animation::new(transition.duration)
                            .with_easing(move |delta| easing.sample(delta)),
                        move |this, delta| {
                            this.bg(Lerp::lerp(&from.background, &to.background, delta))
                        },
                    )
                    .into_any_element()
            } else {
                surface
            };
            element = element.child(surface);
        }

        let ring_transition =
            transition.filter(|transition| transition.from.ring != transition.to.ring);
        let ring = if appearance && (ring_visible || ring_transition.is_some()) {
            let ring_width = cx.theme().style.focus.ring_width;
            let ring_outset = ring_width + cx.theme().style.focus.ring_offset;
            let ring_style = Input::outer_ring_geometry(element.style(), ring_outset, window);
            let ring = div()
                .absolute()
                .top(-ring_outset)
                .right(-ring_outset)
                .bottom(-ring_outset)
                .left(-ring_outset)
                .border(ring_width)
                .border_color(paint.ring)
                .refine_style(&ring_style);
            let ring = if let Some(transition) = ring_transition {
                let from = transition.from;
                let to = transition.to;
                ring.with_animation(
                    input_child_id(&root_id, format!("ring-{}", transition.epoch)),
                    Animation::new(transition.duration)
                        .with_easing(move |delta| easing.sample(delta)),
                    move |this, delta| this.border_color(Lerp::lerp(&from.ring, &to.ring, delta)),
                )
                .into_any_element()
            } else {
                ring.into_any_element()
            };
            Some(ring)
        } else {
            None
        };

        let border_overlay = if appearance && uses_semantic_color_motion {
            let mut border_style = StyleRefinement::default();
            border_style.corner_radii = element.style().corner_radii.clone();
            border_style.border_widths = element.style().border_widths.clone();
            let border = div()
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .left_0()
                .border_color(paint.border)
                .refine_style(&border_style);
            let border = if let Some(transition) = border_transition {
                let from = transition.from;
                let to = transition.to;
                border
                    .with_animation(
                        input_child_id(&root_id, format!("border-{}", transition.epoch)),
                        Animation::new(transition.duration)
                            .with_easing(move |delta| easing.sample(delta)),
                        move |this, delta| {
                            this.border_color(Lerp::lerp(&from.border, &to.border, delta))
                        },
                    )
                    .into_any_element()
            } else {
                border.into_any_element()
            };
            Some(border)
        } else {
            None
        };

        let button_disabled = disabled || read_only;
        let step_button_inset = px(1.);
        let step_button_size = (control_metrics.height - step_button_inset * 2.).max(px(0.));
        let step_button_radius = (metrics.radius - step_button_inset).max(px(0.));
        let no_edges = Edges {
            top: false,
            right: false,
            bottom: false,
            left: false,
        };
        let decrement = Button::new(input_child_id(&root_id, "decrement"))
            .ghost()
            .with_size(self.size)
            .icon(IconName::Minus)
            .aria_label(t!("Input.Decrease Value"))
            .tab_stop(false)
            .disabled(button_disabled)
            .pressed_offset(false)
            .rounded(step_button_radius)
            .size(step_button_size)
            .border_corners(Corners {
                top_left: true,
                top_right: false,
                bottom_right: false,
                bottom_left: true,
            })
            .border_edges(no_edges)
            .on_click({
                let state = self.state.clone();
                move |_, window, cx| Self::decrement(&state, window, cx)
            });
        let increment = Button::new(input_child_id(&root_id, "increment"))
            .ghost()
            .with_size(self.size)
            .icon(IconName::Plus)
            .aria_label(t!("Input.Increase Value"))
            .tab_stop(false)
            .disabled(button_disabled)
            .pressed_offset(false)
            .rounded(step_button_radius)
            .size(step_button_size)
            .border_corners(Corners {
                top_left: false,
                top_right: true,
                bottom_right: true,
                bottom_left: false,
            })
            .border_edges(no_edges)
            .on_click({
                let state = self.state.clone();
                move |_, window, cx| Self::increment(&state, window, cx)
            });
        let input = Input::new(&self.state)
            .role(Role::SpinButton)
            .numeric_accessibility(numeric_value, numeric_step, numeric_min, numeric_max, true)
            .appearance(false)
            .bordered(false)
            .focus_bordered(false)
            .with_size(self.size)
            .disabled(disabled)
            .read_only(read_only)
            .invalid(invalid)
            .gap_0()
            .text_align(TextAlign::Center)
            .flex_1()
            .min_w_0()
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .when_some(self.aria_description, |this, description| {
                this.aria_description(description)
            })
            .when_some(self.prefix, |this, prefix| this.prefix(prefix))
            .when_some(self.suffix, |this, suffix| this.suffix(suffix));

        let divider_transition = border_transition;
        let decrement_divider = div()
            .h_full()
            .flex_none()
            .py(step_button_inset)
            .pl(step_button_inset)
            .when(appearance, |this| {
                this.border_r_1().border_color(paint.border)
            })
            .child(decrement);
        let decrement_divider = if let Some(transition) = divider_transition {
            let from = transition.from;
            let to = transition.to;
            decrement_divider
                .with_animation(
                    input_child_id(&root_id, format!("decrement-divider-{}", transition.epoch)),
                    Animation::new(transition.duration)
                        .with_easing(move |delta| easing.sample(delta)),
                    move |this, delta| {
                        this.border_color(Lerp::lerp(&from.border, &to.border, delta))
                    },
                )
                .into_any_element()
        } else {
            decrement_divider.into_any_element()
        };
        let increment_divider = div()
            .h_full()
            .flex_none()
            .py(step_button_inset)
            .pr(step_button_inset)
            .when(appearance, |this| {
                this.border_l_1().border_color(paint.border)
            })
            .child(increment);
        let increment_divider = if let Some(transition) = divider_transition {
            let from = transition.from;
            let to = transition.to;
            increment_divider
                .with_animation(
                    input_child_id(&root_id, format!("increment-divider-{}", transition.epoch)),
                    Animation::new(transition.duration)
                        .with_easing(move |delta| easing.sample(delta)),
                    move |this, delta| {
                        this.border_color(Lerp::lerp(&from.border, &to.border, delta))
                    },
                )
                .into_any_element()
        } else {
            increment_divider.into_any_element()
        };

        let element = element
            .child(decrement_divider)
            .child(input)
            .child(increment_divider);

        element.children(border_overlay).children(ring)
    }
}

#[cfg(test)]
mod tests {
    use super::{NumberInput, StepAction, step_value};
    use crate::input::InputState;
    use gpui::{AppContext as _, Context, IntoElement, Render, TestAppContext, Window, div};

    struct EmptyView;

    impl Render for EmptyView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    // `test_number_step` lives in `state::tests` because `NumberStep::value`
    // now needs a `Context<InputState>` to invoke the `by_value` closure.

    #[test]
    fn test_step_value() {
        fn some(value: &str) -> Option<String> {
            Some(value.to_string())
        }

        // Step from empty value
        assert_eq!(
            step_value("", StepAction::Increment, 1., None, None),
            some("1")
        );
        assert_eq!(
            step_value("", StepAction::Decrement, 1., None, None),
            some("-1")
        );
        // Invalid intermediate values are treated as 0
        assert_eq!(
            step_value("-", StepAction::Increment, 1., None, None),
            some("1")
        );
        assert_eq!(
            step_value("1", StepAction::Increment, 1., None, None),
            some("2")
        );
        assert_eq!(
            step_value("-2", StepAction::Increment, 1., None, None),
            some("-1")
        );

        // Avoid float precision issue, e.g. 0.1 + 0.2 != 0.30000000000000004
        assert_eq!(
            step_value("0.1", StepAction::Increment, 0.2, None, None),
            some("0.3")
        );
        assert_eq!(
            step_value("0.3", StepAction::Decrement, 0.1, None, None),
            some("0.2")
        );
        // Keep the fraction digits of the current value
        assert_eq!(
            step_value("1.25", StepAction::Increment, 1., None, None),
            some("2.25")
        );

        // Step from empty value always steps into the range
        assert_eq!(
            step_value("", StepAction::Increment, 1., Some(10.), None),
            some("10")
        );
        assert_eq!(
            step_value("", StepAction::Decrement, 1., Some(10.), None),
            some("10")
        );
        // Clamp to min/max
        assert_eq!(
            step_value("99.5", StepAction::Increment, 1., None, Some(100.)),
            some("100.0")
        );
        assert_eq!(
            step_value("1000", StepAction::Decrement, 1., None, Some(100.)),
            some("100")
        );
        // Keep the fraction digits of the clamped bound
        assert_eq!(
            step_value("1", StepAction::Decrement, 1., Some(0.25), None),
            some("0.25")
        );

        // Stepping must move the value in the pressed direction:
        // no-op at the boundary
        assert_eq!(
            step_value("10", StepAction::Decrement, 1., Some(10.), None),
            None
        );
        assert_eq!(
            step_value("100", StepAction::Increment, 1., None, Some(100.)),
            None
        );
        // Decrement on a below-min value (or Increment on an above-max value)
        // does nothing, instead of moving the value in the opposite direction
        assert_eq!(
            step_value("5", StepAction::Decrement, 1., Some(10.), None),
            None
        );
        assert_eq!(
            step_value("1000", StepAction::Increment, 1., None, Some(100.)),
            None
        );

        // Invalid step values never leak NaN or infinity into the input.
        assert_eq!(step_value("1", StepAction::Increment, 0., None, None), None);
        assert_eq!(
            step_value("1", StepAction::Increment, -1., None, None),
            None
        );
        assert_eq!(
            step_value("1", StepAction::Increment, f64::NAN, None, None),
            None
        );
        assert_eq!(
            step_value("1", StepAction::Increment, f64::INFINITY, None, None),
            None
        );
    }

    #[gpui::test]
    fn number_input_builder_preserves_composite_states(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let _ = cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| InputState::new(window, cx).default_value("42"));
            let input = NumberInput::new(&state)
                .read_only(true)
                .invalid(true)
                .aria_label("Quantity")
                .aria_description("Selected quantity");

            assert!(input.read_only);
            assert!(input.invalid);
            assert_eq!(input.aria_label.as_deref(), Some("Quantity"));
            assert_eq!(input.aria_description.as_deref(), Some("Selected quantity"));
            EmptyView
        });
    }
}
