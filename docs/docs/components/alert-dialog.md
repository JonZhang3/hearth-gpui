---
title: AlertDialog
description: Requires an explicit response before returning to the application.
---

# AlertDialog

`AlertDialog` is a modal confirmation surface aligned with shadcn Vega. It blocks background interaction, traps focus, does not close when the overlay is clicked, and has no close button.

## Basic usage

```rust
use hearth_gpui::{
    button::Button,
    dialog::{AlertDialog, AlertDialogAction, AlertDialogCancel},
};

AlertDialog::new(cx)
    .trigger(Button::new("show-dialog").outline().label("Show Dialog"))
    .on_action(|_, window, cx| {
        window.push_notification("Confirmed", cx);
        true
    })
    .content(|content, _, _| {
        content
            .title("Are you absolutely sure?")
            .description(
                "This action cannot be undone. This will permanently delete your account.",
            )
            .cancel(AlertDialogCancel::new("cancel", "Cancel"))
            .action(AlertDialogAction::new("continue", "Continue"))
    })
```

## Small destructive dialog

`Small` centers the content and gives both footer actions equal width.

```rust
use hearth_gpui::{
    button::ButtonVariant,
    dialog::{AlertDialogAction, AlertDialogCancel, AlertDialogSize},
};

content
    .size(AlertDialogSize::Small)
    .media(Icon::new(IconName::TriangleAlert).size_8())
    .title("Delete chat?")
    .description("This will permanently delete this chat conversation.")
    .cancel(AlertDialogCancel::new("cancel", "Cancel"))
    .action(
        AlertDialogAction::new("delete", "Delete")
            .variant(ButtonVariant::Destructive),
    )
```

## Imperative usage

`WindowExt::open_alert_dialog` uses the same content API and behavior as a declarative trigger.

```rust
window.open_alert_dialog(cx, |dialog, _, _| {
    dialog.content(|content, _, _| {
        content
            .title("Session expired")
            .description("Sign in again to continue.")
            .action(AlertDialogAction::new("sign-in", "Sign in"))
    })
});
```

## Custom content and accessibility

Text passed through `title` and `description` becomes the accessible name and description. Use `title_element` or `description_element` for custom GPUI elements, and provide the corresponding `aria_label` or `aria_description`.

`AlertDialogContent` also implements `ParentElement`, so form controls or other custom body content can be inserted with `child`.

## Behavior

- Overlay clicks never dismiss an AlertDialog.
- Escape cancels by default; use `dismiss_on_escape(false)` to disable it.
- Enter and Space activate only the focused button. They do not confirm the dialog globally.
- Returning `false` from `on_action` or `on_cancel` keeps the dialog open.
- Exit content remains mounted until the shared modal animation completes.
- Overlay and content use a stable 100 ms fade. The pinned GPUI renderer does not provide element-level backdrop blur or layout-independent scale for arbitrary element trees, so these CSS-only effects are intentionally omitted.

## Style Presets

Vega is the default visual baseline. Nova and Maia resolve their own modal padding, width, radius, media geometry, footer treatment, overlay opacity, and ring opacity through semantic `ModalMetrics` without component branches on preset IDs.

The website screenshots commonly use Nova. Nova's tinted, separated footer is not part of the Vega default.
