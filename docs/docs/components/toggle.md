---
title: Toggle
description: A two-state button and composable single or multiple selection group.
---

# Toggle

`Toggle` is a controlled button that exposes its state through `aria-pressed`. It supports the
shadcn default and outline variants, semantic sizes, invalid and disabled states, keyboard focus,
and interruptible color and focus-ring transitions.

## Basic usage

```rust
Toggle::new("bold")
    .icon(IconName::Check)
    .label("Bold")
    .checked(self.bold)
    .on_click(cx.listener(|this, checked, _, cx| {
        this.bold = *checked;
        cx.notify();
    }))
```

Use `.aria_label(...)` for an icon-only Toggle. A tooltip is also used as the fallback accessible
name.

```rust
Toggle::new("preview")
    .icon(IconName::Eye)
    .aria_label("Toggle preview")
```

## Variants and sizes

```rust
Toggle::new("default").label("Default");
Toggle::new("outline").outline().label("Outline");

Toggle::new("small").small().label("Small");
Toggle::new("medium").label("Default");
Toggle::new("large").large().label("Large");
```

`XSmall` remains a Hearth GPUI extension and is not part of the shadcn Toggle API.

## States

```rust
Toggle::new("selected").label("Selected").checked(true);
Toggle::new("invalid").label("Invalid").invalid(true);
Toggle::new("disabled").label("Disabled").disabled(true);
Toggle::new("out-of-tab-order").label("Action").tab_stop(false);
```

Enter and Space use native button activation. Pointer focus does not draw a keyboard focus ring.

## Leading and trailing icons

```rust
Toggle::new("options")
    .icon(IconName::Star)
    .label("Options")
    .trailing_icon(IconName::ChevronDown)
```

Typed icon slots allow the active Style Preset to resolve icon size and side-specific padding.

## ToggleGroup

`ToggleGroup` owns selection and contains typed `ToggleGroupItem` children. Item values are stable
strings rather than positional boolean indexes.

### Single selection

```rust
ToggleGroup::new("alignment")
    .mode(ToggleGroupMode::Single)
    .selection(ToggleGroupSelection::Single(self.alignment.clone()))
    .aria_label("Text alignment")
    .child(ToggleGroupItem::new("left").label("Left"))
    .child(ToggleGroupItem::new("center").label("Center"))
    .child(ToggleGroupItem::new("right").label("Right"))
    .on_change(cx.listener(|this, selection, _, cx| {
        if let ToggleGroupSelection::Single(value) = selection {
            this.alignment = value.clone();
            cx.notify();
        }
    }))
```

Selecting the active item clears a single-selection group.

### Multiple selection and connected layout

```rust
ToggleGroup::new("formatting")
    .mode(ToggleGroupMode::Multiple)
    .selection(ToggleGroupSelection::Multiple(self.formats.clone()))
    .outline()
    .spacing(px(0.))
    .aria_label("Text formatting")
    .child(
        ToggleGroupItem::new("bold")
            .icon(IconName::Check)
            .aria_label("Bold"),
    )
    .child(
        ToggleGroupItem::new("preview")
            .icon(IconName::Eye)
            .aria_label("Preview"),
    )
    .on_change(cx.listener(|this, selection, _, cx| {
        if let ToggleGroupSelection::Multiple(values) = selection {
            this.formats = values.clone();
            cx.notify();
        }
    }))
```

The default spacing is 8px, matching shadcn `spacing={2}`. `spacing(px(0.))` joins adjacent borders
and assigns axis-aware first and last corner radii.

### Vertical orientation

```rust
ToggleGroup::new("vertical-tools")
    .orientation(Axis::Vertical)
    .spacing(px(0.))
    .aria_label("Vertical tools")
    .child(ToggleGroupItem::new("one").label("One"))
    .child(ToggleGroupItem::new("two").label("Two"))
```

Horizontal groups support Left/Right; vertical groups support Up/Down. Both support Home and End,
skip disabled items, and expose one Tab entry point.

## Migration from the positional group API

Replace child `Toggle::checked(...)` values and `on_click(&Vec<bool>)` with stable
`ToggleGroupItem::new(value)`, `ToggleGroupSelection`, and `on_change(...)`. Replace `.segmented()`
with `.spacing(px(0.))`.

## Motion

Toggle transitions only border and focus/invalid ring paint. Checked and hover backgrounds change
immediately. It does not animate position, scale, opacity, icons, or child mounting. Rapid focus or
invalid-state reversals continue from the current visible value; reduced motion reaches the target
immediately.
