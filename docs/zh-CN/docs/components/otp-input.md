---
title: OtpInput
description: 具备原生编辑能力并与 shadcn 对齐的可组合一次性验证码输入框。
---

# OtpInput

`OtpInput` 将一次性验证码显示为多个可视 Slot，同时只使用一个真实 `InputState` 负责编辑和辅助功能。文本选择、光标移动、删除、粘贴、IME 和 AccessKit 值更新因此与普通 Input 保持一致。

## 导入

```rust
use hearth_gpui::input::{
    InputEvent, OtpEvent, OtpInput, OtpInputGroup, OtpInputSeparator,
    OtpInputSlot, OtpState,
};
```

## 基础组合

```rust
let otp_state = cx.new(|cx| OtpState::new(6, window, cx));

let first = OtpInputGroup::new()
    .child(OtpInputSlot::new(0))
    .child(OtpInputSlot::new(1))
    .child(OtpInputSlot::new(2));
let second = OtpInputGroup::new()
    .child(OtpInputSlot::new(3))
    .child(OtpInputSlot::new(4))
    .child(OtpInputSlot::new(5));

OtpInput::new(&otp_state)
    .child(first)
    .child(OtpInputSeparator::new())
    .child(second)
    .aria_label("一次性验证码")
```

未提供子组件时，`OtpInput` 会自动创建一个包含全部 Slot 的连续 Group。需要 Separator 或自定义 Slot 样式时，建议使用显式组合 API。

## Pattern 与粘贴

默认 Pattern 只接受数字。每个 Slot 对应一个 Mask Token：`9` 接受数字，`A` 接受字母，`#` 接受字母或数字，`*` 接受任意字符。

```rust
let code_state = cx.new(|cx| {
    OtpState::new(6, window, cx)
        .pattern("######")
        .paste_transformer(|text| text.replace('-', ""))
        .default_value("A1B2")
});
```

剪贴板和 AccessKit 提供的值会先经过转换，再按 Mask 规范化并截断到固定 Slot 数量。全角数字会转换为 ASCII 数字。

## 状态与尺寸

```rust
OtpInput::new(&otp_state).invalid(true);
OtpInput::new(&otp_state).disabled(true);
OtpInput::new(&otp_state).small();
OtpInput::new(&otp_state).large();
OtpInput::new(&otp_state).with_size(px(44.));
```

通过 State 控制掩码显示：

```rust
let otp_state = cx.new(|cx| OtpState::new(6, window, cx).masked(true));

otp_state.update(cx, |state, cx| {
    state.set_masked(false, window, cx);
});
```

Slot 高度、圆角、阴影、焦点 Ring 和表面颜色均来自语义化 Style Preset 与 Color Theme。Vega 是默认基准；Nova 与 Maia 分别解析紧凑和舒适几何，不判断 Preset ID。

## 事件

每次值变化都会发出 `InputEvent::Change`。未完成的验证码首次达到指定长度时，会单独发出一次 `OtpEvent::Complete`。

```rust
cx.subscribe(&otp_state, |this, state, event: &InputEvent, cx| {
    if matches!(event, InputEvent::Change) {
        this.code = state.read(cx).value().clone();
        cx.notify();
    }
});

cx.subscribe(&otp_state, |this, _, event: &OtpEvent, cx| {
    if matches!(event, OtpEvent::Complete) {
        this.submit_code(cx);
    }
});
```

程序化更新不会发出 `InputEvent::Change`：

```rust
otp_state.update(cx, |state, cx| {
    state.set_value("123456", window, cx);
    state.focus(window, cx);
});
```

## 辅助功能

可视 Slot 不会分别进入 Tab 顺序。一个隐藏的编辑器负责暴露 `Role::TextInput`、`OneTimeCode` 内容类型、选择范围和规范化后的值。应提供 `aria_label`，必要时补充 `aria_description`。掩码值不会暴露到辅助功能树。

## API 参考

| 类型 | 主要 API |
| --- | --- |
| `OtpState` | `new`、`default_value`、`pattern`、`paste_transformer`、`masked`、`set_value`、`set_masked`、`value`、`length`、`focus` |
| `OtpInput` | `new`、`child`、`invalid`、`disabled`、`with_size`、`aria_label`、`aria_description` |
| `OtpInputGroup` | `new`、`child(OtpInputSlot)` |
| `OtpInputSlot` | `new(index)` 与 `Styled` 样式覆盖 |
| `OtpInputSeparator` | `new`、`child` 与 `Styled` 样式覆盖 |
