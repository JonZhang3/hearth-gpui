---
title: Card
description: Groups related content and actions on a shadcn Vega surface.
---

# Card

Card provides typed Header, Content, Footer, and Media slots so the selected size is applied consistently to every section. It is a static container and does not add focus or interaction behavior to its children.

## Import

```rust
use gpui::ParentElement as _;
use hearth_gpui::card::{
    Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardMedia,
    CardTitle,
};
```

## Basic usage

```rust
Card::new()
    .header(
        CardHeader::new()
            .title(CardTitle::new().child("Create project"))
            .description(
                CardDescription::new().child("Deploy your new project in one click."),
            ),
    )
    .content(CardContent::new().child("Project settings"))
    .footer(
        CardFooter::new()
            .justify_end()
            .child(Button::new("deploy").label("Deploy")),
    )
```

## Header action

Use `CardAction` for content aligned to the upper-right of the header. The title and description remain in a separate shrinking column so long copy does not overlap the action.

```rust
CardHeader::new()
    .title(CardTitle::new().child("Meeting Notes"))
    .description(CardDescription::new().child("Transcript from the client meeting."))
    .action(
        CardAction::new().child(
            Button::new("transcribe")
                .small()
                .outline()
                .label("Transcribe"),
        ),
    )
```

## Small Card

`small()` selects the compact Card variant. Vega and Maia use 24/16 px default/small spacing; Nova uses 16/12 px. Vega and Nova reduce the Small title to 14 px, while Maia retains its 16 px heading.

```rust
Card::new()
    .small()
    .header(CardHeader::new().title(CardTitle::new().child("Small Card")))
    .content(CardContent::new().child("Compact content"))
```

## Custom spacing

`spacing()` overrides the shared Card gap, vertical padding, and horizontal section padding. It does not change size-dependent title typography.

```rust
Card::new()
    .small()
    .spacing(px(20.))
    .header(CardHeader::new().title(CardTitle::new().child("Custom spacing")))
    .content(CardContent::new().child("20 px section spacing"))
```

## Divided sections

`bordered(true)` adds the section divider and the matching padding required by the Vega layout.

```rust
Card::new()
    .header(
        CardHeader::new()
            .title(CardTitle::new().child("Release Health"))
            .bordered(true),
    )
    .content(CardContent::new().child("24 of 26 checks passed."))
    .footer(CardFooter::new().bordered(true).child("Updated now"))
```

## Edge-to-edge media

The media slot is rendered before the header and removes the Card's top padding. Use `CardMedia::image` for images so the outer Card radius is painted directly on the image.

```rust
Card::new()
    .media(
        CardMedia::image("https://example.com/landscape.jpg")
            .h(px(160.)),
    )
    .header(CardHeader::new().title(CardTitle::new().child("Landscape")))
```

For custom media, apply its background to `CardMedia` itself before adding foreground-only children. GPUI currently clips overflowing descendants with a rectangular mask, so a square child background cannot inherit rounded clipping from its parent. CardMedia inherits the Card's resolved edge radius, including overrides such as `rounded(px(0.))` or custom per-corner values.

Use `bottom_media()` for media rendered after the Footer. Like shadcn's trailing image selector, it retains the Card's normal bottom padding instead of becoming a flush bottom surface.

```rust
Card::new()
    .footer(CardFooter::new().child("Updated now"))
    .bottom_media(
        CardMedia::image("https://example.com/preview.jpg")
            .h(px(120.)),
    )
```

## GPUI composition notes

The React implementation accepts arbitrary direct children and uses CSS Grid for Header layout. This API intentionally keeps typed slots so Card spacing can be propagated without DOM selectors. CardHeader uses the equivalent `1fr + auto` Flex layout: the text column shrinks and the action remains aligned at the upper-right.

GPUI supports `container_query` for explicitly sized regions, so responsive content can be placed inside `CardContent`. Card does not install an automatic Header container query because GPUI container-query children cannot determine the container's own size. This is an intentional platform-specific difference.

Card backgrounds use the Color Theme's `card.background` and `card.foreground` roles. Themes that omit them fall back to `background` and `foreground`. Vega supplies the xs shadow, Nova supplies its compact tinted footer, and Maia supplies the larger radius and header gap through semantic Style Preset properties.

[Card]: https://docs.rs/hearth-gpui/latest/hearth_gpui/card/struct.Card.html
