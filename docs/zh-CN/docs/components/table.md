---
title: Table
description: 一个用于直接渲染表格数据的基础表格组件。
---

# Table

Table 是一个简单、无状态、可组合的表格组件，用于渲染表格型数据。与 [DataTable] 不同，它不包含虚拟滚动、排序或列管理能力，更适合直接用声明式 API 展示较小且静态的数据。

默认视觉遵循 shadcn Vega：透明 Header、中等字重表头、带 muted hover 表面的分隔行、`muted/50` Footer、不换行单元格，以及列宽超过容器时的横向滚动。Maia 和 Nova 通过语义 Data Metrics 调整密度，不按 preset 名称分支。

## 导入

```rust
use hearth_gpui::table::{
    Table, TableHeader, TableBody, TableFooter,
    TableRow, TableHead, TableCell, TableCaption,
};
```

## 用法

### 基础表格

```rust
Table::new()
    .child(TableHeader::new().child(
        TableRow::new()
            .child(TableHead::new().child("Name"))
            .child(TableHead::new().child("Email"))
            .child(TableHead::new().text_right().child("Amount"))
    ))
    .child(TableBody::new()
        .child(TableRow::new()
            .child(TableCell::new().child("John"))
            .child(TableCell::new().child("john@example.com"))
            .child(TableCell::new().text_right().child("$100.00")))
        .child(TableRow::new()
            .child(TableCell::new().child("Jane"))
            .child(TableCell::new().child("jane@example.com"))
            .child(TableCell::new().text_right().child("$200.00")))
    )
    .child(TableCaption::new().child("A list of recent invoices."))
```

### 带 Footer

```rust
Table::new()
    .child(TableHeader::new().child(
        TableRow::new()
            .child(TableHead::new().child("Invoice"))
            .child(TableHead::new().child("Status"))
            .child(TableHead::new().text_right().child("Amount"))
    ))
    .child(TableBody::new()
        .child(TableRow::new()
            .child(TableCell::new().child("INV001"))
            .child(TableCell::new().child("Paid"))
            .child(TableCell::new().text_right().child("$250.00")))
    )
    .child(TableFooter::new().child(
        TableRow::new()
            .child(TableCell::new().child("Total"))
            .child(TableCell::new().child(""))
            .child(TableCell::new().text_right().child("$250.00"))
    ))
```

### 列宽

可以在 `TableHead` 和 `TableCell` 上使用 `.w()` 设置固定列宽：

```rust
TableRow::new()
    .child(TableHead::new().w(px(80.)).child("ID"))
    .child(TableHead::new().child("Name"))
    .child(TableHead::new().w(px(120.)).child("Date"))
```

### 文本对齐

```rust
TableHead::new().text_center().child("Status")

TableCell::new().text_right().child("$1,000.00")
```

### 选中行

通过 `Selectable` 同时设置选中视觉和 `aria-selected`：

```rust
use hearth_gpui::Selectable as _;

TableRow::new()
    .selected(true)
    .child(TableCell::new().child("Selected invoice"))
```

Table 本身无状态，选择状态及鼠标、键盘交互由调用方管理。需要组件管理交互式选择时应使用 [DataTable]。

### 横向溢出

Table 默认保持单元格不换行；列的最小宽度超过可用宽度时可横向滚动。组件不会自动截断单元格内容，需要时可在单独的 Cell 上设置明确宽度或溢出样式。

### 样式覆盖

所有表格子组件都实现了 `Styled`，可以直接自定义样式：

```rust
Table::new()
    .border_1()
    .rounded(cx.theme().style.radii.md)
    .child(TableHeader::new().border_0())
```

### 自定义样式

```rust
TableRow::new()
    .bg(cx.theme().table_even)
    .child(/* ... */)

TableCell::new()
    .px_4()
    .child("Custom padded content")
```

## 子组件

| 组件 | 说明 |
| --- | --- |
| `Table` | 表格根节点及横向溢出容器 |
| `TableHeader` | 透明表头区域，包含行分隔线 |
| `TableBody` | 表体区域 |
| `TableFooter` | 表尾区域 |
| `TableRow` | 带语义分隔线、hover 和选中状态的数据行 |
| `TableHead` | 表头单元格 |
| `TableCell` | 数据单元格 |
| `TableCaption` | 表格下方说明文字 |

## API 摘要

### Table

- `new()` - 创建新表格
- 实现了 `Styled`、`ParentElement`、`Sizable`、`RenderOnce`

### TableHead / TableCell

- `new()` - 创建单元格
- `w(width)` - 设置固定宽度
- `text_center()` - 居中对齐
- `text_right()` - 右对齐

### TableHeader / TableBody / TableFooter / TableRow / TableCaption

- `new()` - 创建实例
- 实现了 `Styled`、`ParentElement`、`RenderOnce`

`TableRow` 还实现了 `Selectable`，用于声明式选中状态和无障碍元数据。

## Table 和 DataTable 的区别

| 特性 | Table | DataTable |
| --- | --- | --- |
| 虚拟滚动 | No | Yes |
| 列排序 | No | Yes |
| 列宽调整 | No | Yes |
| 列拖动 | No | Yes |
| 单元格选择 | No | Yes |
| 行选择 | No | Yes |
| 无限加载 | No | Yes |
| 键盘导航 | No | Yes |
| 状态管理 | Stateless | TableState |
| 适用场景 | 小型静态数据 | 大型交互式数据集 |

[DataTable]: ./data-table.md
