---
title: Stepper
description: A step-by-step progress for users to navigate through a series of steps or stages.
---

# Stepper

A step-by-step progress component that guides users through a series of steps or stages. It supports horizontal and vertical layouts, custom icons, semantic sizes, read-only progress, and interactive navigation.

## Import

```rust
use gpui_component::stepper::{Stepper, StepperItem};
```

## Usage

### Basic Stepper

Use `selected_index` method to set current active step by index (0-based), default is `0`.

```rust
Stepper::new("my-stepper")
    .selected_index(0)
    .items([
        StepperItem::new().label("Step 1"),
        StepperItem::new().label("Step 2"),
        StepperItem::new().label("Step 3"),
    ])
    .on_click(|step, _, _| {
        println!("Clicked step: {}", step);
    })
```

Without `on_click`, Stepper is a read-only progress indicator and does not add its steps to the Tab order. Adding `on_click` enables pointer, Enter, and Space activation for enabled steps.

### With Icons

```rust
use gpui_component::IconName;

Stepper::new("icon-stepper")
    .selected_index(0)
    .items([
        StepperItem::new()
            .icon(IconName::Calendar)
            .child("Order Details"),
        StepperItem::new()
            .icon(IconName::Inbox)
            .child("Shipping"),
        StepperItem::new()
            .icon(IconName::Frame)
            .child("Preview"),
        StepperItem::new()
            .icon(IconName::Info)
            .child("Finish"),
    ])
```

### Vertical Layout

```rust
Stepper::new("vertical-stepper")
    .vertical()
    .selected_index(2)
    .items_center()
    .items([
        StepperItem::new()
            .pb_8()
            .icon(IconName::Building2)
            .child(v_flex().child("Step 1").child("Description for step 1.")),
        StepperItem::new()
            .pb_8()
            .icon(IconName::Asterisk)
            .child(v_flex().child("Step 2").child("Description for step 2.")),
        StepperItem::new()
            .pb_8()
            .icon(IconName::Folder)
            .child(v_flex().child("Step 3").child("Description for step 3.")),
        StepperItem::new()
            .icon(IconName::CircleCheck)
            .child(v_flex().child("Step 4").child("Description for step 4.")),
    ])
```

### Text Center

The `text_center` method centers the text within each step item.

```rust
Stepper::new("center-stepper")
    .selected_index(0)
    .text_center(true)
    .items([
        StepperItem::new().child(
            v_flex()
                .items_center()
                .child("Step 1")
                .child("Desc for step 1."),
        ),
        StepperItem::new().child(
            v_flex()
                .items_center()
                .child("Step 2")
                .child("Desc for step 2."),
        ),
        StepperItem::new().child(
            v_flex()
                .items_center()
                .child("Step 3")
                .child("Desc for step 3."),
        ),
    ])
```

### Different Sizes

```rust
use gpui_component::{Sizable as _, Size};

Stepper::new("stepper")
    .xsmall()
    .items([...])

Stepper::new("stepper")
    .small()
    .items([...])

Stepper::new("stepper")
    .large()
    .items([...])
```

### Disabled State

```rust
Stepper::new("disabled-stepper")
    .disabled(true)
    .items([
        StepperItem::new().child("Step 1"),
        StepperItem::new().child("Step 2"),
    ])
```

### Accessibility and Keyboard Behavior

- The root exposes a named step list. Use `aria_label()` when the surrounding context does not provide a sufficient name.
- `StepperItem::label()` provides both visible text and the accessible step name. Use `aria_label()` for custom composed content.
- The active item exposes `aria-current="step"`; every item reports its position and total set size.
- Interactive enabled steps are reachable with Tab and activate with Enter or Space. Held Space events do not repeat activation.
- Disabled steps are announced as disabled and cannot be focused or activated.
- Out-of-range `selected_index()` values resolve to the final available step. An empty Stepper has no active step.

Stepper does not animate state changes. Color and geometry update atomically, while sizes, gaps, and connector thickness are resolved from the active Style Preset.

### Handle Click Events

```rust
Stepper::new("my-stepper")
    .selected_index(current_step)
    .items([
        StepperItem::new().child("Step 1"),
        StepperItem::new().child("Step 2"),
        StepperItem::new().child("Step 3"),
    ])
    .on_click(cx.listener(|this, step, _, cx| {
        this.current_step = *step;
        cx.notify();
    }))
```

## API Reference

- [Stepper]
- [StepperItem]

### Sizing

Implements [Sizable] trait:

- `xsmall()` - Extra small size
- `small()` - Small size
- `medium()` - Medium size (default)
- `large()` - Large size

### Methods

- `aria_label(label)` - Set the accessible name of the step list.
- `selected_index(index)` - Set the zero-based current step; out-of-range values resolve to the final item.
- `layout(axis)` / `vertical()` - Set horizontal or vertical layout.
- `text_center(bool)` - Center horizontal item content.
- `disabled(bool)` - Disable every step.
- `on_click(handler)` - Enable navigation and receive the activated zero-based step index.

`StepperItem` supports `label()`, `aria_label()`, `icon()`, `disabled()`, custom children, sizing, and styling.

## Examples

### Multi-step Form

```rust
Stepper::new("form-stepper")
    .w_full()
    .selected_index(form_step)
    .items([
        StepperItem::new()
            .icon(IconName::User)
            .child("Personal Info"),
        StepperItem::new()
            .icon(IconName::CreditCard)
            .child("Payment"),
        StepperItem::new()
            .icon(IconName::CircleCheck)
            .child("Confirmation"),
    ])
    .on_click(cx.listener(|this, step, _, cx| {
        this.form_step = *step;
        cx.notify();
    }))
```

### Disabled Individual Steps

```rust
Stepper::new("stepper")
    .selected_index(0)
    .items([
        StepperItem::new().child("Available"),
        StepperItem::new().disabled(true).child("Locked"),
        StepperItem::new().child("Available"),
    ])
```

[Stepper]: https://docs.rs/gpui-component/latest/gpui_component/stepper/struct.Stepper.html
[StepperItem]: https://docs.rs/gpui-component/latest/gpui_component/stepper/struct.StepperItem.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
