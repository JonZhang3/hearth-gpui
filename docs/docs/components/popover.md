---
title: Popover
description: A floating dialog surface positioned relative to a trigger.
---

# Popover

Popover displays rich interactive content next to a trigger. The default surface follows the active Color Theme and Style Preset; Vega is the default visual baseline.

## Import

```rust
use gpui_component::popover::{
    Popover, PopoverAlign, PopoverDescription, PopoverHeader, PopoverSide, PopoverTitle,
    PopoverTrigger,
};
```

## Basic usage

The trigger must implement `PopoverTrigger`, which lets Popover place `aria-expanded` on the trigger's own accessibility node. `Button` and the built-in ColorPicker trigger provide this capability. Content can be added with normal `ParentElement` composition or the dynamic `content` callback.

```rust
use gpui::ParentElement as _;
use gpui_component::{
    button::Button,
    popover::{Popover, PopoverDescription, PopoverHeader, PopoverTitle},
};

Popover::new("profile-popover")
    .trigger(Button::new("profile-trigger").outline().label("Open profile"))
    .aria_label("Profile information")
    .child(
        PopoverHeader::new()
            .child(PopoverTitle::new().child("Profile"))
            .child(PopoverDescription::new().child(
                "Review the account details associated with this profile.",
            )),
    )
```

The standard surface is 288 px wide and includes preset-owned padding, gap, radius, ring, shadow, and typography. Caller `Styled` refinements are applied after these defaults.

## Placement

`side` selects the physical side of the trigger. `align` controls the cross-axis alignment. The default is `Bottom` plus `Center`, with a 4 px side offset.

```rust
Popover::new("placement-popover")
    .side(PopoverSide::Top)
    .align(PopoverAlign::End)
    .side_offset(px(8.))
    .align_offset(px(4.))
    .trigger(Button::new("placement-trigger").outline().label("Placement"))
    .child("Top, end-aligned content")
```

The legacy `.anchor(Anchor)` builder remains available and maps to the equivalent side/alignment pair. GPUI shifts content to remain inside the window, but does not automatically flip it to the opposite side.

## Dynamic content and manual dismissal

`content` receives `PopoverState`, `Window`, and `Context<PopoverState>`. Emit `DismissEvent` to close from inside the content.

```rust
Popover::new("dynamic-popover")
    .trigger(Button::new("dynamic-trigger").outline().label("Open"))
    .content(|_, _, cx| {
        Button::new("close-popover")
            .label("Close")
            .on_click(cx.listener(|_, _, _, cx| cx.emit(DismissEvent)))
    })
```

Avoid creating entities or performing expensive work inside `content`, because it can run on every Popover render.

## Controlled state

```rust
Popover::new("controlled-popover")
    .open(self.popover_open)
    .on_open_change(cx.listener(|this, open: &bool, _, cx| {
        this.popover_open = *open;
        cx.notify();
    }))
    .trigger(Button::new("controlled-trigger").outline().label("Controlled"))
    .child("Controlled content")
```

Use `default_open(true)` only for the initial uncontrolled state. Opening registers the overlay lifecycle and moves focus into the Popover; Escape, focus leaving the content, or an outside pointer press dismisses it and restores the previous focus. Set `overlay_closable(false)` when an owning component manages dismissal itself.

## Custom appearance and trigger methods

`appearance(false)` disables only the standard surface styling. Positioning, lifecycle, focus, and dismissal APIs remain available. `mouse_button(MouseButton::Right)` can be used for a custom right-click surface.

```rust
Popover::new("custom-popover")
    .appearance(false)
    .mouse_button(MouseButton::Right)
    .trigger(Button::new("custom-trigger").label("Right click"))
    .bg(cx.theme().accent)
    .text_color(cx.theme().accent_foreground)
    .p_3()
    .rounded_lg()
    .child("Custom content")
```

The enter and exit transitions use the active semantic motion duration and easing with an 8 px placement-aware slide. Opacity animation is intentionally omitted so the complete Popover surface remains visually stable. GPUI currently has no element transform primitive for exact `zoom-in-95` / `zoom-out-95` parity.
