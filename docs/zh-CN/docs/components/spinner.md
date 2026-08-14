---
title: Spinner
description: 显示无法确定完成百分比的加载状态。
---

# Spinner

`Spinner` 用于表示任务正在进行，但无法提供完成百分比。默认外观与 pinned shadcn Spinner 对齐：16px 的圆弧 Loader，继承周围文字颜色并持续旋转。

## 导入

```rust
use hearth_gpui::spinner::{Spinner, SpinnerAnimation, SpinnerVariant};
```

## 用法

```rust
Spinner::new()
```

默认辅助名称为 `Loading`。如果任务上下文有助于理解状态，可以提供更明确的名称：

```rust
Spinner::new()
    .aria_label("正在加载项目")
```

## 尺寸

默认尺寸为 16px。`Spinner` 实现了 `Sizable`，也支持精确自定义尺寸：

```rust
Spinner::new().with_size(px(12.))
Spinner::new()                    // 16px
Spinner::new().with_size(px(24.))
Spinner::new().with_size(px(32.))
```

GPUI 特有的 `.xsmall()`、`.small()` 和 `.large()` 仍可用于和现有控件组合。

## Variant 和动画

两种内置 Variant 提供相互匹配的图标和动画默认值：

```rust
// LoaderCircle + 持续 linear 旋转（默认，与 shadcn 对齐）
Spinner::new().variant(SpinnerVariant::Circular)

// 原分段 Loader + 语义缓动旋转（GPUI 经典样式）
Spinner::new().variant(SpinnerVariant::Classic)
```

图标和动画可以分别覆盖。无论 Builder 的调用顺序如何，显式覆盖始终优先：

```rust
Spinner::new()
    .icon(IconName::LoaderCircle)
    .animation(SpinnerAnimation::SemanticSpin)
    .variant(SpinnerVariant::Circular)
```

## 颜色和图标

Spinner 默认继承当前文字颜色。只有在周围语义颜色不适合时才需要覆盖：

```rust
Spinner::new().color(cx.theme().muted_foreground)

Spinner::new()
    .icon(IconName::Loader)
    .color(cx.theme().blue)
```

默认图标为 `IconName::LoaderCircle`，对应 shadcn 的圆弧式 Loader2 外观。也可以通过 `.icon(...)` 使用任意兼容的 `Icon`。

## 组合

```rust
Button::new("submit")
    .icon(Spinner::new())
    .label("Submitting")
    .disabled(true)

Badge::new()
    .outline()
    .leading(Spinner::new().xsmall())
    .child("Generating")
```

Spinner 也可以放入 `InputGroupAddon`、Empty 状态及其他元素插槽。

## 动效

- 旋转：完整一周。
- 时长：使用当前 Style Preset 的语义 `motion.loading()` 时长；内置 Preset 均为 1 秒。
- 缓动：Circular 使用 linear；Classic 使用当前 Style Preset 的 move easing。
- 生命周期：挂载期间无限循环，不包含进入、退出、透明度或缩放过渡。
- Reduced Motion：静态显示 Loader。

`SpinnerAnimation::SemanticSpin` 使用当前 Style Preset 的 move easing 完成整周旋转，从而恢复原 Spinner 行为。`.ease(...)` 可以覆盖两种动画的默认 easing；`LinearSpin` 仍是与 shadcn 对齐的默认行为。

## 稳定 ID

`Spinner::new()` 默认从调用位置生成稳定 ID。如果迭代器在同一源码位置创建多个 Spinner，需要提供结构化 ID：

```rust
items.into_iter().enumerate().map(|(index, _)| {
    Spinner::new().id(ElementId::named_usize("row-spinner", index))
})
```

## API

| 方法 | 用途 |
| --- | --- |
| `new()` | 创建 16px 圆弧式加载 Spinner |
| `id(id)` | 覆盖稳定元素 ID |
| `aria_label(text)` | 设置辅助技术播报的加载状态名称 |
| `variant(variant)` | 选择图标与动画组合预设 |
| `icon(icon)` | 替换圆弧 Loader 图标 |
| `animation(animation)` | 独立选择 LinearSpin 或 SemanticSpin 旋转 |
| `color(color)` | 覆盖继承的文字颜色 |
| `with_size(size)` | 设置命名尺寸或精确尺寸 |
| `ease(easing)` | 覆盖默认 linear 缓动 |
