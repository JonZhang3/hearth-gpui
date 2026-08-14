# hearth-gpui Usage Guide

**Contents:** [Setup](#setup) · [Component Types](#component-types) · [Common Components](#common-components) (Button, Input, Select, Checkbox, Icon, Dialog, Notification, Tabs, Tooltip, Form, List) · [Theming](#theming) · [Layout Helpers](#layout-helpers) · [Overlay Layers](#overlay-layers-dialogs-sheets-notifications) · [Shared Traits](#shared-traits)

## Setup

### 1. Cargo.toml

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
hearth-gpui = { git = "https://github.com/JonZhang3/hearth-gpui" }
hearth-gpui-assets = { git = "https://github.com/JonZhang3/hearth-gpui" } # optional icons
```

### 2. Initialization

```rust
fn main() {
    gpui_platform::application()
        .with_assets(hearth_gpui_assets::Assets)
        .run(move |cx| {
            hearth_gpui::init(cx); // MUST be first

            cx.spawn(async move |cx| {
                cx.open_window(WindowOptions::default(), |window, cx| {
                    let view = cx.new(|_| MyApp);
                    cx.new(|cx| Root::new(view, window, cx)) // Root wraps first view
                }).expect("Failed to open window");
            }).detach();
        });
}
```

**`Root` is required** as the first-level child of every window — it enables dialogs, sheets, and notifications.

---

## Component Types

### Stateless (most components)

Used directly in `render`, no stored state:

```rust
use hearth_gpui::button::Button;

impl Render for MyView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Button::new("btn").label("Submit")
            .on_click(|_, _, _| println!("clicked"))
    }
}
```

### Stateful (Input, Select, Combobox, etc.)

Require an `Entity<State>` stored in your view:

```rust
use hearth_gpui::input::{Input, InputState};

struct MyView {
    name: Entity<InputState>,
}

impl MyView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            name: cx.new(|cx| InputState::new(window, cx).placeholder("Your name")),
        }
    }
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.name)
    }
}
```

---

## Common Components

### Button

```rust
use hearth_gpui::{button::{Button, ButtonGroup}, spinner::Spinner, IconName};

// shadcn variants. Default is the primary action style.
Button::new("btn").label("Default")
Button::new("btn").outline().label("Outline")
Button::new("btn").secondary().label("Secondary")
Button::new("btn").destructive().label("Delete")
Button::new("btn").ghost().label("Ghost")
Button::new("btn").link().label("Link")

// States
Button::new("btn").label("Text").disabled(true)
Button::new("btn").label("Text").pressed(true)

// Loading is explicit composition.
Button::new("btn")
    .icon(Spinner::new())
    .label("Saving")
    .disabled(true)

// With icon
Button::new("btn").icon(IconName::Plus).label("Add")

// Sizes
Button::new("btn").xsmall().label("XS")
Button::new("btn").small().label("S")
Button::new("btn").large().label("L")

// Group
ButtonGroup::new("group")
    .aria_label("Document actions")
    .child(Button::new("save").outline().label("Save"))
    .child(Button::new("share").outline().label("Share"))
```

### Input

```rust
use hearth_gpui::input::{Input, InputState};

// State setup (in new/init)
let input = cx.new(|cx| InputState::new(window, cx)
    .placeholder("Enter text...")
    .default_value("Hello")
);

// Render
Input::new(&input)
Input::new(&input).cleanable(true)           // clear button
Input::new(&input).disabled(true)
Input::new(&input).read_only(true)           // focus, selection, and copy still work
Input::new(&input).invalid(true)
Input::new(&input).aria_label("Account name")
Input::new(&input).prefix(Icon::new(IconName::Search).small())
Input::new(&input).suffix(Button::new("b").ghost().icon(IconName::X).xsmall())
Input::new(&input).content_type(InputContentType::Password)
Input::new(&input).mask_toggle()             // password reveal toggle
Input::new(&input).appearance(false)         // remove default border/bg

// Reading value
let value = input.read(cx).value();

// Events
cx.subscribe_in(&input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => { let v = state.read(cx).value(); }
        InputEvent::PressEnter { .. } => { /* submit */ }
        InputEvent::Focus | InputEvent::Blur => {}
    }
});
```

### Select

```rust
use hearth_gpui::select::{Select, SelectState};

// Simple string list
let state = cx.new(|cx| {
    SelectState::new(vec!["Apple", "Orange", "Banana"], Some(IndexPath::default()), window, cx)
});

// Render
Select::new(&state)
Select::new(&state).placeholder("Pick one")
Select::new(&state).invalid(true).aria_label("Fruit")

// Reading selection
let selected = state.read(cx).selected_value();
```

### Checkbox / Switch / Radio

```rust
use hearth_gpui::{checkbox::Checkbox, switch::Switch};

// Stateless (controlled)
Checkbox::new("cb").checked(self.checked)
    .on_click(|checked, _, cx| { /* &bool */ })

Switch::new("sw").checked(self.enabled)
    .on_click(|checked, _, cx| {})
```

### Icon

```rust
use hearth_gpui::{Icon, IconName};

Icon::new(IconName::Check)
Icon::new(IconName::Search).small()
Icon::new(IconName::Plus).large().text_color(cx.theme().primary)
```

### Dialog

```rust
use hearth_gpui::{
    WindowExt as _,
    button::Button,
    dialog::{DialogAction, DialogClose, DialogFooter},
};

// Open from window context
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Confirm")
        .description("Review the action before continuing.")
        .child(div().child("Are you sure?"))
        .footer(|_, _| {
            DialogFooter::new()
                .child(DialogClose::new(
                    Button::new("cancel").outline().label("Cancel"),
                ))
                .child(DialogAction::new(Button::new("ok").label("OK")))
        })
});
```

### Notification

```rust
// Simple string message
window.push_notification("Saved successfully!", cx);

// With type variant
window.push_notification(Notification::success("Upload complete"), cx);
```

### Tabs

```rust
use hearth_gpui::tab::{Tab, TabBar};

TabBar::new("tabs")
    .child(Tab::new().label("Overview"))
    .child(Tab::new().label("Settings"))
    .child(Tab::new().label("Logs"))
```

### Tooltip

```rust
// On any element with .id(), add .tooltip():
div()
    .id("my-btn")
    .tooltip(|window, cx| Tooltip::new("Delete item").build(window, cx))
    .child("Delete")

// Or on a Button directly:
Button::new("btn").icon(IconName::Trash).tooltip("Delete")
```

### Form

```rust
use hearth_gpui::form::{field, h_form, v_form, FieldBody, FieldContent, FieldLabel};

// Vertical form
v_form()
    .child(
        field("name")
            .aria_label("Name")
            .content(move |_| FieldBody::new()
                .child(FieldLabel::new("Name"))
                .child(FieldContent::new().child(Input::new(&self.name)))),
    )
    .child(
        field("email")
            .aria_label("Email")
            .content(move |_| FieldBody::new()
                .child(FieldLabel::new("Email"))
                .child(FieldContent::new().child(Input::new(&self.email)))),
    )

// Horizontal label alignment
h_form()
    .child(
        field("username")
            .aria_label("Username")
            .content(move |_| FieldBody::new()
                .child(FieldLabel::new("Username"))
                .child(FieldContent::new().child(Input::new(&self.username)))),
    )
```

### List (searchable, virtualized)

```rust
use hearth_gpui::list::{List, ListState, ListDelegate, ListItem, ListEvent};

// Implement ListDelegate for your data type, then:
let list_state = cx.new(|cx| ListState::new(MyDelegate::new(), window, cx));

// Render
List::new(&list_state)
// Events
cx.subscribe(&list_state, |this, _, event, cx| {
    if let ListEvent::Select(index_path) = event {
        // handle selection
    }
});
```

---

## Theming

Color Themes own colors, typography, and syntax highlighting. Style Presets own density,
geometry, radii, focus treatment, elevation, overlay spacing, and motion. Vega is the default;
Nova is compact and Maia is comfortable.

```rust
use hearth_gpui::ActiveTheme as _;

// Access colors
cx.theme().primary
cx.theme().background
cx.theme().foreground
cx.theme().border
cx.theme().surface
cx.theme().muted
cx.theme().destructive

// Access semantic Style Preset metrics
cx.theme().style.controls.md.height
cx.theme().style.radii.md
cx.theme().style.motion.normal()

// Use in styles
div()
    .bg(cx.theme().surface)
    .text_color(cx.theme().foreground)
    .border_color(cx.theme().border)
```

### Switch Color Theme and Style Preset Independently

```rust
use hearth_gpui::Theme;

// Changes geometry and motion without changing colors or syntax highlighting.
Theme::set_style("nova", cx)?;

// Changes colors and typography without changing the active Style Preset.
Theme::set_color_theme(theme_config, cx);
```

Component code must consume semantic fields from `cx.theme().style`; it must never branch on the
`vega`, `nova`, or `maia` preset id. Reduced Motion is an application accessibility preference and
must render the final state without delayed unmount.

---

## Layout Helpers

hearth-gpui extends GPUI with convenient layout methods:

```rust
h_flex()    // div().flex().flex_row().items_center()
v_flex()    // div().flex().flex_col()

// Common patterns
h_flex().gap_2().items_center()
    .child(Icon::new(IconName::User))
    .child(label("Username"))

v_flex().gap_4().p_4()
    .child(Input::new(&self.name))
    .child(Input::new(&self.email))
    .child(Button::new("submit").label("Submit"))
```

---

## Overlay Layers (Dialogs, Sheets, Notifications)

To render overlays, add these to your first-level view's render:

```rust
impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(self.main_content(window, cx))
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
```

---

## Shared Traits

All components follow the builder pattern `Component::new("id").method().method()`:
- `Sizable`: `.xsmall()` / `.small()` / `.medium()` (default) / `.large()`
- `Disableable`: `.disabled(bool)`
- `Selectable`: `.selected(bool)`
- `Styled`: any GPUI style methods (`.w()`, `.bg()`, `.p_2()`, etc.)

For any component not covered here, fetch its doc from:
`https://jonzhang3.github.io/hearth-gpui/docs/components/{name}.md`
