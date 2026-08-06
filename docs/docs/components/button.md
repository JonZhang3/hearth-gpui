---
title: Button
description: Displays an action using the shadcn Vega visual baseline.
---

# Button

`Button` follows the shadcn Vega baseline for geometry, variants, interaction states, and composition.

## Variants

```rust
Button::new("default").label("Default");
Button::new("outline").outline().label("Outline");
Button::new("secondary").secondary().label("Secondary");
Button::new("ghost").ghost().label("Ghost");
Button::new("destructive").destructive().label("Destructive");
Button::new("link").link().label("Link");
```

`Default` is the primary action style. Navigation should use the `Link` component instead of changing a button's accessibility role.

Persistent toggle actions can use `.pressed(bool)`. This keeps Button keyboard behavior and exposes the state as `aria-pressed`; use `Toggle` or `ToggleGroup` for ordinary option sets.

## Sizes

```rust
Button::new("xs").xsmall().label("Extra Small");
Button::new("sm").small().label("Small");
Button::new("md").label("Default");
Button::new("lg").large().label("Large");
```

Icon-only buttons use the same height and width. Always provide `aria_label`.

```rust
Button::new("move-up")
    .outline()
    .icon(IconName::ArrowUp)
    .aria_label("Move up");
```

## Icons and loading

Use `icon` for the leading slot and `trailing_icon` for the trailing slot. Loading is explicit composition with `Spinner`; disable the action while work is pending.

```rust
Button::new("branch")
    .outline()
    .icon(IconName::Github)
    .label("New Branch");

Button::new("generating")
    .outline()
    .icon(Spinner::new())
    .label("Generating")
    .disabled(true);
```

## Rounded

`rounded_full` derives a pill radius from the final control height. `rounded(px(...))` applies an explicit override.

```rust
Button::new("round")
    .outline()
    .rounded_full()
    .icon(IconName::ArrowUp)
    .aria_label("Move up");
```

## Button group

`ButtonGroup` composes actions and preserves each button's callback. It supports nested groups, text, separators, orientation, and an accessible group label. Selection belongs to `Toggle` or `ToggleGroup`.

```rust
ButtonGroup::new("message-actions")
    .aria_label("Message actions")
    .child(Button::new("back").outline().icon(IconName::ArrowLeft).aria_label("Back"))
    .group(
        ButtonGroup::new("archive-report")
            .child(Button::new("archive").outline().label("Archive"))
            .child(Button::new("report").outline().label("Report")),
    )
    .separator(ButtonGroupSeparator::new())
    .text(ButtonGroupText::new("More"));
```
