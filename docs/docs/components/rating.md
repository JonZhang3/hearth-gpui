---
title: Rating
description: A theme-aware, accessible star rating component.
---

# Rating

A theme-aware star rating component that supports pointer and keyboard selection, custom colors, disabled and read-only states, and all semantic component sizes.

## Import

```rust
use gpui_component::rating::Rating;
```

## Usage

### Basic Rating

```rust
Rating::new("my-rating")
    .aria_label("Product rating")
    .value(3)
    .max(5)
    .on_click(|value, _, _| {
        println!("Rating changed to: {}", value);
    })
```

### Controlled Rating

```rust
struct MyView {
    rating: usize,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Rating::new("rating")
            .value(self.rating)
            .max(5)
            .on_click(cx.listener(|view, value: &usize, _, cx| {
                view.rating = *value;
                cx.notify();
            }))
    }
}
```

### Different Sizes

The Rating component supports the [Sizable] trait for different sizes.

```rust
Rating::new("rating").xsmall().value(3).max(5)
Rating::new("rating").small().value(3).max(5)
Rating::new("rating").value(3).max(5) // default (Medium)
Rating::new("rating").large().value(3).max(5)
```

### Custom Color

By default, the rating uses the theme's `yellow` color. You can customize it with the `color` method.

```rust
Rating::new("rating")
    .value(4)
    .max(5)
    .color(cx.theme().green)
```

### Disabled State

```rust
Rating::new("rating")
    .value(2)
    .max(5)
    .disabled(true)
```

### Read-only State

Use `read_only(true)` to present a value without reducing its visual emphasis or allowing interaction.

```rust
Rating::new("rating")
    .aria_label("Average customer rating")
    .value(4)
    .read_only(true)
```

### Custom Maximum

The default maximum is 5 stars, but you can set a different maximum value.

```rust
Rating::new("rating")
    .value(7)
    .max(10)
```

### Click Behavior

The rating component has the following pointer behavior:

- Clicking a different star selects that exact value.
- Clicking the current final star reduces the value by one.
- Hovering previews both higher and lower values without committing them.

The `on_click` callback receives the new rating value as `&usize`.

```rust
Rating::new("rating")
    .value(3)
    .max(5)
    .on_click(|new_value, _, _| {
        println!("New rating: {}", new_value);
    })
```

### Keyboard and Accessibility

Rating is exposed as a horizontal slider with a numeric range from `0` to `max`.

- `Left` / `Down`: decrease by one
- `Right` / `Up`: increase by one
- `Home`: set to zero
- `End`: set to the maximum

Use `aria_label(...)` to describe the rated subject. Disabled and read-only states are exposed to assistive technology.

## Theme and Style Presets

- Active stars use the Color Theme's `yellow` color unless `color(...)` overrides it.
- Inactive stars use `muted_foreground`.
- Item padding, spacing, focus ring, and radius consume semantic Style Preset metrics.
- Custom `Styled` refinements remain authoritative on the outer Rating element.

## API Reference

- [Rating]

### Methods

- `new(id: impl Into<ElementId>)` - Create a new Rating component
- `with_size(size: impl Into<Size>)` - Set the star size (implements [Sizable])
- `value(value: usize)` - Set the initial rating value (0..=max)
- `max(max: usize)` - Set the maximum number of stars (default: 5)
- `color(color: impl Into<Hsla>)` - Set the active color (default: theme yellow)
- `aria_label(label: impl Into<SharedString>)` - Set the accessible name
- `disabled(disabled: bool)` - Disable interaction (implements [Disableable])
- `read_only(read_only: bool)` - Present a non-interactive value without disabled styling
- `on_click(handler: Fn(&usize, &mut Window, &mut App))` - Set click handler

## Examples

### Read-only Display

```rust
Rating::new("rating")
    .value(4)
    .max(5)
    .read_only(true)
```

### Interactive Rating with State

```rust
struct ProductView {
    user_rating: usize,
}

impl Render for ProductView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                Rating::new("product-rating")
                    .value(self.user_rating)
                    .max(5)
                    .on_click(cx.listener(|view, value: &usize, _, cx| {
                        view.user_rating = *value;
                        // Save rating to backend, etc.
                        cx.notify();
                    }))
            )
            .child(format!("Your rating: {}/5", self.user_rating))
    }
}
```

### Large Rating with Custom Color

```rust
Rating::new("rating")
    .large()
    .value(5)
    .max(5)
    .color(cx.theme().orange)
```

[Rating]: https://docs.rs/gpui-component/latest/gpui_component/rating/struct.Rating.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
[Disableable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Disableable.html
