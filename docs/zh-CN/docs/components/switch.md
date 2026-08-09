---
title: Switch
description: 用于在选中和未选中之间切换的开关控件。
---

# Switch

Switch 是一个二元开关组件，适合表示开启 / 关闭状态。它支持平滑动画、不同尺寸、标签、禁用状态和自定义颜色。

## 导入

```rust
use gpui_component::switch::Switch;
```

## 用法

### 基础 Switch

```rust
Switch::new("my-switch")
    .aria_label("Airplane mode")
    .checked(false)
    .on_click(|checked, _, _| {
        println!("Switch is now: {}", checked);
    })
```

### 受控 Switch

```rust
struct MyView {
    is_enabled: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Switch::new("switch")
            .aria_label("Enable feature")
            .checked(self.is_enabled)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.is_enabled = *checked;
                cx.notify();
            }))
    }
}
```

### 带标签

```rust
Switch::new("notifications")
    .label("Enable notifications")
    .checked(true)
    .on_click(|checked, _, _| {
        println!("Notifications: {}", if *checked { "enabled" } else { "disabled" });
    })
```

### 不同尺寸

```rust
Switch::new("small-switch")
    .small()
    .label("Small switch")

Switch::new("medium-switch")
    .label("Medium switch")

Switch::new("custom-switch")
    .with_size(Size::Small)
    .label("Custom size")
```

### 禁用状态

```rust
Switch::new("disabled-off")
    .label("Disabled (off)")
    .disabled(true)
    .checked(false)

Switch::new("disabled-on")
    .label("Disabled (on)")
    .disabled(true)
    .checked(true)
```

### 无效状态与可访问名称

```rust
Switch::new("required-setting")
    .aria_label("Enable required setting")
    .invalid(true)
    .checked(false)
```

具有 `on_click` handler 的启用状态 Switch 可通过键盘聚焦，按 `Space` 切换状态。没有
activation handler 的 Switch 按只读状态呈现，不会产生无效的 Tab stop。焦点环只用于
键盘焦点，鼠标操作不会显示 keyboard focus-visible ring。

### 自定义颜色

`color()` 用于覆盖选中状态下的背景色；禁用态透明度会自动叠加：

```rust
Switch::new("switch")
    .label("Success")
    .checked(true)
    .color(cx.theme().success)

Switch::new("switch")
    .label("Danger")
    .checked(true)
    .color(cx.theme().danger)

Switch::new("switch")
    .label("Disabled")
    .checked(true)
    .color(cx.theme().success)
    .disabled(true)
```

### 带 Tooltip

```rust
Switch::new("switch")
    .label("Airplane mode")
    .tooltip("Enable airplane mode to disable all wireless connections")
    .checked(false)
```

## API 参考

### Switch

| 方法 | 说明 |
| --- | --- |
| `new(id)` | 使用给定 ID 创建开关 |
| `checked(bool)` | 设置当前选中状态 |
| `label(text)` | 设置标签文本 |
| `aria_label(text)` | 设置独立于可见标签的可访问名称 |
| `label_side(side)` | 设置标签位置，`Side::Left` 或 `Side::Right` |
| `invalid(bool)` | 设置 destructive 无效状态及其可访问语义 |
| `tab_stop(bool)` | 设置是否参与顺序键盘焦点 |
| `tab_index(index)` | 设置键盘 Tab index |
| `disabled(bool)` | 设置禁用状态 |
| `tooltip(text)` | 添加提示文本 |
| `color(color)` | 设置选中时的背景色，默认 `theme.primary` |
| `on_click(fn)` | 点击回调，参数为新的 `&bool` 状态 |

### 样式

实现了 `Sizable` 和 `Disableable` trait：

- `small()`：shadcn 小尺寸，`24x14px`，thumb 为 `12px`
- `medium()`：shadcn 默认尺寸，`32x18.4px`，thumb 为 `16px`
- `with_size(size)`：显式设置尺寸
- `disabled(bool)`：禁用状态

## 示例

### 设置面板

```rust
struct SettingsView {
    marketing_emails: bool,
    security_emails: bool,
    push_notifications: bool,
}

v_flex()
    .gap_4()
    .child(
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .child(Label::new("Marketing emails").text_lg())
                            .child(
                                Label::new("Receive emails about new products and features")
                                    .text_color(theme.muted_foreground)
                            )
                    )
                    .child(
                        Switch::new("marketing")
                            .checked(self.marketing_emails)
                            .on_click(cx.listener(|view, checked, _, cx| {
                                view.marketing_emails = *checked;
                                cx.notify();
                            }))
                    )
            )
    )
```

### 紧凑设置列表

```rust
v_flex()
    .gap_3()
    .child(
        Switch::new("wifi")
            .label("Wi-Fi")
            .label_side(Side::Left)
            .checked(true)
            .small()
    )
    .child(
        Switch::new("bluetooth")
            .label("Bluetooth")
            .label_side(Side::Left)
            .checked(false)
            .small()
    )
```

## 动画

Switch 使用当前 Style Preset 的语义 motion tokens：

- Track 背景、边框、焦点环、禁用透明度与 Thumb 位置共享同一个 150ms normal
  transition 采样进度，保证每一帧同步。
- Thumb 在选中与未选中位置之间移动，不进行条件挂载。
- 快速反向切换从当前采样值继续，不会从端点重新开始。
- Reduced motion 立即显示最终状态。
- GPUI 无法插值任意 fill，因此 gradient 背景采用原子切换。
