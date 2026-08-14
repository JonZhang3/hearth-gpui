---
order: -4
---

# Theme

All components support theming through the built-in Theme system, the [ActiveTheme] trait provides access to the current theme colors:

```rs
use hearth_gpui::{ActiveTheme as _};

// Access theme colors in your components
cx.theme().primary
cx.theme().background
cx.theme().foreground
```

So if you want use the colors from the current theme, you should keep your component or view have [App] context.

## Color Themes and Style Presets

Color Themes and Style Presets are independent inputs. A Color Theme owns semantic colors,
backgrounds, syntax highlighting, and typography. A Style Preset owns component geometry,
density, radii, focus-ring geometry, elevation, overlay spacing, and motion. Components read the
resolved values from the single global `Theme`:

```rs
use gpui::{App, SharedString};
use hearth_gpui::{StylePreset, StyleRegistry, Theme};

pub fn configure_style(cx: &mut App) -> anyhow::Result<()> {
    // Vega is the default. Nova is compact and Maia is comfortable.
    Theme::set_style("nova", cx)?;

    // Custom presets are registered in Rust and validated before becoming available.
    let mut custom = StylePreset::vega();
    custom.id = SharedString::from("product");
    custom.name = SharedString::from("Product");
    StyleRegistry::register(custom, cx)?;
    Theme::set_style("product", cx)
}
```

Calling `Theme::set_style` preserves the selected Color Theme and syntax highlighting. Applying a
Color Theme with `Theme::set_color_theme` or `Theme::apply_config` preserves the selected Style
Preset. Style Presets are intentionally not loaded from JSON and do not support inheritance or hot
reload. Registration rejects empty identifiers, non-finite or negative metrics, unordered control
heights, radii, data-row heights or motion durations, invalid overlay scale, and duplicate ids.

### Migration from flat appearance fields

The former `Theme.radius`, `Theme.radius_lg`, and `Theme.shadow` fields and the corresponding Theme
JSON keys were removed. Component code should use semantic resolved metrics such as
`cx.theme().style.radii.md`, `cx.theme().style.controls`, and
`cx.theme().style.elevation.enabled`. Theme JSON files should contain colors, typography, syntax,
and runtime theme settings only.

## Gradient Backgrounds

Theme color values remain backward compatible with the existing string format:

```json
{
  "colors": {
    "button.primary.background": "#4F46E5"
  }
}
```

Background tokens that opt in to gradient rendering can also use CSS-style two-stop linear gradients:

```json
{
  "colors": {
    "button.primary.background": "linear-gradient(135deg, #4F46E5, #06B6D4)",
    "button.primary.hover.background": "linear-gradient(to right, red-500 25%, blue-600 75%)"
  }
}
```

Top-level theme fields, such as `cx.theme().button_primary`, remain solid `Hsla` values for compatibility. Code that needs the full resolved token can use `cx.theme().tokens.button_primary`; its `.color` field is the solid representative color, and its `.background` field contains the configured `Background`, including gradients.

## Theme Registry

There have more than 20 built-in themes available in [themes](https://github.com/JonZhang3/hearth-gpui/tree/main/themes) folder.

https://github.com/JonZhang3/hearth-gpui/tree/main/themes

And we have a [ThemeRegistry] to help us to load themes.

Use the `name` of an entry in the `themes` array, such as `Ayu Light`, when looking up a theme from the registry.

```rs
use std::path::PathBuf;
use gpui::{App, SharedString};
use hearth_gpui::{Theme, ThemeRegistry};

pub fn init(cx: &mut App) {
    let theme_name = SharedString::from("Ayu Light");
    // Load and watch themes from ./themes directory
    if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
        if let Some(theme) = ThemeRegistry::global(cx)
            .themes()
            .get(&theme_name)
            .cloned()
        {
            Theme::global_mut(cx).apply_config(&theme);
        }
    }) {
        tracing::error!("Failed to watch themes directory: {}", err);
    }
}
```

[ActiveTheme]: https://docs.rs/hearth-gpui/latest/hearth_gpui/theme/trait.ActiveTheme.html
[ThemeRegistry]: https://docs.rs/hearth-gpui/latest/hearth_gpui/theme/struct.ThemeRegistry.html
[StyleRegistry]: https://docs.rs/hearth-gpui/latest/hearth_gpui/theme/struct.StyleRegistry.html
[App]: https://docs.rs/gpui/latest/gpui/struct.App.html
