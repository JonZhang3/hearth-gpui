---
title: Label
description: 用于表单控件和行内组合内容的标签组件。
---

# Label

`Label` 默认采用 shadcn Vega 基准：行内 flex 布局、`gap-2`、`text-sm`、中等字重和紧凑行高。组件没有内置动效。

## 导入

```rust
use gpui_component::{Disableable as _, label::Label};
```

## 基础用法

```rust
Label::new("Username")
```

## 与 Input 组合

使用 `for_focus` 让鼠标点击 Label 时聚焦关联控件。GPUI 当前没有公开跨元素的 `labelled_by` 关系，因此控件仍需提供自己的无障碍名称。

```rust
let input_focus = input_state.read(cx).focus_handle(cx);

v_flex()
    .gap_2()
    .child(Label::new("Username").for_focus(&input_focus))
    .child(Input::new(&input_state).aria_label("Username"))
```

## 禁用状态

禁用后的 Label 使用 50% 透明度，并且不会聚焦关联控件。

```rust
Label::new("Username")
    .for_focus(&input_focus)
    .disabled(true)
```

## 组合内容

使用 `empty` 和 `ParentElement` 组合图标或其他行内元素。

```rust
Label::empty()
    .child(Icon::new(IconName::Info).xsmall())
    .child("Additional information")
```

对于 `Checkbox`、`Radio` 和 `Switch`，优先使用组件自身的 `label` API，确保视觉标签和无障碍语义保持一致。

## 项目扩展能力

原有的次要文本、掩码、高亮和 `Styled` 覆盖能力继续保留。

```rust
Label::new("Company Address")
    .secondary("(optional)")
    .highlights("company")

Label::new("9,182.1 USD").masked(true)
```

`HighlightsMatch::Prefix` 只高亮从第一个字符开始的匹配。匹配不区分大小写，并保证返回有效的 UTF-8 字节边界。

## API

| 方法 | 说明 |
| --- | --- |
| `Label::new(text)` | 创建文本标签 |
| `Label::empty()` | 创建用于组合子元素的标签 |
| `.for_focus(&handle)` | 主鼠标键按下时聚焦启用的目标 |
| `.disabled(bool)` | 设置禁用样式和交互 |
| `.secondary(text)` | 添加弱化的次要文本 |
| `.highlights(match)` | 高亮完整匹配或前缀匹配 |
| `.masked(bool)` | 使用圆点替换显示字符 |
| `.child(element)` | 添加行内组合内容 |

可以使用所有标准 `Styled` 方法覆盖默认样式。
