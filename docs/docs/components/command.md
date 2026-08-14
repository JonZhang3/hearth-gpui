---
title: Command
description: A searchable command menu with grouped actions and keyboard navigation.
---

# Command

`Command` provides a GPUI-native command palette. It keeps filtering, active selection, checked state, and activation separate while reusing the virtualized List keyboard and accessibility contracts.

## Import

```rust
use gpui_component::command::{
    Command, CommandDialog, CommandEmpty, CommandGroup, CommandInput,
    CommandItem, CommandList, CommandSeparator, CommandShortcut, CommandState,
};
```

## Create state

```rust
let commands = CommandList::new()
    .empty(CommandEmpty::new("No command found."))
    .group(
        CommandGroup::new("suggestions")
            .heading("Suggestions")
            .item(
                CommandItem::new("calendar", "Calendar")
                    .icon(IconName::Calendar)
                    .keyword("date"),
            ),
    )
    .separator(CommandSeparator::new())
    .group(
        CommandGroup::new("settings")
            .heading("Settings")
            .item(
                CommandItem::new("profile", "Profile")
                    .shortcut(CommandShortcut::new("⌘P")),
            ),
    );

let command = cx.new(|cx| CommandState::new(commands, window, cx));
```

## Inline command

```rust
Command::new(&command)
    .input(CommandInput::new().placeholder("Type a command or search..."))
    .w(px(420.))
```

`CommandList` grows with the filtered groups and items up to the shadcn-aligned
`288px` maximum. Searching for fewer items therefore reduces the Command surface
height. Apply `.h(...)` only when a deliberately fixed-height surface is required.

## Command dialog

```rust
CommandDialog::new(&command)
    .trigger(Button::new("open-command").label("Open Command Palette"))
```

Use `CommandDialog::open` for programmatic presentation. Its semantic title and description are hidden visually, the close button is disabled by default, and modal lifecycle motion comes from the existing Dialog component.

## Interaction and filtering

- Filtering is case-insensitive substring matching over each item's label and keywords. Original group and item order is preserved; no fuzzy ranking dependency is added.
- Up/Down, Home/End, Enter, and Escape follow the existing GPUI List keyboard contract. Disabled commands are skipped.
- `checked` is independent from the active keyboard item. A shortcut replaces the trailing check indicator, matching shadcn composition.
- Command itself defines no enter, exit, or color transition because the pinned shadcn Command styles declare none.
- Vega is the default visual baseline. Nova and Maia resolve density, spacing, and radius through semantic Style Preset data without preset-ID branching.

## Events

Subscribe to `CommandEvent::Select(id)` for confirmed commands and `CommandEvent::Cancel` for Escape. Item-level `on_select` handlers are also available for direct actions.
