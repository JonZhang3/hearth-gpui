---
title: Icon
description: 以不同尺寸、颜色和变换方式显示 SVG 图标。
---

# Icon

Icon 是一个灵活的图标组件，可渲染内置图标库或自定义资源路径中的 SVG 图标。图标基于 Lucide.dev，并支持尺寸、颜色、变换与无障碍语义定制。

在开始之前，建议先阅读 [Icons & Assets](../assets.md)，了解如何在 GPUI 与 GPUI Component 应用中使用 SVG。

## 导入

```rust
use gpui_component::{Icon, IconName};
```

## 用法

### 基础图标

```rust
IconName::Heart

Icon::new(IconName::Heart)
```

### 自定义尺寸

```rust
Icon::new(IconName::Search).xsmall()
Icon::new(IconName::Search).small()
Icon::new(IconName::Search).with_size(Size::Medium)
Icon::new(IconName::Search).large()

Icon::new(IconName::Search).with_size(px(20.))
```

### 自定义颜色

```rust
Icon::new(IconName::Heart)
    .text_color(cx.theme().red)

Icon::new(IconName::Star)
    .text_color(gpui::red())
```

### 旋转图标

```rust
use gpui::{radians, Transformation};
use std::f32::consts::{FRAC_PI_2, PI};

Icon::new(IconName::ArrowUp)
    .rotate(radians(FRAC_PI_2))

Icon::new(IconName::ChevronRight)
    .transform(Transformation::rotate(radians(PI)))
```

### 自定义 SVG 路径

```rust
Icon::empty()
    .path("icons/my-custom-icon.svg")
```

### 信息型图标

图标默认作为装饰元素，不进入无障碍树。仅当图标表达了可见文字中不存在的信息时，才使用 `informative`：

```rust
Icon::informative(
    "connection-status-icon",
    IconName::Wifi,
    "已连接",
)
```

## 可用图标

`IconName` 枚举内置了一组常见图标：

### 导航

- `ArrowUp`、`ArrowDown`、`ArrowLeft`、`ArrowRight`
- `ChevronUp`、`ChevronDown`、`ChevronLeft`、`ChevronRight`
- `ChevronsUpDown`

### 操作

- `Check`、`Close`、`Plus`、`Minus`
- `Copy`、`Delete`、`Search`、`Replace`
- `Maximize`、`Minimize`、`WindowRestore`

### 文件与文件夹

- `File`、`Folder`、`FolderOpen`、`FolderClosed`
- `BookOpen`、`Inbox`

### UI 元素

- `Menu`、`Settings`、`Settings2`、`Ellipsis`、`EllipsisVertical`
- `Eye`、`EyeOff`、`Bell`、`Info`

### 社交与外链

- `Github`、`Globe`、`ExternalLink`
- `Heart`、`HeartOff`、`Star`、`StarOff`
- `ThumbsUp`、`ThumbsDown`

### 状态与提醒

- `CircleCheck`、`CircleX`、`TriangleAlert`
- `Loader`、`LoaderCircle`

### 面板与布局

- `PanelLeft`、`PanelRight`、`PanelBottom`
- `PanelLeftOpen`、`PanelRightOpen`、`PanelBottomOpen`
- `LayoutDashboard`、`Frame`

### 用户与身份

- `User`、`CircleUser`、`Bot`

### 其它

- `Calendar`、`Map`、`Palette`、`Inspector`
- `Sun`、`Moon`、`Building2`

## 图标尺寸

| 尺寸 | 方法 | CSS Class | 像素 |
| ----------- | --------------------- | ------------ | ------ |
| 超小 | `.xsmall()` | `size_3()` | 12px |
| 小 | `.small()` | `size_3p5()` | 14px |
| 中 | `.with_size(Size::Medium)` | `size_4()` | 16px |
| 大 | `.large()` | `size_6()` | 24px |
| 自定义 | `.with_size(px(n))` | - | n px |

## 自定义 `IconName`

如果你需要更贴合业务的图标命名，可以自己定义 `IconName` 并实现 `IconNamed` trait。

```rust
use gpui_component::IconNamed;

pub enum IconName {
    Encounters,
    Monsters,
    Spells,
}

impl IconNamed for IconName {
    fn path(self) -> gpui::SharedString {
        match self {
            IconName::Encounters => "icons/encounters.svg",
            IconName::Monsters => "icons/monsters.svg",
            IconName::Spells => "icons/spells.svg",
        }
        .into()
    }
}

Button::new("my-button").icon(IconName::Spells);
Icon::new(IconName::Monsters);
```

如果你希望在元素树中直接 `render` 自定义 `IconName`，还需要实现 `RenderOnce` 并为 `IconName` 派生 `IntoElement`：

```rust
impl RenderOnce for IconName {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::empty().path(self.path())
    }
}

div()
    .child(IconName::Monsters)
```

## 示例

### 按钮中的图标

```rust
use gpui_component::button::Button;

Button::new("like-btn")
    .icon(
        Icon::new(IconName::Heart)
            .text_color(cx.theme().red)
            .large()
    )
    .label("Like")
```

### 旋转加载图标

```rust
Icon::new(IconName::LoaderCircle)
    .text_color(cx.theme().muted_foreground)
```

### 状态图标

```rust
Icon::new(IconName::CircleCheck)
    .text_color(cx.theme().green)

Icon::new(IconName::CircleX)
    .text_color(cx.theme().red)

Icon::new(IconName::TriangleAlert)
    .text_color(cx.theme().yellow)
```

### 导航图标

```rust
Icon::new(IconName::ArrowLeft)
    .with_size(Size::Medium)
    .text_color(cx.theme().foreground)

Icon::new(IconName::ChevronDown)
    .small()
    .text_color(cx.theme().muted_foreground)
```

### 来自资源包的自定义图标

```rust
Icon::empty()
    .path("icons/my-brand-logo.svg")
    .large()
    .text_color(cx.theme().primary)
```

## 说明

- 图标以 SVG 形式渲染，可使用完整的样式能力。
- 如果未显式指定尺寸和颜色，它们会跟随当前文字样式。
- `Sizable` 方法会设置固定的正方形尺寸；显式 `.w(...)` 和 `.h(...)` 样式分别覆盖对应轴。
- `Icon::empty()` 在设置自定义路径前只渲染占位空间，不会查询资源加载器。
- 图标默认是装饰性的；有独立语义的图标应使用 `Icon::informative(...)`。
- 图标默认带有 `flex-shrink-0`，避免在 Flex 布局中被意外压缩。
- 所有图标路径都相对于 assets bundle 根目录。
- Lucide.dev 图标在 16px 下效果最佳，并且在其它尺寸下也有良好缩放表现。
