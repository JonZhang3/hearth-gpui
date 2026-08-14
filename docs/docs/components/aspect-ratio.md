---
title: AspectRatio
description: A layout container that preserves a width-to-height ratio.
---

# AspectRatio

`AspectRatio` keeps its content at a fixed `width / height` ratio while adapting to the available space. It maps directly to GPUI's native aspect-ratio layout support.

## Import

```rust
use hearth_gpui::aspect_ratio::AspectRatio;
```

## Usage

Constrain the width or height through the parent or the component's `Styled` API:

```rust
AspectRatio::new(16.0 / 9.0)
    .w(px(480.))
    .rounded(cx.theme().style.radii.lg)
    .bg(cx.theme().muted)
    .child(content)
```

The ratio is expressed as `width / height`. Common values include `16.0 / 9.0`, `1.0`, and `9.0 / 16.0`.

## API Reference

| Method | Description |
| --- | --- |
| `new(ratio)` | Creates a container with the given width-to-height ratio |
| `ratio(ratio)` | Replaces the ratio |
| `child(c)` / `children(cs)` | Adds content to the container |

`AspectRatio` implements `Styled`. Invalid, non-positive, or non-finite ratios safely fall back to `1:1`.

## Notes

- The component only owns layout. It does not add a background, radius, clipping, shadow, or animation.
- The parent must constrain at least one axis. The default container fills the available width and derives its height from the ratio.
- Vega, Nova, and Maia use identical ratio behavior. Visual styling belongs to the caller and remains independent from the Style Preset.
- Apply image radius directly to the image when rounded clipping is required; GPUI does not yet provide a complete rounded clip chain for arbitrary children.
