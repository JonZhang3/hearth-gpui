---
title: Popover
description: 相对于触发元素定位的浮动对话框表面。
---

# Popover

Popover 用于在触发元素附近展示富交互内容。默认表面会消费当前 Color Theme 与 Style Preset，Vega 是默认视觉基准。

## 导入

```rust
use hearth_gpui::popover::{
    Popover, PopoverAlign, PopoverDescription, PopoverHeader, PopoverSide, PopoverTitle,
    PopoverTrigger,
};
```

## 基础用法

触发器需要实现 `PopoverTrigger`，以便 Popover 把 `aria-expanded` 写入 Trigger 自身的可访问性节点。`Button` 和内置 ColorPicker Trigger 已提供该能力。内容既可以使用普通 `ParentElement` 组合，也可以通过动态 `content` 回调构造。

```rust
use gpui::ParentElement as _;
use hearth_gpui::{
    button::Button,
    popover::{Popover, PopoverDescription, PopoverHeader, PopoverTitle},
};

Popover::new("profile-popover")
    .trigger(Button::new("profile-trigger").outline().label("打开资料"))
    .aria_label("资料信息")
    .child(
        PopoverHeader::new()
            .child(PopoverTitle::new().child("资料"))
            .child(PopoverDescription::new().child(
                "查看与此资料关联的账户信息。",
            )),
    )
```

标准表面宽度为 288 px，padding、gap、圆角、ring、阴影和文字规格由当前 Style Preset 提供。调用方的 `Styled` 覆盖会在默认样式之后生效。

## 定位

`side` 决定内容位于触发器的哪个物理方向，`align` 决定交叉轴对齐方式。默认值是 `Bottom` 与 `Center`，默认间距为 4 px。

```rust
Popover::new("placement-popover")
    .side(PopoverSide::Top)
    .align(PopoverAlign::End)
    .side_offset(px(8.))
    .align_offset(px(4.))
    .trigger(Button::new("placement-trigger").outline().label("定位"))
    .child("顶部、尾端对齐的内容")
```

旧 `.anchor(Anchor)` builder 仍可使用，并会映射到等价的 side/align 组合。GPUI 会移动内容以避免超出窗口，但不会自动翻转到相反方向。

## 动态内容与手动关闭

`content` 会收到 `PopoverState`、`Window` 与 `Context<PopoverState>`。在内容中发出 `DismissEvent` 可主动关闭 Popover。

```rust
Popover::new("dynamic-popover")
    .trigger(Button::new("dynamic-trigger").outline().label("打开"))
    .content(|_, _, cx| {
        Button::new("close-popover")
            .label("关闭")
            .on_click(cx.listener(|_, _, _, cx| cx.emit(DismissEvent)))
    })
```

`content` 可能在每次 Popover 渲染时执行，不要在其中创建 Entity 或执行高成本操作。

## 受控状态

```rust
Popover::new("controlled-popover")
    .open(self.popover_open)
    .on_open_change(cx.listener(|this, open: &bool, _, cx| {
        this.popover_open = *open;
        cx.notify();
    }))
    .trigger(Button::new("controlled-trigger").outline().label("受控 Popover"))
    .child("受控内容")
```

`default_open(true)` 只用于非受控模式的初始状态。打开时会完整注册 overlay 生命周期并把焦点移入 Popover；Escape、焦点离开内容或点击外部会关闭 Popover，并恢复之前的焦点。由外层组件自行管理关闭时可设置 `overlay_closable(false)`。

## 自定义外观与触发方式

`appearance(false)` 只关闭标准表面样式，定位、生命周期、焦点和关闭 API 仍然有效。`mouse_button(MouseButton::Right)` 可用于自定义右键浮层。

```rust
Popover::new("custom-popover")
    .appearance(false)
    .mouse_button(MouseButton::Right)
    .trigger(Button::new("custom-trigger").label("右键点击"))
    .bg(cx.theme().accent)
    .text_color(cx.theme().accent_foreground)
    .p_3()
    .rounded_lg()
    .child("自定义内容")
```

进入与退出使用当前语义 motion 时长和 easing，并执行与定位方向对应的 8 px 位移。为保持整个 Popover 表面的视觉稳定，透明度动画被有意省略。GPUI 当前没有元素 transform primitive，因此无法精确实现 `zoom-in-95` / `zoom-out-95`。
