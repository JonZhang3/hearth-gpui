---
title: Toggle
description: 双状态按钮，以及可组合的单选或多选按钮组。
---

# Toggle

`Toggle` 是受控的双状态按钮，通过 `aria-pressed` 暴露状态。组件支持 shadcn Default、
Outline、语义尺寸、Invalid、Disabled、键盘焦点，以及可中断的颜色与焦点环过渡。

## 基础用法

```rust
Toggle::new("bold")
    .icon(IconName::Check)
    .label("Bold")
    .checked(self.bold)
    .on_click(cx.listener(|this, checked, _, cx| {
        this.bold = *checked;
        cx.notify();
    }))
```

纯图标 Toggle 必须设置 `.aria_label(...)`。Tooltip 也可以作为可访问名称的后备来源。

```rust
Toggle::new("preview")
    .icon(IconName::Eye)
    .aria_label("Toggle preview")
```

## 变体与尺寸

```rust
Toggle::new("default").label("Default");
Toggle::new("outline").outline().label("Outline");

Toggle::new("small").small().label("Small");
Toggle::new("medium").label("Default");
Toggle::new("large").large().label("Large");
```

`XSmall` 是 Hearth GPUI 扩展，不属于 shadcn Toggle API。

## 状态

```rust
Toggle::new("selected").label("Selected").checked(true);
Toggle::new("invalid").label("Invalid").invalid(true);
Toggle::new("disabled").label("Disabled").disabled(true);
Toggle::new("out-of-tab-order").label("Action").tab_stop(false);
```

Enter 与 Space 使用原生 Button 激活行为。鼠标获得焦点时不会绘制键盘焦点环。

## 前置与后置图标

```rust
Toggle::new("options")
    .icon(IconName::Star)
    .label("Options")
    .trailing_icon(IconName::ChevronDown)
```

明确的图标槽位让当前 Style Preset 可以解析图标大小与两侧 padding。

## ToggleGroup

`ToggleGroup` 管理受控选择值，并包含明确的 `ToggleGroupItem`。Item 使用稳定字符串 value，
不再依赖位置型 `Vec<bool>`。

### 单选

```rust
ToggleGroup::new("alignment")
    .mode(ToggleGroupMode::Single)
    .selection(ToggleGroupSelection::Single(self.alignment.clone()))
    .aria_label("Text alignment")
    .child(ToggleGroupItem::new("left").label("Left"))
    .child(ToggleGroupItem::new("center").label("Center"))
    .child(ToggleGroupItem::new("right").label("Right"))
    .on_change(cx.listener(|this, selection, _, cx| {
        if let ToggleGroupSelection::Single(value) = selection {
            this.alignment = value.clone();
            cx.notify();
        }
    }))
```

再次选择当前 Item 会清空单选值。

### 多选与连接布局

```rust
ToggleGroup::new("formatting")
    .mode(ToggleGroupMode::Multiple)
    .selection(ToggleGroupSelection::Multiple(self.formats.clone()))
    .outline()
    .spacing(px(0.))
    .aria_label("Text formatting")
    .child(
        ToggleGroupItem::new("bold")
            .icon(IconName::Check)
            .aria_label("Bold"),
    )
    .child(
        ToggleGroupItem::new("preview")
            .icon(IconName::Eye)
            .aria_label("Preview"),
    )
    .on_change(cx.listener(|this, selection, _, cx| {
        if let ToggleGroupSelection::Multiple(values) = selection {
            this.formats = values.clone();
            cx.notify();
        }
    }))
```

默认间距为 8px，对应 shadcn `spacing={2}`。`spacing(px(0.))` 会连接相邻边框，并按水平或
垂直方向设置首尾圆角。

### 垂直方向

```rust
ToggleGroup::new("vertical-tools")
    .orientation(Axis::Vertical)
    .spacing(px(0.))
    .aria_label("Vertical tools")
    .child(ToggleGroupItem::new("one").label("One"))
    .child(ToggleGroupItem::new("two").label("Two"))
```

水平组支持 Left/Right，垂直组支持 Up/Down；两者都支持 Home/End、跳过 Disabled Item，
并且整个 Group 只有一个 Tab 入口。

## 从旧位置型 API 迁移

将子项 `Toggle::checked(...)` 和 `on_click(&Vec<bool>)` 替换为稳定的
`ToggleGroupItem::new(value)`、`ToggleGroupSelection` 和 `on_change(...)`；将
`.segmented()` 替换为 `.spacing(px(0.))`。

## 动效

Toggle 只过渡边框以及 Focus/Invalid ring。Checked 和 Hover 背景会立即切换，不增加位移、
缩放、透明度、图标或挂载动画。Focus 或 Invalid 状态快速反转时会从当前可见值继续；
Reduced Motion 会立即到达最终状态。
