---
title: Kbd
description: 展示键盘输入和组合快捷键。
---

# Kbd

`Kbd` 用于展示按键、快捷键文本、图标或其他静态键盘输入。`KbdGroup` 用于组合多个按键，不会引入交互或焦点行为。

## 导入

```rust
use gpui_component::kbd::{Kbd, KbdGroup};
```

## 基础用法

```rust
use gpui::ParentElement as _;

Kbd::new().child("Ctrl")
Kbd::new().child("⌘K")
Kbd::new().child("Ctrl + B")
```

## 组合按键

```rust
KbdGroup::new()
    .child(Kbd::new().child("Ctrl"))
    .child(Kbd::new().child("Shift"))
    .child(Kbd::new().child("P"))
```

也可以在按键之间直接组合分隔符：

```rust
KbdGroup::new()
    .child(Kbd::new().child("Ctrl"))
    .child("+")
    .child(Kbd::new().child("B"))
```

## 平台快捷键

需要根据平台显示快捷键时，使用 `from_keystroke`：

```rust
use gpui::Keystroke;

let stroke = Keystroke::parse("cmd-shift-p").unwrap();
let kbd = Kbd::from_keystroke(stroke.clone());
let kbd: Kbd = stroke.into();
```

macOS 使用符号并省略分隔符；Windows 和 Linux 使用文字标签与 `+` 分隔符。

| 输入 | macOS | Windows/Linux |
| --- | --- | --- |
| `cmd-a` | `⌘A` | `Win+A` |
| `ctrl-shift-a` | `⌃⇧A` | `Ctrl+Shift+A` |
| `escape` | `⎋` | `Esc` |
| `enter` | `⏎` | `Enter` |

## 图标与文字

图标应使用 12px 的 `xsmall` 尺寸，与 shadcn Kbd 基线一致。

```rust
use gpui_component::{Icon, IconName, Sizable as _};

Kbd::new()
    .child(Icon::new(IconName::ArrowLeft).xsmall())
    .child("Left")
```

Kbd 是静态内容，不应在内部放置按钮或其他交互控件。

## Input Group

```rust
InputGroup::new("search")
    .input(Input::new(&input_state))
    .addon(
        InputGroupAddon::new()
            .align(InputGroupAddonAlign::InlineEnd)
            .child(Kbd::from_keystroke(
                Keystroke::parse("cmd-k").unwrap(),
            )),
    )
```

## Action 绑定

```rust
if let Some(kbd) = Kbd::binding_for_action(&MyAction {}, None, window) {
    // 渲染当前平台解析出的快捷键。
}

if let Some(kbd) =
    Kbd::binding_for_action_in(&MyAction {}, &focus_handle, window)
{
    // 渲染当前焦点上下文中的快捷键。
}
```

只需要格式化文字时，使用 `Kbd::format(&stroke)`。

## 项目扩展

默认样式与 shadcn 对齐，同时保留两个项目特有的显式选项：

```rust
Kbd::new().child("Outline").outline()
Kbd::new().child("Unstyled").appearance(false)
```

## 样式

默认样式使用语义主题和当前 Style Preset：

- 高度和最小宽度均为 20px
- 水平内边距与子元素间距均为 4px
- `text-xs` 与中等字重
- `muted` 背景与 `muted_foreground` 文字
- `radii.sm` 圆角
- 不包含过渡或动画

`Styled` 覆盖会在默认样式之后应用。
