use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, Styled, Window, px,
};
use gpui_component::{
    IconName, WindowExt as _,
    button::Button,
    command::{
        Command, CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList,
        CommandSeparator, CommandShortcut, CommandState,
    },
    dock::PanelControl,
    v_flex,
};

use crate::section;

pub struct CommandStory {
    focus_handle: FocusHandle,
    inline: Entity<CommandState>,
    dialog: Entity<CommandState>,
}

impl CommandStory {
    /// Builds the shared command catalog used by inline and dialog examples.
    fn commands(close_dialog: bool) -> CommandList {
        let select = move |label: &'static str| {
            CommandItem::new(label.to_lowercase(), label).on_select(move |window, cx| {
                if close_dialog {
                    window.close_dialog(cx);
                }
            })
        };

        CommandList::new()
            .empty(CommandEmpty::new("No command found."))
            .group(
                CommandGroup::new("suggestions")
                    .heading("Suggestions")
                    .item(select("Calendar").icon(IconName::Calendar).keyword("date"))
                    .item(
                        select("Search Emoji")
                            .icon(IconName::Search)
                            .keyword("reaction"),
                    )
                    .item(select("Calculator").icon(IconName::Star).disabled(true)),
            )
            .separator(CommandSeparator::new())
            .group(
                CommandGroup::new("settings")
                    .heading("Settings")
                    .item(
                        select("Profile")
                            .icon(IconName::User)
                            .shortcut(CommandShortcut::new("⌘P")),
                    )
                    .item(
                        select("Billing")
                            .icon(IconName::Inbox)
                            .shortcut(CommandShortcut::new("⌘B")),
                    )
                    .item(
                        select("Settings")
                            .icon(IconName::Settings2)
                            .shortcut(CommandShortcut::new("⌘S")),
                    ),
            )
    }

    /// Creates Story state with independent inline and modal selection models.
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            inline: cx.new(|cx| CommandState::new(Self::commands(false), window, cx)),
            dialog: cx.new(|cx| CommandState::new(Self::commands(true), window, cx)),
        }
    }

    /// Creates a shared Command Story entity.
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for CommandStory {
    fn title() -> &'static str {
        "Command"
    }

    fn description() -> &'static str {
        "Fast, compositional command menus with filtering and keyboard navigation."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for CommandStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CommandStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .child(
                section("Inline").child(
                    Command::new(&self.inline)
                        .input(CommandInput::new().placeholder("Type a command or search..."))
                        .w(px(420.))
                        .h(px(336.))
                        .border_1(),
                ),
            )
            .child(
                section("Dialog").child(
                    CommandDialog::new(&self.dialog)
                        .trigger(Button::new("open-command").label("Open Command Palette")),
                ),
            )
    }
}
