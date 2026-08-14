use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window, div, px,
};
use hearth_gpui::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::Button,
    dock::PanelControl,
    empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle},
    h_flex,
    input::{Input, InputGroup, InputGroupAddon, InputGroupAddonAlign, InputState},
    v_flex,
};

use crate::section;

pub struct EmptyStory {
    focus_handle: FocusHandle,
    search: Entity<InputState>,
}

impl EmptyStory {
    /// Creates the Story state and its embedded search field.
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            search: cx.new(|cx| InputState::new(window, cx).placeholder("Search documentation...")),
        }
    }

    /// Creates a shared Empty Story entity.
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for EmptyStory {
    fn title() -> &'static str {
        "Empty"
    }

    fn description() -> &'static str {
        "Displays an empty or no-result state with optional media and actions."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for EmptyStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Builds the shared example header used by the outline and muted variants.
fn example_header(icon: IconName, title: &'static str, description: &'static str) -> EmptyHeader {
    EmptyHeader::new()
        .child(EmptyMedia::icon(icon))
        .child(EmptyTitle::new().child(title))
        .child(EmptyDescription::new().child(description))
}

impl Render for EmptyStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .child(
                section("Basic").child(
                    Empty::new()
                        .min_h(px(320.))
                        .child(
                            EmptyHeader::new()
                                .child(EmptyTitle::new().child("No projects yet"))
                                .child(EmptyDescription::new().child(
                                    "Create a project to organize your files and collaborate with your team.",
                                )),
                        )
                        .child(
                            EmptyContent::new().child(
                                h_flex()
                                    .gap_2()
                                    .child(Button::new("create-project").label("Create project"))
                                    .child(Button::new("import-project").outline().label("Import")),
                            ),
                        ),
                ),
            )
            .child(
                section("Icon and Outline").child(
                    Empty::new()
                        .min_h(px(320.))
                        .border_1()
                        .border_color(cx.theme().border)
                        .child(example_header(
                            IconName::Inbox,
                            "No messages",
                            "New conversations will appear here.",
                        ))
                        .child(
                            EmptyContent::new().child(
                                Button::new("new-message")
                                    .icon(IconName::Plus)
                                    .label("New message"),
                            ),
                        ),
                ),
            )
            .child(
                section("Custom Media and Muted Surface").child(
                    Empty::new()
                        .min_h(px(320.))
                        .bg(cx.theme().muted.opacity(0.55))
                        .child(
                            EmptyHeader::new()
                                .child(
                                    EmptyMedia::new().child(
                                        div()
                                            .flex()
                                            .size_12()
                                            .items_center()
                                            .justify_center()
                                            .rounded_full()
                                            .bg(cx.theme().primary)
                                            .text_color(cx.theme().primary_foreground)
                                            .font_medium()
                                            .child("GP"),
                                    ),
                                )
                                .child(EmptyTitle::new().child("Invite a teammate"))
                                .child(EmptyDescription::new().child(
                                    "Custom illustrations, avatars, and avatar groups can be used as media.",
                                )),
                        ),
                ),
            )
            .child(
                section("Input Group").child(
                    Empty::new()
                        .min_h(px(320.))
                        .child(example_header(
                            IconName::Search,
                            "No results found",
                            "Try another search term or clear your filters.",
                        ))
                        .child(
                            EmptyContent::new().child(
                                InputGroup::new("empty-search")
                                    .input(Input::new(&self.search))
                                    .addon(InputGroupAddon::new().child(Icon::new(IconName::Search)))
                                    .addon(
                                        InputGroupAddon::new()
                                            .align(InputGroupAddonAlign::InlineEnd)
                                            .child(
                                                Button::new("clear-empty-search")
                                                    .xsmall()
                                                    .ghost()
                                                    .label("Clear"),
                                            ),
                                    ),
                            ),
                        ),
                ),
            )
    }
}
