---
title: DropdownButton
description: A DropdownButton is a combination of a button and a trigger button. It allows us to display a dropdown menu when the trigger is clicked, but the left Button can still respond to independent events.
---

# DropdownButton

A [DropdownButton] combines a primary action button with an attached menu trigger. The primary
button preserves its own click handler, while the trailing segment opens a [PopupMenu].

The component forwards its variant, size, disabled, selected, and radius configuration to both
segments. Geometry, colors, focus rings, elevation, and density therefore follow the active Color
Theme and Style Preset through [Button] instead of duplicating fixed values.

## Import

```rust
use hearth_gpui::button::{Button, DropdownButton};
```

## Usage

```rust
use gpui::Anchor;

DropdownButton::new("dropdown")
    .aria_label("Document actions")
    .menu_aria_label("Open document options")
    .button(
        Button::new("dropdown-primary")
            .label("Save")
            .on_click(|_, _, _| {
                // Perform the primary action.
            }),
    )
    .dropdown_menu(|menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
            .menu("Option 2", Box::new(MyAction))
            .separator()
            .menu("Option 3", Box::new(MyAction))
    })
```

### Variants

Same as [Button], DropdownButton supports different variants.

```rust
DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Default"))
    .dropdown_menu(|menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

### With custom anchor

```rust
// With custom anchor
DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Click Me"))
    .dropdown_menu_with_anchor(Anchor::BottomRight, |menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

### Accessibility

Use `.aria_label(...)` to name the composite group when its surrounding context is insufficient.
The menu trigger has a localized "More options" accessible name by default; use
`.menu_aria_label(...)` when the menu has a more specific purpose.

[Button]: https://docs.rs/hearth-gpui/latest/hearth_gpui/button/struct.Button.html
[DropdownButton]: https://docs.rs/hearth-gpui/latest/hearth_gpui/button/struct.DropdownButton.html
[PopupMenu]: https://docs.rs/hearth-gpui/latest/hearth_gpui/menu/struct.PopupMenu.html
[Sizable]: https://docs.rs/hearth-gpui/latest/hearth_gpui/trait.Sizable.html
