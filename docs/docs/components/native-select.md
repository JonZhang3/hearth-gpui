---
title: Native Select
description: A compact single-value selector backed by the operating system option menu.
---

# Native Select

`NativeSelect` is the GPUI desktop equivalent of shadcn's native `<select>` composition. Its
trigger follows the current Color Theme and Style Preset, while the option menu is rendered by the
operating system on macOS and Windows. On macOS, the selected menu item is anchored to the trigger
using AppKit's native select positioning. Linux uses the existing GPUI PopupMenu fallback.

Use [`Select`](select) when options need search, rich rows, or virtualization.

## Import

```rust
use hearth_gpui::native_select::{
    NativeSelect, NativeSelectOptGroup, NativeSelectOption,
};
```

## Basic usage

```rust
NativeSelect::new("status")
    .value(self.status.clone())
    .aria_label("Status")
    .child(NativeSelectOption::new("", "Select status"))
    .child(NativeSelectOption::new("todo", "Todo"))
    .child(NativeSelectOption::new("in-progress", "In Progress"))
    .child(NativeSelectOption::new("done", "Done"))
    .on_change(cx.listener(|this, value, _, cx| {
        this.status = value.clone();
        cx.notify();
    }))
```

Use `default_value(...)` instead of `value(...)` for an uncontrolled selector.

## Groups and disabled options

```rust
NativeSelect::new("department")
    .child(NativeSelectOption::new("", "Select department"))
    .child(
        NativeSelectOptGroup::new("Engineering")
            .child(NativeSelectOption::new("frontend", "Frontend"))
            .child(NativeSelectOption::new("backend", "Backend"))
            .child(NativeSelectOption::new("devops", "DevOps").disabled(true)),
    )
    .child(
        NativeSelectOptGroup::new("Sales")
            .child(NativeSelectOption::new("sales-rep", "Sales Rep"))
            .child(NativeSelectOption::new("account-manager", "Account Manager")),
    )
```

## Sizes and states

```rust
NativeSelect::new("small").small()
NativeSelect::new("disabled").disabled(true)
NativeSelect::new("invalid").invalid(true)
```

`Tab` focuses the trigger. `Enter` or `Space` opens the native menu. Arrow Up/Down, Home, End, and
printable-character typeahead change the value without opening it and skip disabled options. The
trigger exposes ComboBox value, disabled, and invalid state to AccessKit. Popup appearance,
navigation, and dismissal inside the native menu follow the desktop platform.

## Style Presets

Vega is the default baseline. Nova and Maia resolve their control height, density, radius,
elevation, focus geometry, and motion through semantic Style Metrics; the component does not branch
on preset identifiers.
