---
title: Empty
description: A compositional empty or no-result state with optional media and actions.
---

# Empty

`Empty` presents an empty, unavailable, or no-result state. Its typed sections keep the layout consistent while allowing arbitrary GPUI content.

## Import

```rust
use hearth_gpui::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle,
};
```

## Basic usage

```rust
Empty::new()
    .min_h(px(320.))
    .child(
        EmptyHeader::new()
            .child(EmptyTitle::new().child("No projects yet"))
            .child(
                EmptyDescription::new()
                    .child("Create a project to organize your work."),
            ),
    )
    .child(
        EmptyContent::new()
            .child(Button::new("create-project").label("Create project")),
    )
```

## Icon media

`EmptyMedia::icon` applies the current Style Preset's icon container and icon dimensions:

```rust
EmptyHeader::new()
    .child(EmptyMedia::icon(IconName::Inbox))
    .child(EmptyTitle::new().child("No messages"))
    .child(EmptyDescription::new().child("New conversations will appear here."))
```

Use `EmptyMedia::new().child(...)` for avatars, avatar groups, illustrations, or other custom content. For a custom child on the icon surface, select `EmptyMediaVariant::Icon` and size the child explicitly.

## Outline and background

The root declares dashed border styling but does not show a border until a width is supplied:

```rust
Empty::new()
    .border_1()
    .border_color(cx.theme().border)
    .child(content)
```

`Empty` and every typed section implement `Styled`, so callers can override dimensions, spacing, background, border, and alignment.

## Composition

```text
Empty
├── EmptyHeader
│   ├── EmptyMedia
│   ├── EmptyTitle
│   └── EmptyDescription
└── EmptyContent
```

## Behavior and accessibility

- Empty is a static layout component with no built-in transition or lifecycle state, matching shadcn.
- It does not automatically use alert, status, or live-region semantics. Interactive children retain their own roles and accessible names.
- Vega is the default visual baseline. Nova uses compact padding, typography, and icon geometry; Maia uses comfortable radii without changing the composition.
- GPUI currently uses normal constrained wrapping instead of the browser-only `text-wrap: balance` behavior.
