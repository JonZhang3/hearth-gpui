---
title: HoverCard
description: A non-modal preview surface opened by pointer hover or keyboard focus.
---

# HoverCard

`HoverCard` previews information behind a link or another focusable trigger. It opens after a
700ms delay and closes after 300ms. A safe pointer corridor keeps the card open while the pointer
moves from the trigger to the content.

## Import

```rust
use gpui_component::hover_card::{HoverCard, HoverCardAlign, HoverCardSide};
```

## Usage

```rust
HoverCard::new("user-preview")
    .trigger(
        Button::new("profile-link")
            .label("@huacnlee")
            .link()
    )
    .child(
        h_flex()
            .gap_3()
            .items_start()
            .child(Avatar::new("avatar", "Jason Lee"))
            .child(
                v_flex()
                    .gap_1()
                    .child(div().font_semibold().child("Jason Lee"))
                    .child(div().text_color(cx.theme().muted_foreground).child(
                        "The author of GPUI Component."
                    ))
            )
    )
```

The default placement is below the trigger, centered, with a 4px side offset. Vega uses a 256px
surface with 16px padding. Nova and Maia resolve their own geometry from the active Style Preset.

## Placement

`side` and `align` are independent:

```rust
HoverCard::new("placement")
    .side(HoverCardSide::Right)
    .align(HoverCardAlign::Start)
    .side_offset(px(8.))
    .align_offset(px(4.))
    .trigger(Button::new("trigger").label("Preview"))
    .child("Preview content")
```

Supported sides are `Top`, `Right`, `Bottom`, and `Left`. Supported alignments are `Start`,
`Center`, and `End`. The legacy `anchor(Anchor)` builder remains available and maps onto this
model.

## Controlled state

```rust
HoverCard::new("controlled")
    .open(self.preview_open)
    .on_open_change(cx.listener(|this, open, _, cx| {
        this.preview_open = *open;
        cx.notify();
    }))
    .trigger(Button::new("trigger").label("Preview"))
    .child("Controlled preview")
```

Use `default_open(true)` for an uncontrolled card that starts open.

## Custom timing and appearance

```rust
HoverCard::new("custom")
    .open_delay(Duration::from_millis(500))
    .close_delay(Duration::from_millis(200))
    .appearance(false)
    .w(px(320.))
    .p_4()
    .rounded_lg()
    .bg(cx.theme().popover)
    .trigger(Button::new("trigger").label("Preview"))
    .child("Custom preview")
```

## Interaction and accessibility

- Pointer hover and keyboard focus both open the preview.
- The wrapper does not add a Tab stop; the trigger retains its own Enter and click behavior.
- The content is non-modal and does not trap or move focus.
- Do not place required information, buttons, inputs, or workflows exclusively inside a
  `HoverCard`. Use `Popover` or `Dialog` for interactive content.
- GPUI currently has no subtree-level accessibility hiding equivalent to the web implementation's
  screen-reader exclusion. Keep preview children non-interactive and duplicate essential
  information in accessible content.

## API

- `new(id)`
- `trigger(element)`
- `content(builder)`
- `side(HoverCardSide)` / `align(HoverCardAlign)`
- `side_offset(Pixels)` / `align_offset(Pixels)`
- `anchor(Anchor)` compatibility mapping
- `default_open(bool)` / `open(bool)`
- `open_delay(Duration)` / `close_delay(Duration)`
- `on_open_change(callback)`
- `appearance(bool)`
- all `Styled` builders

Enter motion uses a 100ms fade with an 8px placement-aware translation. Exit uses a 100ms fade
without translation, matching shadcn's directional-motion contract. GPUI does not currently apply
shadcn's `scale(0.95)` transform because arbitrary element subtrees lack a layout-independent scale
transform. Opacity is multiplied into individual GPUI paint primitives rather than compositing the
finished subtree as one isolated layer.

[Popover]: ./popover.md
