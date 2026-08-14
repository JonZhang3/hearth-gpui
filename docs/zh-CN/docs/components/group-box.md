---
title: GroupBox
description: 用于组织相关内容的轻量语义容器。
---

# GroupBox

`GroupBox` 用于组织相关控件或内容，不引入 Card 的 elevation 和交互行为。它支持普通、填充和描边表面，同时保留应用对内容组合的控制。

## 导入

```rust
use hearth_gpui::group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants as _};
```

## 基础用法

```rust
GroupBox::new()
    .id("subscriptions")
    .aria_label("订阅设置")
    .title("Subscriptions")
    .child(Checkbox::new("all").label("All"))
    .child(Checkbox::new("newsletter").label("Newsletter"))
    .child(Button::new("save").label("Save"))
```

显式 ID 允许 GPUI 将根节点暴露为可访问性 `Group`。当分组包含交互控件或 title 是自定义元素时，应设置 `aria_label`。

## 变体

```rust
// 不绘制表面，也不增加内容 padding。
GroupBox::new()
    .id("plain")
    .normal()
    .title("Plain")
    .child("Content")

// 使用语义 GroupBox 背景和密度感知的内容 padding。
GroupBox::new()
    .id("filled")
    .fill()
    .title("Filled")
    .child("Content")

// 使用语义边框和密度感知的内容 padding。
GroupBox::new()
    .id("outlined")
    .outline()
    .title("Outlined")
    .child("Content")
```

| 变体 | 背景 | 边框 | 内容 padding |
| --- | --- | --- | --- |
| `Normal` | 无 | 无 | 无 |
| `Fill` | `tokens.group_box` | 无 | Style Preset density |
| `Outline` | 无 | Theme `border` | Style Preset density |

GroupBox 不增加阴影。需要 elevation 或分区表面时应使用 Card。

## Theme 与 Style Preset

GroupBox 消费以下语义值：

- `group_box.background`
- `group_box.foreground`
- `group_box.title.foreground`
- Theme `border`
- Style Preset `radii.md`
- Style Preset `density`

Compact、Standard 和 Comfortable 会调整内容 padding、内容 gap、标题与内容间距以及标题行高。实现不会判断 Vega、Nova 或 Maia ID。

## 样式层级

```rust
GroupBox::new()
    .id("custom")
    .aria_label("自定义分组")
    .outline()
    // Styled refinement 作用于外层分组布局。
    .gap_6()
    .title("Custom title")
    // title_style 只作用于标题包装层。
    .title_style(StyleRefinement::default().font_semibold())
    // content_style 只作用于内容表面。
    .content_style(
        StyleRefinement::default()
            .rounded_lg()
            .border_2()
    )
    .child("Content")
```

内置 metrics 会先应用，调用方的显式 style refinement 保持最高优先级。

## 长内容

根容器、标题和内容表面均使用 `min_w_0`，可在受限父容器中正常收缩。具体换行或截断行为仍由子内容决定。

## API 参考

### GroupBox

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建普通 GroupBox |
| `id(id)` | 设置稳定的 GPUI 与可访问性身份 |
| `aria_label(label)` | 设置可访问分组名称 |
| `title(element)` | 设置可选标题内容 |
| `title_style(style)` | 调整标题包装层样式 |
| `content_style(style)` | 调整内容表面样式 |
| `normal()` | 使用普通变体 |
| `fill()` | 使用填充变体 |
| `outline()` | 使用描边变体 |

### GroupBoxVariant

`GroupBoxVariant` 支持 `Normal`、`Fill` 和 `Outline`，并提供 `from_str` 与 `as_str` 用于设置持久化。

## 相关组件

- **Settings** 使用 GroupBox 作为视觉分组表面。
- **Card** 提供更强的分区和可选 elevation 表面。
- **Accordion** 用于组织可折叠内容。
