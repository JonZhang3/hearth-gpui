---
title: Alert
description: Displays a callout for user attention.
---

# Alert

Alert displays important status or guidance. Its icon, title, description, and action are independent optional slots.

## Import

```rust
use hearth_gpui::alert::Alert;
```

## Basic

```rust
Alert::new("payment-success")
    .icon(IconName::CircleCheck)
    .title("Payment successful")
    .description(
        "Your payment of $29.99 has been processed. A receipt has been sent to your email address."
    )
```

Every content slot is optional:

```rust
Alert::new("title-only").title("Changes saved")

Alert::new("description-only")
    .description("This alert has no title or icon.")
```

## Destructive

Use `destructive` for errors or failed actions. Variants do not add icons automatically.

```rust
Alert::new("payment-failed")
    .destructive()
    .icon(IconName::TriangleAlert)
    .title("Payment failed")
    .description("Your payment could not be processed. Please try again.")
```

## Action

Use `action` to place a button or another element in the top-right corner.

```rust
Alert::new("dark-mode")
    .title("Dark mode is now available")
    .description("Enable it under your profile settings to get started.")
    .action(Button::new("enable-dark-mode").xsmall().label("Enable"))
```

## Custom Colors

Alert implements `Styled`. Root foreground overrides are inherited by the icon and title, while the default description remains muted.

```rust
Alert::new("subscription-warning")
    .icon(IconName::TriangleAlert)
    .title("Your subscription will expire in 3 days.")
    .description("Renew now to avoid service interruption.")
    .bg(cx.theme().warning.opacity(0.08))
    .border_color(cx.theme().warning.opacity(0.5))
    .text_color(cx.theme().warning)
```

## Closable

`on_close` replaces a custom action with an accessible icon-only close button. The callback owns visibility state.

```rust
Alert::new("closable")
    .title("Maintenance scheduled")
    .description("The service will be unavailable tonight.")
    .visible(is_visible)
    .on_close(cx.listener(|this, _, _, cx| {
        this.is_visible = false;
        cx.notify();
    }))
```

## Banner

Banner appearance removes the border and radius without hiding content slots.

```rust
Alert::new("maintenance-banner")
    .banner()
    .icon(IconName::Info)
    .title("Maintenance scheduled")
    .description("The service will be unavailable tonight.")
```

## Rich Content

`title` and `description` retain ordinary strings for accessibility metadata. Use `title_element` or `description_element` for arbitrary GPUI elements. Because arbitrary elements cannot be converted back to text reliably, provide an `aria_label` that summarizes all important content.

```rust
Alert::new("validation-error")
    .destructive()
    .title("Validation failed")
    .description_element(markdown(
        "Please correct the following errors:\n\
        - Email address is required\n\
        - Password must be at least 8 characters"
    ))
    .aria_label(
        "Validation failed. Email address is required. Password must be at least 8 characters."
    )
```

## Accessibility

Alert exposes the AccessKit Alert role. Text titles become its accessible name, and text descriptions become its accessible description. A description-only Alert uses its description as the name. Use `aria_label` when custom element slots do not provide suitable text metadata.

```rust
Alert::new("sync-status")
    .aria_label("Synchronization failed")
    .destructive()
    .description_element(custom_status_view)
```

## API Reference

- [Alert]
- [AlertVariant]

[Alert]: https://docs.rs/hearth-gpui/latest/hearth_gpui/alert/struct.Alert.html
[AlertVariant]: https://docs.rs/hearth-gpui/latest/hearth_gpui/alert/enum.AlertVariant.html
