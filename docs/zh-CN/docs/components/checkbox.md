---
title: Checkbox
description: 用于切换选中与未选中状态的复选框组件。
---

# Checkbox

Checkbox 是一个与 shadcn 对齐的二元及混合状态选择组件。默认尺寸遵循 Vega，同时保留 GPUI Component 扩展尺寸。

## 导入

```rust
use gpui_component::checkbox::Checkbox;
```

## 用法

### 基础 Checkbox

```rust
Checkbox::new("my-checkbox")
    .label("Accept terms and conditions")
    .checked(false)
    .on_click(|checked, _, _| {
        println!("Checkbox is now: {}", checked);
    })
```

`on_click` 会在用户切换状态时触发，接收到的是切换后的新状态。

### 受控 Checkbox

```rust
struct MyView {
    is_checked: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Checkbox::new("checkbox")
            .label("Option")
            .checked(self.is_checked)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.is_checked = *checked;
                cx.notify();
            }))
    }
}
```

### 不同尺寸

```rust
Checkbox::new("cb-xs").xsmall().label("Extra Small")
Checkbox::new("cb-sm").small().label("Small")
Checkbox::new("cb").label("Medium")
Checkbox::new("cb-lg").large().label("Large")
```

默认 Checkbox 为 16px，Indicator 为 14px，与 shadcn 一致。其他尺寸是 GPUI Component 的扩展能力。

### 禁用状态

```rust
Checkbox::new("checkbox")
    .label("Disabled checkbox")
    .disabled(true)
    .checked(false)
```

### 不带标签

```rust
Checkbox::new("checkbox")
    .aria_label("Toggle standalone option")
    .checked(true)
```

Checkbox 没有可见标签时，必须通过 `aria_label(...)` 提供可访问名称。

### 自定义 Tab 顺序

```rust
Checkbox::new("checkbox")
    .label("Custom tab order")
    .tab_index(2)
    .tab_stop(true)
```

## API 参考

- [Checkbox]

### 样式

实现了 `Sizable` 和 `Disableable` trait：

- `xsmall()`：超小 Checkbox
- `small()`：小号 Checkbox
- `large()`：大号 Checkbox
- 不设置尺寸修饰方法时使用默认中号 Checkbox
- `disabled(bool)`：禁用状态

## 示例

### 复选框列表

```rust
v_flex()
    .gap_2()
    .child(Checkbox::new("cb1").label("Option 1").checked(true))
    .child(Checkbox::new("cb2").label("Option 2").checked(false))
    .child(Checkbox::new("cb3").label("Option 3").checked(false))
```

### 表单集成

```rust
struct FormView {
    agree_terms: bool,
    subscribe: bool,
}

v_flex()
    .gap_3()
    .child(
        Checkbox::new("terms")
            .label("I agree to the terms and conditions")
            .checked(self.agree_terms)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.agree_terms = *checked;
                cx.notify();
            }))
    )
    .child(
        Checkbox::new("subscribe")
            .label("Subscribe to newsletter")
            .checked(self.subscribe)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.subscribe = *checked;
                cx.notify();
            }))
    )
```

## 不确定与无效状态

```rust
Checkbox::new("partial").label("已选择部分项目").indeterminate(true)
Checkbox::new("invalid").label("必须接受条款").invalid(true)
```

`indeterminate(true)` 在视觉和可访问性上优先于 `checked`，映射为 AccessKit `Toggled::Mixed`，激活后产生选中值。`invalid(true)` 使用语义化 danger 边框和焦点环，并映射为 AccessKit `Invalid::True`。

## 键盘与焦点

- `Tab` 聚焦可用的 Checkbox。
- `Space` 切换当前聚焦 Checkbox。
- 禁用 Checkbox 不进入 Tab 顺序。
- Focus 和 Invalid Ring 只包围 Checkbox 方框，不包围整行标签。

默认 Vega 外观使用 4px 圆角和轻微阴影；Nova 使用无阴影的 4px 圆角；Maia 使用 6px 圆角。颜色仍由当前 Color Theme 提供。

动效同样由当前 Style Preset 决定：Vega 和 Maia 过渡 Focus 或 Invalid Ring，Nova 过渡控件颜色。Indicator 按照 shadcn 的 `transition-none` 立即切换；Reduced Motion 下直接显示最终状态。

[Checkbox]: https://docs.rs/gpui-component/latest/gpui_component/checkbox/struct.Checkbox.html
