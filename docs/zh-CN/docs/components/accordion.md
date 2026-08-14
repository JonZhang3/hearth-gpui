---
title: Accordion
description: 基于稳定 value、同时支持受控与非受控状态的折叠组。
---

# Accordion

Accordion 通过垂直排列的 trigger 展开内容。每个 item 必须声明稳定 value，因此调整 item 顺序后，状态仍具有明确含义。

当前 Style Preset 负责间距和外观。Vega、Nova 使用普通分割列表，Maia 使用统一圆角外框及展开背景。只有页面需要覆盖 Preset 时才使用 `framed()`。

## 导入

```rust
use gpui_component::accordion::Accordion;
```

## 非受控单选 Accordion

```rust
Accordion::single("shipping-faq")
    .default_open_values(["shipping"])
    .item("shipping", |item| {
        item.title("有哪些配送方式？")
            .child("支持标准、加急和次日达配送。")
    })
    .item("returns", |item| {
        item.title("退货政策是什么？")
            .child("支持 30 天内退货。")
    })
```

`default_open_values()` 只初始化一次内部状态，后续状态由用户交互维护。

## 受控 Accordion

```rust
Accordion::single("shipping-faq")
    .open_values(open_values.clone())
    .on_open_change(cx.listener(|this, values, _, cx| {
        this.open_values = values.to_vec();
        cx.notify();
    }))
    .item("shipping", |item| item.title("配送").child("配送详情"))
    .item("returns", |item| item.title("退货").child("退货详情"))
```

受控模式下，`open_values()` 是状态权威。`on_open_change()` 按 item 声明顺序返回建议的新 values。

## 多项展开

```rust
Accordion::multiple("settings")
    .default_open_values(["general", "advanced"])
    .item("general", |item| item.title("常规").child("常规设置"))
    .item("advanced", |item| item.title("高级").child("高级设置"))
```

## 不允许全部关闭

```rust
Accordion::single("required-section")
    .collapsible(false)
    .default_open_values(["details"])
    .item("details", |item| item.title("详情").child("必填详情"))
```

## 外框与禁用状态

```rust
Accordion::single("framed-faq")
    .framed(true)
    .item("enabled", |item| item.title("可用").child("内容"))
    .item("disabled", |item| {
        item.disabled(true)
            .title("不可用")
            .child("不可用内容")
    })
```

Accordion 的 `disabled(true)` 会禁用整个组。item 与 group 的 disabled 状态使用逻辑或合并。

## 键盘操作

| 按键 | 行为 |
|---|---|
| `Enter` / `Space` | 切换当前 item |
| `ArrowDown` / `ArrowUp` | 在可用 trigger 之间循环移动焦点 |
| `Home` / `End` | 聚焦第一个或最后一个可用 trigger |

Trigger 会向 AccessKit 提供 Button role、expanded 和 disabled 状态。自定义 title 无法提供可靠名称时，应设置 `aria_label()`。

## API 参考

- `Accordion::single(id)` 创建单选组。
- `Accordion::multiple(id)` 创建多选组。
- `collapsible(bool)` 控制单选项能否全部关闭。
- `default_open_values(values)` 初始化非受控状态。
- `open_values(values)` 提供受控状态。
- `on_open_change(callback)` 返回建议的新稳定 values。
- `framed(bool)` 覆盖 Style Preset 的外框策略。
- `disabled(bool)` 禁用整个组。
- `item(value, builder)` 使用必填稳定 value 添加 item。

[Accordion]: https://docs.rs/gpui-component/latest/gpui_component/accordion/struct.Accordion.html
[AccordionItem]: https://docs.rs/gpui-component/latest/gpui_component/accordion/struct.AccordionItem.html
