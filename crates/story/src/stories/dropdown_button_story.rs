use gpui::{
    Action, Anchor, App, AppContext as _, Context, Entity, Focusable, IntoElement,
    ParentElement as _, Render, Styled as _, Window,
};
use serde::Deserialize;

use crate::section;
use gpui_component::{
    Disableable, Selectable as _, Sizable as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    checkbox::Checkbox,
    h_flex, v_flex,
};

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = dropdown_button_story, no_json)]
enum ButtonAction {
    Disabled,
    Selected,
}

pub struct DropdownButtonStory {
    focus_handle: gpui::FocusHandle,
    disabled: bool,
    selected: bool,
}

impl DropdownButtonStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            disabled: false,
            selected: false,
        })
    }
}

impl super::Story for DropdownButtonStory {
    fn title() -> &'static str {
        "DropdownButton"
    }

    fn description() -> &'static str {
        "A button with an attached dropdown menu for additional options."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for DropdownButtonStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DropdownButtonStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.disabled;
        let selected = self.selected;

        v_flex()
            .gap_6()
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Checkbox::new("disabled-button")
                            .label("Disabled")
                            .checked(self.disabled)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.disabled = !view.disabled;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("selected-button")
                            .label("Selected")
                            .checked(self.selected)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.selected = !view.selected;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("Dropdown Button").child(
                    DropdownButton::new("btn0")
                        .button(Button::new("btn").label("Primary Dropdown"))
                        .disabled(self.disabled)
                        .selected(selected)
                        .dropdown_menu_with_anchor(Anchor::BottomRight, move |this, _, _| {
                            this.menu_with_check(
                                "Disabled",
                                disabled,
                                Box::new(ButtonAction::Disabled),
                            )
                            .menu_with_check(
                                "Selected",
                                selected,
                                Box::new(ButtonAction::Selected),
                            )
                        }),
                ),
            )
            .child(
                section("Small Size").child(
                    DropdownButton::new("btn-sm")
                        .small()
                        .button(Button::new("btn").label("Small Dropdown"))
                        .disabled(self.disabled)
                        .selected(selected)
                        .dropdown_menu(move |this, _, _| {
                            this.menu_with_check(
                                "Disabled",
                                disabled,
                                Box::new(ButtonAction::Disabled),
                            )
                            .menu_with_check(
                                "Selected",
                                selected,
                                Box::new(ButtonAction::Selected),
                            )
                        }),
                ),
            )
            .child(
                section("Outline").child(
                    DropdownButton::new("btn-outline")
                        .outline()
                        .button(Button::new("btn").label("Outline Dropdown"))
                        .disabled(self.disabled)
                        .selected(selected)
                        .dropdown_menu(move |this, _, _| {
                            this.menu_with_check(
                                "Disabled",
                                disabled,
                                Box::new(ButtonAction::Disabled),
                            )
                            .menu_with_check(
                                "Selected",
                                selected,
                                Box::new(ButtonAction::Selected),
                            )
                        }),
                ),
            )
            .child(
                section("Ghost").child(
                    DropdownButton::new("btn-ghost")
                        .ghost()
                        .button(Button::new("btn").label("Ghost Dropdown"))
                        .disabled(self.disabled)
                        .selected(selected)
                        .dropdown_menu(move |this, _, _| {
                            this.menu_with_check(
                                "Disabled",
                                disabled,
                                Box::new(ButtonAction::Disabled),
                            )
                            .menu_with_check(
                                "Selected",
                                selected,
                                Box::new(ButtonAction::Selected),
                            )
                        }),
                ),
            )
    }
}
