---
title: Button
description: 使用 shadcn Vega 视觉基准展示操作。
---

# Button

`Button` 的几何尺寸、变体、交互状态与组合方式以 shadcn Vega 为默认基准。

## 变体

```rust
Button::new("default").label("Default");
Button::new("outline").outline().label("Outline");
Button::new("secondary").secondary().label("Secondary");
Button::new("ghost").ghost().label("Ghost");
Button::new("destructive").destructive().label("Destructive");
Button::new("link").link().label("Link");
```

`Default` 是主要操作样式。真实导航应使用 `Link` 组件，不应改变 Button 的可访问性角色。

需要保持按压状态的操作可以使用 `.pressed(bool)`。它会保留 Button 的键盘行为并通过 `aria-pressed` 暴露状态；普通选项组仍应使用 `Toggle` 或 `ToggleGroup`。

## 尺寸

```rust
Button::new("xs").xsmall().label("Extra Small");
Button::new("sm").small().label("Small");
Button::new("md").label("Default");
Button::new("lg").large().label("Large");
```

纯图标按钮的宽高相同，并且必须设置 `aria_label`。

```rust
Button::new("move-up")
    .outline()
    .icon(IconName::ArrowUp)
    .aria_label("Move up");
```

## 图标与加载状态

`icon` 是前置插槽，`trailing_icon` 是后置插槽。加载状态通过 `Spinner` 显式组合，并在任务执行期间禁用按钮。

```rust
Button::new("generating")
    .outline()
    .icon(Spinner::new())
    .label("Generating")
    .disabled(true);
```

## 圆角

`rounded_full` 根据最终控件高度计算胶囊圆角；`rounded(px(...))` 用于显式覆盖。

```rust
Button::new("round")
    .outline()
    .rounded_full()
    .icon(IconName::ArrowUp)
    .aria_label("Move up");
```

## Button Group

`ButtonGroup` 用于组合操作，并保留每个 Button 自身的回调。它支持嵌套 Group、文本、分隔线、方向和可访问名称。选择行为应使用 `Toggle` 或 `ToggleGroup`。

```rust
ButtonGroup::new("message-actions")
    .aria_label("Message actions")
    .child(Button::new("back").outline().icon(IconName::ArrowLeft).aria_label("Back"))
    .group(
        ButtonGroup::new("archive-report")
            .child(Button::new("archive").outline().label("Archive"))
            .child(Button::new("report").outline().label("Report")),
    )
    .separator(ButtonGroupSeparator::new())
    .text(ButtonGroupText::new("More"));
```
