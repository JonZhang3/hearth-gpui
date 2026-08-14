---
title: DataTable
description: 支持虚拟滚动、排序、筛选和列管理的高性能数据表格。
---

# DataTable

DataTable 是一个面向大数据集场景的高性能表格组件。它支持虚拟滚动、列配置、排序、筛选、行/列/单元格选择、自定义单元格渲染以及右键菜单，适合展示成千上万条数据同时保持流畅体验。

## 核心特性

- 多种选择模式：支持行、列和单元格选择
- 虚拟滚动：适合大体量数据集
- 列管理：支持固定列、调整列宽、列移动
- 排序：内建排序能力
- 键盘导航：完整键盘交互支持
- 自定义单元格：每个单元格都可以渲染任意 GPUI 内容
- 右键菜单：支持行和单元格上下文菜单
- 无限加载：滚动到底部时按需加载更多数据

## 导入

```rust
use hearth_gpui::table::{
    DataTable, TableState, TableDelegate,
    Column, ColumnSort, ColumnFixed,
    TableEvent
};
```

## 用法

### 基础表格

要创建一个 DataTable，需要实现 `TableDelegate`，提供列定义和数据渲染逻辑，再通过 `TableState` 管理状态。

```rust
use std::ops::Range;
use gpui::{App, Context, Window, IntoElement};
use hearth_gpui::table::{DataTable, TableDelegate, Column, ColumnSort};

struct MyData {
    id: usize,
    name: String,
    age: u32,
    email: String,
}

struct MyTableDelegate {
    data: Vec<MyData>,
    columns: Vec<Column>,
}

impl MyTableDelegate {
    fn new() -> Self {
        Self {
            data: vec![
                MyData { id: 1, name: "John".to_string(), age: 30, email: "john@example.com".to_string() },
                MyData { id: 2, name: "Jane".to_string(), age: 25, email: "jane@example.com".to_string() },
            ],
            columns: vec![
                Column::new("id", "ID").w(60.),
                Column::new("name", "Name").w(150.).sortable(),
                Column::new("age", "Age").w(80.).sortable(),
                Column::new("email", "Email").w(200.),
            ],
        }
    }
}

impl TableDelegate for MyTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.data.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, _: &mut Context<TableState<Self>>) -> impl IntoElement {
        let row = &self.data[row_ix];
        let col = &self.columns[col_ix];

        match col.key.as_ref() {
            "id" => row.id.to_string(),
            "name" => row.name.clone(),
            "age" => row.age.to_string(),
            "email" => row.email.clone(),
            _ => "".to_string(),
        }
    }
}

let delegate = MyTableDelegate::new();
let state = cx.new(|cx| TableState::new(delegate, window, cx));
```

## 列配置

列支持丰富的配置能力：

```rust
Column::new("id", "ID")

Column::new("name", "Name")
    .sortable()
    .w(150.)

Column::new("price", "Price")
    .text_right()
    .sortable()

Column::new("actions", "Actions")
    .fixed(ColumnFixed::Left)
    .resizable(false)
    .movable(false)
```

常用能力包括：

- `sortable()` 开启排序
- `width()` 设置宽度
- `fixed(ColumnFixed::Left)` 固定到左侧
- `resizable(false)` 禁止调整列宽
- `movable(false)` 禁止拖动列顺序
- `text_right()` / `text_center()` 设置对齐

## 虚拟滚动

DataTable 默认面向大数据场景设计。即使数据量达到数千或上万行，也只会渲染当前可见区域附近的内容。

```rust
impl TableDelegate for LargeDataDelegate {
    fn rows_count(&self, _: &App) -> usize {
        self.data.len()
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, _: &mut Context<TableState<Self>>) -> impl IntoElement {
        let row = &self.data[row_ix];
        format_cell_data(row, col_ix)
    }

    fn visible_rows_changed(&mut self, visible_range: Range<usize>, _: &mut Window, _: &mut Context<TableState<Self>>) {
        // 可在这里根据可见区做数据预取或缓存更新
    }
}
```

## 排序

排序逻辑需要由你的 `TableDelegate` 实现：

```rust
impl TableDelegate for MyTableDelegate {
    fn perform_sort(&mut self, col_ix: usize, sort: ColumnSort, _: &mut Window, _: &mut Context<TableState<Self>>) {
        let col = &self.columns[col_ix];

        match col.key.as_ref() {
            "name" => match sort {
                ColumnSort::Ascending => self.data.sort_by(|a, b| a.name.cmp(&b.name)),
                ColumnSort::Descending => self.data.sort_by(|a, b| b.name.cmp(&a.name)),
                ColumnSort::Default => self.data.sort_by(|a, b| a.id.cmp(&b.id)),
            },
            "age" => match sort {
                ColumnSort::Ascending => self.data.sort_by(|a, b| a.age.cmp(&b.age)),
                ColumnSort::Descending => self.data.sort_by(|a, b| b.age.cmp(&a.age)),
                ColumnSort::Default => self.data.sort_by(|a, b| a.id.cmp(&b.id)),
            },
            _ => {}
        }
    }
}
```

## 右键菜单

你可以为行或单元格提供右键菜单：

```rust
impl TableDelegate for MyTableDelegate {
    fn context_menu(&mut self, row_ix: usize, menu: PopupMenu, _: &mut Window, _: &mut Context<TableState<Self>>) -> PopupMenu {
        let row = &self.data[row_ix];
        menu.menu(format!("Edit {}", row.name), Box::new(EditRowAction(row_ix)))
            .menu("Delete", Box::new(DeleteRowAction(row_ix)))
            .separator()
            .menu("Duplicate", Box::new(DuplicateRowAction(row_ix)))
    }
}
```

## 自定义单元格

DataTable 的一个重要能力是每个单元格都可以渲染复杂内容：

```rust
impl TableDelegate for MyTableDelegate {
    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let row = &self.data[row_ix];
        let col = &self.columns[col_ix];

        match col.key.as_ref() {
            "status" => {
                let (color, text) = match row.status {
                    Status::Active => (cx.theme().green, "Active"),
                    Status::Inactive => (cx.theme().red, "Inactive"),
                    Status::Pending => (cx.theme().yellow, "Pending"),
                };

                div()
                    .px_2()
                    .py_1()
                    .rounded(px(4.))
                    .bg(color.opacity(0.1))
                    .text_color(color)
                    .child(text)
            }
            _ => row.get_field_value(col.key.as_ref()).into_any_element(),
        }
    }
}
```

## 选择模式

DataTable 支持三种主要选择模式：

```rust
let state = cx.new(|cx| {
    TableState::new(delegate, window, cx)
        .row_selectable(true)
        .col_selectable(false)
        .cell_selectable(false)
});

let state = cx.new(|cx| {
    TableState::new(delegate, window, cx)
        .row_selectable(false)
        .col_selectable(true)
        .cell_selectable(false)
});

let state = cx.new(|cx| {
    TableState::new(delegate, window, cx)
        .row_selectable(true)
        .col_selectable(false)
        .cell_selectable(true)
});
```

### 单元格选择

启用 `cell_selectable(true)` 后：

- 点击单元格可以直接选中
- 可以用方向键在单元格之间移动
- 可以监听双击和右键事件
- 可以程序化设置当前选中单元格

```rust
if let Some((row_ix, col_ix)) = state.read(cx).selected_cell() {
    println!("Current cell: ({}, {})", row_ix, col_ix);
}

state.update(cx, |state, cx| {
    state.set_selected_cell(5, 3, cx);
});
```

## 列宽调整与列移动

```rust
let state = cx.new(|cx| {
    TableState::new(delegate, window, cx)
        .col_resizable(true)
        .col_movable(true)
        .sortable(true)
        .col_selectable(true)
        .row_selectable(true)
});
```

可以通过事件监听这些变化：

```rust
cx.subscribe_in(&state, window, |view, table, event, _, cx| {
    match event {
        TableEvent::ColumnWidthsChanged(widths) => {
            save_column_widths(widths);
        }
        TableEvent::MoveColumn(from_ix, to_ix) => {
            save_column_order(from_ix, to_ix);
        }
        _ => {}
    }
}).detach();
```

## 无限加载

如果你的数据来自分页接口或流式加载，可以在 delegate 中实现按需加载：

```rust
impl TableDelegate for MyTableDelegate {
    fn has_more(&self, _: &App) -> bool {
        self.has_more_data
    }

    fn load_more_threshold(&self) -> usize {
        50
    }

    fn loading(&self, _: &App) -> bool {
        self.loading
    }
}
```

## 表格样式

`DataTable` 消费 Table 共用的 `DataMetrics`，因此 Vega、Nova 和 Maia 可以独立控制行高与单元格内边距，而不会改变数据行为。组件同时实现了 `Sizable` 与 `Styled`：预设尺寸用于标准密度，自定义像素尺寸用于统一行高，GPUI 样式 refinement 用于覆盖容器样式。

默认表面遵循 shadcn Data Table 的组合方式：透明表头、语义行分割线、`muted/50` hover、`muted` 行选中状态，以及可选的圆角外边框。斑马纹、虚拟滚动、固定列、列宽调整和单元格选择继续作为 GPUI 原生扩展保留。

```rust
use gpui::px;
use hearth_gpui::Sizable as _;
use gpui::Styled as _;

DataTable::new(&state)
    .aria_label("Stocks")
    .with_size(px(48.))
    .stripe(true)
    .bordered(true)
    .scrollbar_visible(true, true)
    .max_h(px(640.))
```

获得焦点的 DataTable 会以可访问 Grid 暴露。可见行、列头和单元格提供稳定的行列索引，根节点同时声明完整虚拟数据集的总行数与总列数。

## 键盘快捷键

### 行选择模式

- `↑/↓` 在行之间移动
- `←/→` 在列之间移动
- `Home` / `End` 跳到首尾
- `PageUp/PageDown` 按页移动
- `Escape` 清除选中

### 单元格选择模式

- `↑/↓` 在当前列中上下移动
- `←/→` 在当前行中左右移动
- `Tab` 移动到下一个单元格
- `Shift+Tab` 移动到上一个单元格
- `Escape` 清除选中

## API 参考

### 核心类型

- [DataTable] - 表格组件本体
- [TableState] - 表格状态管理
- [TableDelegate] - 数据源和渲染协议
- [Column] - 列定义
- [TableEvent] - 表格事件

### 常见方法

#### TableState

- `new(delegate, window, cx)`
- `cell_selectable(bool)`
- `row_selectable(bool)`
- `col_selectable(bool)`
- `selected_cell()`
- `set_selected_cell(row_ix, col_ix, cx)`
- `clear_selection(cx)`
- `scroll_to_row(row_ix, cx)`
- `scroll_to_col(col_ix, cx)`

#### Column

- `new(key, name)`
- `width(pixels)`
- `sortable()`
- `ascending()`
- `descending()`
- `text_right()`
- `text_center()`
- `fixed(ColumnFixed)`
- `resizable(bool)`
- `movable(bool)`
- `selectable(bool)`

[DataTable]: https://docs.rs/hearth-gpui/latest/hearth_gpui/table/struct.DataTable.html
[TableState]: https://docs.rs/hearth-gpui/latest/hearth_gpui/table/struct.TableState.html
[TableDelegate]: https://docs.rs/hearth-gpui/latest/hearth_gpui/table/trait.TableDelegate.html
[Column]: https://docs.rs/hearth-gpui/latest/hearth_gpui/table/struct.Column.html
[TableEvent]: https://docs.rs/hearth-gpui/latest/hearth_gpui/table/enum.TableEvent.html
[ColumnSort]: https://docs.rs/hearth-gpui/latest/hearth_gpui/table/enum.ColumnSort.html
[ColumnFixed]: https://docs.rs/hearth-gpui/latest/hearth_gpui/table/enum.ColumnFixed.html
