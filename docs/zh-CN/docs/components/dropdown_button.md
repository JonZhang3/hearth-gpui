---
title: DropdownButton
description: DropdownButton 由一个主按钮和一个触发下拉菜单的按钮组合而成。
---

# DropdownButton

[DropdownButton] 将主操作按钮与相邻的菜单 Trigger 组合在一起。左侧主按钮保留自己的点击处理逻辑，右侧按钮负责打开 [PopupMenu]。

组件会把变体、尺寸、禁用状态、选中状态和圆角配置统一传递给两个按钮。几何、颜色、焦点环、阴影和密度直接复用 [Button] 的 Color Theme 与 Style Preset 语义，不维护额外的固定样式。

## 导入

```rust
use hearth_gpui::button::{Button, DropdownButton};
```

## 用法

```rust
use gpui::Anchor;

DropdownButton::new("dropdown")
    .aria_label("文档操作")
    .menu_aria_label("打开文档选项")
    .button(
        Button::new("dropdown-primary")
            .label("保存")
            .on_click(|_, _, _| {
                // 执行主操作。
            }),
    )
    .dropdown_menu(|menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
            .menu("Option 2", Box::new(MyAction))
            .separator()
            .menu("Option 3", Box::new(MyAction))
    })
```

### 变体

与 [Button] 一样，DropdownButton 支持不同视觉变体：

```rust
DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Default"))
    .dropdown_menu(|menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

### 自定义锚点

```rust
DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Click Me"))
    .dropdown_menu_with_anchor(Anchor::BottomRight, |menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

### 无障碍

当周围上下文不能充分说明组合按钮用途时，使用 `.aria_label(...)` 为组合控件命名。菜单 Trigger 默认使用本地化的“更多选项”无障碍名称；菜单用途更具体时可通过 `.menu_aria_label(...)` 覆盖。

[Button]: https://docs.rs/hearth-gpui/latest/hearth_gpui/button/struct.Button.html
[DropdownButton]: https://docs.rs/hearth-gpui/latest/hearth_gpui/button/struct.DropdownButton.html
[PopupMenu]: https://docs.rs/hearth-gpui/latest/hearth_gpui/menu/struct.PopupMenu.html
[Sizable]: https://docs.rs/hearth-gpui/latest/hearth_gpui/trait.Sizable.html
