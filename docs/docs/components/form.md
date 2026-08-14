---
title: Form and Field
description: Composable form-field primitives with GPUI-native grid layout and validation semantics.
---

# Form and Field

`Field` follows shadcn's compositional model. `Form` is the GPUI-native layout layer for arranging Fields in rows or grids; validation and submitted values remain application state.

## Import

```rust
use gpui_component::form::{
    field, v_form, FieldBody, FieldContent, FieldDescription, FieldError,
    FieldGroup, FieldLabel, FieldLegend, FieldSet,
};
```

## Basic field

Every Field requires a stable ID. Use semantic text on the control itself and an explicit Field accessibility label because GPUI does not currently expose a complete `labelled-by` relationship.

```rust
field("account-email")
    .aria_label("Email")
    .required(true)
    .content(move |state| {
        FieldBody::new()
            .child(FieldLabel::new("Email").required(state.required()))
            .child(
                FieldContent::new()
                    .child(Input::new(&email).aria_label("Email").disabled(state.disabled()))
                    .child(FieldDescription::new("Used for account notifications.")),
            )
    })
```

`FieldLabel::for_focus(&focus_handle)` gives the label native pointer-to-focus behavior. The target control must still provide its own accessible name.

## Validation

```rust
field("account-email")
    .aria_label("Email")
    .aria_description("Enter a valid email address.")
    .required(true)
    .invalid(true)
    .content(move |state| {
        FieldBody::new()
            .child(FieldLabel::new("Email").required(state.required()))
            .child(
                FieldContent::new()
                    .child(Input::new(&email).aria_label("Email").invalid(state.invalid()))
                    .child(FieldError::new(
                        "account-email-error",
                        "Enter a valid email address.",
                    )),
            )
    })
```

`FieldError` exposes `Role::Alert`. Its `errors(...)` builder removes duplicate messages while preserving their order. `FieldState` carries the effective Form and Field state into the content builder; controls consume the states they support.

## Field groups

```rust
FieldSet::new("notification-preferences")
    .aria_label("Notification preferences")
    .content(|state| {
        FieldBody::new()
            .child(FieldLegend::new("Notification preferences"))
            .child(
                FieldGroup::new()
                    .selection()
                    .child(Checkbox::new("email-updates").label("Email updates").disabled(state.disabled()))
                    .child(Checkbox::new("product-updates").label("Product updates").disabled(state.disabled())),
            )
    })
```

The compositional family also includes `FieldTitle` and `FieldSeparator`. `FieldGroup::selection()` selects the tighter spacing used by checkbox and radio collections. `FieldLegendVariant::Label` selects the smaller legend typography.

## Form layout

```rust
v_form()
    .columns(2)
    .child(
        field("first-name")
            .aria_label("First name")
            .content(move |_| FieldBody::new()
                .child(FieldLabel::new("First name"))
                .child(FieldContent::new().child(Input::new(&first_name)))),
    )
    .child(
        field("last-name")
            .aria_label("Last name")
            .content(move |_| FieldBody::new()
                .child(FieldLabel::new("Last name"))
                .child(FieldContent::new().child(Input::new(&last_name)))),
    )
    .child(
        field("biography")
            .col_span(2)
            .aria_label("Biography")
            .content(move |_| FieldBody::new()
                .child(FieldLabel::new("Biography"))
                .child(FieldContent::new().child(Input::new(&biography)))),
    )
```

Use `h_form()` for horizontal Fields. Give `FieldLabel` an explicit width when several horizontal fields must align:

```rust
h_form().child(
    field("username")
        .aria_label("Username")
        .content(move |_| FieldBody::new()
            .child(FieldLabel::new("Username").w(px(120.)).flex_shrink_0())
            .child(FieldContent::new().child(Input::new(&username)))),
)
```

`Form` supports `Sizable`, `Styled`, `Disableable`, `columns`, and Field grid positioning. `Form::disabled(true)` is included in every child Field's effective state. A zero column count is normalized to one. `Field::visible(false)` removes the Field from layout and the accessibility tree.

## Style Presets

The family resolves spacing from semantic Style Preset density. Vega is the default baseline; Nova is compact and Maia is comfortable. The pinned shadcn Field sources define no transition, so these components do not add state animation.
