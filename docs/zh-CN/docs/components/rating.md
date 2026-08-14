---
title: Rating
description: 适配主题并支持无障碍操作的星级评分组件。
---

# Rating

Rating 是一个适配主题的星级评分组件，支持鼠标与键盘选择、自定义颜色、禁用与只读状态，以及全部语义尺寸。

## 导入

```rust
use hearth_gpui::rating::Rating;
```

## 用法

### 基础评分

```rust
Rating::new("my-rating")
    .aria_label("商品评分")
    .value(3)
    .max(5)
    .on_click(|value, _, _| {
        println!("Rating changed to: {}", value);
    })
```

### 受控评分

```rust
struct MyView {
    rating: usize,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Rating::new("rating")
            .value(self.rating)
            .max(5)
            .on_click(cx.listener(|view, value: &usize, _, cx| {
                view.rating = *value;
                cx.notify();
            }))
    }
}
```

### 不同尺寸

Rating 实现了 [Sizable] trait：

```rust
Rating::new("rating").xsmall().value(3).max(5)
Rating::new("rating").small().value(3).max(5)
Rating::new("rating").value(3).max(5)
Rating::new("rating").large().value(3).max(5)
```

### 自定义颜色

默认使用主题中的 `yellow` 颜色。你也可以通过 `color` 方法覆盖：

```rust
Rating::new("rating")
    .value(4)
    .max(5)
    .color(cx.theme().green)
```

### 禁用状态

```rust
Rating::new("rating")
    .value(2)
    .max(5)
    .disabled(true)
```

### 只读状态

使用 `read_only(true)` 展示不可交互的评分，同时保留正常的视觉强调。

```rust
Rating::new("rating")
    .aria_label("顾客平均评分")
    .value(4)
    .read_only(true)
```

### 自定义最大值

默认最大值为 5 星，也可以设置为任意数量：

```rust
Rating::new("rating")
    .value(7)
    .max(10)
```

### 点击行为

Rating 的鼠标行为如下：

- 点击其他星星会选择该星星对应的准确值。
- 点击当前最后一颗星，会将评分减少 1。
- Hover 会预览更高或更低的值，不会立即提交。

`on_click` 回调接收到的新值类型为 `&usize`。

```rust
Rating::new("rating")
    .value(3)
    .max(5)
    .on_click(|new_value, _, _| {
        println!("New rating: {}", new_value);
    })
```

### 键盘与可访问性

Rating 以水平 Slider 语义暴露，数值范围为 `0` 到 `max`。

- `Left` / `Down`：减少 1
- `Right` / `Up`：增加 1
- `Home`：设为 0
- `End`：设为最大值

使用 `aria_label(...)` 描述被评分的对象。禁用和只读状态会同步暴露给辅助技术。

## Theme 与 Style Preset

- 激活星星默认使用 Color Theme 的 `yellow`，也可以通过 `color(...)` 覆盖。
- 未激活星星使用 `muted_foreground`。
- Item padding、间距、焦点环和圆角均消费语义 Style Preset metrics。
- 自定义 `Styled` refinement 对 Rating 外层元素保持最高优先级。

## API 参考

- [Rating]

### 方法

- `new(id: impl Into<ElementId>)`：创建新的 Rating 组件。
- `with_size(size: impl Into<Size>)`：设置星星尺寸，支持 [Sizable]。
- `value(value: usize)`：设置当前评分值，范围 `0..=max`。
- `max(max: usize)`：设置最大星数，默认值为 5。
- `color(color: impl Into<Hsla>)`：设置激活颜色，默认使用主题黄色。
- `aria_label(label: impl Into<SharedString>)`：设置可访问名称。
- `disabled(disabled: bool)`：禁用交互，支持 [Disableable]。
- `read_only(read_only: bool)`：以正常视觉强调展示不可交互的评分。
- `on_click(handler: Fn(&usize, &mut Window, &mut App))`：设置点击处理函数。

## 示例

### 只读展示

```rust
Rating::new("rating")
    .value(4)
    .max(5)
    .read_only(true)
```

### 带状态的交互评分

```rust
struct ProductView {
    user_rating: usize,
}

impl Render for ProductView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                Rating::new("product-rating")
                    .value(self.user_rating)
                    .max(5)
                    .on_click(cx.listener(|view, value: &usize, _, cx| {
                        view.user_rating = *value;
                        cx.notify();
                    }))
            )
            .child(format!("Your rating: {}/5", self.user_rating))
    }
}
```

### 自定义颜色的大尺寸评分

```rust
Rating::new("rating")
    .large()
    .value(5)
    .max(5)
    .color(cx.theme().orange)
```

[Rating]: https://docs.rs/hearth-gpui/latest/hearth_gpui/rating/struct.Rating.html
[Sizable]: https://docs.rs/hearth-gpui/latest/hearth_gpui/trait.Sizable.html
[Disableable]: https://docs.rs/hearth-gpui/latest/hearth_gpui/trait.Disableable.html
