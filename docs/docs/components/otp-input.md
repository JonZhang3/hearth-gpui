---
title: OtpInput
description: A composable one-time-code input with native editing and shadcn-aligned slots.
---

# OtpInput

`OtpInput` presents a one-time code as visual slots while keeping one real `InputState` as the editing and accessibility authority. Selection, cursor movement, deletion, paste, IME, and AccessKit value updates therefore follow the ordinary Input behavior.

## Import

```rust
use hearth_gpui::input::{
    InputEvent, OtpEvent, OtpInput, OtpInputGroup, OtpInputSeparator,
    OtpInputSlot, OtpState,
};
```

## Basic composition

```rust
let otp_state = cx.new(|cx| OtpState::new(6, window, cx));

let first = OtpInputGroup::new()
    .child(OtpInputSlot::new(0))
    .child(OtpInputSlot::new(1))
    .child(OtpInputSlot::new(2));
let second = OtpInputGroup::new()
    .child(OtpInputSlot::new(3))
    .child(OtpInputSlot::new(4))
    .child(OtpInputSlot::new(5));

OtpInput::new(&otp_state)
    .child(first)
    .child(OtpInputSeparator::new())
    .child(second)
    .aria_label("One-time code")
```

When no children are supplied, `OtpInput` creates one contiguous group containing every slot. Explicit composition is recommended whenever separators or custom slot styles are needed.

## Patterns and paste

The default pattern accepts digits. Use one mask token per slot: `9` accepts a digit, `A` a letter, `#` an alphanumeric character, and `*` any character.

```rust
let code_state = cx.new(|cx| {
    OtpState::new(6, window, cx)
        .pattern("######")
        .paste_transformer(|text| text.replace('-', ""))
        .default_value("A1B2")
});
```

Clipboard and AccessKit values are transformed, normalized against the mask, and truncated to the configured slot count before they reach the editor. Full-width digits are normalized to ASCII digits.

## States and sizes

```rust
OtpInput::new(&otp_state).invalid(true);
OtpInput::new(&otp_state).disabled(true);
OtpInput::new(&otp_state).small();
OtpInput::new(&otp_state).large();
OtpInput::new(&otp_state).with_size(px(44.));
```

Mask the rendered value through the state:

```rust
let otp_state = cx.new(|cx| OtpState::new(6, window, cx).masked(true));

otp_state.update(cx, |state, cx| {
    state.set_masked(false, window, cx);
});
```

Slot height, radius, shadow, focus ring, and surface color consume semantic Style Preset and Color Theme values. Vega is the default baseline; Nova and Maia resolve their compact and comfortable geometry without preset-ID branches.

## Events

`InputEvent::Change` is emitted for every value mutation. `OtpEvent::Complete` is emitted once when an incomplete code reaches the configured length.

```rust
cx.subscribe(&otp_state, |this, state, event: &InputEvent, cx| {
    if matches!(event, InputEvent::Change) {
        this.code = state.read(cx).value().clone();
        cx.notify();
    }
});

cx.subscribe(&otp_state, |this, _, event: &OtpEvent, cx| {
    if matches!(event, OtpEvent::Complete) {
        this.submit_code(cx);
    }
});
```

Programmatic updates do not emit `InputEvent::Change`:

```rust
otp_state.update(cx, |state, cx| {
    state.set_value("123456", window, cx);
    state.focus(window, cx);
});
```

## Accessibility

The visual slots are not separate tab stops. A single hidden editor exposes `Role::TextInput`, the `OneTimeCode` content type, selection, and normalized value. Supply `aria_label` and optionally `aria_description`. Masked values are not exposed to the accessibility tree.

## API reference

| Type | Main API |
| --- | --- |
| `OtpState` | `new`, `default_value`, `pattern`, `paste_transformer`, `masked`, `set_value`, `set_masked`, `value`, `length`, `focus` |
| `OtpInput` | `new`, `child`, `invalid`, `disabled`, `with_size`, `aria_label`, `aria_description` |
| `OtpInputGroup` | `new`, `child(OtpInputSlot)` |
| `OtpInputSlot` | `new(index)` plus `Styled` refinements |
| `OtpInputSeparator` | `new`, `child` plus `Styled` refinements |
