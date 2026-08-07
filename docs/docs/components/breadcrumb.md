---
title: Breadcrumb
description: Displays the current location within a hierarchy.
---

# Breadcrumb

Breadcrumb uses explicit Link, Page, Separator, and Ellipsis elements so navigation semantics do not depend on child position.

## Import

```rust
use gpui::ParentElement as _;
use gpui_component::{
    IconName, Sizable as _,
    breadcrumb::{
        Breadcrumb, BreadcrumbEllipsis, BreadcrumbItem, BreadcrumbLink, BreadcrumbPage,
        BreadcrumbSeparator,
    },
    button::Button,
    menu::{DropdownMenu as _, PopupMenuItem},
};
```

## Basic usage

```rust
Breadcrumb::new("docs-breadcrumb")
    .child(
        BreadcrumbItem::new("home-item")
            .child(
                BreadcrumbLink::new("home-link")
                    .label("Home")
                    .href("https://example.com"),
            ),
    )
    .child(BreadcrumbSeparator::new())
    .child(
        BreadcrumbItem::new("components-item")
            .child(
                BreadcrumbLink::new("components-link")
                    .label("Components")
                    .href("https://example.com/components"),
            ),
    )
    .child(BreadcrumbSeparator::new())
    .child(
        BreadcrumbItem::new("current-item")
            .child(BreadcrumbPage::new("current-page").label("Breadcrumb")),
    )
```

`BreadcrumbLink` supports `href`, `on_click`, and `disabled`. URL navigation and the callback are composed rather than replacing each other. Enabled links are Tab stops and Enter activates them.

A label-only `BreadcrumbLink` has no activation target, so it renders as non-interactive text and does not enter the Tab order. Use `href` or `on_click` whenever the item should behave as a link.

## Migration from the legacy API

The compositional API is an explicitly approved breaking redesign. Replace positional strings and automatic separators:

```rust
// Legacy
Breadcrumb::new().child("Home").child("Components")

// Compositional
Breadcrumb::new("docs-breadcrumb")
    .child(
        BreadcrumbItem::new("home-item").child(
            BreadcrumbLink::new("home-link")
                .label("Home")
                .href("https://example.com"),
        ),
    )
    .child(BreadcrumbSeparator::new())
    .child(
        BreadcrumbItem::new("current-item")
            .child(BreadcrumbPage::new("current-page").label("Components")),
    )
```

`Breadcrumb::new` and `BreadcrumbItem::new` now receive stable element IDs. Visible text belongs to `BreadcrumbLink::label` or `BreadcrumbPage::label`.

## Custom separator

```rust
BreadcrumbSeparator::new().child("/")
```

## Collapsed items

`BreadcrumbEllipsis` is presentational and can stand in for collapsed intermediate locations:

```rust
BreadcrumbItem::new("collapsed-item").child(BreadcrumbEllipsis::new())
```

For a collapsed-items menu, use Button's icon API so the trigger receives the Style Preset's icon-only square geometry. The Button must provide the accessible name.

```rust
Button::new("collapsed-items")
    .ghost()
    .small()
    .icon(IconName::Ellipsis)
    .aria_label("Show collapsed breadcrumb items")
    .dropdown_menu(|menu, _, _| {
        menu.item(PopupMenuItem::new("Documentation"))
            .item(PopupMenuItem::new("Themes"))
    })
```

`BreadcrumbPage` is non-interactive and exposes the current-page state to assistive technologies. Use `aria_label` whenever Link or Page contains custom content without a text label.
