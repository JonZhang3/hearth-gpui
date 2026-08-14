---
title: GroupBox
description: A lightweight semantic container for grouping related content.
---

# GroupBox

`GroupBox` groups related controls or content without introducing Card elevation or interaction behavior. It supports plain, filled, and outlined surfaces while preserving application-defined content composition.

## Import

```rust
use hearth_gpui::group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants as _};
```

## Basic usage

```rust
GroupBox::new()
    .id("subscriptions")
    .aria_label("Subscription settings")
    .title("Subscriptions")
    .child(Checkbox::new("all").label("All"))
    .child(Checkbox::new("newsletter").label("Newsletter"))
    .child(Button::new("save").label("Save"))
```

An explicit ID lets GPUI expose the root as an accessibility `Group`. Add `aria_label` when the group contains interactive controls or when its title is a custom element.

## Variants

```rust
// Plain content without a painted surface or content padding.
GroupBox::new()
    .id("plain")
    .normal()
    .title("Plain")
    .child("Content")

// Semantic GroupBox background with density-aware content padding.
GroupBox::new()
    .id("filled")
    .fill()
    .title("Filled")
    .child("Content")

// Semantic border with density-aware content padding.
GroupBox::new()
    .id("outlined")
    .outline()
    .title("Outlined")
    .child("Content")
```

| Variant | Background | Border | Content padding |
| --- | --- | --- | --- |
| `Normal` | None | None | None |
| `Fill` | `tokens.group_box` | None | Style Preset density |
| `Outline` | None | Theme `border` | Style Preset density |

GroupBox does not add shadows. Use Card when content needs an elevated, sectioned surface.

## Theme and Style Presets

GroupBox consumes these semantic values:

- `group_box.background`
- `group_box.foreground`
- `group_box.title.foreground`
- Theme `border`
- Style Preset `radii.md`
- Style Preset `density`

Compact, Standard, and Comfortable presets adjust content padding, content gap, title-to-content gap, and title line height. The implementation never branches on Vega, Nova, or Maia IDs.

## Styling layers

```rust
GroupBox::new()
    .id("custom")
    .aria_label("Custom group")
    .outline()
    // Styled refinements apply to the outer group layout.
    .gap_6()
    .title("Custom title")
    // title_style applies only to the title wrapper.
    .title_style(StyleRefinement::default().font_semibold())
    // content_style applies only to the content surface.
    .content_style(
        StyleRefinement::default()
            .rounded_lg()
            .border_2()
    )
    .child("Content")
```

Built-in metrics are applied before refinements, so explicit caller styles remain authoritative.

## Long content

The root, title, and content surfaces use `min_w_0`, allowing text and nested layouts to shrink inside constrained parents. Child content still owns its desired wrapping or truncation behavior.

## API reference

### GroupBox

| Method | Description |
| --- | --- |
| `new()` | Create a plain GroupBox |
| `id(id)` | Set stable GPUI and accessibility identity |
| `aria_label(label)` | Set the accessible group name |
| `title(element)` | Set optional title content |
| `title_style(style)` | Refine the title wrapper |
| `content_style(style)` | Refine the content surface |
| `normal()` | Use the plain variant |
| `fill()` | Use the filled variant |
| `outline()` | Use the outlined variant |

### GroupBoxVariant

`GroupBoxVariant` supports `Normal`, `Fill`, and `Outline`, plus `from_str` and `as_str` for settings persistence.

## Related components

- **Settings** uses GroupBox as its visual grouping surface.
- **Card** provides a stronger, sectioned and optionally elevated surface.
- **Accordion** groups collapsible content.
