---
title: Input
description: 带校验、掩码和多种扩展能力的文本输入组件。
---

# Input

Input 是一个灵活的文本输入组件，支持校验、输入掩码、前后缀元素以及多种交互状态。

## 导入

```rust
use gpui_component::input::{InputState, Input};
```

## 用法

### 基础输入框

```rust
let input = cx.new(|cx| InputState::new(window, cx));

Input::new(&input)
```

### Placeholder

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Enter your name...")
);

Input::new(&input)
```

### 默认值

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .default_value("John Doe")
);

Input::new(&input)
```

### 可清空

```rust
Input::new(&input)
    .cleanable(true)
```

### 前缀和后缀

```rust
use gpui_component::{Icon, IconName};

Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())

Input::new(&input)
    .suffix(
        Button::new("info")
            .ghost()
            .icon(IconName::Info)
            .xsmall()
    )

Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())
    .suffix(Button::new("btn").ghost().icon(IconName::Info).xsmall())
```

需要在统一表面中组合文字、操作、快捷键或块级内容时，请使用 [Input Group](./input-group.md)。`prefix` 和 `suffix` 继续用于由单个 Input 管理的简单装饰内容。

### 密码输入

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .masked(true)
        .default_value("password123")
);

Input::new(&input)
    .content_type(InputContentType::Password)
    .mask_toggle()
```

### 尺寸

```rust
Input::new(&input).large()
Input::new(&input)
Input::new(&input).small()
```

### 禁用态

```rust
Input::new(&input).disabled(true)
```

禁用 Input 会退出键盘焦点顺序，并忽略指针、键盘和辅助功能写入操作。

### 只读态

```rust
Input::new(&input).read_only(true)
```

只读 Input 仍可聚焦，用户可以导航、选择并复制其中的值。

### 按 ESC 清空

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .clean_on_escape()
);

Input::new(&input)
```

### 输入校验

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .validate(|s, _| s.parse::<f32>().is_ok())
);

let input = cx.new(|cx|
    InputState::new(window, cx)
        .pattern(regex::Regex::new(r"^[a-zA-Z0-9]*$").unwrap())
);

Input::new(&input)
    .invalid(true)
    .aria_description("输入值只能包含字母和数字")
```

`invalid(true)` 会应用与 shadcn 对齐的 destructive 边框和外环，并向辅助技术暴露无效状态。

### 可访问性

当 Input 没有关联的可见 Label 时，应提供可访问名称：

```rust
Input::new(&input)
    .aria_label("电子邮箱")
    .aria_description("用于接收账户通知")
```

密码值不会通过辅助功能 value 暴露。禁用 Input 不提供 `SetValue`；只读 Input 保留读取和选择能力，但不提供编辑操作。

### Style Preset 与动效

Input 使用语义化 Style Preset metrics。Vega 是默认基线；Nova 使用紧凑几何，Maia 使用舒适的胶囊几何。焦点、无效态、背景与边框过渡使用当前 preset 的 motion tokens，并自动遵循 reduced-motion 设置。

### 输入掩码

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("(999)-999-9999")
);

let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("AAA-###-AAA")
);

use gpui_component::input::MaskPattern;

let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(3),
        })
);
```

### 监听事件

```rust
let input = cx.new(|cx| InputState::new(window, cx));

cx.subscribe_in(&input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let text = state.read(cx).value();
            println!("Input changed: {}", text);
        }
        InputEvent::PressEnter { secondary } => {
            println!("Enter pressed, secondary: {}", secondary);
        }
        InputEvent::Focus => println!("Input focused"),
        InputEvent::Blur => println!("Input blurred"),
    }
});
```

### 自定义外观

```rust
Input::new(&input).appearance(false)

div()
    .border_b_2()
    .px_6()
    .py_3()
    .border_color(cx.theme().border)
    .bg(cx.theme().secondary)
    .child(Input::new(&input).appearance(false))
```

## 示例

### 搜索输入框

```rust
let search = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Search...")
);

Input::new(&search)
    .prefix(Icon::new(IconName::Search).small())
```

### 金额输入

```rust
let amount = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(2),
        })
);

div()
    .child(Input::new(&amount))
    .child(format!("Value: {}", amount.read(cx).value()))
```

### 多输入表单

```rust
struct FormView {
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
}

v_flex()
    .gap_3()
    .child(Input::new(&self.name_input))
    .child(Input::new(&self.email_input))
```

## 只读与无效状态

```rust
Input::new(&input).read_only(true)
Input::new(&invalid_input).invalid(true)
```

只读输入框仍可聚焦、选择和复制，但会阻止用户编辑、粘贴、剪切、撤销、重做、IME 替换、清除操作和 AccessKit `SetValue`；程序化 `InputState` 更新仍可使用。该状态映射为 AccessKit `ReadOnly`。无效输入框使用语义化 danger 边框，并映射为 AccessKit `Invalid::True`。
