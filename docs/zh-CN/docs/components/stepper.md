---
title: Stepper
description: 用于引导用户按步骤完成流程的进度组件。
---

# Stepper

Stepper 用于按步骤展示流程进度，适合表单向导、订单流程和安装步骤等场景。支持横向和纵向布局、自定义图标、语义尺寸、只读进度和交互式导航。

## 导入

```rust
use gpui_component::stepper::{Stepper, StepperItem};
```

## 用法

### 基础 Stepper

使用 `selected_index` 设置当前步骤，索引从 `0` 开始，默认值也是 `0`。

```rust
Stepper::new("my-stepper")
    .selected_index(0)
    .items([
        StepperItem::new().label("Step 1"),
        StepperItem::new().label("Step 2"),
        StepperItem::new().label("Step 3"),
    ])
    .on_click(|step, _, _| {
        println!("Clicked step: {}", step);
    })
```

未设置 `on_click` 时，Stepper 是只读进度指示器，步骤不会进入 Tab 顺序。设置 `on_click` 后，启用的步骤支持鼠标、Enter 和 Space 激活。

### 带图标的 Stepper

```rust
use gpui_component::IconName;

Stepper::new("icon-stepper")
    .selected_index(0)
    .items([
        StepperItem::new()
            .icon(IconName::Calendar)
            .child("Order Details"),
        StepperItem::new()
            .icon(IconName::Inbox)
            .child("Shipping"),
        StepperItem::new()
            .icon(IconName::Frame)
            .child("Preview"),
        StepperItem::new()
            .icon(IconName::Info)
            .child("Finish"),
    ])
```

### 纵向布局

```rust
Stepper::new("vertical-stepper")
    .vertical()
    .selected_index(2)
    .items_center()
    .items([
        StepperItem::new()
            .pb_8()
            .icon(IconName::Building2)
            .child(v_flex().child("Step 1").child("Description for step 1.")),
        StepperItem::new()
            .pb_8()
            .icon(IconName::Asterisk)
            .child(v_flex().child("Step 2").child("Description for step 2.")),
        StepperItem::new()
            .pb_8()
            .icon(IconName::Folder)
            .child(v_flex().child("Step 3").child("Description for step 3.")),
        StepperItem::new()
            .icon(IconName::CircleCheck)
            .child(v_flex().child("Step 4").child("Description for step 4.")),
    ])
```

### 文本居中

```rust
Stepper::new("center-stepper")
    .selected_index(0)
    .text_center(true)
    .items([
        StepperItem::new().child(
            v_flex()
                .items_center()
                .child("Step 1")
                .child("Desc for step 1."),
        ),
        StepperItem::new().child(
            v_flex()
                .items_center()
                .child("Step 2")
                .child("Desc for step 2."),
        ),
        StepperItem::new().child(
            v_flex()
                .items_center()
                .child("Step 3")
                .child("Desc for step 3."),
        ),
    ])
```

### 不同尺寸

```rust
use gpui_component::{Sizable as _, Size};

Stepper::new("stepper")
    .xsmall()
    .items([...])

Stepper::new("stepper")
    .small()
    .items([...])

Stepper::new("stepper")
    .large()
    .items([...])
```

### 禁用状态

```rust
Stepper::new("disabled-stepper")
    .disabled(true)
    .items([
        StepperItem::new().child("Step 1"),
        StepperItem::new().child("Step 2"),
    ])
```

### 可访问性与键盘行为

- 根节点暴露具名步骤列表；上下文不能提供清晰名称时使用 `aria_label()`。
- `StepperItem::label()` 同时提供可见文本和可访问名称；组合式自定义内容应使用 `aria_label()`。
- 当前项目暴露 `aria-current="step"`，每个项目提供其位置和列表总数。
- 交互模式下，启用步骤可通过 Tab 到达，并通过 Enter 或 Space 激活；长按 Space 不会重复触发。
- 禁用步骤会被辅助技术识别为 disabled，且不能聚焦或激活。
- 超出范围的 `selected_index()` 会限制到最后一个可用步骤；空 Stepper 没有当前步骤。

Stepper 不对状态变化应用动画。颜色和几何会原子更新，尺寸、间距和连接线宽度由当前 Style Preset 解析。

## API 参考

- [Stepper]
- [StepperItem]

### 尺寸

实现了 [Sizable] trait：

- `xsmall()`：超小尺寸
- `small()`：小尺寸
- `medium()`：中尺寸，默认值
- `large()`：大尺寸

### 方法

- `aria_label(label)`：设置步骤列表的可访问名称
- `selected_index(index)`：设置从 0 开始的当前步骤；越界值会限制到最后一个项目
- `layout(axis)` / `vertical()`：设置横向或纵向布局
- `text_center(bool)`：让横向项目内容居中
- `disabled(bool)`：禁用所有步骤
- `on_click(handler)`：启用导航并返回激活步骤的索引

`StepperItem` 支持 `label()`、`aria_label()`、`icon()`、`disabled()`、自定义子元素、尺寸和样式。

## 示例

### 多步骤表单

```rust
Stepper::new("form-stepper")
    .w_full()
    .selected_index(form_step)
    .items([
        StepperItem::new()
            .icon(IconName::User)
            .child("Personal Info"),
        StepperItem::new()
            .icon(IconName::CreditCard)
            .child("Payment"),
        StepperItem::new()
            .icon(IconName::CircleCheck)
            .child("Confirmation"),
    ])
    .on_click(cx.listener(|this, step, _, cx| {
        this.form_step = *step;
        cx.notify();
    }))
```

### 禁用单个步骤

```rust
Stepper::new("stepper")
    .selected_index(0)
    .items([
        StepperItem::new().child("Available"),
        StepperItem::new().disabled(true).child("Locked"),
        StepperItem::new().child("Available"),
    ])
```

[Stepper]: https://docs.rs/gpui-component/latest/gpui_component/stepper/struct.Stepper.html
[StepperItem]: https://docs.rs/gpui-component/latest/gpui_component/stepper/struct.StepperItem.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
