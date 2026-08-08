---
title: Input Group
description: 将 Input 与行内或块级附加内容、辅助文字和操作组合为统一控件。
---

# Input Group

Input Group 将一个 `Input` 及其相关内容呈现为统一控件表面。根组件使用 typed-slot API：通过 `input(...)` 设置输入控件，通过 `addon(...)` 添加周边内容。

## 导入

```rust
use gpui_component::input::{
    Input, InputGroup, InputGroupAddon, InputGroupAddonAlign,
    InputGroupButton, InputGroupButtonSize, InputGroupText, InputState,
};
```

## 行内附加内容

```rust
let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));

InputGroup::new("search-group")
    .input(Input::new(&search))
    .addon(
        InputGroupAddon::new()
            .child(Icon::new(IconName::Search)),
    )
    .addon(
        InputGroupAddon::new()
            .align(InputGroupAddonAlign::InlineEnd)
            .child(InputGroupText::new().child("12 results")),
    )
```

点击非交互式 addon 会聚焦关联的 Input。`InputGroupButton` 等交互式子元素保留自己的指针行为。

## 操作按钮

```rust
InputGroup::new("website-group")
    .input(Input::new(&website))
    .addon(InputGroupAddon::new().child(
        InputGroupText::new().child("https://"),
    ))
    .addon(
        InputGroupAddon::new()
            .align(InputGroupAddonAlign::InlineEnd)
            .child(
                InputGroupButton::new(
                    Button::new("website-info")
                        .icon(IconName::Info)
                        .aria_label("Website information"),
                )
                .size(InputGroupButtonSize::IconXs),
            ),
    )
```

`InputGroupButton` 适配已有 `Button`；事件处理、禁用状态和辅助功能元数据仍由 Button 负责。它默认使用 shadcn Ghost variant，并实现 `ButtonVariants` 以支持显式 variant 选择。

## 块级附加内容与多行输入

```rust
let message = cx.new(|cx| {
    InputState::new(window, cx)
        .auto_grow(3, 8)
        .placeholder("Ask, search, or chat...")
});

InputGroup::new("message-group")
    .input(Input::new(&message))
    .addon(
        InputGroupAddon::new()
            .align(InputGroupAddonAlign::BlockStart)
            .child(InputGroupText::new().child("message.txt")),
    )
    .addon(
        InputGroupAddon::new()
            .align(InputGroupAddonAlign::BlockEnd)
            .child(InputGroupText::new().child("Markdown supported")),
    )
```

单行、多行和自动增高模式继续使用同一个 `Input` API，Input Group 不会引入第二套编辑器实现。

## 状态

```rust
InputGroup::new("disabled-group")
    .input(Input::new(&input))
    .disabled(true)

InputGroup::new("invalid-group")
    .input(Input::new(&input))
    .invalid(true)
    .aria_label("Invalid email")
```

焦点、无效态、禁用态、边框、背景、阴影与动效统一由外层表面负责。状态会传播给内部 Input，不会产生嵌套边框或焦点环。

## 组合契约

| 组件 | 作用 |
| --- | --- |
| `InputGroup` | 管理统一表面和唯一 Input slot |
| `InputGroupAddon` | 将可组合内容放置在行内或块级边缘 |
| `InputGroupText` | 为 addon 内的辅助文字应用 muted 样式 |
| `InputGroupButton` | 将已有 Button 适配为紧凑的组内几何 |

根组件有意不接收任意 child，以保持焦点、无效态、禁用态和辅助功能归属明确；addon 内部仍通过项目统一的 `ParentElement` API 自由组合内容。

## Style Preset 与动效

Vega 是默认基线。Nova 与 Maia 的几何通过语义化 Style Preset metrics 解析，不按 preset ID 分支。表面过渡复用 Input motion tokens，可从当前插值状态反向，并遵循 reduced-motion 设置。
