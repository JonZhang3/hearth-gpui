// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public types: `OtpEvent`, `OtpInputSlot`, `OtpInputGroup`, `OtpInputSeparator`,
//   `OtpInputChild`.
// - Added public methods: `pattern`, `paste_transformer`, `length`, `new`, `child`, `invalid`,
//   `aria_label`, `aria_description`.
// - Removed public methods: `groups`.
// - Removed or replaced `sync_to_input_state`, `on_input_mouse_down`, `to_digit_char`, `on_focus`,
//   `on_blur`, `pause_blink_cursor`, `groups`.
// - Reworked Otp Input around accessibility semantics and ARIA state, interruptible and
//   reduced-motion-aware transitions, semantic Style Preset geometry and density, keyboard
//   navigation and activation behavior, focus-visible and focus restoration behavior, invalid and
//   validation state handling.
use std::{rc::Rc, time::Instant};

use gpui::{
    Animation, AnimationExt as _, AnyElement, App, AppContext as _, Context, ElementId, Entity,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, ParentElement as _, Pixels, Render, RenderOnce, SharedString, StyleRefinement,
    Styled, Subscription, Window, div, prelude::FluentBuilder as _, px,
};

use super::{
    Input, InputContentType, InputEvent, InputState, MaskPattern,
    blink_cursor::CURSOR_WIDTH,
    input::{InputMotionKind, InputMotionState, InputPaintState, input_child_id, input_metrics},
};
use crate::animation::Lerp;
use crate::{
    ActiveTheme, Density, Disableable, Icon, IconName, Sizable, Size, StylePreset, StyledExt,
    h_flex,
};

/// OTP-specific events that are not shared by ordinary text inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OtpEvent {
    /// Emitted once when an incomplete value becomes complete.
    Complete,
}

type PasteTransformer = Rc<dyn Fn(&str) -> String>;

/// State for a composed one-time-password input.
///
/// A real [`InputState`] remains the only editing authority. The visible OTP
/// slots are projections of its value, cursor, selection, and focus state.
pub struct OtpState {
    value: SharedString,
    length: usize,
    masked: bool,
    pattern: MaskPattern,
    paste_transformer: Option<PasteTransformer>,
    input_state: Entity<InputState>,
    input_prepared: bool,
    _subscriptions: Vec<Subscription>,
}

impl OtpState {
    /// Creates an OTP state with a fixed number of digit slots.
    pub fn new(length: usize, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let length = length.max(1);
        let pattern = Self::digit_pattern(length);
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .mask_pattern(pattern.clone())
                .masked(false)
        });

        let _subscriptions = vec![cx.subscribe(
            &input_state,
            |this, input_state, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    this.sync_from_input_state(&input_state, cx);
                }
                InputEvent::Focus => {
                    cx.emit(InputEvent::Focus);
                    cx.notify();
                }
                InputEvent::Blur => {
                    cx.emit(InputEvent::Blur);
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => {}
            },
        )];

        Self {
            value: SharedString::default(),
            length,
            masked: false,
            pattern,
            paste_transformer: None,
            input_state,
            input_prepared: false,
            _subscriptions,
        }
    }

    /// Sets the initial value, normalized against the configured pattern.
    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        let value = value.into();
        self.value = self.normalize_value(&value).into();
        self
    }

    /// Sets the accepted per-slot mask pattern.
    ///
    /// Use `9` for digits, `A` for letters, `#` for alphanumeric values, and
    /// `*` for any character. Patterns should contain one token per OTP slot.
    pub fn pattern(mut self, pattern: impl Into<MaskPattern>) -> Self {
        self.pattern = pattern.into();
        self.value = self.normalize_value(&self.value).into();
        self
    }

    /// Transforms clipboard text before OTP normalization and insertion.
    pub fn paste_transformer(mut self, transformer: impl Fn(&str) -> String + 'static) -> Self {
        self.paste_transformer = Some(Rc::new(transformer));
        self
    }

    /// Sets the OTP value programmatically without emitting a change event.
    pub fn set_value(
        &mut self,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = value.into();
        self.value = self.normalize_value(&value).into();
        let value = self.value.clone();
        self.input_state.update(cx, |state, cx| {
            state.set_value(value, window, cx);
        });
        cx.notify();
    }

    /// Returns the normalized OTP value.
    pub fn value(&self) -> &SharedString {
        &self.value
    }

    /// Enables or disables masked slot rendering.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Updates masked rendering and accessibility value exposure.
    pub fn set_masked(&mut self, masked: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.masked = masked;
        self.input_state.update(cx, |state, cx| {
            state.set_masked(masked, window, cx);
        });
        cx.notify();
    }

    /// Focuses the single underlying editor.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.input_state.update(cx, |state, cx| {
            state.focus(window, cx);
        });
    }

    /// Returns the fixed number of visual slots.
    pub fn length(&self) -> usize {
        self.length
    }

    fn digit_pattern(length: usize) -> MaskPattern {
        MaskPattern::new("9".repeat(length).as_str())
    }

    /// Converts full-width digits to ASCII before applying the slot pattern.
    fn normalize_character(character: char) -> char {
        let Some(digit) = (character as u32).checked_sub('０' as u32) else {
            return character;
        };
        char::from_digit(digit, 10).unwrap_or(character)
    }

    /// Filters a candidate value through the current slot pattern and length.
    fn normalize_value(&self, value: &str) -> String {
        Self::normalize_with_pattern(value, self.length, &self.pattern)
    }

    fn normalize_with_pattern(value: &str, length: usize, pattern: &MaskPattern) -> String {
        let mut normalized = String::new();
        for character in value.chars() {
            if normalized.chars().count() >= length {
                break;
            }
            let character = Self::normalize_character(character);
            let position = normalized.chars().count();
            if pattern.is_valid_at(character, position) {
                normalized.push(character);
            }
        }
        normalized
    }

    /// Synchronizes builder-time state into the editor before it is rendered.
    fn prepare_input_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.input_prepared {
            return;
        }
        let value = self.value.clone();
        let masked = self.masked;
        let pattern = self.pattern.clone();
        let paste_pattern = pattern.clone();
        let length = self.length;
        let paste_transformer = self.paste_transformer.clone();
        let external_transformer: PasteTransformer = Rc::new(move |text| {
            let text = paste_transformer
                .as_ref()
                .map(|transformer| transformer(text))
                .unwrap_or_else(|| text.to_string());
            Self::normalize_with_pattern(&text, length, &paste_pattern)
        });
        self.input_state.update(cx, |state, cx| {
            state.set_mask_pattern(pattern, window, cx);
            state.paste_transformer = Some(external_transformer);
            state.max_paste_characters = Some(length);
            if state.masked != masked {
                state.set_masked(masked, window, cx);
            }
            if state.value() != value {
                state.set_value(value, window, cx);
            }
        });
        self.input_prepared = true;
    }

    /// Mirrors an editor mutation and preserves separate change/complete events.
    fn sync_from_input_state(&mut self, input_state: &Entity<InputState>, cx: &mut Context<Self>) {
        let raw_value = input_state.read(cx).value();
        let value = self.normalize_value(&raw_value);
        let was_complete = self.value.chars().count() == self.length;
        if self.value.as_ref() == value {
            return;
        }

        self.value = value.into();
        let is_complete = self.value.chars().count() == self.length;
        cx.emit(InputEvent::Change);
        if !was_complete && is_complete {
            cx.emit(OtpEvent::Complete);
        }
        cx.notify();
    }

    /// Focuses the editor and places the caret at the requested visual slot.
    fn focus_slot(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let cursor = self
            .value
            .char_indices()
            .nth(index)
            .map(|(offset, _)| offset)
            .unwrap_or(self.value.len());
        self.input_state.update(cx, |state, cx| {
            state.set_selected_range(cursor..cursor, cx);
            state.focus(window, cx);
        });
    }

    /// Handles full-width digit key events that the ASCII mask would reject.
    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(character) = event
            .keystroke
            .key_char
            .as_deref()
            .and_then(|value| value.chars().next())
        else {
            return;
        };
        let normalized = Self::normalize_character(character);
        if normalized == character || !normalized.is_ascii_digit() {
            return;
        }

        self.input_state.update(cx, |state, cx| {
            state.replace(normalized.to_string(), window, cx);
        });
        let input_state = self.input_state.clone();
        self.sync_from_input_state(&input_state, cx);
        window.prevent_default();
        cx.stop_propagation();
    }
}

impl Focusable for OtpState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input_state.focus_handle(cx)
    }
}

impl EventEmitter<InputEvent> for OtpState {}
impl EventEmitter<OtpEvent> for OtpState {}

impl Render for OtpState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// One visual slot in an [`OtpInputGroup`].
pub struct OtpInputSlot {
    index: usize,
    style: StyleRefinement,
}

impl OtpInputSlot {
    /// Creates a slot bound to the corresponding character index.
    pub fn new(index: usize) -> Self {
        Self {
            index,
            style: StyleRefinement::default(),
        }
    }
}

impl Styled for OtpInputSlot {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// A contiguous group of OTP slots.
pub struct OtpInputGroup {
    slots: Vec<OtpInputSlot>,
    style: StyleRefinement,
}

impl OtpInputGroup {
    /// Creates an empty group populated with typed `child` calls.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Appends one visual OTP slot.
    pub fn child(mut self, slot: OtpInputSlot) -> Self {
        self.slots.push(slot);
        self
    }
}

impl Default for OtpInputGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for OtpInputGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// A semantic separator between OTP groups.
pub struct OtpInputSeparator {
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl OtpInputSeparator {
    /// Creates a separator that renders the standard Minus icon.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Replaces the default Minus icon with custom separator content.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl Default for OtpInputSeparator {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for OtpInputSeparator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// Typed children accepted by [`OtpInput::child`].
pub enum OtpInputChild {
    /// A contiguous set of OTP slots.
    Group(OtpInputGroup),
    /// Visual and semantic separation between groups.
    Separator(OtpInputSeparator),
}

impl From<OtpInputGroup> for OtpInputChild {
    fn from(value: OtpInputGroup) -> Self {
        Self::Group(value)
    }
}

impl From<OtpInputSeparator> for OtpInputChild {
    fn from(value: OtpInputSeparator) -> Self {
        Self::Separator(value)
    }
}

/// Component-local geometry derived from semantic Style Preset properties.
#[derive(Clone, Copy)]
struct OtpMetrics {
    slot_size: Pixels,
    radius: Pixels,
    separator_size: Pixels,
    shadow: bool,
}

impl OtpMetrics {
    fn resolve(style: &StylePreset, size: Size) -> Self {
        let control = style.controls.for_size(size);
        let radius = match style.density {
            Density::Standard => style.radii.md,
            Density::Compact => style.radii.lg,
            Density::Comfortable => style.radii.xl,
        };
        Self {
            slot_size: control.height,
            radius,
            separator_size: style.controls.md.icon_size,
            shadow: style.density == Density::Standard && style.elevation.enabled,
        }
    }
}

/// A composable one-time-password input backed by one native editor.
#[derive(IntoElement)]
pub struct OtpInput {
    state: Entity<OtpState>,
    children: Vec<OtpInputChild>,
    style: StyleRefinement,
    size: Size,
    disabled: bool,
    invalid: bool,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
}

impl OtpInput {
    /// Creates an OTP input bound to the supplied state.
    pub fn new(state: &Entity<OtpState>) -> Self {
        Self {
            state: state.clone(),
            children: Vec::new(),
            style: StyleRefinement::default(),
            size: Size::Medium,
            disabled: false,
            invalid: false,
            aria_label: None,
            aria_description: None,
        }
    }

    /// Appends an [`OtpInputGroup`] or [`OtpInputSeparator`].
    pub fn child(mut self, child: impl Into<OtpInputChild>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Sets whether the value is invalid.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets the accessible name announced for the single editor.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets supporting text announced for the single editor.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }
}

impl Disableable for OtpInput {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Sizable for OtpInput {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for OtpInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for OtpInput {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state.update(cx, |state, cx| {
            state.prepare_input_state(window, cx);
        });

        let root_id: ElementId = ("otp-input", self.state.entity_id()).into();
        let metrics = OtpMetrics::resolve(&cx.theme().style, self.size);
        let input_metrics = input_metrics(&cx.theme().style);
        let background = Input::surface_background(input_metrics, false, cx);
        let foreground = cx.theme().foreground;
        let input_state = self.state.read(cx).input_state.clone();
        let (value, length, masked) = {
            let state = self.state.read(cx);
            (state.value.to_string(), state.length, state.masked)
        };
        let focused = input_state.focus_handle(cx).is_focused(window) && !self.disabled;
        let (selection, cursor, show_cursor) = {
            let editor = input_state.read(cx);
            let selection = editor.selected_range();
            let byte_to_character = |offset: usize| {
                value
                    .get(..offset.min(value.len()))
                    .map(str::chars)
                    .map(Iterator::count)
                    .unwrap_or_else(|| value.chars().count())
            };
            (
                byte_to_character(selection.start)..byte_to_character(selection.end),
                byte_to_character(editor.cursor()).min(length.saturating_sub(1)),
                editor.show_cursor(window, cx),
            )
        };

        if self.children.is_empty() {
            let mut group = OtpInputGroup::new();
            for index in 0..length {
                group = group.child(OtpInputSlot::new(index));
            }
            self.children.push(group.into());
        }

        let hidden_editor = Input::new(&input_state)
            .appearance(false)
            .bordered(false)
            .focus_bordered(false)
            .disabled(self.disabled)
            .invalid(self.invalid)
            .content_type(InputContentType::OneTimeCode)
            .when_some(self.aria_label.clone(), |this, label| {
                this.aria_label(label)
            })
            .when_some(self.aria_description.clone(), |this, description| {
                this.aria_description(description)
            })
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .left_0()
            .opacity(0.);

        let mut visual_children = Vec::with_capacity(self.children.len());
        for child in self.children {
            match child {
                OtpInputChild::Separator(separator) => {
                    let mut children = separator.children;
                    if children.is_empty() {
                        children.push(
                            Icon::new(IconName::Minus)
                                .with_size(Size::Size(metrics.separator_size))
                                .into_any_element(),
                        );
                    }
                    visual_children.push(
                        h_flex()
                            .items_center()
                            .justify_center()
                            .refine_style(&separator.style)
                            .children(children)
                            .into_any_element(),
                    );
                }
                OtpInputChild::Group(group) => {
                    let slot_count = group.slots.len();
                    let group_width = metrics.slot_size * slot_count as f32;
                    let mut slot_elements = Vec::with_capacity(slot_count);
                    let mut ring_elements = Vec::new();

                    for (position, slot) in group.slots.into_iter().enumerate() {
                        let index = slot.index;
                        let first = position == 0;
                        let last = position + 1 == slot_count;
                        let selected = !selection.is_empty()
                            && index >= selection.start
                            && index < selection.end;
                        let active =
                            focused && (selected || (selection.is_empty() && index == cursor));
                        let character = value.chars().nth(index);
                        let invalid_ring =
                            cx.theme()
                                .danger
                                .opacity(if cx.theme().is_dark() { 0.4 } else { 0.2 });
                        let ring = if active {
                            if self.invalid {
                                invalid_ring
                            } else {
                                cx.theme().ring.opacity(0.5)
                            }
                        } else {
                            cx.theme().transparent
                        };
                        let border = if self.invalid {
                            cx.theme().danger
                        } else if active {
                            cx.theme().ring
                        } else {
                            cx.theme().input
                        };
                        let paint = InputPaintState {
                            background,
                            border,
                            ring,
                        };
                        let motion_key =
                            input_child_id(&root_id, format!("slot-{}-motion", slot.index));
                        let motion_state = window
                            .use_keyed_state(motion_key, cx, |_, _| InputMotionState::new(paint));
                        let duration = if cx.reduce_motion() {
                            std::time::Duration::ZERO
                        } else {
                            cx.theme().style.motion.normal()
                        };
                        let easing = cx.theme().style.motion.move_easing;
                        let transition = motion_state.update(cx, |state, _| {
                            state.transition_to(
                                paint,
                                Instant::now(),
                                duration,
                                easing,
                                InputMotionKind::ColorsAndShadow,
                            )
                        });

                        let surface = div()
                            .absolute()
                            .top_0()
                            .right_0()
                            .bottom_0()
                            .left_0()
                            .bg(paint.background)
                            .border_color(paint.border)
                            .border_t_1()
                            .border_b_1()
                            .border_r_1()
                            .when(first, |this| this.border_l_1().rounded_l(metrics.radius))
                            .when(last, |this| this.rounded_r(metrics.radius));
                        let surface: AnyElement = if let Some(transition) = transition {
                            let from = transition.from;
                            let to = transition.to;
                            surface
                                .with_animation(
                                    input_child_id(
                                        &root_id,
                                        format!("slot-{}-surface-{}", index, transition.epoch),
                                    ),
                                    Animation::new(transition.duration)
                                        .with_easing(move |delta| easing.sample(delta)),
                                    move |this, delta| {
                                        this.bg(Lerp::lerp(&from.background, &to.background, delta))
                                            .border_color(Lerp::lerp(
                                                &from.border,
                                                &to.border,
                                                delta,
                                            ))
                                    },
                                )
                                .into_any_element()
                        } else {
                            surface.into_any_element()
                        };

                        let slot_state = self.state.clone();
                        let mut slot_element = div()
                            .id(("otp-slot", index))
                            .relative()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(metrics.slot_size)
                            .text_sm()
                            .text_color(foreground)
                            .when(metrics.shadow, |this| this.shadow_xs())
                            .refine_style(&slot.style)
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                crate::global_state::GlobalState::suppress_text_selection(cx);
                                slot_state.update(cx, |state, cx| {
                                    state.focus_slot(index, window, cx);
                                });
                            })
                            .child(surface);

                        slot_element = match character {
                            Some(_) if masked => slot_element.child(
                                Icon::new(IconName::Asterisk)
                                    .with_size(Size::Small)
                                    .text_color(foreground),
                            ),
                            Some(character) => slot_element.child(character.to_string()),
                            None if active && selection.is_empty() && show_cursor => {
                                slot_element.child(div().h_4().w(CURSOR_WIDTH).bg(cx.theme().caret))
                            }
                            None => slot_element,
                        };
                        slot_elements.push(slot_element.into_any_element());

                        if active || transition.is_some() {
                            let ring_width = cx.theme().style.focus.ring_width;
                            let ring_radius = if first || last {
                                metrics.radius + ring_width
                            } else {
                                px(0.)
                            };
                            let ring_left = metrics.slot_size * position as f32 - ring_width;
                            let ring_size = metrics.slot_size + ring_width * 2.;
                            let ring_element = div()
                                .absolute()
                                .top(-ring_width)
                                .left(ring_left)
                                .size(ring_size)
                                .border(ring_width)
                                .border_color(paint.ring)
                                .when(first, |this| this.rounded_l(ring_radius))
                                .when(last, |this| this.rounded_r(ring_radius));
                            let ring_element: AnyElement = if let Some(transition) = transition {
                                let from = transition.from;
                                let to = transition.to;
                                ring_element
                                    .with_animation(
                                        input_child_id(
                                            &root_id,
                                            format!("slot-{}-ring-{}", index, transition.epoch),
                                        ),
                                        Animation::new(transition.duration)
                                            .with_easing(move |delta| easing.sample(delta)),
                                        move |this, delta| {
                                            this.border_color(Lerp::lerp(
                                                &from.ring, &to.ring, delta,
                                            ))
                                        },
                                    )
                                    .into_any_element()
                            } else {
                                ring_element.into_any_element()
                            };
                            ring_elements.push(ring_element);
                        }
                    }

                    let invalid_ring_width = cx.theme().style.focus.ring_width;
                    let invalid_ring = self.invalid.then(|| {
                        div()
                            .absolute()
                            .top(-invalid_ring_width)
                            .left(-invalid_ring_width)
                            .w(group_width + invalid_ring_width * 2.)
                            .h(metrics.slot_size + invalid_ring_width * 2.)
                            .border(invalid_ring_width)
                            .border_color(cx.theme().danger.opacity(if cx.theme().is_dark() {
                                0.4
                            } else {
                                0.2
                            }))
                            .rounded(metrics.radius + invalid_ring_width)
                    });

                    visual_children.push(
                        h_flex()
                            .relative()
                            .items_center()
                            .w(group_width)
                            .h(metrics.slot_size)
                            .refine_style(&group.style)
                            .when_some(invalid_ring, |this, ring| this.child(ring))
                            .children(slot_elements)
                            .children(ring_elements)
                            .into_any_element(),
                    );
                }
            }
        }

        let key_state = self.state.clone();
        h_flex()
            .id(root_id)
            .relative()
            .items_center()
            .gap_2()
            .when(self.disabled, |this| this.opacity(0.5))
            .refine_style(&self.style)
            .on_key_down(move |event, window, cx| {
                key_state.update(cx, |state, cx| {
                    state.on_key_down(event, window, cx);
                });
            })
            .child(hidden_editor)
            .children(visual_children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otp_digits_are_normalized_and_truncated() {
        let pattern = OtpState::digit_pattern(4);
        assert_eq!(
            OtpState::normalize_with_pattern("1 a２-345", 4, &pattern),
            "1234"
        );
        assert_eq!(
            OtpState::normalize_with_pattern("no digits", 4, &pattern),
            ""
        );
        assert_eq!(
            OtpState::normalize_with_pattern("１２３", 4, &pattern),
            "123"
        );
    }

    #[test]
    fn alphanumeric_pattern_accepts_letters_and_digits() {
        let pattern = MaskPattern::new("######");
        assert!(pattern.is_valid_at('A', 0));
        assert!(pattern.is_valid_at('7', 1));
        assert!(!pattern.is_valid_at('-', 2));
    }

    #[test]
    fn preset_metrics_match_shadcn_slot_geometry() {
        assert_eq!(
            OtpMetrics::resolve(&StylePreset::vega(), Size::Medium).slot_size,
            px(36.)
        );
        assert_eq!(
            OtpMetrics::resolve(&StylePreset::nova(), Size::Medium).slot_size,
            px(32.)
        );
        assert_eq!(
            OtpMetrics::resolve(&StylePreset::maia(), Size::Medium).slot_size,
            px(36.)
        );
    }
}
