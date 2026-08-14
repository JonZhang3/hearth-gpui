// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `aria_label`, `gap_4`, `label_side`, `invalid`.
// - Reworked Switch story around accessibility semantics and ARIA state, semantic Style Preset
//   geometry and density, invalid and validation state handling.
// - Replaced legacy radius access with `Theme.style.radii.md`.
use gpui::{
    App, AppContext, Context, Div, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, SharedString, Styled, Window, px,
};

use hearth_gpui::{
    ActiveTheme, Disableable as _, Sizable, h_flex, label::Label, switch::Switch, v_flex,
};

use crate::section;

pub struct SwitchStory {
    focus_handle: FocusHandle,
    switch1: bool,
    switch2: bool,
    switch3: bool,
    switch4: bool,
    switch5: bool,
    invalid1: bool,
    invalid2: bool,
}

impl super::Story for SwitchStory {
    fn title() -> &'static str {
        "Switch"
    }

    fn description() -> &'static str {
        "A control that allows the user to toggle between checked and not checked."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SwitchStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            switch1: true,
            switch2: false,
            switch3: true,
            switch4: true,
            switch5: false,
            invalid1: false,
            invalid2: true,
        }
    }
}

impl Focusable for SwitchStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SwitchStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        fn title(title: impl Into<SharedString>) -> Div {
            v_flex().flex_1().gap_2().child(Label::new(title).text_xl())
        }

        fn card(cx: &Context<SwitchStory>) -> Div {
            h_flex()
                .items_center()
                .gap_4()
                .p_4()
                .w_full()
                .rounded(cx.theme().style.radii.md)
                .border_1()
                .border_color(cx.theme().border)
        }

        v_flex()
            .w_full()
            .gap_3()
            .child(
                card(cx)
                    .child(
                        title("Marketing emails").child(
                            Label::new("Receive emails about new products, features, and more.")
                                .text_color(theme.muted_foreground),
                        ),
                    )
                    .child(
                        h_flex().gap_2().child("Subscribe").child(
                            Switch::new("switch1")
                                .aria_label("Subscribe to marketing emails")
                                .checked(self.switch1)
                                .on_click(cx.listener(move |view, checked, _, cx| {
                                    view.switch1 = *checked;
                                    cx.notify();
                                })),
                        ),
                    ),
            )
            .child(
                card(cx)
                    .child(
                        title("Security emails").child(
                            Label::new(
                                "Receive emails about your account security. \
                                    When turn off, you never receive email again.",
                            )
                            .text_color(theme.muted_foreground),
                        ),
                    )
                    .child(
                        Switch::new("switch2")
                            .aria_label("Receive security emails")
                            .checked(self.switch2)
                            .on_click(cx.listener(move |view, checked, _, cx| {
                                view.switch2 = *checked;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("Disabled")
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                Switch::new("disabled-unchecked")
                                    .label("Disabled (unchecked)")
                                    .disabled(true),
                            )
                            .child(
                                Switch::new("disabled-checked")
                                    .label("Disabled (checked)")
                                    .checked(true)
                                    .disabled(true),
                            ),
                    )
                    .child(
                        Switch::new("disabled-left-label")
                            .w(px(200.))
                            .label("Airplane Mode")
                            .label_side(hearth_gpui::Side::Left)
                            .checked(true)
                            .disabled(true),
                    ),
            )
            .child(
                section("Invalid").child(
                    h_flex()
                        .gap_4()
                        .child(
                            Switch::new("invalid-unchecked")
                                .checked(self.invalid1)
                                .invalid(true)
                                .label("Required")
                                .on_click(cx.listener(|view, checked, _, cx| {
                                    view.invalid1 = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Switch::new("invalid-checked")
                                .checked(self.invalid2)
                                .invalid(true)
                                .label("Invalid checked")
                                .on_click(cx.listener(|view, checked, _, cx| {
                                    view.invalid2 = *checked;
                                    cx.notify();
                                })),
                        ),
                ),
            )
            .child(
                section("Sizes").child(
                    h_flex()
                        .gap_4()
                        .child(
                            Switch::new("size-default")
                                .checked(self.switch3)
                                .label("Default")
                                .on_click(cx.listener(|view, checked, _, cx| {
                                    view.switch3 = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Switch::new("size-small")
                                .checked(self.switch3)
                                .label("Small")
                                .small()
                                .on_click(cx.listener(|view, checked, _, cx| {
                                    view.switch3 = *checked;
                                    cx.notify();
                                })),
                        ),
                ),
            )
            .child(
                section("Custom Color").child(
                    h_flex()
                        .gap_4()
                        .child(
                            Switch::new("switch4")
                                .checked(self.switch4)
                                .label("Success")
                                .color(theme.success)
                                .on_click(cx.listener(|view, checked, _, cx| {
                                    view.switch4 = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Switch::new("switch5")
                                .checked(self.switch5)
                                .label("Destructive")
                                .color(theme.danger)
                                .on_click(cx.listener(|view, checked, _, cx| {
                                    view.switch5 = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Switch::new("switch4_disabled")
                                .checked(true)
                                .label("Disabled")
                                .color(theme.success)
                                .disabled(true),
                        ),
                ),
            )
    }
}
