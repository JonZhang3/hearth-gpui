---
title: Sheet
description: 从窗口边缘滑入的模态内容面板。
---

# Sheet

Sheet 用于在窗口边缘显示辅助导航、表单或设置内容。组件支持四个方向、焦点陷阱、响应式侧边尺寸、滚动、背景关闭、退出动效和焦点恢复。

## 导入

```rust
use gpui_component::{Placement, WindowExt};
```

窗口的第一层视图必须是 [`Root`](/docs/root)。使用自定义 Root 组合时，需要渲染
`Root::render_sheet_layer`。

## 基础用法

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("编辑个人资料")
        .description("修改资料后保存。")
        .child(profile_form)
        .footer(
            v_flex()
                .gap_2()
                .child(Button::new("save").label("保存修改").w_full())
                .child(
                    Button::new("cancel")
                        .outline()
                        .label("取消")
                        .w_full()
                        .on_click(|_, window, cx| window.close_sheet(cx)),
                ),
        )
})
```

`open_sheet` 默认从右侧打开。左右 Sheet 默认使用可用宽度的 75%，最大为 384px；上下
Sheet 默认使用内容高度。

## 方向与尺寸

```rust
window.open_sheet_at(Placement::Left, cx, |sheet, _, _| {
    sheet.title("导航").child(navigation)
});

window.open_sheet_at(Placement::Bottom, cx, |sheet, _, _| {
    sheet
        .title("活动记录")
        .description("最近的工作区活动")
        .size(px(320.))
        .child(activity)
});
```

`size` 覆盖左右方向的宽度或上下方向的高度。Sheet 不提供拖拽调整尺寸功能。

## Header 组合

文字标题和描述会自动成为可访问名称与描述：

```rust
sheet
    .title("应用设置")
    .description("配置外观和通知")
```

自定义元素需要同时提供明确的辅助功能信息：

```rust
sheet
    .title_element(custom_title)
    .description_element(custom_description)
    .aria_label("应用设置")
    .aria_description("配置外观和通知")
```

标题和描述都不存在时，不会生成空白 Header。

## Close 按钮与背景层

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("隐藏关闭按钮")
        .show_close_button(false)
        .overlay(true)
        .overlay_closable(true)
        .child("使用 Escape 或点击背景层关闭。")
})
```

- `show_close_button(false)` 隐藏图标按钮，并且不会留下空白标题行。
- `overlay(false)` 只隐藏背景绘制，仍然保留模态遮挡和焦点陷阱。
- `overlay_closable` 只控制可见背景层的主键点击关闭。
- Escape 可以关闭 Sheet。
- `on_close` 监听 Escape、背景点击和内置关闭按钮产生的用户关闭操作；直接调用
  `window.close_sheet(cx)` 属于程序化关闭，不触发该回调。

## 初始焦点

```rust
let name_focus = name_input.read(cx).focus_handle(cx);

window.open_sheet(cx, move |sheet, _, _| {
    sheet
        .title("编辑个人资料")
        .initial_focus(name_focus.clone())
        .child(Input::new(&name_input))
})
```

未设置 `initial_focus` 时，Sheet 会聚焦第一个有效 tab stop；没有可聚焦子元素时，焦点保留在
Dialog surface。退出动效完成后，焦点返回此前获得焦点的控件。

## 原生 TitleBar 安全区

与 Web 组件不同，Sheet 会保留桌面窗口 TitleBar 的拖拽区域。需要修改时，在打开 Sheet 前配置
Theme：

```rust
theme.sheet.margin_top = px(32.);
```

Sheet 不提供单独的 `margin_top` builder。

## 自定义样式

`Sheet` 实现了 `Styled`。样式 refinement 只作用于 Sheet surface，不再重复应用到 Body：

```rust
window.open_sheet(cx, |sheet, _, cx| {
    sheet
        .title("自定义 Sheet")
        .bg(cx.theme().accent)
        .text_color(cx.theme().accent_foreground)
        .border_color(cx.theme().primary)
        .child(content)
})
```

## API

| 方法 | 说明 |
|---|---|
| `title(text)` | 设置语义标题和默认可访问名称 |
| `title_element(element)` | 设置自定义标题内容 |
| `description(text)` | 设置语义描述 |
| `description_element(element)` | 设置自定义描述内容 |
| `aria_label(text)` | 覆盖可访问名称 |
| `aria_description(text)` | 覆盖可访问描述 |
| `child(element)` | 添加可滚动 Body 内容 |
| `footer(element)` | 设置 Footer 内容 |
| `size(length)` | 覆盖对应方向轴上的宽度或高度 |
| `show_close_button(bool)` | 显示或隐藏关闭按钮 |
| `overlay(bool)` | 显示或隐藏背景绘制 |
| `overlay_closable(bool)` | 启用背景点击关闭 |
| `initial_focus(handle)` | 设置首选初始焦点 |
| `on_close(handler)` | 监听用户发起的关闭操作 |

[`Root`]: /docs/root
