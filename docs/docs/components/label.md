---
title: Label
description: A label for form controls and composed inline content.
---

# Label

`Label` uses the shadcn Vega baseline: inline flex layout, `gap-2`, `text-sm`, medium weight, and compact line height. It has no built-in motion.

## Import

```rust
use gpui_component::{Disableable as _, label::Label};
```

## Basic

```rust
Label::new("Username")
```

## With an Input

Use `for_focus` to focus a control when the label is clicked. The control must still provide its own accessible name because GPUI does not currently expose a cross-element `labelled_by` relation.

```rust
let input_focus = input_state.read(cx).focus_handle(cx);

v_flex()
    .gap_2()
    .child(Label::new("Username").for_focus(&input_focus))
    .child(Input::new(&input_state).aria_label("Username"))
```

## Disabled

Disabled labels use 50% opacity and do not focus their associated control.

```rust
Label::new("Username")
    .for_focus(&input_focus)
    .disabled(true)
```

## Composed Content

Use `empty` and `ParentElement` composition for icons or other inline elements.

```rust
Label::empty()
    .child(Icon::new(IconName::Info).xsmall())
    .child("Additional information")
```

For `Checkbox`, `Radio`, and `Switch`, prefer each component's integrated `label` API so its visual label and accessibility semantics stay together.

## Project Extensions

The existing secondary text, masking, highlighting, and `Styled` overrides remain available.

```rust
Label::new("Company Address")
    .secondary("(optional)")
    .highlights("company")

Label::new("9,182.1 USD").masked(true)
```

`HighlightsMatch::Prefix` limits highlighting to a match beginning at the first character. Matching is case-insensitive and preserves valid UTF-8 byte boundaries.

## API

| Method | Description |
| --- | --- |
| `Label::new(text)` | Creates a text label |
| `Label::empty()` | Creates a label for composed children |
| `.for_focus(&handle)` | Focuses an enabled target on primary mouse press |
| `.disabled(bool)` | Applies disabled presentation and interaction |
| `.secondary(text)` | Adds muted secondary text |
| `.highlights(match)` | Highlights full or prefix matches |
| `.masked(bool)` | Replaces displayed characters with bullets |
| `.child(element)` | Adds composed inline content |

All standard `Styled` methods can refine the default presentation.
