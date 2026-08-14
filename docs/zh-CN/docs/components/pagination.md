---
title: Pagination
description: 提供页码、上一页和下一页导航的分页组件。
---

# Pagination

[Pagination] 组件提供居中的页码、上一页和下一页导航。视觉层级与 shadcn Vega Pagination 对齐，同时保留可打开隐藏页码菜单的交互式省略号，作为 GPUI 桌面增强。

## 导入

```rust
use hearth_gpui::pagination::Pagination;
```

## 用法

### 基础分页

```rust
Pagination::new("my-pagination")
    .aria_label("搜索结果分页")
    .current_page(5)
    .total_pages(10)
    .on_click(|page, _, cx| {
        println!("Navigated to page: {}", page);
    })
```

### 自定义可见页数

默认最多显示 5 个项目，其中包含首尾页及省略号。可以通过 `visible_pages()` 调整最大数量；小于 5 的值会归一化为 5。

```rust
Pagination::new("my-pagination")
    .current_page(1)
    .total_pages(50)
    .visible_pages(10)
    .on_click(|page, _, cx| {
        // 处理页码切换
    })
```

### 紧凑模式

紧凑模式只显示上一页和下一页按钮，不显示具体页码：

```rust
Pagination::new("my-pagination")
    .compact()
    .current_page(3)
    .total_pages(10)
    .on_click(|page, _, cx| {
        // 处理页码切换
    })
```

### 不同尺寸

```rust
use hearth_gpui::{Sizable as _, Size};

Pagination::new("my-pagination")
    .xsmall()
    .current_page(1)
    .total_pages(10)

Pagination::new("my-pagination")
    .small()
    .current_page(1)
    .total_pages(10)

Pagination::new("my-pagination")
    .current_page(1)
    .total_pages(10)

Pagination::new("my-pagination")
    .large()
    .current_page(1)
    .total_pages(10)
```

### 禁用状态

```rust
Pagination::new("my-pagination")
    .current_page(4)
    .total_pages(10)
    .disabled(true)
    .on_click(|_, _, _| {})
```

### 可访问性与键盘行为

- 根节点暴露具名 Navigation 区域；同一页面存在多个分页时应使用 `aria_label()` 区分。
- 当前页暴露 `aria-current="page"`。
- 上一页和下一页提供本地化操作名称，紧凑图标模式同样有效。
- 所有可操作页码均可通过 Tab 到达，并通过共享 Button 契约响应 Enter 或 Space。
- 省略号会用原生 Popup Menu 展示隐藏页码，这是相对 shadcn 装饰性省略号的桌面增强。

Pagination 本身没有独立动效；hover、active 和 focus-visible 状态复用已对齐的 Button 行为。

### 处理页码变化

`on_click` 会在用户点击页码、上一页或下一页时返回新的页码：

```rust
Pagination::new("my-pagination")
    .current_page(current_page)
    .total_pages(total_pages)
    .on_click(|page, _, cx| {
        // 用新的页码更新状态
        // 页码从 1 开始
    })
```

## API 参考

### 尺寸

实现了 [Sizable] trait：

- `xsmall()`：超小尺寸
- `small()`：小尺寸
- `medium()`：中尺寸，默认值
- `large()`：大尺寸
- `with_size(size)`：设置自定义尺寸

### 方法

- `current_page(page: usize)`：设置当前页，页码从 1 开始，超出范围时会自动限制到 `1..=total_pages`
- `total_pages(pages: usize)`：设置总页数
- `visible_pages(max: usize)`：设置最多显示多少个页码和省略号项目，默认值和最小值均为 `5`
- `aria_label(label)`：设置分页 Navigation 区域的可访问名称
- `compact()`：启用紧凑模式，仅显示前后翻页按钮
- `disabled(bool)`：设置禁用状态
- `on_click(handler)`：设置页码切换回调

## 示例

### 结合状态管理

```rust
let mut current_page = 1;
let total_pages = 20;

Pagination::new("pagination")
    .current_page(current_page)
    .total_pages(total_pages)
    .on_click({
        let entity = entity.clone();
        move |page, _, cx| {
            entity.update(cx, |this, cx| {
                this.current_page = *page;
                cx.notify();
            });
        }
    })
```

### 大数据集分页

```rust
Pagination::new("large-pagination")
    .current_page(25)
    .total_pages(100)
    .visible_pages(10)
    .on_click(|page, _, cx| {
        // 加载新页的数据
    })
```

[Pagination]: https://docs.rs/hearth-gpui/latest/hearth_gpui/pagination/struct.Pagination.html
[Sizable]: https://docs.rs/hearth-gpui/latest/hearth_gpui/trait.Sizable.html
