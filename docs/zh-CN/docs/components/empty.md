---
title: Empty
description: 可组合的空状态或无结果状态，支持媒体、说明与操作区域。
---

# Empty

`Empty` 用于展示空数据、不可用或无搜索结果状态。类型化区域可以保持布局一致，同时允许组合任意 GPUI 内容。

## 导入

```rust
use hearth_gpui::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle,
};
```

## 基础用法

```rust
Empty::new()
    .min_h(px(320.))
    .child(
        EmptyHeader::new()
            .child(EmptyTitle::new().child("暂无项目"))
            .child(EmptyDescription::new().child("创建项目以组织你的工作。")),
    )
    .child(
        EmptyContent::new()
            .child(Button::new("create-project").label("创建项目")),
    )
```

## 图标媒体

`EmptyMedia::icon` 会使用当前 Style Preset 对应的图标容器和图标尺寸：

```rust
EmptyHeader::new()
    .child(EmptyMedia::icon(IconName::Inbox))
    .child(EmptyTitle::new().child("暂无消息"))
    .child(EmptyDescription::new().child("新会话会显示在这里。"))
```

头像、头像组、插图或其他自定义内容使用 `EmptyMedia::new().child(...)`。如果自定义子元素需要图标表面，可设置 `EmptyMediaVariant::Icon`，并显式指定子元素尺寸。

## 边框与背景

根组件预设虚线边框样式，但只有设置边框宽度后才会显示：

```rust
Empty::new()
    .border_1()
    .border_color(cx.theme().border)
    .child(content)
```

`Empty` 及全部类型化区域均实现 `Styled`，可以覆盖尺寸、间距、背景、边框和对齐方式。

## 组合结构

```text
Empty
├── EmptyHeader
│   ├── EmptyMedia
│   ├── EmptyTitle
│   └── EmptyDescription
└── EmptyContent
```

## 行为与可访问性

- Empty 是静态布局组件，不包含进入、退出或状态动画，与 shadcn 保持一致。
- 组件不会自动声明 Alert、Status 或 live-region；内部交互组件保留自身的角色和可访问名称。
- Vega 是默认视觉基线；Nova 使用紧凑的 padding、字号和图标几何，Maia 使用舒适圆角，但组合结构不变。
- GPUI 当前使用普通约束换行代替浏览器专属的 `text-wrap: balance`。
