---
title: HoverCard
description: 通过鼠标悬停或键盘焦点打开的非模态预览浮层。
---

# HoverCard

`HoverCard` 用于预览链接或其他可聚焦触发器背后的信息。默认延迟 700ms 打开、300ms
关闭。指针从 Trigger 移向内容时，安全移动区域会防止浮层意外关闭。

## 导入

```rust
use gpui_component::hover_card::{HoverCard, HoverCardAlign, HoverCardSide};
```

## 使用

```rust
HoverCard::new("user-preview")
    .trigger(
        Button::new("profile-link")
            .label("@huacnlee")
            .link()
    )
    .child(
        h_flex()
            .gap_3()
            .items_start()
            .child(Avatar::new("avatar", "Jason Lee"))
            .child(
                v_flex()
                    .gap_1()
                    .child(div().font_semibold().child("Jason Lee"))
                    .child(div().text_color(cx.theme().muted_foreground).child(
                        "The author of GPUI Component."
                    ))
            )
    )
```

默认显示在 Trigger 下方并居中，side offset 为 4px。Vega 使用 256px 宽度和 16px
内边距；Nova 与 Maia 根据当前 Style Preset 的语义 metrics 解析对应几何。

## 定位

`side` 与 `align` 相互独立：

```rust
HoverCard::new("placement")
    .side(HoverCardSide::Right)
    .align(HoverCardAlign::Start)
    .side_offset(px(8.))
    .align_offset(px(4.))
    .trigger(Button::new("trigger").label("Preview"))
    .child("Preview content")
```

side 支持 `Top`、`Right`、`Bottom`、`Left`；align 支持 `Start`、`Center`、`End`。
旧的 `anchor(Anchor)` builder 仍可使用，并会映射到新的定位模型。

## 受控状态

```rust
HoverCard::new("controlled")
    .open(self.preview_open)
    .on_open_change(cx.listener(|this, open, _, cx| {
        this.preview_open = *open;
        cx.notify();
    }))
    .trigger(Button::new("trigger").label("Preview"))
    .child("Controlled preview")
```

使用 `default_open(true)` 可创建初始打开的非受控 HoverCard。

## 自定义延迟与外观

```rust
HoverCard::new("custom")
    .open_delay(Duration::from_millis(500))
    .close_delay(Duration::from_millis(200))
    .appearance(false)
    .w(px(320.))
    .p_4()
    .rounded_lg()
    .bg(cx.theme().popover)
    .trigger(Button::new("trigger").label("Preview"))
    .child("Custom preview")
```

## 交互与可访问性

- 鼠标悬停和键盘焦点都能打开预览。
- 外层不会增加额外 Tab stop，Trigger 自身的 Enter 与点击行为保持不变。
- 内容是非模态浮层，不移动焦点，也不建立 focus trap。
- 不要把必要信息、Button、Input 或完整流程只放在 HoverCard 中；交互内容应使用
  `Popover` 或 `Dialog`。
- GPUI 当前没有等价于 Web 版本的 AccessKit 子树隐藏能力。预览内容应保持非交互，必要
  信息需要在可访问内容中同时提供。

## API

- `new(id)`
- `trigger(element)`
- `content(builder)`
- `side(HoverCardSide)` / `align(HoverCardAlign)`
- `side_offset(Pixels)` / `align_offset(Pixels)`
- `anchor(Anchor)` 兼容映射
- `default_open(bool)` / `open(bool)`
- `open_delay(Duration)` / `close_delay(Duration)`
- `on_open_change(callback)`
- `appearance(bool)`
- 所有 `Styled` builders

进入动画使用 100ms 淡入和方向感知的 8px 位移；退出动画使用 100ms 淡出且不发生位移，
与 shadcn 的方向动效约束一致。GPUI 尚不支持对任意元素子树执行不影响布局的缩放，因此
当前不模拟 shadcn 的 `scale(0.95)`。透明度通过各个 GPUI 绘制 primitive 分别相乘实现，
而不是把完成绘制的子树作为独立图层统一合成。

[Popover]: ./popover.md
