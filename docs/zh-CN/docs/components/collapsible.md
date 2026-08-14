---
title: Collapsible
description: 可展开和收起内容的交互式组件。
---

# Collapsible

Collapsible 是一个用于展开和收起内容的交互式组件。

## 导入

```rust
use hearth_gpui::collapsible::Collapsible;
```

## 用法

### 基础用法

```rust
Collapsible::new()
    .id("account-details")
    .max_w_128()
    .gap_1()
    .open(self.open)
    .child(
        "This is a collapsible component. \
        Click the header to expand or collapse the content.",
    )
    .content(
        "This is the full content of the Collapsible component. \
        It is only visible when the component is expanded. \n\
        You can put any content you like here, including text, images, \
        or other UI elements.",
    )
    .child(
        h_flex().justify_center().child(
            Button::new("toggle1")
                .icon(IconName::ChevronDown)
                .label("Show more")
                .when(open, |this| {
                    this.icon(IconName::ChevronUp).label("Show less")
                })
                .xsmall()
                .link()
                .on_click({
                    cx.listener(move |this, _, _, cx| {
                        this.open = !this.open;
                        cx.notify();
                    })
                }),
        ),
    )
```

通过 `open` 控制展开状态。设置稳定的 `id` 后，组件会测量动态内容高度，并使用共享 Style motion 完成进入和退出动画；关闭内容会保留到退出结束，关闭过程中重新打开会安全中断旧任务，reduced motion 会移除等待时间。未设置 `id` 时保留原有即时显示/隐藏行为。

[Collapsible]: https://docs.rs/hearth-gpui/latest/hearth_gpui/collapsible/struct.Collapsible.html
