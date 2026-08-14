---
title: Badge
description: 用于展示状态或元数据的紧凑标签，并支持覆盖式角标。
---

# Badge

`Badge` 是与 shadcn Vega 对齐的非交互内联标签。当数字、圆点或图标需要覆盖在另一个元素上时，使用 `OverlayBadge`。

## 导入

```rust
use hearth_gpui::{
    badge::{Badge, BadgeVariants as _, OverlayBadge},
    Sizable as _,
};
```

## Variants

```rust
Badge::new().child("Default")
Badge::new().secondary().child("Secondary")
Badge::new().destructive().child("Destructive")
Badge::new().outline().child("Outline")
Badge::new().ghost().child("Ghost")
Badge::new().link().child("Link")
```

所有 variants 都使用 Color Theme 语义颜色，并自动适配 light 和 dark 模式。

## 图标和 Spinner

使用 `leading` 和 `trailing` 添加紧凑的边缘槽位。槽位应用 Vega 图标间距，不改变 Badge 高度。

```rust
Badge::new()
    .leading(Icon::new(IconName::CircleCheck).xsmall())
    .child("Verified")

Badge::new()
    .secondary()
    .child("Continue")
    .trailing(Icon::new(IconName::ArrowRight).xsmall())

Badge::new()
    .outline()
    .leading(Spinner::new().xsmall())
    .child("Generating")
```

## 自定义颜色

`Badge` 实现了 `Styled`。额外的状态色或分类颜色应直接覆盖样式，不需要增加组件专属 variant。

```rust
Badge::new()
    .bg(cx.theme().success)
    .text_color(cx.theme().success_foreground)
    .child("Success")

Badge::new()
    .outline()
    .border_color(cx.theme().warning)
    .text_color(cx.theme().warning)
    .child("Pending")
```

## 交互

Badge 是纯展示组件，不提供 click、focus 或 Button 语义。需要交互时，将它组合到 GPUI `Link` 或 `Button` 中；这是 shadcn `asChild` 在 GPUI 中的原生等价方案。

## OverlayBadge

`OverlayBadge` 将数字、状态圆点或图标定位到目标元素上。

### 数字

```rust
OverlayBadge::new()
    .count(5)
    .child(Icon::new(IconName::Bell).large())

OverlayBadge::new()
    .count(120)
    .max(99) // 显示 "99+"
    .child(Icon::new(IconName::Inbox).large())
```

数字为零时不会显示角标。

### 圆点和下角图标

```rust
OverlayBadge::new()
    .dot()
    .color(cx.theme().green)
    .child(Avatar::decorative())

OverlayBadge::new()
    .icon(IconName::Check)
    .color(cx.theme().cyan)
    .child(Avatar::decorative())
```

Number 和 Dot 位于右上角，Icon 位于右下角。

### 尺寸

```rust
OverlayBadge::new().count(2).small().child(target)
OverlayBadge::new().count(12).child(target)
OverlayBadge::new().count(212).large().child(target)
```

## 从 Tag 和旧 Badge 迁移

| 旧 API | 替代 API |
|---|---|
| `Tag::primary()` | `Badge::new()` |
| `Tag::secondary()` | `Badge::new().secondary()` |
| `Tag::danger()` | `Badge::new().destructive()` |
| `Tag::success()`、`warning()`、`info()`、`color()`、`custom()` | 使用 `Badge::new()` 和语义化 `Styled` 覆盖 |
| `Badge::new().count(...)` | `OverlayBadge::new().count(...)` |
| `Badge::new().dot()` | `OverlayBadge::new().dot()` |
| `Badge::new().icon(...)` | `OverlayBadge::new().icon(...)` |

`Tag` 及其自定义尺寸和圆角 API 已删除。常规场景使用固定 Vega Badge 几何，只在特殊视觉需求中使用 `Styled`。

## API 参考

- [Badge]
- [OverlayBadge]

[Badge]: https://docs.rs/hearth_gpui/latest/hearth_gpui/badge/struct.Badge.html
[OverlayBadge]: https://docs.rs/hearth_gpui/latest/hearth_gpui/badge/struct.OverlayBadge.html
