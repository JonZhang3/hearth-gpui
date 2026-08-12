---
title: Tree
description: 支持选择、展开折叠和键盘导航的虚拟化层级视图。
---

# Tree

`Tree` 通过虚拟化扁平列表显示层级数据。应用负责节点内容组合，Tree 负责展开状态、选择、键盘导航、滚动和可访问性语义。

## 导入

```rust
use gpui_component::{
    IconName, Sizable as _, h_flex,
    list::ListItem,
    tree::{TreeItem, TreeState, tree},
};
```

## 基础用法

```rust
let tree_state = cx.new(|cx| {
    TreeState::new(cx).items(vec![
        TreeItem::new("src", "src")
            .expanded(true)
            .child(TreeItem::new("src/lib.rs", "lib.rs"))
            .child(TreeItem::new("src/main.rs", "main.rs")),
        TreeItem::new("Cargo.toml", "Cargo.toml"),
        TreeItem::new("README.md", "README.md"),
    ])
});

tree(&tree_state, |ix, entry, _selected, _window, cx| {
    let icon = if !entry.is_folder() {
        IconName::File
    } else if entry.is_expanded() {
        IconName::FolderOpen
    } else {
        IconName::Folder
    };

    ListItem::new(ix)
        .pl(entry.content_inset(cx))
        .child(
            h_flex()
                .gap(entry.content_gap(cx))
                .child(icon)
                .child(entry.item().label.clone()),
        )
})
.aria_label("项目文件")
```

Tree 会自动为 renderer 返回的 `ListItem` 设置统一尺寸、禁用状态、选中状态和右键次级选中状态。renderer 只需负责业务节点内容与可选操作。

## 尺寸与 Style Preset

```rust
tree(&tree_state, render_tree_item).small();
tree(&tree_state, render_tree_item); // Medium
tree(&tree_state, render_tree_item).large();
```

Tree 通过 `ListItem` 消费语义 Data Metrics，以确定行高、文字大小和 padding。`TreeEntry::content_inset` 使用语义 Control Metrics 计算层级缩进。因此 Vega、Nova 和 Maia 能在不判断 preset ID 的情况下呈现不同几何。

## 禁用节点

```rust
TreeItem::new("protected", "Protected")
    .disabled(true)
    .child(TreeItem::new("protected/secret.txt", "secret.txt"))
```

禁用节点保持可见，并向辅助技术暴露禁用状态。鼠标和键盘选择都会跳过禁用节点。

## Context Menu

```rust
tree(&tree_state, render_tree_item).context_menu(|_ix, entry, menu, _window, _cx| {
    menu.when(!entry.is_folder(), |menu| {
        menu.menu("Open", Box::new(OpenFile))
    })
    .menu("Rename", Box::new(Rename))
    .separator()
    .menu("Delete", Box::new(Delete))
})
```

## 编程式控制

```rust
tree_state.update(cx, |state, cx| {
    state.set_selected_index(Some(2), cx);
});

tree_state.update(cx, |state, cx| {
    state.set_selected_item(Some(&item), cx);
});

tree_state.update(cx, |state, cx| {
    state.reveal_item(&item.id, gpui::ScrollStrategy::Center, cx);
});

if let Some(entry) = tree_state.read(cx).selected_entry() {
    println!("Selected: {}", entry.item().label);
}
```

越界索引和禁用节点会清除选择。选择隐藏节点时会自动展开祖先节点。如果折叠操作隐藏了当前选中的后代，选择会移动到被折叠的祖先节点。

## 键盘行为

| 按键 | 行为 |
| --- | --- |
| `Up` | 选择上一个已启用的可见节点 |
| `Down` | 选择下一个已启用的可见节点 |
| `Left` | 折叠已展开节点，否则移动到父节点 |
| `Right` | 展开已折叠节点，否则移动到第一个已启用子节点 |
| `Enter` | 切换选中文件夹的展开状态 |

导航会在可见列表边界循环；不存在已启用节点时保持原状态。

## 可访问性

- 根元素使用 `Tree` role，并可通过 `aria_label` 设置名称。
- 可见节点使用稳定 ID 和 `TreeItem` role。
- 暴露层级、同级位置、同级数量、选中、展开和禁用状态。
- 键盘焦点保持在复合 Tree 上，选中节点作为 active descendant。

## API 参考

### Tree

| 方法 | 说明 |
| --- | --- |
| `aria_label(label)` | 设置 Tree 的可访问名称 |
| `with_size(size)` / `small()` / `large()` | 设置语义行尺寸 |
| `context_menu(builder)` | 构建右键菜单 |

### TreeState

| 方法 | 说明 |
| --- | --- |
| `items(items)` | 设置初始节点 |
| `set_items(items, cx)` | 替换节点并清除选择 |
| `selected_index()` | 获取选中的可见索引 |
| `selected_item()` / `selected_entry()` | 获取选中数据 |
| `set_selected_index(index, cx)` | 选择有效且已启用的可见索引 |
| `set_selected_item(item, cx)` | 选择节点并展开其祖先 |
| `reveal_item(id, strategy, cx)` | 展开祖先并滚动到节点 |
| `scroll_to_item(index, strategy)` | 滚动到可见索引 |
| `focus(window, cx)` | 聚焦复合 Tree |

### TreeEntry

| 方法 | 说明 |
| --- | --- |
| `item()` | 获取源 `TreeItem` |
| `depth()` | 获取从零开始的层级深度 |
| `content_inset(cx)` | 根据 Tree 尺寸解析语义左侧缩进 |
| `content_gap(cx)` | 根据 Tree 尺寸解析语义内容间距 |
| `is_folder()` | 节点是否包含子节点 |
| `is_expanded()` | 子节点是否可见 |
| `is_disabled()` | 是否禁用交互 |
