---
title: Badge
description: A compact label for status or metadata, with optional overlay indicators.
---

# Badge

`Badge` is a non-interactive inline label aligned with the shadcn Vega variants. Use `OverlayBadge` when a count, dot, or icon must be positioned over another element.

## Import

```rust
use hearth_gpui::{
    badge::{Badge, BadgeVariants as _, OverlayBadge},
    Sizable as _,
};
```

## Variants

```rust
Badge::new().child("Default")
Badge::new().secondary().child("Secondary")
Badge::new().destructive().child("Destructive")
Badge::new().outline().child("Outline")
Badge::new().ghost().child("Ghost")
Badge::new().link().child("Link")
```

The variants use semantic Color Theme values and automatically adapt to light and dark modes.

## Icons and Spinner

Use `leading` and `trailing` for compact edge slots. These slots apply the Vega icon spacing without changing the Badge height.

```rust
Badge::new()
    .leading(Icon::new(IconName::CircleCheck).xsmall())
    .child("Verified")

Badge::new()
    .secondary()
    .child("Continue")
    .trailing(Icon::new(IconName::ArrowRight).xsmall())

Badge::new()
    .outline()
    .leading(Spinner::new().xsmall())
    .child("Generating")
```

## Custom Colors

`Badge` implements `Styled`. Apply exceptional status or category colors directly instead of adding component-specific variants.

```rust
Badge::new()
    .bg(cx.theme().success)
    .text_color(cx.theme().success_foreground)
    .child("Success")

Badge::new()
    .outline()
    .border_color(cx.theme().warning)
    .text_color(cx.theme().warning)
    .child("Pending")
```

## Interaction

Badge is a display component. It does not add click, focus, or button semantics. Compose it inside a GPUI `Link` or `Button` when interaction is required; this is the native equivalent of shadcn's `asChild` usage.

## OverlayBadge

`OverlayBadge` positions a numeric count, status dot, or icon over a target element.

### Count

```rust
OverlayBadge::new()
    .count(5)
    .child(Icon::new(IconName::Bell).large())

OverlayBadge::new()
    .count(120)
    .max(99) // Displays "99+"
    .child(Icon::new(IconName::Inbox).large())
```

A numeric overlay is hidden when its count is zero.

### Dot and Lower Icon

```rust
OverlayBadge::new()
    .dot()
    .color(cx.theme().green)
    .child(Avatar::decorative())

OverlayBadge::new()
    .icon(IconName::Check)
    .color(cx.theme().cyan)
    .child(Avatar::decorative())
```

Number and Dot overlays use the upper-right corner. Icon overlays use the lower-right corner.

### Sizes

```rust
OverlayBadge::new().count(2).small().child(target)
OverlayBadge::new().count(12).child(target)
OverlayBadge::new().count(212).large().child(target)
```

## Migrating from Tag and the Previous Badge

| Previous API | Replacement |
|---|---|
| `Tag::primary()` | `Badge::new()` |
| `Tag::secondary()` | `Badge::new().secondary()` |
| `Tag::danger()` | `Badge::new().destructive()` |
| `Tag::success()`, `warning()`, `info()`, `color()`, `custom()` | `Badge::new()` with semantic `Styled` overrides |
| `Badge::new().count(...)` | `OverlayBadge::new().count(...)` |
| `Badge::new().dot()` | `OverlayBadge::new().dot()` |
| `Badge::new().icon(...)` | `OverlayBadge::new().icon(...)` |

`Tag`, its custom size API, and its custom radius API have been removed. Use the fixed Vega Badge geometry and `Styled` only for exceptional visual overrides.

## API Reference

- [Badge]
- [OverlayBadge]

[Badge]: https://docs.rs/hearth_gpui/latest/hearth_gpui/badge/struct.Badge.html
[OverlayBadge]: https://docs.rs/hearth_gpui/latest/hearth_gpui/badge/struct.OverlayBadge.html
