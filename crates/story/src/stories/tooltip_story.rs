// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `trigger`, `side`, `aria_label`, `gap_1`, `show_delay`, `show_arrow` and 1
//   more.
// - Removed examples using `tooltip`, `danger`.
// - Reworked Tooltip story around accessibility semantics and ARIA state.
use gpui::{
    App, AppContext, Context, Entity, Focusable, KeyBinding, ParentElement, Render, Styled, Window,
    actions, div, prelude::FluentBuilder as _,
};

use hearth_gpui::{
    ActiveTheme as _, Disableable as _, IconName, StyledExt as _,
    button::{Button, ButtonVariant, ButtonVariants, Toggle},
    checkbox::Checkbox,
    clipboard::Clipboard,
    dock::PanelControl,
    h_flex,
    radio::Radio,
    switch::Switch,
    tooltip::{Tooltip, TooltipAlign, TooltipSide, TooltipTrigger},
    v_flex,
};

use crate::{Story, section};

actions!(tooltip_story, [Info]);

pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-delete", Info, Some("Tooltip"))]);
}

pub struct TooltipStory {
    focus_handle: gpui::FocusHandle,
    removable_button_visible: bool,
}

impl TooltipStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            removable_button_visible: true,
        }
    }
}

impl Story for TooltipStory {
    fn title() -> &'static str {
        "Tooltip"
    }

    fn description() -> &'static str {
        "A popup that displays information related to an element when the element receives keyboard focus or the mouse hovers over it."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for TooltipStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TooltipStory {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .child(
                section("Tooltip for Button")
                    .child(
                        Button::new("btn0")
                            .label("Search")
                            .with_variant(ButtonVariant::Default)
                            .tooltip("This is a search Button."),
                    )
                    .child(Button::new("btn1").label("Info").tooltip_with_action(
                        "This is a tooltip with Action for display keybinding.",
                        &Info,
                        Some("Tooltip"),
                    ))
                    .child(
                        Button::new("btn3")
                            .label("Hover me")
                            .tooltip("This is tooltip 3"),
                    ),
            )
            .child(
                section("Checkbox Tooltip").child(
                    Checkbox::new("check")
                        .label("Remember me")
                        .checked(true)
                        .tooltip("This is a tooltip"),
                ),
            )
            .child(
                section("Radio Tooltip").child(
                    Radio::new("radio")
                        .label("Radio with tooltip")
                        .checked(true)
                        .tooltip("This is a radio button"),
                ),
            )
            .child(
                section("Switch Tooltip").child(
                    Switch::new("switch")
                        .checked(true)
                        .tooltip("This is a switch"),
                ),
            )
            .child(
                section("Toggle Tooltip").child(
                    h_flex()
                        .gap_2()
                        .child(Toggle::new("toggle1").label("Bold").tooltip("Toggle bold"))
                        .child(
                            Toggle::new("toggle2")
                                .icon(IconName::Heart)
                                .tooltip("Toggle favorite"),
                        ),
                ),
            )
            .child(
                section("Clipboard Tooltip").child(
                    Clipboard::new("clip1")
                        .value("Hello, World!")
                        .tooltip("Copy to clipboard"),
                ),
            )
            .child(
                section("Placement")
                    .child(
                        TooltipTrigger::new("tooltip-top")
                            .trigger(Button::new("tooltip-top-button").label("Top"))
                            .text("Tooltip above the trigger")
                            .side(TooltipSide::Top),
                    )
                    .child(
                        TooltipTrigger::new("tooltip-right")
                            .trigger(Button::new("tooltip-right-button").label("Right"))
                            .text("Tooltip to the right")
                            .side(TooltipSide::Right),
                    )
                    .child(
                        TooltipTrigger::new("tooltip-bottom")
                            .trigger(Button::new("tooltip-bottom-button").label("Bottom"))
                            .text("Tooltip below the trigger")
                            .side(TooltipSide::Bottom),
                    )
                    .child(
                        TooltipTrigger::new("tooltip-left")
                            .trigger(Button::new("tooltip-left-button").label("Left"))
                            .text("Tooltip to the left")
                            .side(TooltipSide::Left),
                    ),
            )
            .child(
                section("Alignment")
                    .child(
                        TooltipTrigger::new("tooltip-align-start")
                            .trigger(Button::new("tooltip-align-start-button").label("Start"))
                            .text("Start aligned")
                            .align(TooltipAlign::Start),
                    )
                    .child(
                        TooltipTrigger::new("tooltip-align-center")
                            .trigger(Button::new("tooltip-align-center-button").label("Center"))
                            .text("Center aligned")
                            .align(TooltipAlign::Center),
                    )
                    .child(
                        TooltipTrigger::new("tooltip-align-end")
                            .trigger(Button::new("tooltip-align-end-button").label("End"))
                            .text("End aligned")
                            .align(TooltipAlign::End),
                    ),
            )
            .child(
                section("Content")
                    .child(
                        TooltipTrigger::new("tooltip-icon")
                            .trigger(
                                Button::new("tooltip-icon-button")
                                    .icon(IconName::Info)
                                    .aria_label("Information"),
                            )
                            .text("Information"),
                    )
                    .child(
                        TooltipTrigger::new("tooltip-long")
                            .trigger(Button::new("tooltip-long-button").label("Long content"))
                            .text("Tooltips wrap long supplementary text within the shadcn maximum width instead of expanding without a limit."),
                    )
                    .child(
                        TooltipTrigger::new("tooltip-kbd")
                            .trigger(Button::new("tooltip-kbd-button").label("With shortcut"))
                            .content(|window, cx| {
                                Tooltip::new("Delete item")
                                    .action(&Info, Some("Tooltip"))
                                    .build(window, cx)
                            }),
                    )
                    .child(
                        TooltipTrigger::new("tooltip-formatted")
                            .trigger(Button::new("tooltip-formatted-button").label("Formatted"))
                            .content(|window, cx| {
                                Tooltip::element(|_, cx| {
                                    v_flex()
                                        .gap_1()
                                        .child(div().font_medium().child("Project status"))
                                        .child(
                                            div()
                                                .text_color(cx.theme().background.opacity(0.8))
                                                .child("All checks passed"),
                                        )
                                })
                                .build(window, cx)
                            }),
                    ),
            )
            .child(
                section("States")
                    .child(
                        TooltipTrigger::new("tooltip-disabled")
                            .trigger(Button::new("tooltip-disabled-button").label("Disabled").disabled(true))
                            .text("Disabled controls remain available to pointer hover."),
                    )
                    .child(
                        TooltipTrigger::new("tooltip-no-delay")
                            .trigger(Button::new("tooltip-no-delay-button").label("No delay"))
                            .text("Opens without pointer delay")
                            .show_delay(std::time::Duration::ZERO),
                    )
                    .child(
                        TooltipTrigger::new("tooltip-no-arrow")
                            .trigger(Button::new("tooltip-no-arrow-button").label("No arrow"))
                            .text("Arrow disabled")
                            .show_arrow(false),
                    ),
            )
            .child(
                section("Tooltip trigger removed on click").child(
                    h_flex()
                        .gap_2()
                        .when(self.removable_button_visible, |this| {
                            this.child(
                                Button::new("remove-tooltip-trigger")
                                    .destructive()
                                    .label("Remove me")
                                    .tooltip("Clicking this button removes the trigger.")
                                    .on_click(cx.listener(|story, _, _, cx| {
                                        story.removable_button_visible = false;
                                        cx.notify();
                                    })),
                            )
                        })
                        .when(!self.removable_button_visible, |this| {
                            this.child(
                                Button::new("restore-tooltip-trigger")
                                    .label("Restore button")
                                    .on_click(cx.listener(|story, _, _, cx| {
                                        story.removable_button_visible = true;
                                        cx.notify();
                                    })),
                            )
                        }),
                ),
            )
    }
}
