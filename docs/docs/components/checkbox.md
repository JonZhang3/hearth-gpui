---
title: Checkbox
description: A control that allows the user to toggle between checked and not checked.
---

# Checkbox

A shadcn-aligned checkbox for binary and mixed selection. The default size follows Vega, while GPUI-specific size variants remain available.

## Import

```rust
use gpui_component::checkbox::Checkbox;
```

## Usage

### Basic Checkbox

```rust
Checkbox::new("my-checkbox")
    .label("Accept terms and conditions")
    .checked(false)
    .on_click(|checked, _, _| {
        println!("Checkbox is now: {}", checked);
    })
```

The `on_click` callback is triggered when the user toggles the checkbox, receiving the **new checked state**.

### Controlled Checkbox

```rust
struct MyView {
    is_checked: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Checkbox::new("checkbox")
            .label("Option")
            .checked(self.is_checked)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.is_checked = *checked;
                cx.notify();
            }))
    }
}
```

### Different Sizes

```rust
Checkbox::new("cb-xs").xsmall().label("Extra Small")
Checkbox::new("cb-sm").small().label("Small")
Checkbox::new("cb").label("Medium") // default
Checkbox::new("cb-lg").large().label("Large")
```

The default Checkbox is 16 px with a 14 px indicator, matching shadcn. The additional sizes are a GPUI Component extension.

### Disabled State

```rust
Checkbox::new("checkbox")
    .label("Disabled checkbox")
    .disabled(true)
    .checked(false)
```

### Without Label

```rust
Checkbox::new("checkbox")
    .aria_label("Toggle standalone option")
    .checked(true)
```

Use `aria_label(...)` whenever a Checkbox has no visible label.

### Custom Tab Order

```rust
Checkbox::new("checkbox")
    .label("Custom tab order")
    .tab_index(2)
    .tab_stop(true)
```

## API Reference

- [Checkbox]

### Styling

Implements `Sizable` and `Disableable` traits:

- `xsmall()` - Extra small Checkbox
- `small()` - Small Checkbox
- `large()` - Large Checkbox
- Omitting a size modifier uses the default medium Checkbox
- `disabled(bool)` - Disabled state

## Examples

### Checkbox List

```rust
v_flex()
    .gap_2()
    .child(Checkbox::new("cb1").label("Option 1").checked(true))
    .child(Checkbox::new("cb2").label("Option 2").checked(false))
    .child(Checkbox::new("cb3").label("Option 3").checked(false))
```

### Form Integration

```rust
struct FormView {
    agree_terms: bool,
    subscribe: bool,
}

v_flex()
    .gap_3()
    .child(
        Checkbox::new("terms")
            .label("I agree to the terms and conditions")
            .checked(self.agree_terms)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.agree_terms = *checked;
                cx.notify();
            }))
    )
    .child(
        Checkbox::new("subscribe")
            .label("Subscribe to newsletter")
            .checked(self.subscribe)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.subscribe = *checked;
                cx.notify();
            }))
    )
```

## Indeterminate and invalid states

```rust
Checkbox::new("partial").label("Some items selected").indeterminate(true)
Checkbox::new("invalid").label("Accept the required terms").invalid(true)
```

`indeterminate(true)` takes visual and accessibility precedence over `checked`, maps to AccessKit `Toggled::Mixed`, and produces a checked value when activated. `invalid(true)` applies the semantic danger border and focus ring and maps to AccessKit `Invalid::True`.

## Keyboard and focus

- `Tab` focuses enabled checkboxes.
- `Space` toggles the focused checkbox.
- Disabled checkboxes are excluded from the tab order.
- Focus and invalid rings are drawn around the Checkbox control, not the label row.

The default Vega appearance uses a 4 px radius and subtle elevation. Nova keeps the 4 px radius without elevation, while Maia uses a 6 px radius. Colors continue to come from the active Color Theme.

Motion also follows the active Style Preset. Vega and Maia transition the focus or invalid ring, while Nova transitions the control colors. The indicator itself changes immediately, matching shadcn `transition-none`. Reduced Motion renders the final state without a transition.

[Checkbox]: https://docs.rs/gpui-component/latest/gpui_component/checkbox/struct.Checkbox.html
