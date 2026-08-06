---
title: Accordion
description: A value-driven disclosure group with controlled and uncontrolled state.
---

# Accordion

Accordion displays vertically stacked triggers that reveal content. Every item has a stable value, so state remains meaningful when items are reordered.

The active Style Preset owns spacing and appearance. Vega and Nova render a plain divided list, while Maia renders a unified rounded frame with a muted open state. Use `framed()` only when a screen must override the preset.

## Import

```rust
use gpui_component::accordion::Accordion;
```

## Uncontrolled single Accordion

```rust
Accordion::single("shipping-faq")
    .default_open_values(["shipping"])
    .item("shipping", |item| {
        item.title("What are your shipping options?")
            .child("Standard, express, and overnight shipping are available.")
    })
    .item("returns", |item| {
        item.title("What is your return policy?")
            .child("Returns are accepted within 30 days.")
    })
```

`default_open_values()` initializes internal state once. User interaction owns subsequent changes.

## Controlled Accordion

```rust
Accordion::single("shipping-faq")
    .open_values(open_values.clone())
    .on_open_change(cx.listener(|this, values, _, cx| {
        this.open_values = values.to_vec();
        cx.notify();
    }))
    .item("shipping", |item| item.title("Shipping").child("Shipping details"))
    .item("returns", |item| item.title("Returns").child("Return details"))
```

`open_values()` is authoritative in controlled mode. `on_open_change()` reports proposed values in item declaration order.

## Multiple items

```rust
Accordion::multiple("settings")
    .default_open_values(["general", "advanced"])
    .item("general", |item| item.title("General").child("General settings"))
    .item("advanced", |item| item.title("Advanced").child("Advanced settings"))
```

## Non-collapsible single item

```rust
Accordion::single("required-section")
    .collapsible(false)
    .default_open_values(["details"])
    .item("details", |item| item.title("Details").child("Required details"))
```

## Appearance and disabled state

```rust
Accordion::single("framed-faq")
    .framed(true)
    .item("enabled", |item| item.title("Enabled").child("Content"))
    .item("disabled", |item| {
        item.disabled(true)
            .title("Disabled")
            .child("Unavailable content")
    })
```

`disabled(true)` on the Accordion disables the whole group. Item and group disabled states are combined.

## Keyboard behavior

| Key | Behavior |
|---|---|
| `Enter` / `Space` | Toggle the focused item |
| `ArrowDown` / `ArrowUp` | Move focus between enabled triggers and wrap |
| `Home` / `End` | Move focus to the first or last enabled trigger |

Triggers expose button role, expanded state, and disabled state to AccessKit. Use `aria_label()` when a custom title does not provide a reliable accessible name.

## API Reference

- `Accordion::single(id)` creates a single-selection group.
- `Accordion::multiple(id)` creates a multiple-selection group.
- `collapsible(bool)` controls whether an open single item may close.
- `default_open_values(values)` initializes uncontrolled state.
- `open_values(values)` supplies controlled state.
- `on_open_change(callback)` receives proposed stable values.
- `framed(bool)` overrides the Style Preset frame policy.
- `disabled(bool)` disables the group.
- `item(value, builder)` adds an item with a required stable value.

[Accordion]: https://docs.rs/gpui-component/latest/gpui_component/accordion/struct.Accordion.html
[AccordionItem]: https://docs.rs/gpui-component/latest/gpui_component/accordion/struct.AccordionItem.html
