---
title: AspectRatio
description: 保持固定宽高比的布局容器。
---

# AspectRatio

`AspectRatio` 在适应可用空间的同时,让内容保持固定的 `宽度 / 高度` 比例。它直接使用 GPUI 原生的宽高比布局能力。

## 引入

```rust
use gpui_component::aspect_ratio::AspectRatio;
```

## 用法

通过父容器或组件的 `Styled` API 约束宽度或高度:

```rust
AspectRatio::new(16.0 / 9.0)
    .w(px(480.))
    .rounded(cx.theme().style.radii.lg)
    .bg(cx.theme().muted)
    .child(content)
```

比例使用 `宽度 / 高度` 表示。常用值包括 `16.0 / 9.0`、`1.0` 和 `9.0 / 16.0`。

## API 参考

| 方法 | 说明 |
| --- | --- |
| `new(ratio)` | 使用指定宽高比创建容器 |
| `ratio(ratio)` | 替换当前宽高比 |
| `child(c)` / `children(cs)` | 向容器添加内容 |

`AspectRatio` 实现了 `Styled`。无效、非正数或非有限比例会安全回退到 `1:1`。

## 注意事项

- 组件只负责布局,不会默认添加背景、圆角、裁切、阴影或动画。
- 父级必须约束至少一个轴。容器默认填满可用宽度,并根据比例计算高度。
- Vega、Nova 和 Maia 使用完全相同的比例行为。视觉样式由调用方负责,与 Style Preset 相互独立。
- 需要圆角图片时,应直接为图片设置圆角;GPUI 目前尚未为任意子元素提供完整的圆角 Clip Chain。
