---
title: Breadcrumb
description: 显示当前位置在层级结构中的路径。
---

# Breadcrumb

Breadcrumb 使用明确的 Link、Page、Separator 和 Ellipsis 元素，导航语义不再依赖子元素的位置。

## 导入

```rust
use gpui::ParentElement as _;
use gpui_component::{
    IconName, Sizable as _,
    breadcrumb::{
        Breadcrumb, BreadcrumbEllipsis, BreadcrumbItem, BreadcrumbLink, BreadcrumbPage,
        BreadcrumbSeparator,
    },
    button::Button,
    menu::{DropdownMenu as _, PopupMenuItem},
};
```

## 基本用法

```rust
Breadcrumb::new("docs-breadcrumb")
    .child(
        BreadcrumbItem::new("home-item")
            .child(
                BreadcrumbLink::new("home-link")
                    .label("首页")
                    .href("https://example.com"),
            ),
    )
    .child(BreadcrumbSeparator::new())
    .child(
        BreadcrumbItem::new("components-item")
            .child(
                BreadcrumbLink::new("components-link")
                    .label("组件")
                    .href("https://example.com/components"),
            ),
    )
    .child(BreadcrumbSeparator::new())
    .child(
        BreadcrumbItem::new("current-item")
            .child(BreadcrumbPage::new("current-page").label("Breadcrumb")),
    )
```

`BreadcrumbLink` 支持 `href`、`on_click` 和 `disabled`。URL 跳转与回调会组合执行，不会互相覆盖。可用链接可通过 Tab 聚焦，并使用 Enter 激活。

只有标签而没有激活目标的 `BreadcrumbLink` 会渲染为不可交互文本，不会进入 Tab 顺序。需要链接行为时必须设置 `href` 或 `on_click`。

## 从旧 API 迁移

组合 API 是经过明确批准的破坏性重构。应将位置字符串和自动分隔符替换为明确的组合元素：

```rust
// 旧 API
Breadcrumb::new().child("首页").child("组件")

// 组合 API
Breadcrumb::new("docs-breadcrumb")
    .child(
        BreadcrumbItem::new("home-item").child(
            BreadcrumbLink::new("home-link")
                .label("首页")
                .href("https://example.com"),
        ),
    )
    .child(BreadcrumbSeparator::new())
    .child(
        BreadcrumbItem::new("current-item")
            .child(BreadcrumbPage::new("current-page").label("组件")),
    )
```

`Breadcrumb::new` 和 `BreadcrumbItem::new` 现在接收稳定元素 ID，可见文字由 `BreadcrumbLink::label` 或 `BreadcrumbPage::label` 提供。

## 自定义分隔符

```rust
BreadcrumbSeparator::new().child("/")
```

## 折叠路径

`BreadcrumbEllipsis` 是展示元素，可以表示被折叠的中间路径：

```rust
BreadcrumbItem::new("collapsed-item").child(BreadcrumbEllipsis::new())
```

折叠路径菜单应使用 Button 的图标 API，使触发器获得当前 Style Preset 的 icon-only 正方形几何。Button 必须提供可访问名称。

```rust
Button::new("collapsed-items")
    .ghost()
    .small()
    .icon(IconName::Ellipsis)
    .aria_label("显示折叠的路径")
    .dropdown_menu(|menu, _, _| {
        menu.item(PopupMenuItem::new("文档"))
            .item(PopupMenuItem::new("主题"))
    })
```

`BreadcrumbPage` 不可交互，并向辅助技术公开当前页面状态。当 Link 或 Page 使用不含文字标签的自定义内容时，应设置 `aria_label`。
