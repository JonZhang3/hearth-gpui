---
title: Separator
description: Visually separates content in horizontal or vertical layouts.
---

# Separator

Separator renders a one-pixel line using the active Color Theme's semantic `border` color. It is
decorative and non-interactive. The default orientation is horizontal.

## Import

```rust
use hearth_gpui::separator::Separator;
```

## Horizontal

```rust
v_flex()
    .gap_4()
    .child("First section")
    .child(Separator::new())
    .child("Second section")
```

`Separator::horizontal()` is an equivalent convenience constructor.

## Vertical

```rust
h_flex()
    .h_5()
    .gap_4()
    .child("Blog")
    .child(Separator::vertical())
    .child("Docs")
    .child(Separator::vertical())
    .child("Source")
```

Orientation can also be selected explicitly with `Separator::new().orientation(Axis::Vertical)`.

## GPUI extensions

Dashed lines, labels, and color overrides are retained as Hearth GPUI extensions:

```rust
Separator::horizontal_dashed()
Separator::horizontal().label("OR")
Separator::vertical().color(cx.theme().danger)
```

The pinned shadcn source declares no transition for Separator, so it does not animate.
