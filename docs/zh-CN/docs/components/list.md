---
title: List
description: 支持分组、搜索、选择、禁用项和增量加载的虚拟列表。
---

# List

`List` 通过 `ListDelegate` 渲染等高行。`items_count` 是严格契约：每个声明的索引都必须能渲染，并提供无障碍标签。

## 基础用法

```rust
use gpui_component::{
    IndexPath,
    list::{List, ListDelegate, ListItem, ListState},
};

struct MyListDelegate {
    items: Vec<String>,
    selected: Option<IndexPath>,
}

impl ListDelegate for MyListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        self.items.len()
    }

    fn item_label(&self, ix: IndexPath, _: &App) -> SharedString {
        self.items[ix.row].clone().into()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Self::Item {
        ListItem::new(ix).child(self.items[ix.row].clone())
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.selected = ix;
    }
}

let state = cx.new(|cx| {
    ListState::new(delegate, window, cx)
        .initial_selected_index(Some(IndexPath::new(0)))
});

List::new(&state).aria_label("文件")
```

## 搜索与键盘交互

使用 `.searchable(true)` 启用内置单行搜索框，并实现 `perform_search`。查询变化时旧任务会被取消，只有最新搜索结果会更新选择状态。

键盘行为：

- Up/Down 在可用项之间循环，并自动跨越空分组和禁用项。
- 列表区域获得焦点时，Home/End 选择第一个或最后一个可用项；搜索框获得焦点时保留原生文本光标行为。
- Enter 确认当前项。
- 启用 `reset_on_cancel` 时，Escape 清除当前选择。

通过 `is_item_enabled` 禁止鼠标和键盘操作指定项：

```rust
fn is_item_enabled(&self, ix: IndexPath, _: &App) -> bool {
    !self.items[ix.row].archived
}
```

## 分组

通过 `sections_count` 返回分组数量。`items_count` 为零的分组不会渲染 header 和 footer。用于虚拟化测量的行可以位于任意非空分组。

## 增量加载

`load_more` 必须返回覆盖完整请求生命周期的 `Task<()>`。`List` 同时只执行一个加载任务；如果任务完成后没有新增行，不会立即重复请求。

```rust
fn load_more(
    &mut self,
    window: &mut Window,
    cx: &mut Context<ListState<Self>>,
) -> Task<()> {
    cx.spawn_in(window, async move |state, window| {
        let rows = fetch_more().await;
        _ = state.update_in(window, |state, _, cx| {
            state.delegate_mut().items.extend(rows);
            cx.notify();
        });
    })
}
```

## 事件与无障碍

当前项变化时触发 `ListEvent::Select`。鼠标激活会依次触发一次 `Select` 和一次 `Confirm`。可选择列表使用 `ListBox`/`ListBoxOption`，只读列表使用 `List`/`ListItem`。`item_label` 与 `List::aria_label` 分别提供列表项和列表的无障碍名称。
