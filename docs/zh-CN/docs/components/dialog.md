---
title: Dialog
description: 用于集中展示内容与操作的模态表面。
---

# Dialog

`Dialog` 提供遮罩、焦点陷阱、焦点恢复、键盘关闭以及可中断的退出动效。应用首层视图必须是 [`Root`](../getting-started)，并渲染 Root 管理的 Dialog layer。

## 导入

```rust
use hearth_gpui::{
    WindowExt as _,
    button::Button,
    dialog::{Dialog, DialogAction, DialogClose, DialogFooter},
};
```

## 声明式 Trigger

`trigger` 接收 `Button`。Dialog 会追加打开回调，不会覆盖 Button 已有的 `on_click`；鼠标和 Enter/Space 因而共享同一套控件语义。

```rust
Dialog::new(cx)
    .trigger(Button::new("edit-profile").outline().label("Edit profile"))
    .title("Edit profile")
    .description("Update your public profile information.")
    .content(move |content, _, _| {
        content.child(Input::new(&name_input))
    })
    .footer(|_, _| {
        DialogFooter::new()
            .child(DialogClose::new(
                Button::new("cancel").outline().label("Cancel"),
            ))
            .child(DialogAction::new(
                Button::new("save").label("Save changes"),
            ))
    })
    .on_ok(|_, window, cx| {
        window.push_notification("Profile saved", cx);
        true
    })
```

`DialogClose` 和 `DialogAction` 会直接向 Button 追加行为，不再增加影响尺寸或焦点的外层容器，同时保留 Button 原有的点击回调。

## 命令式 Dialog

已有事件需要打开 Dialog 时，使用 `WindowExt::open_dialog`。

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Keyboard shortcuts")
        .description("Review the shortcuts available in this window.")
        .child(shortcuts_view)
        .footer(|_, _| {
            DialogFooter::new().child(DialogClose::new(
                Button::new("done").label("Done"),
            ))
        })
})
```

## 自定义标题和描述

文字 slot 会自动提供 Dialog 的可访问名称和描述。自定义元素使用 renderer closure，以支持同一个声明式 Trigger 重复打开；同时必须显式提供可访问文本。

```rust
Dialog::new(cx)
    .aria_label("Connection settings")
    .aria_description("Configure the active server connection.")
    .title_element(|_, _| h_flex().child(Icon::new(IconName::Server)).child("Connection"))
    .description_element(|_, _| div().child("Changes apply after reconnecting."))
```

## 焦点与关闭行为

- 打开时优先聚焦显式 `initial_focus`；否则聚焦第一个启用的 Tab stop；仍不存在时保留 Dialog surface 焦点。
- Tab 和 Shift-Tab 被限制在最上层 Dialog 内。
- 关闭后恢复此前控件的焦点。
- Escape 和点击遮罩统一分发取消，并遵循返回 `false` 的 `on_cancel` 回调。
- Enter 分发标准 Dialog 确认操作；Space 继续激活当前聚焦的 Button。
- `AlertDialog` 不使用全局 Enter 确认。

```rust
dialog
    .initial_focus(input_focus)
    .dismiss_on_escape(true)
    .confirm_on_enter(true)
    .dismiss_on_overlay_click(true)
    .show_close_button(true)
    .show_overlay(true)
```

## 尺寸和位置

默认宽度由当前 Style Preset 解析，并在视口两侧保留至少 16 px。显式宽度和最大宽度同样受该安全范围限制。

```rust
dialog
    .w(px(560.))
    .max_w(px(720.))
    .margin_top(px(96.))
```

## Style Preset 与动效

- Vega 是默认基准：首选宽度 448 px、24 px padding、紧凑标题和不分隔的 Footer。
- Nova 使用紧凑的 384 px 几何以及带分隔线和弱化背景的 Footer。
- Maia 使用舒适密度、更大圆角和更强遮罩。
- Dialog 只读取语义 Theme 和 Style Metrics，不判断 Preset ID。

项目有意保留桌面 Dialog 动效，不采用 shadcn 的居中 zoom：Surface 最终位于视口顶部约 10%，使用语义化的 250 ms emphasis 时长、进出缓动、透明度、位移和阴影过渡。退出完成前保持挂载，支持中途反向，并在 Reduced Motion 下立即到达最终状态。

当前固定版本的 GPUI renderer 不支持元素级 backdrop filter。遮罩颜色和透明度已按 Style Preset 对齐，backdrop blur 记录在 `docs/shadcn/TODO.md` 中延期实现。
