---
title: Tooltip
description: 在鼠标悬停或键盘聚焦时显示紧凑的补充信息。
---

# Tooltip

Tooltip 用于为 Trigger 显示简短的补充信息。应用的 `Root` 统一管理 provider 和 overlay 生命周期，调用侧不需要额外放置 provider 组件。

## 导入

```rust
use hearth_gpui::tooltip::{
    Tooltip, TooltipAlign, TooltipSide, TooltipTrigger,
};
```

## 组件内置支持

`Button`、`Checkbox`、`Radio`、`Switch`、`Toggle` 等控件提供文本快捷 API：

```rust
Button::new("save")
    .label("Save")
    .tooltip("Save the current document")
```

`Button::tooltip_with_action` 可以显示 action 对应的平台快捷键：

```rust
Button::new("save")
    .label("Save")
    .tooltip_with_action("Save document", &SaveDocument, Some("Editor"))
```

## 组合式 API

任意元素、自定义定位或富内容应使用 `TooltipTrigger`：

```rust
TooltipTrigger::new("project-tooltip")
    .trigger(Button::new("project").label("Project"))
    .text("Open project settings")
    .side(TooltipSide::Right)
    .align(TooltipAlign::Start)
```

### 自定义内容

```rust
TooltipTrigger::new("status-tooltip")
    .trigger(Button::new("status").label("Status"))
    .content(|window, cx| {
        Tooltip::element(|_, cx| {
            v_flex()
                .gap_1()
                .child(div().font_medium().child("Project status"))
                .child(
                    div()
                        .text_color(cx.theme().background.opacity(0.8))
                        .child("All checks passed"),
                )
        })
        .build(window, cx)
    })
```

### 延迟和 Arrow

```rust
TooltipTrigger::new("instant-tooltip")
    .trigger(Button::new("instant").label("Instant"))
    .text("Opens immediately")
    .show_delay(Duration::ZERO)
    .hide_delay(Duration::from_millis(100))
    .side_offset(px(6.))
    .align_offset(px(4.))
    .show_arrow(false)
```

鼠标悬停默认采用桌面端习惯的 500 ms 延迟；相邻 Tooltip 共享 300 ms grace period。键盘聚焦会立即打开。鼠标按下和 Escape 会关闭 Tooltip。

## API 参考

### `TooltipTrigger`

| 方法 | 说明 |
| --- | --- |
| `new(id)` | 使用稳定 ID 创建 Trigger 状态 |
| `trigger(element)` | 设置 Trigger 子树 |
| `text(text)` | 设置文本内容和可访问性描述 |
| `content(builder)` | 构建自定义 `Tooltip` 内容 |
| `side(side)` | 使用 `Top`、`Right`、`Bottom` 或 `Left` |
| `align(align)` | 使用 `Start`、`Center` 或 `End` 交叉轴对齐 |
| `side_offset(px)` | 设置 Trigger 与 Surface 的距离 |
| `align_offset(px)` | 设置交叉轴偏移 |
| `show_delay(duration)` | 设置鼠标打开延迟 |
| `hide_delay(duration)` | 设置关闭延迟 |
| `show_arrow(bool)` | 显示或隐藏随方向定位的 Arrow |
| `arrow_color(color)` | 覆盖 Arrow 的语义颜色 |

### `Tooltip`

| 方法 | 说明 |
| --- | --- |
| `new(text)` | 创建文本内容 |
| `element(builder)` | 创建自定义元素内容 |
| `action(action, context)` | 解析并显示 action 快捷键 |
| `key_binding(stroke)` | 显示明确指定的平台快捷键 |
| `build(window, cx)` | 将 Surface 构建为 `AnyView` |

## 视觉和动效契约

- 使用 foreground 背景和 background 文字
- `text-xs`、12 px 水平 padding、6 px 垂直 padding、6 px 内容 gap
- 最大宽度 320 px
- 无边框、无阴影
- 圆角根据 Vega、Nova、Maia 的语义 Density 和 Radius 解析
- 使用语义 `motion.fast` 执行随 Side 变化的 8 px 位移和透明度过渡
- 退出与进入方向相反；Reduced Motion 直接到达最终状态

GPUI 当前无法对任意元素子树应用不影响布局的 scale，因此暂不实现 shadcn 的 `zoom-in-95` / `zoom-out-95`。

## 可访问性

- 文本 Tooltip 会将文本暴露为 Trigger 的辅助描述。
- Tooltip Surface 使用 Tooltip accessibility role。
- Trigger 保留自身 role、键盘激活和焦点行为。
- Tooltip 只能承载补充信息；必要说明必须同时出现在持久 UI 中。
