---
title: AlertDialog
description: 要求用户明确响应后才能返回应用。
---

# AlertDialog

`AlertDialog` 是与 shadcn Vega 对齐的模态确认组件。它会阻止背景交互、约束焦点，不会因点击 Overlay 而关闭，也不显示关闭按钮。

## 基础用法

```rust
use hearth_gpui::{
    button::Button,
    dialog::{AlertDialog, AlertDialogAction, AlertDialogCancel},
};

AlertDialog::new(cx)
    .trigger(Button::new("show-dialog").outline().label("Show Dialog"))
    .on_action(|_, window, cx| {
        window.push_notification("Confirmed", cx);
        true
    })
    .content(|content, _, _| {
        content
            .title("Are you absolutely sure?")
            .description(
                "This action cannot be undone. This will permanently delete your account.",
            )
            .cancel(AlertDialogCancel::new("cancel", "Cancel"))
            .action(AlertDialogAction::new("continue", "Continue"))
    })
```

## Small 危险操作

`Small` 使用居中内容和两列等宽 Footer 操作。

```rust
use hearth_gpui::{
    button::ButtonVariant,
    dialog::{AlertDialogAction, AlertDialogCancel, AlertDialogSize},
};

content
    .size(AlertDialogSize::Small)
    .media(Icon::new(IconName::TriangleAlert).size_8())
    .title("Delete chat?")
    .description("This will permanently delete this chat conversation.")
    .cancel(AlertDialogCancel::new("cancel", "Cancel"))
    .action(
        AlertDialogAction::new("delete", "Delete")
            .variant(ButtonVariant::Destructive),
    )
```

## 命令式调用

`WindowExt::open_alert_dialog` 与声明式 Trigger 使用同一套内容 API 和行为。

```rust
window.open_alert_dialog(cx, |dialog, _, _| {
    dialog.content(|content, _, _| {
        content
            .title("Session expired")
            .description("Sign in again to continue.")
            .action(AlertDialogAction::new("sign-in", "Sign in"))
    })
});
```

## 自定义内容与可访问性

通过 `title`、`description` 设置的文本会自动成为 accessible name 和 accessible description。使用 `title_element` 或 `description_element` 时，应同时设置对应的 `aria_label` 或 `aria_description`。

`AlertDialogContent` 实现了 `ParentElement`，因此可以通过 `child` 插入表单控件或其他自定义正文内容。

## 行为

- 点击 Overlay 不会关闭 AlertDialog。
- Escape 默认执行取消；使用 `dismiss_on_escape(false)` 可以禁用。
- Enter 和 Space 只激活当前聚焦按钮，不会全局确认。
- `on_action` 或 `on_cancel` 返回 `false` 时保持打开。
- 退出内容会保持挂载，直到共享模态动画结束。
- Overlay 和内容使用稳定的 100ms 淡入淡出。当前锁定的 GPUI 渲染器没有元素级背景模糊，也不能对任意元素树执行不影响布局的缩放，因此不使用会引发布局重排的模拟效果。

## Style Preset

Vega 是默认视觉基准。Nova 和 Maia 通过语义 `ModalMetrics` 解析各自的宽度、内边距、圆角、Media 几何、Footer 样式、Overlay 透明度和 Ring 透明度，组件不会判断 Preset ID。

shadcn 网站截图通常使用 Nova；Nova 的浅色分隔 Footer 不是 Vega 默认样式。
