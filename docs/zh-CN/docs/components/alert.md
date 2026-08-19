---
title: Alert
description: 用于吸引用户注意的重要提示组件。
---

# Alert

Alert 用于展示重要状态或操作提示。图标、标题、描述和操作均为相互独立的可选 Slot。

## 导入

```rust
use hearth_gpui::alert::Alert;
```

## 基础用法

```rust
Alert::new("payment-success")
    .icon(IconName::CircleCheck)
    .title("Payment successful")
    .description(
        "Your payment of $29.99 has been processed. A receipt has been sent to your email address."
    )
```

所有内容 Slot 均可省略：

```rust
Alert::new("title-only").title("Changes saved")

Alert::new("description-only")
    .description("This alert has no title or icon.")
```

## Destructive

错误或失败状态使用 `destructive`。Variant 不会自动添加图标。

```rust
Alert::new("payment-failed")
    .destructive()
    .icon(IconName::TriangleAlert)
    .title("Payment failed")
    .description("Your payment could not be processed. Please try again.")
```

## Action

使用 `action` 将 Button 或其他操作元素放置在右上角。

```rust
Alert::new("dark-mode")
    .title("Dark mode is now available")
    .description("Enable it under your profile settings to get started.")
    .action(Button::new("enable-dark-mode").xsmall().label("Enable"))
```

## 自定义颜色

Alert 实现了 `Styled`。根节点前景色会被图标和标题继承，Default description 仍使用弱化颜色。

```rust
Alert::new("subscription-warning")
    .icon(IconName::TriangleAlert)
    .title("Your subscription will expire in 3 days.")
    .description("Renew now to avoid service interruption.")
    .bg(cx.theme().warning.opacity(0.08))
    .border_color(cx.theme().warning.opacity(0.5))
    .text_color(cx.theme().warning)
```

## 可关闭 Alert

`on_close` 会用可访问的纯图标关闭按钮替换自定义 Action。回调负责更新可见状态。

```rust
Alert::new("closable")
    .title("Maintenance scheduled")
    .description("The service will be unavailable tonight.")
    .visible(is_visible)
    .on_close(cx.listener(|this, _, _, cx| {
        this.is_visible = false;
        cx.notify();
    }))
```

## Banner

Banner 外观会移除边框和圆角，但不会隐藏任何内容 Slot。

```rust
Alert::new("maintenance-banner")
    .banner()
    .icon(IconName::Info)
    .title("Maintenance scheduled")
    .description("The service will be unavailable tonight.")
```

## 富文本内容

`title` 和 `description` 会保留普通字符串并生成可访问性信息。任意 GPUI 元素使用 `title_element` 或 `description_element`。由于任意元素无法可靠还原为文本，需要通过 `aria_label` 概括所有重要内容。

```rust
let description = cx.new(|cx| Markdown::new(
    "Please correct the following errors:\n- Email address is required\n- Password must be at least 8 characters",
    cx,
));
let description = MarkdownElement::new(
    description,
    MarkdownStyle::themed(MarkdownFont::Preview, window, cx),
);

Alert::new("validation-error")
    .destructive()
    .title("Validation failed")
    .description_element(description)
    .aria_label(
        "Validation failed. Email address is required. Password must be at least 8 characters."
    )
```

## 可访问性

Alert 暴露 AccessKit Alert role。文本标题会成为可访问名称，文本描述会成为可访问描述；只有描述时，描述会作为名称。自定义元素 Slot 无法提供合适文本信息时，使用 `aria_label` 显式命名。

```rust
Alert::new("sync-status")
    .aria_label("Synchronization failed")
    .destructive()
    .description_element(custom_status_view)
```

## API 参考

- [Alert]
- [AlertVariant]

[Alert]: https://docs.rs/hearth-gpui/latest/hearth_gpui/alert/struct.Alert.html
[AlertVariant]: https://docs.rs/hearth-gpui/latest/hearth_gpui/alert/enum.AlertVariant.html
