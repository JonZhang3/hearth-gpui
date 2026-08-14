// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added story helpers for `toggle_otp_masked`, `group`, `six_slots`, `separated_pairs`.
// - Removed or replaced story helpers: `toggle_opt_masked`.
// - Added examples for `pattern`, `into_iter`, `aria_label`, `gap_3`, `aria_description`,
//   `invalid`.
// - Removed examples using `masked`, `groups`, `large`.
// - Reworked Otp Input story around accessibility semantics and ARIA state, focus-visible and focus
//   restoration behavior, invalid and validation state handling.
use gpui::{
    App, AppContext as _, Context, Entity, Focusable, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, Styled as _, Subscription, Window, px,
};
use gpui_component::{
    Disableable as _, Sizable as _, StyledExt as _,
    checkbox::Checkbox,
    h_flex,
    input::{
        InputEvent, OtpEvent, OtpInput, OtpInputGroup, OtpInputSeparator, OtpInputSlot, OtpState,
    },
    v_flex,
};

use crate::section;

pub fn init(_: &mut App) {}

pub struct OtpInputStory {
    otp_masked: bool,
    otp_state: Entity<OtpState>,
    otp_value: SharedString,
    otp_complete: bool,
    alphanumeric_state: Entity<OtpState>,
    four_digit_state: Entity<OtpState>,
    separator_state: Entity<OtpState>,
    invalid_state: Entity<OtpState>,
    disabled_state: Entity<OtpState>,
    custom_size_state: Entity<OtpState>,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for OtpInputStory {
    fn title() -> &'static str {
        "OtpInput"
    }

    fn description() -> &'static str {
        "A composable one-time-code input with native editing, selection, paste, and accessibility behavior."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl OtpInputStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let otp_state = cx.new(|cx| OtpState::new(6, window, cx).masked(true));
        let _subscriptions = vec![
            cx.subscribe(&otp_state, |this, state, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.otp_value = state.read(cx).value().clone();
                    this.otp_complete = false;
                    cx.notify();
                }
            }),
            cx.subscribe(&otp_state, |this, _, event: &OtpEvent, cx| {
                if matches!(event, OtpEvent::Complete) {
                    this.otp_complete = true;
                    cx.notify();
                }
            }),
        ];

        Self {
            otp_masked: true,
            otp_state,
            otp_value: SharedString::default(),
            otp_complete: false,
            alphanumeric_state: cx.new(|cx| {
                OtpState::new(6, window, cx)
                    .pattern("######")
                    .default_value("A1B2")
            }),
            four_digit_state: cx.new(|cx| OtpState::new(4, window, cx)),
            separator_state: cx.new(|cx| OtpState::new(6, window, cx)),
            invalid_state: cx.new(|cx| OtpState::new(6, window, cx).default_value("000000")),
            disabled_state: cx.new(|cx| OtpState::new(6, window, cx).default_value("123456")),
            custom_size_state: cx.new(|cx| OtpState::new(6, window, cx)),
            _subscriptions,
        }
    }

    fn toggle_otp_masked(&mut self, _: &bool, window: &mut Window, cx: &mut Context<Self>) {
        self.otp_masked = !self.otp_masked;
        self.otp_state.update(cx, |state, cx| {
            state.set_masked(self.otp_masked, window, cx);
        });
        cx.notify();
    }

    fn group(indices: impl IntoIterator<Item = usize>) -> OtpInputGroup {
        indices
            .into_iter()
            .fold(OtpInputGroup::new(), |group, index| {
                group.child(OtpInputSlot::new(index))
            })
    }

    fn six_slots(state: &Entity<OtpState>) -> OtpInput {
        OtpInput::new(state)
            .child(Self::group(0..3))
            .child(OtpInputSeparator::new())
            .child(Self::group(3..6))
            .aria_label("One-time code")
    }

    fn separated_pairs(state: &Entity<OtpState>) -> OtpInput {
        OtpInput::new(state)
            .child(Self::group(0..2))
            .child(OtpInputSeparator::new())
            .child(Self::group(2..4))
            .child(OtpInputSeparator::new())
            .child(Self::group(4..6))
            .aria_label("Grouped one-time code")
    }
}

impl Focusable for OtpInputStory {
    fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        self.otp_state.focus_handle(cx)
    }
}

impl Render for OtpInputStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("otp-input-story")
            .size_full()
            .gap_5()
            .child(
                h_flex().items_center().child(
                    Checkbox::new("otp-mask")
                        .label("Masked")
                        .checked(self.otp_masked)
                        .on_click(cx.listener(Self::toggle_otp_masked)),
                ),
            )
            .child(
                section("Default")
                    .v_flex()
                    .gap_3()
                    .child(Self::six_slots(&self.otp_state))
                    .child(if self.otp_value.is_empty() {
                        "Enter your one-time code.".to_string()
                    } else if self.otp_complete {
                        format!("Complete code: {}", self.otp_value)
                    } else {
                        format!("Current value: {}", self.otp_value)
                    }),
            )
            .child(section("Separator").child(Self::separated_pairs(&self.separator_state)))
            .child(
                section("Alphanumeric").child(
                    Self::six_slots(&self.alphanumeric_state)
                        .aria_description("Accepts ASCII letters and digits."),
                ),
            )
            .child(
                section("Four Digits").child(
                    OtpInput::new(&self.four_digit_state)
                        .child(Self::group(0..4))
                        .aria_label("Four-digit PIN"),
                ),
            )
            .child(
                section("Invalid").child(
                    Self::six_slots(&self.invalid_state)
                        .invalid(true)
                        .aria_description("Invalid code. Please try again."),
                ),
            )
            .child(section("Disabled").child(Self::six_slots(&self.disabled_state).disabled(true)))
            .child(
                section("Custom Size")
                    .child(Self::six_slots(&self.custom_size_state).with_size(px(44.))),
            )
    }
}
