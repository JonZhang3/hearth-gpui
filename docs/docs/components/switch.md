---
title: Switch
description: A control that allows the user to toggle between checked and not checked.
---

# Switch

A toggle switch component for binary on/off states. Features smooth animations, different sizes, labels, disabled state, and customizable positioning.

## Import

```rust
use hearth_gpui::switch::Switch;
```

## Usage

### Basic Switch

```rust
Switch::new("my-switch")
    .aria_label("Airplane mode")
    .checked(false)
    .on_click(|checked, _, _| {
        println!("Switch is now: {}", checked);
    })
```

### Controlled Switch

```rust
struct MyView {
    is_enabled: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Switch::new("switch")
            .aria_label("Enable feature")
            .checked(self.is_enabled)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.is_enabled = *checked;
                cx.notify();
            }))
    }
}
```

### With Label

```rust
Switch::new("notifications")
    .label("Enable notifications")
    .checked(true)
    .on_click(|checked, _, _| {
        println!("Notifications: {}", if *checked { "enabled" } else { "disabled" });
    })
```

### Different Sizes

```rust
// Small switch
Switch::new("small-switch")
    .small()
    .label("Small switch")

// Medium switch (default)
Switch::new("medium-switch")
    .label("Medium switch")

// Using explicit size
Switch::new("custom-switch")
    .with_size(Size::Small)
    .label("Custom size")
```

### Disabled State

```rust
// Disabled unchecked
Switch::new("disabled-off")
    .label("Disabled (off)")
    .disabled(true)
    .checked(false)

// Disabled checked
Switch::new("disabled-on")
    .label("Disabled (on)")
    .disabled(true)
    .checked(true)
```

### Invalid and Accessible Name

```rust
Switch::new("required-setting")
    .aria_label("Enable required setting")
    .invalid(true)
    .checked(false)
```

Enabled switches with an `on_click` handler are keyboard focusable. Press `Space` to toggle the
value. A Switch without an activation handler is presented as read-only and does not create a dead
Tab stop. The focus ring is shown for keyboard focus, while pointer activation does not add a
keyboard focus-visible ring.

### Custom Color

Use `.color()` to override the checked-state background color. The disabled alpha is applied automatically on top of the custom color.

```rust
// Success color when checked
Switch::new("switch")
    .label("Success")
    .checked(true)
    .color(cx.theme().success)

// Danger color when checked
Switch::new("switch")
    .label("Danger")
    .checked(true)
    .color(cx.theme().danger)

// Custom color + disabled: color is shown at 50% opacity
Switch::new("switch")
    .label("Disabled")
    .checked(true)
    .color(cx.theme().success)
    .disabled(true)
```

### With Tooltip

```rust
Switch::new("switch")
    .label("Airplane mode")
    .tooltip("Enable airplane mode to disable all wireless connections")
    .checked(false)
```

## API Reference

### Switch

| Method             | Description                                                 |
| ------------------ | ----------------------------------------------------------- |
| `new(id)`          | Create a new switch with the given ID                       |
| `checked(bool)`    | Set the checked/toggled state                               |
| `label(text)`      | Set label text for the switch                               |
| `aria_label(text)` | Set an accessible name independently from the visible label |
| `label_side(side)` | Position label (Side::Left or Side::Right)                  |
| `invalid(bool)`    | Set the destructive invalid presentation and semantics      |
| `tab_stop(bool)`   | Include or exclude the switch from sequential focus          |
| `tab_index(index)` | Set the keyboard tab index                                   |
| `disabled(bool)`   | Set disabled state                                          |
| `tooltip(text)`    | Add tooltip text                                            |
| `color(color)`     | Set background color when checked (default: `theme.primary`) |
| `on_click(fn)`     | Callback when clicked, receives `&bool` (new checked state) |

### Styling

Implements `Sizable` and `Disableable` traits:

- `small()` - shadcn small switch size (`24x14px`, `12px` thumb)
- `medium()` - shadcn default switch size (`32x18.4px`, `16px` thumb)
- `with_size(size)` - Set explicit size
- `disabled(bool)` - Disabled state

### Styling Properties

The switch can also be styled using GPUI's styling methods:

- `w(width)` - Custom width
- `h(height)` - Custom height
- Standard margin, padding, and positioning methods

## Examples

### Settings Panel

```rust
struct SettingsView {
    marketing_emails: bool,
    security_emails: bool,
    push_notifications: bool,
}

v_flex()
    .gap_4()
    .child(
        // Setting with description
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .child(Label::new("Marketing emails").text_lg())
                            .child(
                                Label::new("Receive emails about new products and features")
                                    .text_color(theme.muted_foreground)
                            )
                    )
                    .child(
                        Switch::new("marketing")
                            .checked(self.marketing_emails)
                            .on_click(cx.listener(|view, checked, _, cx| {
                                view.marketing_emails = *checked;
                                cx.notify();
                            }))
                    )
            )
    )
    .child(
        // Simple setting
        h_flex()
            .items_center()
            .justify_between()
            .child(Label::new("Push notifications"))
            .child(
                Switch::new("push")
                    .checked(self.push_notifications)
                    .on_click(cx.listener(|view, checked, _, cx| {
                        view.push_notifications = *checked;
                        cx.notify();
                    }))
            )
    )
```

### Compact Settings List

```rust
v_flex()
    .gap_3()
    .child(
        Switch::new("wifi")
            .label("Wi-Fi")
            .label_side(Side::Left)
            .checked(true)
            .small()
    )
    .child(
        Switch::new("bluetooth")
            .label("Bluetooth")
            .label_side(Side::Left)
            .checked(false)
            .small()
    )
    .child(
        Switch::new("airplane")
            .label("Airplane Mode")
            .label_side(Side::Left)
            .checked(false)
            .disabled(true)
            .small()
    )
```

### Form Integration

```rust
struct FormData {
    subscribe_newsletter: bool,
    enable_notifications: bool,
    remember_me: bool,
}

v_flex()
    .gap_4()
    .p_4()
    .border_1()
    .border_color(theme.border)
    .rounded(theme.radius)
    .child(
        Switch::new("newsletter")
            .label("Subscribe to newsletter")
            .checked(self.subscribe_newsletter)
            .tooltip("Receive monthly updates about new features")
            .on_click(cx.listener(|view, checked, _, cx| {
                view.subscribe_newsletter = *checked;
                cx.notify();
            }))
    )
    .child(
        Switch::new("notifications")
            .label("Enable notifications")
            .checked(self.enable_notifications)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.enable_notifications = *checked;
                cx.notify();
            }))
    )
    .child(
        Switch::new("remember")
            .label("Remember me")
            .checked(self.remember_me)
            .small()
            .on_click(cx.listener(|view, checked, _, cx| {
                view.remember_me = *checked;
                cx.notify();
            }))
    )
```

### Custom Styling

```rust
Switch::new("custom")
    .label("Custom styled switch")
    .w(px(200.))
    .checked(true)
    .on_click(|checked, _, _| {
        println!("Custom switch: {}", checked);
    })
```

## Animation

The switch uses the active Style Preset motion tokens:

- Track background, border, focus ring, disabled opacity, and Thumb position share one sampled
  150ms normal transition so their frames remain synchronized.
- The Thumb translates between its checked and unchecked positions without conditional mounting.
- Rapid reversal resumes from the currently sampled value instead of restarting from an endpoint.
- Reduced motion resolves immediately to the final state.
- Renderable gradient backgrounds switch atomically because GPUI cannot interpolate arbitrary fills.
