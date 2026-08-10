---
title: Sheet
description: A modal surface that slides in from an edge of the window.
---

# Sheet

Sheet displays supplementary navigation, forms, or settings in a modal surface attached to one
edge of the window. It provides four placements, focus trapping, responsive side sizing, scrolling,
backdrop dismissal, exit motion, and focus restoration.

## Import

```rust
use gpui_component::{Placement, WindowExt};
```

The first window view must be [`Root`](/docs/root). Render `Root::render_sheet_layer` when using a
custom root composition.

## Basic usage

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Edit profile")
        .description("Make changes to your profile, then save them.")
        .child(profile_form)
        .footer(
            v_flex()
                .gap_2()
                .child(Button::new("save").label("Save changes").w_full())
                .child(
                    Button::new("cancel")
                        .outline()
                        .label("Cancel")
                        .w_full()
                        .on_click(|_, window, cx| window.close_sheet(cx)),
                ),
        )
})
```

`open_sheet` uses the right placement. Left and right sheets default to 75% of the available width,
capped at 384 px. Top and bottom sheets use their content height.

## Placement and size

```rust
window.open_sheet_at(Placement::Left, cx, |sheet, _, _| {
    sheet.title("Navigation").child(navigation)
});

window.open_sheet_at(Placement::Bottom, cx, |sheet, _, _| {
    sheet
        .title("Activity")
        .description("Recent workspace activity")
        .size(px(320.))
        .child(activity)
});
```

`size` overrides the width for left/right placements and the height for top/bottom placements.
Sheet does not implement drag resizing.

## Header composition

Text titles and descriptions automatically provide the accessible name and description:

```rust
sheet
    .title("Application settings")
    .description("Configure appearance and notifications")
```

Use element slots for custom rendering and provide explicit accessibility metadata:

```rust
sheet
    .title_element(custom_title)
    .description_element(custom_description)
    .aria_label("Application settings")
    .aria_description("Configure appearance and notifications")
```

The header is omitted when neither a title nor a description is supplied.

## Close button and backdrop

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("No close button")
        .show_close_button(false)
        .overlay(true)
        .overlay_closable(true)
        .child("Use Escape or click the backdrop to dismiss.")
})
```

- `show_close_button(false)` removes the icon button without leaving an empty header row.
- `overlay(false)` hides the backdrop paint but preserves modal occlusion and the focus trap.
- `overlay_closable` controls primary-click dismissal only when the visual backdrop is present.
- Escape dismisses the Sheet.
- `on_close` observes user dismissal through Escape, backdrop click, or the built-in close button.
  A direct `window.close_sheet(cx)` is programmatic and does not invoke it.

## Initial focus

```rust
let name_focus = name_input.read(cx).focus_handle(cx);

window.open_sheet(cx, move |sheet, _, _| {
    sheet
        .title("Edit profile")
        .initial_focus(name_focus.clone())
        .child(Input::new(&name_input))
})
```

Without `initial_focus`, Sheet focuses the first valid tab stop. If none exists, focus remains on the
dialog surface. Focus returns to the previously focused control after exit motion completes.

## Native title-bar safe area

Unlike the web component, Sheet preserves the desktop window title-bar drag area. Configure the
global safe area through the Theme before opening a Sheet:

```rust
theme.sheet.margin_top = px(32.);
```

There is no per-Sheet `margin_top` builder.

## Custom styling

`Sheet` implements `Styled`. Style refinements apply to the Sheet surface rather than being
duplicated on the body:

```rust
window.open_sheet(cx, |sheet, _, cx| {
    sheet
        .title("Styled Sheet")
        .bg(cx.theme().accent)
        .text_color(cx.theme().accent_foreground)
        .border_color(cx.theme().primary)
        .child(content)
})
```

## API

| Method | Description |
|---|---|
| `title(text)` | Set semantic title text and default accessible name |
| `title_element(element)` | Set custom title content |
| `description(text)` | Set semantic description text |
| `description_element(element)` | Set custom description content |
| `aria_label(text)` | Override the accessible name |
| `aria_description(text)` | Override the accessible description |
| `child(element)` | Append scrollable body content |
| `footer(element)` | Set footer content |
| `size(length)` | Override width or height along the placement axis |
| `show_close_button(bool)` | Show or hide the close button |
| `overlay(bool)` | Show or hide backdrop paint |
| `overlay_closable(bool)` | Enable backdrop-click dismissal |
| `initial_focus(handle)` | Set the preferred initial focus target |
| `on_close(handler)` | Observe user-initiated dismissal |

[`Root`]: /docs/root
