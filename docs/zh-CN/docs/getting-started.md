---
title: 开始使用
description: 学习如何在项目中安装并使用 Hearth GPUI。
order: -2
---

# 开始使用

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
hearth-gpui = { git = "https://github.com/JonZhang3/hearth-gpui" }
# 可选：使用内置默认资源
hearth-gpui-assets = { git = "https://github.com/JonZhang3/hearth-gpui" }
anyhow = "1.0"
```

:::tip
`hearth-gpui-assets` 是可选依赖。

如果你希望自行管理图标与资源文件，可以不添加它。更多说明见 [资源与图标](./assets.md)。
:::

## 快速开始

下面是一个最小可运行示例：

```rust
use gpui::*;
use hearth_gpui::{button::*, *};

pub struct HelloWorld;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(hearth_gpui_assets::Assets);

    app.run(move |cx| {
        hearth_gpui::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

:::info
请确保在 `app.run` 闭包中尽早调用 `hearth_gpui::init(cx);`。它会初始化主题和全局配置。
:::

## 主题与样式预设

Color Theme 负责颜色、排版和语法高亮；Style Preset 负责组件密度、尺寸、圆角、阴影、
Focus 处理和动效。Vega 是默认 Style Preset，Nova 和 Maia 是可选预设。

```rust
use hearth_gpui::{ActiveTheme as _, Theme};

cx.theme().primary;
cx.theme().style.controls.md.height;
cx.theme().style.radii.md;

Theme::set_style("nova", cx)?;
```

`Theme::set_style` 不会改变当前 Color Theme。组件应读取 `cx.theme().style` 中的语义化 metrics，
不应按 `vega`、`nova` 或 `maia` ID 分支。

## Button variants

Default variant 用于主操作：

```rust
Button::new("default");
Button::new("outline").outline();
Button::new("secondary").secondary();
Button::new("delete").destructive();
Button::new("ghost").ghost();
Button::new("link").link();
```

加载状态使用 `Spinner` 与 Button 显式组合，并在任务执行期间禁用 Button。

## 后续阅读

- [组件总览](./components/index)
- [资源与图标](./assets.md)

