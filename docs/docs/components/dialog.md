---
title: Dialog
description: A modal surface for focused content and actions.
---

# Dialog

`Dialog` provides a modal surface with an overlay, focus trap, focus restoration, keyboard dismissal, and interruptible exit motion. The first application view must be a [`Root`](../getting-started) and must render the Root dialog layer.

## Import

```rust
use hearth_gpui::{
    WindowExt as _,
    button::Button,
    dialog::{Dialog, DialogAction, DialogClose, DialogFooter},
};
```

## Declarative trigger

`trigger` accepts a `Button`. The Dialog appends its open handler without replacing the Button's existing `on_click` handler, so pointer and Enter/Space activation use the same control semantics.

```rust
Dialog::new(cx)
    .trigger(Button::new("edit-profile").outline().label("Edit profile"))
    .title("Edit profile")
    .description("Update your public profile information.")
    .content(move |content, _, _| {
        content.child(Input::new(&name_input))
    })
    .footer(|_, _| {
        DialogFooter::new()
            .child(DialogClose::new(
                Button::new("cancel").outline().label("Cancel"),
            ))
            .child(DialogAction::new(
                Button::new("save").label("Save changes"),
            ))
    })
    .on_ok(|_, window, cx| {
        window.push_notification("Profile saved", cx);
        true
    })
```

`DialogClose` and `DialogAction` attach behavior directly to their Button. They do not add a sizing or focus wrapper, and existing Button click handlers are preserved.

## Imperative dialog

Use `WindowExt::open_dialog` when an existing event opens the modal.

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Keyboard shortcuts")
        .description("Review the shortcuts available in this window.")
        .child(shortcuts_view)
        .footer(|_, _| {
            DialogFooter::new().child(DialogClose::new(
                Button::new("done").label("Done"),
            ))
        })
})
```

## Custom title or description

Text slots automatically provide the Dialog accessible name and description. Custom elements use renderer closures because a declarative trigger may open the same Dialog repeatedly; provide explicit accessibility text with them.

```rust
Dialog::new(cx)
    .aria_label("Connection settings")
    .aria_description("Configure the active server connection.")
    .title_element(|_, _| h_flex().child(Icon::new(IconName::Server)).child("Connection"))
    .description_element(|_, _| div().child("Changes apply after reconnecting."))
```

## Focus and dismissal

- Opening focuses an explicit `initial_focus` handle, otherwise the first enabled Tab stop, otherwise the Dialog surface.
- Tab and Shift-Tab remain trapped inside the topmost Dialog.
- Closing restores focus to the previous control.
- Escape and overlay click dispatch cancellation and respect an `on_cancel` callback that returns `false`.
- Enter dispatches the standard Dialog confirmation action. Space continues to activate the focused Button.
- `AlertDialog` intentionally does not use global Enter confirmation.

```rust
dialog
    .initial_focus(input_focus)
    .dismiss_on_escape(true)
    .confirm_on_enter(true)
    .dismiss_on_overlay_click(true)
    .show_close_button(true)
    .show_overlay(true)
```

## Size and position

The default width is resolved from the active Style Preset and is clamped to a 16 px viewport inset on each side. Explicit width and maximum width remain subject to that safety inset.

```rust
dialog
    .w(px(560.))
    .max_w(px(720.))
    .margin_top(px(96.))
```

## Style Presets and motion

- Vega is the default baseline: 448 px preferred width, 24 px padding, compact title, and an unseparated footer.
- Nova uses compact 384 px geometry and a separated, tinted footer.
- Maia uses comfortable geometry, larger radii, and a stronger overlay.
- Dialog reads semantic Theme and Style Metrics and never branches on Preset IDs.

The project intentionally retains its desktop Dialog motion instead of shadcn's centered zoom: the surface settles at approximately 10% of the viewport height using the semantic 250 ms emphasis duration, enter/exit easing, opacity, translation, and shadow transitions. Exit remains mounted until animation completion, supports interruption, and becomes immediate under Reduced Motion.

The pinned GPUI renderer does not expose element-level backdrop filtering. Overlay color and opacity align with each Style Preset, while backdrop blur remains deferred in `docs/shadcn/TODO.md`.
