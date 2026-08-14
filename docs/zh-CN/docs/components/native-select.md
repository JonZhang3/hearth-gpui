---
title: Native Select
description: 使用操作系统选项菜单的紧凑型单值选择器。
---

# Native Select

`NativeSelect` 是 shadcn 原生 `<select>` 组合在 GPUI 桌面端的对应实现。Trigger 使用当前的
Color Theme 与 Style Preset；macOS 和 Windows 中的选项菜单由操作系统绘制。macOS 通过 AppKit
将选中菜单项原生定位到 Trigger；Linux 使用项目现有的 GPUI PopupMenu fallback。

如果选项需要搜索、复杂行内容或虚拟化，请使用 [`Select`](select)。

## 导入

```rust
use gpui_component::native_select::{
    NativeSelect, NativeSelectOptGroup, NativeSelectOption,
};
```

## 基础用法

```rust
NativeSelect::new("status")
    .value(self.status.clone())
    .aria_label("Status")
    .child(NativeSelectOption::new("", "Select status"))
    .child(NativeSelectOption::new("todo", "Todo"))
    .child(NativeSelectOption::new("in-progress", "In Progress"))
    .child(NativeSelectOption::new("done", "Done"))
    .on_change(cx.listener(|this, value, _, cx| {
        this.status = value.clone();
        cx.notify();
    }))
```

非受控模式使用 `default_value(...)`，受控模式使用 `value(...)`。

## 分组与禁用选项

```rust
NativeSelect::new("department")
    .child(NativeSelectOption::new("", "Select department"))
    .child(
        NativeSelectOptGroup::new("Engineering")
            .child(NativeSelectOption::new("frontend", "Frontend"))
            .child(NativeSelectOption::new("backend", "Backend"))
            .child(NativeSelectOption::new("devops", "DevOps").disabled(true)),
    )
    .child(
        NativeSelectOptGroup::new("Sales")
            .child(NativeSelectOption::new("sales-rep", "Sales Rep"))
            .child(NativeSelectOption::new("account-manager", "Account Manager")),
    )
```

## 尺寸与状态

```rust
NativeSelect::new("small").small()
NativeSelect::new("disabled").disabled(true)
NativeSelect::new("invalid").invalid(true)
```

`Tab` 聚焦 Trigger，`Enter` 或 `Space` 打开原生菜单。Arrow Up/Down、Home、End 以及可打印字符
快速选择可在不打开菜单的情况下切换值，并跳过禁用选项。Trigger 通过 AccessKit 暴露 ComboBox
值、禁用和无效状态。原生菜单内部的外观、导航及关闭行为遵循桌面平台。

## Style Preset

Vega 是默认基线。Nova 与 Maia 通过语义化 Style Metrics 解析控件高度、密度、圆角、阴影、焦点
几何和动效；组件不会判断 Style Preset 标识。
