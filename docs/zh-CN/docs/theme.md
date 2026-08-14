---
order: -4
---

# 主题

所有组件都支持内置主题系统。[ActiveTheme] trait 用于访问当前主题中的颜色值：

```rs
use hearth_gpui::{ActiveTheme as _};

// Access theme colors in your components
cx.theme().primary
cx.theme().background
cx.theme().foreground
```

因此，如果你希望组件使用当前主题的颜色，组件或视图就需要运行在带有 [App] 上下文的环境中。

## Color Theme 与 Style Preset

Color Theme 与 Style Preset 是两个独立输入。Color Theme 管理语义颜色、背景、语法高亮和字体；Style Preset 管理组件几何、密度、圆角、焦点环几何、阴影、Overlay 间距和动画。组件只读取全局 `Theme` 中已经解析的值：

```rs
use gpui::{App, SharedString};
use hearth_gpui::{StylePreset, StyleRegistry, Theme};

pub fn configure_style(cx: &mut App) -> anyhow::Result<()> {
    // Vega 是默认 preset；Nova 更紧凑，Maia 更宽松。
    Theme::set_style("nova", cx)?;

    // 自定义 preset 在 Rust 中注册，并在可用前完成校验。
    let mut custom = StylePreset::vega();
    custom.id = SharedString::from("product");
    custom.name = SharedString::from("Product");
    StyleRegistry::register(custom, cx)?;
    Theme::set_style("product", cx)
}
```

调用 `Theme::set_style` 不会改变当前 Color Theme 和语法高亮；通过 `Theme::set_color_theme` 或 `Theme::apply_config` 应用 Color Theme 也不会改变当前 Style Preset。Style Preset 不从 JSON 加载，也不支持继承或热重载。注册时会拒绝空标识、非有限值或负数指标、顺序错误的控件高度、圆角、数据行高或动画时长、无效 Overlay scale，以及重复 id。

### 从扁平外观字段迁移

原有的 `Theme.radius`、`Theme.radius_lg`、`Theme.shadow` 及对应 Theme JSON 字段已经移除。组件代码应改用 `cx.theme().style.radii.md`、`cx.theme().style.controls`、`cx.theme().style.elevation.enabled` 等语义指标。Theme JSON 仅保留颜色、字体、语法高亮和运行时主题设置。

## 渐变背景

主题颜色值继续兼容既有的字符串格式：

```json
{
  "colors": {
    "button.primary.background": "#4F46E5"
  }
}
```

支持渐变渲染的背景 token 也可以使用 CSS 风格的两段线性渐变：

```json
{
  "colors": {
    "button.primary.background": "linear-gradient(135deg, #4F46E5, #06B6D4)",
    "button.primary.hover.background": "linear-gradient(to right, red-500 25%, blue-600 75%)"
  }
}
```

`cx.theme().button_primary` 等顶层字段仍然是纯色 `Hsla`，保持兼容。需要完整 resolved token 时使用 `cx.theme().tokens.button_primary`；其中 `.color` 是纯色代表色，`.background` 是实际配置的 `Background`，包含渐变。

## Theme Registry

仓库在 [themes](https://github.com/JonZhang3/hearth-gpui/tree/main/themes) 目录下内置了 20+ 主题。

你可以通过 [ThemeRegistry] 来加载和监听这些主题文件：

从 registry 查找主题时使用 `themes` 数组中条目的 `name`，例如 `Ayu Light`。

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
