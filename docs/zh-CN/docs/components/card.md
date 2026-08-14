---
title: Card
description: 使用 shadcn Vega 表面组织相关内容和操作。
---

# Card

Card 提供类型明确的 Header、Content、Footer 和 Media 插槽，使选定尺寸能够统一应用到所有 section。它是静态容器，不会给子元素增加焦点或交互行为。

## 导入

```rust
use gpui::ParentElement as _;
use hearth_gpui::card::{
    Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardMedia,
    CardTitle,
};
```

## 基本用法

```rust
Card::new()
    .header(
        CardHeader::new()
            .title(CardTitle::new().child("创建项目"))
            .description(CardDescription::new().child("一键部署新项目。")),
    )
    .content(CardContent::new().child("项目设置"))
    .footer(
        CardFooter::new()
            .justify_end()
            .child(Button::new("deploy").label("部署")),
    )
```

## Header 操作

使用 `CardAction` 将内容对齐到 Header 右上角。标题和描述位于独立的可收缩列中，长文本不会与操作区域重叠。

```rust
CardHeader::new()
    .title(CardTitle::new().child("会议记录"))
    .description(CardDescription::new().child("与客户会议的文字记录。"))
    .action(
        CardAction::new().child(
            Button::new("transcribe")
                .small()
                .outline()
                .label("转录"),
        ),
    )
```

## Small Card

`small()` 会选择紧凑 Card 变体。Vega 和 Maia 的 Default/Small 间距为 24/16 px，Nova 为 16/12 px。Vega 和 Nova 的 Small 标题为 14 px，Maia 保持 16 px 标题。

```rust
Card::new()
    .small()
    .header(CardHeader::new().title(CardTitle::new().child("紧凑 Card")))
    .content(CardContent::new().child("紧凑内容"))
```

## 自定义间距

`spacing()` 会覆盖 Card 的统一 gap、垂直内边距和各 section 的水平内边距，但不会改变由 size 决定的标题字号。

```rust
Card::new()
    .small()
    .spacing(px(20.))
    .header(CardHeader::new().title(CardTitle::new().child("自定义间距")))
    .content(CardContent::new().child("20 px section 间距"))
```

## Section 分隔线

`bordered(true)` 会添加分隔线以及 Vega 布局所需的匹配内边距。

```rust
Card::new()
    .header(
        CardHeader::new()
            .title(CardTitle::new().child("发布状态"))
            .bordered(true),
    )
    .content(CardContent::new().child("26 项检查中有 24 项通过。"))
    .footer(CardFooter::new().bordered(true).child("刚刚更新"))
```

## 贴边媒体

媒体插槽渲染在 Header 之前，并会移除 Card 顶部内边距。图片应使用 `CardMedia::image`，使 Card 外圆角直接绘制到图片自身。

```rust
Card::new()
    .media(
        CardMedia::image("https://example.com/landscape.jpg")
            .h(px(160.)),
    )
    .header(CardHeader::new().title(CardTitle::new().child("风景")))
```

自定义媒体应将背景设置在 `CardMedia` 自身，再添加只绘制前景的子元素。GPUI 当前使用矩形 mask 裁剪溢出子元素，因此带方形背景的子元素无法继承父元素的圆角裁剪。CardMedia 会继承 Card 最终解析出的边缘圆角，包括 `rounded(px(0.))` 或按角设置的自定义值。

使用 `bottom_media()` 在 Footer 之后渲染媒体。它与 shadcn 的末尾图片选择器一致，会保留 Card 的常规底部内边距，不会成为完全贴底的表面。

```rust
Card::new()
    .footer(CardFooter::new().child("刚刚更新"))
    .bottom_media(
        CardMedia::image("https://example.com/preview.jpg")
            .h(px(120.)),
    )
```

## GPUI 组合说明

React 实现允许任意直接子元素，并使用 CSS Grid 布局 Header。本项目有意保留类型明确的插槽，使 Card 无需 DOM selector 也能传播统一间距。CardHeader 使用语义等价的 `1fr + auto` Flex 布局：文本列可以收缩，操作区保持右上对齐。

GPUI 支持在尺寸明确的区域使用 `container_query`，因此响应式内容可以放在 `CardContent` 内。Card 不会自动给 Header 安装 container query，因为 GPUI 的 container-query 子元素无法反向决定容器自身尺寸。这是有意保留的平台差异。

Card 背景使用 Color Theme 的 `card.background` 和 `card.foreground`。未声明这些字段的主题会分别回退到 `background` 和 `foreground`。Vega 提供 xs shadow，Nova 提供紧凑的着色 Footer，Maia 通过语义 Style Preset 属性提供更大的圆角和 Header gap。

[Card]: https://docs.rs/hearth-gpui/latest/hearth_gpui/card/struct.Card.html
