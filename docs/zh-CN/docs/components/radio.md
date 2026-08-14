---
title: Radio Group
description: 与 shadcn Radio Group 对齐的一组互斥选项。
---

# Radio Group

`RadioGroup` 通过受控 value 协调 `RadioGroupItem` 子项的选择、焦点、键盘导航、禁用状态和
Accessibility。`Radio` 仍可用于独立的受控渲染；互斥选项应优先使用 `RadioGroup`。

## 导入

```rust
use hearth_gpui::radio::{Radio, RadioGroup, RadioGroupItem};
```

## 基本用法

```rust
struct SettingsView {
    density: Option<SharedString>,
}

RadioGroup::new("density")
    .aria_label("Density")
    .value(self.density.clone())
    .child(RadioGroupItem::new("default").label("Default"))
    .child(RadioGroupItem::new("comfortable").label("Comfortable"))
    .child(RadioGroupItem::new("compact").label("Compact"))
    .on_change(cx.listener(|this, value: &SharedString, _, cx| {
        this.density = Some(value.clone());
        cx.notify();
    }))
```

value 在项目重新排序后仍保持稳定。再次选择当前项目不会清空选择。

## 排列方向

```rust
RadioGroup::horizontal("language")
    .aria_label("Language")
    .value(Some("rust"))
    .children([
        RadioGroupItem::new("rust").label("Rust"),
        RadioGroupItem::new("go").label("Go"),
        RadioGroupItem::new("swift").label("Swift"),
    ])
```

`RadioGroup::new` 和 `RadioGroup::vertical` 默认使用垂直排列。方向同时控制布局和方向键行为。

## 标签与描述

```rust
RadioGroup::vertical("plan")
    .aria_label("Plan")
    .value(Some("pro"))
    .child(
        RadioGroupItem::new("plus")
            .label("Plus")
            .aria_description("For individuals and small teams")
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child("For individuals and small teams"),
            ),
    )
    .child(
        RadioGroupItem::new("pro")
            .label("Pro")
            .aria_description("For growing businesses")
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child("For growing businesses"),
            ),
    )
```

集成标签会成为项目的可访问名称。补充内容如果包含选择所必需的信息，还应显式设置
`aria_description`。

## 禁用与无效状态

```rust
RadioGroup::vertical("notifications")
    .aria_label("Notifications")
    .value(Some("email"))
    .child(RadioGroupItem::new("email").label("Email"))
    .child(RadioGroupItem::new("sms").label("SMS").disabled(true))
    .child(
        RadioGroupItem::new("push")
            .label("Push")
            .invalid(true),
    )
```

Group 的 `disabled(true)` 会在渲染时与项目自身的 disabled 状态合并，不会永久改写项目状态。

## 独立 Radio

```rust
Radio::new("standalone-radio")
    .label("Standalone option")
    .checked(self.checked)
    .on_click(cx.listener(|this, checked: &bool, _, cx| {
        this.checked = *checked;
        cx.notify();
    }))
```

Radio 激活只会请求 `true`；再次激活已经选中的 Radio 不会取消选择。

## 键盘行为

| 按键 | 行为 |
|---|---|
| `Tab` / `Shift+Tab` | 通过已选项目进入或离开 Group；没有选择时使用首个可用项目 |
| `ArrowLeft` / `ArrowRight` | 在横向 Group 内移动并选择，到边界后循环 |
| `ArrowUp` / `ArrowDown` | 在纵向 Group 内移动并选择，到边界后循环 |
| `Home` / `End` | 选择首个或最后一个可用项目 |
| `Space` | 选择当前获得焦点的项目 |

导航会跳过禁用项目。鼠标焦点不会显示仅供键盘使用的 Focus Ring。

## 视觉与动效

- 默认 Vega 几何为 16px 圆形控件和 8px 选中圆点。
- Vega、Maia 的 Group 间距为 12px；紧凑的 Nova 为 8px。
- Light 模式下未选中控件透明；Dark 模式使用语义化 input 表面色。
- Checked、unchecked、invalid 和 focus 的颜色变化立即完成。固定版本 shadcn 没有为 Radio Group
  声明 Indicator 或颜色过渡。
- `Sizable` 是 Hearth GPUI 为特殊紧凑或大尺寸组合保留的扩展；默认尺寸是 shadcn 验收基准。

## API

### RadioGroup

| 方法 | 说明 |
|---|---|
| `new(id)` | 创建纵向受控 Group |
| `horizontal(id)` / `vertical(id)` | 创建指定方向的 Group |
| `orientation(Axis)` | 修改布局和方向键导航轴 |
| `value(Option<T>)` | 设置受控选择值 |
| `aria_label(text)` | 设置 Group 的可访问名称 |
| `child(item)` / `children(items)` | 添加带稳定 value 的类型化项目 |
| `disabled(bool)` | 在渲染时禁用全部项目 |
| `on_change(fn)` | 返回新选择的稳定 value |

### RadioGroupItem

| 方法 | 说明 |
|---|---|
| `new(value)` | 使用稳定 value 和默认 ID 创建项目 |
| `label(text)` | 设置可见标签和可访问名称 |
| `aria_label(text)` | 设置不依赖可见文本的可访问名称 |
| `aria_description(text)` | 设置补充可访问描述 |
| `disabled(bool)` | 禁用当前项目 |
| `invalid(bool)` | 设置无效语义和视觉状态 |
| `tooltip(text)` | 添加 Tooltip |

`Radio`、`RadioGroupItem` 和 `RadioGroup` 均实现 `Styled`。`Radio` 和 `RadioGroupItem` 还实现
`Sizable`。
