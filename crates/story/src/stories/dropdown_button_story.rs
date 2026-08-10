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
    primary_action_count: usize,
}

impl DropdownButtonStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            disabled: false,
            selected: false,
            primary_action_count: 0,
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
        let primary_label = format!("Primary Dropdown ({})", self.primary_action_count);

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
                        .aria_label("Primary actions")
                        .menu_aria_label("Open primary options")
                        .button(Button::new("btn0-main").label(primary_label).on_click(
                            cx.listener(|view, _, _, cx| {
                                view.primary_action_count += 1;
                                cx.notify();
                            }),
                        ))
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
                        .button(Button::new("btn-sm-main").label("Small Dropdown"))
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
                section("Large Size").child(
                    DropdownButton::new("btn-lg")
                        .large()
                        .button(Button::new("btn-lg-main").label("Large Dropdown"))
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
                        .button(Button::new("btn-outline-main").label("Outline Dropdown"))
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
                section("Secondary").child(
                    DropdownButton::new("btn-secondary")
                        .secondary()
                        .button(Button::new("btn-secondary-main").label("Secondary Dropdown"))
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
                section("Destructive").child(
                    DropdownButton::new("btn-destructive")
                        .destructive()
                        .button(Button::new("btn-destructive-main").label("Delete"))
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
                        .button(Button::new("btn-ghost-main").label("Ghost Dropdown"))
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
