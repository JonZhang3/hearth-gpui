---
title: Radio Group
description: A set of mutually exclusive options aligned with shadcn Radio Group.
---

# Radio Group

`RadioGroup` owns a controlled value and coordinates selection, focus, keyboard navigation,
disabled state, and accessibility for typed `RadioGroupItem` children. `Radio` remains available
for standalone controlled rendering, but mutually exclusive choices should use `RadioGroup`.

## Import

```rust
use gpui_component::radio::{Radio, RadioGroup, RadioGroupItem};
```

## Basic usage

```rust
struct SettingsView {
    density: Option<SharedString>,
}

RadioGroup::new("density")
    .aria_label("Density")
    .value(self.density.clone())
    .child(RadioGroupItem::new("default").label("Default"))
    .child(RadioGroupItem::new("comfortable").label("Comfortable"))
    .child(RadioGroupItem::new("compact").label("Compact"))
    .on_change(cx.listener(|this, value: &SharedString, _, cx| {
        this.density = Some(value.clone());
        cx.notify();
    }))
```

The value is stable across item reordering. Selecting the current item again does not clear it.

## Orientation

```rust
RadioGroup::horizontal("language")
    .aria_label("Language")
    .value(Some("rust"))
    .children([
        RadioGroupItem::new("rust").label("Rust"),
        RadioGroupItem::new("go").label("Go"),
        RadioGroupItem::new("swift").label("Swift"),
    ])
```

`RadioGroup::new` and `RadioGroup::vertical` use vertical orientation. Orientation controls both
layout and arrow-key behavior.

## Labels and descriptions

```rust
RadioGroup::vertical("plan")
    .aria_label("Plan")
    .value(Some("pro"))
    .child(
        RadioGroupItem::new("plus")
            .label("Plus")
            .aria_description("For individuals and small teams")
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child("For individuals and small teams"),
            ),
    )
    .child(
        RadioGroupItem::new("pro")
            .label("Pro")
            .aria_description("For growing businesses")
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child("For growing businesses"),
            ),
    )
```

The integrated label is the item's accessible name. Supplemental content should also provide an
explicit `aria_description` when it conveys information required to choose an option.

## Disabled and invalid states

```rust
RadioGroup::vertical("notifications")
    .aria_label("Notifications")
    .value(Some("email"))
    .child(RadioGroupItem::new("email").label("Email"))
    .child(RadioGroupItem::new("sms").label("SMS").disabled(true))
    .child(
        RadioGroupItem::new("push")
            .label("Push")
            .invalid(true),
    )
```

Group-level `disabled(true)` is combined with each item's own disabled state and does not mutate
the item permanently.

## Standalone Radio

```rust
Radio::new("standalone-radio")
    .label("Standalone option")
    .checked(self.checked)
    .on_click(cx.listener(|this, checked: &bool, _, cx| {
        this.checked = *checked;
        cx.notify();
    }))
```

Radio activation only requests `true`; activating an already selected Radio does not unselect it.

## Keyboard behavior

| Key | Behavior |
|---|---|
| `Tab` / `Shift+Tab` | Enters or leaves the group through its selected item, or the first enabled item |
| `ArrowLeft` / `ArrowRight` | Moves and selects within a horizontal group, wrapping at the ends |
| `ArrowUp` / `ArrowDown` | Moves and selects within a vertical group, wrapping at the ends |
| `Home` / `End` | Selects the first or last enabled item |
| `Space` | Selects the focused item |

Disabled items are skipped. Pointer focus does not display the keyboard-only focus ring.

## Visual and motion behavior

- Default geometry follows Vega: a 16 px circular control and an 8 px selected indicator.
- Vega and Maia groups use a 12 px gap; compact Nova groups use an 8 px gap.
- Light-mode unchecked controls are transparent; dark mode uses the semantic input surface.
- Checked, unchecked, invalid, and focus paint changes are immediate. The pinned shadcn source
  does not define an indicator or color transition for Radio Group.
- `Sizable` remains a GPUI Component extension for exceptional compact or large compositions;
  default size is the shadcn acceptance baseline.

## API

### RadioGroup

| Method | Description |
|---|---|
| `new(id)` | Creates a vertical controlled group |
| `horizontal(id)` / `vertical(id)` | Creates a group with explicit orientation |
| `orientation(Axis)` | Changes layout and arrow-key navigation axis |
| `value(Option<T>)` | Sets the controlled selected value |
| `aria_label(text)` | Sets the accessible group name |
| `child(item)` / `children(items)` | Adds typed value-bearing items |
| `disabled(bool)` | Disables all items at render time |
| `on_change(fn)` | Reports a newly selected stable value |

### RadioGroupItem

| Method | Description |
|---|---|
| `new(value)` | Creates an item with a stable selection value and default ID |
| `label(text)` | Sets visible label and accessible name |
| `aria_label(text)` | Sets an accessible name without visible text |
| `aria_description(text)` | Sets supplemental accessible description |
| `disabled(bool)` | Disables this item |
| `invalid(bool)` | Applies invalid semantics and visual state |
| `tooltip(text)` | Adds a tooltip |

`Radio`, `RadioGroupItem`, and `RadioGroup` implement `Styled`. `Radio` and `RadioGroupItem` also
implement `Sizable`.
