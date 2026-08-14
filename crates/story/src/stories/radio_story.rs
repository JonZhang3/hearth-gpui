// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `invalid`, `aria_label`, `max_w_md`, `aria_description`.
// - Removed examples using `selected_index`.
// - Reworked Radio story around accessibility semantics and ARIA state, semantic Style Preset
//   geometry and density, invalid and validation state handling.
// - Replaced legacy radius access with `Theme.style.radii.md`.
use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, SharedString,
    Styled, Window, div, px,
};

use hearth_gpui::{
    ActiveTheme, Disableable, Sizable, h_flex,
    radio::{Radio, RadioGroup, RadioGroupItem},
    v_flex,
};

use crate::section;

pub struct RadioStory {
    focus_handle: gpui::FocusHandle,
    radio_check1: bool,
    radio_check2: bool,
    radio_group_value: Option<SharedString>,
}

impl super::Story for RadioStory {
    fn title() -> &'static str {
        "Radio"
    }

    fn description() -> &'static str {
        "A set of checkable buttons—known as radio buttons—where no more than one of the buttons can be checked at a time."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl RadioStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            radio_check1: false,
            radio_check2: true,
            radio_group_value: Some("two".into()),
        }
    }
}

impl Focusable for RadioStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RadioStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Radio")
                    .max_w_md()
                    .child(
                        Radio::new("radio1")
                            .checked(self.radio_check1)
                            .on_click(cx.listener(|this, checked, _, _| {
                                this.radio_check1 = *checked;
                            })),
                    )
                    .child(
                        Radio::new("radio2")
                            .label("Radio 2")
                            .checked(self.radio_check2)
                            .on_click(cx.listener(|this, checked, _, _| {
                                this.radio_check2 = *checked;
                            })),
                    ),
            )
            .child(
                section("Disabled")
                    .child(Radio::new("a").label("Disabled").disabled(true))
                    .child(
                        Radio::new("b")
                            .label("Disabled with Checked")
                            .checked(true)
                            .disabled(true),
                    ),
            )
            .child(
                section("Invalid")
                    .child(
                        Radio::new("invalid-radio")
                            .label("Invalid option")
                            .invalid(true),
                    )
                    .child(
                        Radio::new("invalid-radio-selected")
                            .label("Invalid selected option")
                            .checked(true)
                            .invalid(true),
                    ),
            )
            .child(
                section("Multi-line Label").child(
                    Radio::new("radio3")
                        .label("The long long label text.")
                        .child(
                            div()
                                .text_color(cx.theme().muted_foreground)
                                .child("This line should wrap when the text is too long."),
                        )
                        .w(px(300.))
                        .checked(true)
                        .disabled(true),
                ),
            )
            .child(
                section("Sizeable").child(
                    h_flex()
                        .h_full()
                        .gap_x_4()
                        .child(
                            Radio::new("xsmall")
                                .label("Small")
                                .xsmall()
                                .checked(self.radio_check2)
                                .on_click(cx.listener(|this, v, _, _| {
                                    this.radio_check2 = *v;
                                })),
                        )
                        .child(
                            Radio::new("large")
                                .label("Large")
                                .large()
                                .checked(self.radio_check2)
                                .on_click(cx.listener(|this, v, _, _| {
                                    this.radio_check2 = *v;
                                })),
                        ),
                ),
            )
            .child(
                section("Radio Group").max_w_md().child(
                    v_flex().child(
                        RadioGroup::horizontal("radio_group_1")
                            .aria_label("Horizontal options")
                            .children([
                                RadioGroupItem::new("one").label("One"),
                                RadioGroupItem::new("two").label("Two"),
                                RadioGroupItem::new("three").label("Three"),
                            ])
                            .value(self.radio_group_value.clone())
                            .on_change(cx.listener(|this, value: &SharedString, _, cx| {
                                this.radio_group_value = Some(value.clone());
                                cx.notify();
                            })),
                    ),
                ),
            )
            .child(
                section("Radio Group With Descriptions").max_w_md().child(
                    RadioGroup::vertical("radio_group_descriptions")
                        .aria_label("Plan")
                        .value(self.radio_group_value.clone())
                        .child(
                            RadioGroupItem::new("one")
                                .label("Plus")
                                .aria_description("For individuals and small teams")
                                .child(
                                    div()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("For individuals and small teams"),
                                ),
                        )
                        .child(
                            RadioGroupItem::new("two")
                                .label("Pro")
                                .aria_description("For growing businesses")
                                .child(
                                    div()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("For growing businesses"),
                                ),
                        )
                        .child(
                            RadioGroupItem::new("three")
                                .label("Enterprise")
                                .disabled(true)
                                .child(
                                    div()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("Disabled option"),
                                ),
                        )
                        .on_change(cx.listener(|this, value: &SharedString, _, cx| {
                            this.radio_group_value = Some(value.clone());
                            cx.notify();
                        })),
                ),
            )
            .child(
                section("Radio Group Vertical (With container style)")
                    .max_w_md()
                    .child(
                        v_flex().items_center().content_center().child(
                            RadioGroup::vertical("radio_group_2")
                                .aria_label("Country")
                                .w(px(220.))
                                .p_2()
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded(cx.theme().style.radii.md)
                                .disabled(true)
                                .child(RadioGroupItem::new("us").label("United States"))
                                .child(RadioGroupItem::new("ca").label("Canada"))
                                .child(RadioGroupItem::new("mx").label("Mexico"))
                                .value(Some("ca")),
                        ),
                    ),
            )
    }
}
