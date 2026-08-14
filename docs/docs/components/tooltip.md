---
title: Tooltip
description: Display compact supplementary information on pointer hover or keyboard focus.
---

# Tooltip

Tooltip presents short supplementary information for a trigger. The application `Root` owns the shared provider and overlay lifecycle; no provider component is required at the call site.

## Import

```rust
use hearth_gpui::tooltip::{
    Tooltip, TooltipAlign, TooltipSide, TooltipTrigger,
};
```

## Built-in component support

Controls such as `Button`, `Checkbox`, `Radio`, `Switch`, and `Toggle` expose a text convenience API:

```rust
Button::new("save")
    .label("Save")
    .tooltip("Save the current document")
```

`Button::tooltip_with_action` adds the platform key binding associated with an action:

```rust
Button::new("save")
    .label("Save")
    .tooltip_with_action("Save document", &SaveDocument, Some("Editor"))
```

## Compositional API

Use `TooltipTrigger` for arbitrary elements, custom placement, or rich content:

```rust
TooltipTrigger::new("project-tooltip")
    .trigger(Button::new("project").label("Project"))
    .text("Open project settings")
    .side(TooltipSide::Right)
    .align(TooltipAlign::Start)
```

### Custom content

```rust
TooltipTrigger::new("status-tooltip")
    .trigger(Button::new("status").label("Status"))
    .content(|window, cx| {
        Tooltip::element(|_, cx| {
            v_flex()
                .gap_1()
                .child(div().font_medium().child("Project status"))
                .child(
                    div()
                        .text_color(cx.theme().background.opacity(0.8))
                        .child("All checks passed"),
                )
        })
        .build(window, cx)
    })
```

### Timing and arrow

```rust
TooltipTrigger::new("instant-tooltip")
    .trigger(Button::new("instant").label("Instant"))
    .text("Opens immediately")
    .show_delay(Duration::ZERO)
    .hide_delay(Duration::from_millis(100))
    .side_offset(px(6.))
    .align_offset(px(4.))
    .show_arrow(false)
```

Pointer hover uses a desktop-oriented 500 ms default delay and a 300 ms shared grace period between nearby tooltips. Keyboard focus opens immediately. Pointer press and Escape dismiss the surface.

## API reference

### `TooltipTrigger`

| Method | Description |
| --- | --- |
| `new(id)` | Creates a trigger with stable state identity |
| `trigger(element)` | Sets the trigger subtree |
| `text(text)` | Sets text content and an accessible description |
| `content(builder)` | Builds custom `Tooltip` content |
| `side(side)` | Uses `Top`, `Right`, `Bottom`, or `Left` |
| `align(align)` | Uses `Start`, `Center`, or `End` cross-axis alignment |
| `side_offset(px)` | Sets trigger-to-surface distance |
| `align_offset(px)` | Offsets the cross-axis alignment |
| `show_delay(duration)` | Sets pointer open delay |
| `hide_delay(duration)` | Sets close delay |
| `show_arrow(bool)` | Shows or hides the placement-aware arrow |
| `arrow_color(color)` | Overrides the semantic arrow color |

### `Tooltip`

| Method | Description |
| --- | --- |
| `new(text)` | Creates text content |
| `element(builder)` | Creates custom element content |
| `action(action, context)` | Resolves and displays an action key binding |
| `key_binding(stroke)` | Displays an explicit platform-aware key binding |
| `build(window, cx)` | Builds the surface as an `AnyView` |

## Visual and motion contract

- Foreground-colored surface with background-colored text
- `text-xs`, 12 px horizontal padding, 6 px vertical padding, 6 px content gap
- Maximum width of 320 px
- No border or shadow
- Density-derived radius for Vega, Nova, and Maia
- Side-aware 8 px translation and opacity transition using semantic `motion.fast` timing
- Exit reverses the entry direction; reduced motion reaches the final state immediately

GPUI currently cannot apply layout-independent scale to an arbitrary element subtree, so shadcn's `zoom-in-95` / `zoom-out-95` portion is intentionally deferred.

## Accessibility

- Text tooltips expose their text as the trigger description.
- Tooltip surfaces use the Tooltip accessibility role.
- The trigger retains its own role, keyboard activation, and focus behavior.
- Tooltip content must remain supplementary; required instructions must also appear in persistent UI.
