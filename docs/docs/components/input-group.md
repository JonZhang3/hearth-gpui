---
title: Input Group
description: Compose an Input with inline or block addons, helper text, and actions.
---

# Input Group

Input Group presents one `Input` and its related content as a single control surface. The root API uses typed slots: add the control with `input(...)` and surrounding content with `addon(...)`.

## Import

```rust
use hearth_gpui::input::{
    Input, InputGroup, InputGroupAddon, InputGroupAddonAlign,
    InputGroupButton, InputGroupButtonSize, InputGroupText, InputState,
};
```

## Inline addons

```rust
let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));

InputGroup::new("search-group")
    .input(Input::new(&search))
    .addon(
        InputGroupAddon::new()
            .child(Icon::new(IconName::Search)),
    )
    .addon(
        InputGroupAddon::new()
            .align(InputGroupAddonAlign::InlineEnd)
            .child(InputGroupText::new().child("12 results")),
    )
```

Clicking a non-interactive addon focuses the associated Input. Interactive children such as `InputGroupButton` preserve their own pointer behavior.

## Action button

```rust
InputGroup::new("website-group")
    .input(Input::new(&website))
    .addon(InputGroupAddon::new().child(
        InputGroupText::new().child("https://"),
    ))
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
    )
```

`InputGroupButton` adapts an existing `Button`; event handlers, disabled state, and accessibility metadata remain Button responsibilities. It defaults to the shadcn Ghost variant and implements `ButtonVariants` for explicit variant selection.

## Block addons and multiline input

```rust
let message = cx.new(|cx| {
    InputState::new(window, cx)
        .auto_grow(3, 8)
        .placeholder("Ask, search, or chat...")
});

InputGroup::new("message-group")
    .input(Input::new(&message))
    .addon(
        InputGroupAddon::new()
            .align(InputGroupAddonAlign::BlockStart)
            .child(InputGroupText::new().child("message.txt")),
    )
    .addon(
        InputGroupAddon::new()
            .align(InputGroupAddonAlign::BlockEnd)
            .child(InputGroupText::new().child("Markdown supported")),
    )
```

The same `Input` API supports single-line, multiline, and auto-growing editor states. Input Group does not introduce a second editor implementation.

## States

```rust
InputGroup::new("disabled-group")
    .input(Input::new(&input))
    .disabled(true)

InputGroup::new("invalid-group")
    .input(Input::new(&input))
    .invalid(true)
    .aria_label("Invalid email")
```

Focus, invalid, disabled, border, background, shadow, and motion are owned by the outer surface. State is propagated to the contained Input, so the group does not render nested borders or focus rings.

## Composition contract

| Component | Purpose |
| --- | --- |
| `InputGroup` | Owns the unified surface and exactly one Input slot |
| `InputGroupAddon` | Places composable content at an inline or block edge |
| `InputGroupText` | Applies muted helper-text styling inside an addon |
| `InputGroupButton` | Adapts the existing Button to compact group geometry |

The root intentionally does not accept arbitrary children. This keeps focus, invalid, disabled, and accessibility ownership explicit while addon content remains composable through the repository-standard `ParentElement` API.

## Style Presets and motion

Vega is the default baseline. Nova and Maia geometry is resolved from semantic Style Preset metrics without branching on preset identifiers. Surface transitions reuse Input motion tokens, support interruption from the current interpolated value, and respect reduced-motion settings.
