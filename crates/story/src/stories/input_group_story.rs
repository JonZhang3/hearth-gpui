use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, Keystroke, ParentElement as _, Render,
    Styled as _, Window,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName,
    button::Button,
    input::{
        Input, InputGroup, InputGroupAddon, InputGroupAddonAlign, InputGroupButton,
        InputGroupButtonSize, InputGroupText, InputState,
    },
    kbd::Kbd,
    spinner::Spinner,
    v_flex,
};

use crate::section;

pub struct InputGroupStory {
    search: Entity<InputState>,
    shortcut: Entity<InputState>,
    website: Entity<InputState>,
    loading: Entity<InputState>,
    disabled: Entity<InputState>,
    invalid: Entity<InputState>,
    message: Entity<InputState>,
}

impl super::Story for InputGroupStory {
    fn title() -> &'static str {
        "Input Group"
    }

    fn description() -> &'static str {
        "Compose inputs with inline or block addons, helper text, and actions."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl InputGroupStory {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            search: cx.new(|cx| InputState::new(window, cx).placeholder("Search...")),
            shortcut: cx.new(|cx| InputState::new(window, cx).placeholder("Search commands...")),
            website: cx.new(|cx| InputState::new(window, cx).placeholder("example.com")),
            loading: cx.new(|cx| InputState::new(window, cx).placeholder("Searching...")),
            disabled: cx.new(|cx| InputState::new(window, cx).placeholder("Unavailable")),
            invalid: cx.new(|cx| InputState::new(window, cx).placeholder("Email address")),
            message: cx.new(|cx| {
                InputState::new(window, cx)
                    .auto_grow(3, 8)
                    .placeholder("Ask, search, or chat...")
            }),
        }
    }
}

impl Render for InputGroupStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .child(
                section("Inline Addons").max_w_md().child(
                    v_flex()
                        .w_full()
                        .gap_4()
                        .child(
                            InputGroup::new("search-input-group")
                                .input(Input::new(&self.search))
                                .addon(InputGroupAddon::new().child(Icon::new(IconName::Search)))
                                .addon(
                                    InputGroupAddon::new()
                                        .align(InputGroupAddonAlign::InlineEnd)
                                        .child(InputGroupText::new().child("12 results")),
                                ),
                        )
                        .child(
                            InputGroup::new("website-input-group")
                                .input(Input::new(&self.website))
                                .addon(
                                    InputGroupAddon::new()
                                        .child(InputGroupText::new().child("https://")),
                                )
                                .addon(
                                    InputGroupAddon::new()
                                        .align(InputGroupAddonAlign::InlineEnd)
                                        .child(
                                            InputGroupButton::new(
                                                Button::new("website-info")
                                                    .icon(IconName::Info)
                                                    .aria_label("Website information"),
                                            )
                                            .size(InputGroupButtonSize::IconXs),
                                        ),
                                ),
                        )
                        .child(
                            InputGroup::new("shortcut-input-group")
                                .input(Input::new(&self.shortcut))
                                .addon(InputGroupAddon::new().child(Icon::new(IconName::Search)))
                                .addon(
                                    InputGroupAddon::new()
                                        .align(InputGroupAddonAlign::InlineEnd)
                                        .child(Kbd::from_keystroke(
                                            Keystroke::parse("cmd-k").unwrap(),
                                        )),
                                ),
                        ),
                ),
            )
            .child(
                section("States").max_w_md().child(
                    v_flex()
                        .w_full()
                        .gap_4()
                        .child(
                            InputGroup::new("loading-input-group")
                                .input(Input::new(&self.loading).disabled(true))
                                .addon(
                                    InputGroupAddon::new()
                                        .align(InputGroupAddonAlign::InlineEnd)
                                        .child(Spinner::new()),
                                ),
                        )
                        .child(
                            InputGroup::new("disabled-input-group")
                                .input(Input::new(&self.disabled))
                                .disabled(true)
                                .addon(InputGroupAddon::new().child(Icon::new(IconName::Info))),
                        )
                        .child(
                            InputGroup::new("invalid-input-group")
                                .input(Input::new(&self.invalid))
                                .invalid(true)
                                .aria_label("Invalid email")
                                .addon(
                                    InputGroupAddon::new()
                                        .align(InputGroupAddonAlign::InlineEnd)
                                        .child(InputGroupText::new().child("Required")),
                                ),
                        ),
                ),
            )
            .child(
                section("Block Addons").max_w_lg().child(
                    InputGroup::new("message-input-group")
                        .input(Input::new(&self.message))
                        .addon(
                            InputGroupAddon::new()
                                .align(InputGroupAddonAlign::BlockStart)
                                .border_b_1()
                                .border_color(cx.theme().border)
                                .child(InputGroupText::new().child("message.txt")),
                        )
                        .addon(
                            InputGroupAddon::new()
                                .align(InputGroupAddonAlign::BlockEnd)
                                .border_t_1()
                                .border_color(cx.theme().border)
                                .child(InputGroupText::new().child("Markdown supported"))
                                .child(
                                    InputGroupButton::new(
                                        Button::new("send-message")
                                            .icon(IconName::ArrowUp)
                                            .aria_label("Send message"),
                                    )
                                    .size(InputGroupButtonSize::IconXs)
                                    .bg(cx.theme().primary)
                                    .text_color(cx.theme().primary_foreground),
                                ),
                        ),
                ),
            )
    }
}
