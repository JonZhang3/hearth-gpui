---
title: Separator
description: 在水平或垂直布局中分隔内容。
---

# Separator

Separator 使用当前 Color Theme 的语义 `border` 颜色绘制一条 1px 分隔线。它是装饰性的，
不参与交互。默认方向为水平。

## 导入

```rust
use gpui_component::separator::Separator;
```

## 水平分隔线

```rust
v_flex()
    .gap_4()
    .child("First section")
    .child(Separator::new())
    .child("Second section")
```

`Separator::horizontal()` 是等价的快捷构造方法。

## 垂直分隔线

```rust
h_flex()
    .h_5()
    .gap_4()
    .child("Blog")
    .child(Separator::vertical())
    .child("Docs")
    .child(Separator::vertical())
    .child("Source")
```

也可以使用 `Separator::new().orientation(Axis::Vertical)` 显式选择方向。

## GPUI 扩展

虚线、标签和颜色覆盖作为 GPUI Component 扩展继续保留：

```rust
Separator::horizontal_dashed()
Separator::horizontal().label("OR")
Separator::vertical().color(cx.theme().danger)
```

pinned shadcn 源码没有为 Separator 声明 transition，因此该组件不包含动画。
