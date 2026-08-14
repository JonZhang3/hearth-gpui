---
title: Kbd
description: Displays textual keyboard input and grouped shortcuts.
---

# Kbd

`Kbd` displays a key, shortcut label, icon, or other static keyboard input. `KbdGroup` groups multiple keys without adding interaction or focus behavior.

## Import

```rust
use hearth_gpui::kbd::{Kbd, KbdGroup};
```

## Basic

```rust
use gpui::ParentElement as _;

Kbd::new().child("Ctrl")
Kbd::new().child("⌘K")
Kbd::new().child("Ctrl + B")
```

## Group

```rust
KbdGroup::new()
    .child(Kbd::new().child("Ctrl"))
    .child(Kbd::new().child("Shift"))
    .child(Kbd::new().child("P"))
```

Separators may be composed directly between keys:

```rust
KbdGroup::new()
    .child(Kbd::new().child("Ctrl"))
    .child("+")
    .child(Kbd::new().child("B"))
```

## Platform-aware keystrokes

Use `from_keystroke` when the displayed shortcut must follow platform conventions.

```rust
use gpui::Keystroke;

let stroke = Keystroke::parse("cmd-shift-p").unwrap();
let kbd = Kbd::from_keystroke(stroke.clone());
let kbd: Kbd = stroke.into();
```

On macOS, modifiers use symbols and omit separators. Windows and Linux use textual labels separated by `+`.

| Input | macOS | Windows/Linux |
| --- | --- | --- |
| `cmd-a` | `⌘A` | `Win+A` |
| `ctrl-shift-a` | `⌃⇧A` | `Ctrl+Shift+A` |
| `escape` | `⎋` | `Esc` |
| `enter` | `⏎` | `Enter` |

## Icons and text

Icons should use the 12px `xsmall` size to match the shadcn Kbd baseline.

```rust
use hearth_gpui::{Icon, IconName, Sizable as _};

Kbd::new()
    .child(Icon::new(IconName::ArrowLeft).xsmall())
    .child("Left")
```

Kbd is static content. Do not place buttons or other interactive controls inside it.

## Input Group

```rust
InputGroup::new("search")
    .input(Input::new(&input_state))
    .addon(
        InputGroupAddon::new()
            .align(InputGroupAddonAlign::InlineEnd)
            .child(Kbd::from_keystroke(
                Keystroke::parse("cmd-k").unwrap(),
            )),
    )
```

## Action bindings

```rust
if let Some(kbd) = Kbd::binding_for_action(&MyAction {}, None, window) {
    // Render the resolved platform-aware shortcut.
}

if let Some(kbd) =
    Kbd::binding_for_action_in(&MyAction {}, &focus_handle, window)
{
    // Render the shortcut for the focused context.
}
```

Use `Kbd::format(&stroke)` when only the formatted text is needed.

## Project extensions

The default appearance follows shadcn. Two project-specific opt-ins remain available:

```rust
Kbd::new().child("Outline").outline()
Kbd::new().child("Unstyled").appearance(false)
```

## Styling

The default surface uses semantic theme values and the active Style Preset:

- 20px height and minimum width
- 4px horizontal padding and child gap
- `text-xs` with medium weight
- `muted` background and `muted_foreground` text
- `radii.sm` corner radius
- no transition or animation

`Styled` refinements are applied after the defaults.
