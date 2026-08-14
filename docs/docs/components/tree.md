---
title: Tree
description: A virtualized hierarchical view with selection, disclosure, and keyboard navigation.
---

# Tree

`Tree` displays hierarchical data through a virtualized flat list. Applications keep control of row composition while Tree owns expansion, selection, keyboard navigation, scrolling, and accessibility semantics.

## Import

```rust
use gpui_component::{
    IconName, Sizable as _, h_flex,
    list::ListItem,
    tree::{TreeItem, TreeState, tree},
};
```

## Basic usage

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
.aria_label("Project files")
```

The returned `ListItem` automatically receives the Tree size, disabled state, selected state, and secondary context-menu selection state. The renderer should only describe application-specific row content and optional actions.

## Size and Style Presets

```rust
tree(&tree_state, render_tree_item).small();
tree(&tree_state, render_tree_item); // Medium
tree(&tree_state, render_tree_item).large();
```

Tree delegates row height, text size, and padding to `ListItem`, which consumes semantic Data Metrics. `TreeEntry::content_inset` derives hierarchy indentation from semantic Control Metrics. Vega, Nova, and Maia therefore change geometry without preset-ID branches.

## Disabled nodes

```rust
TreeItem::new("protected", "Protected")
    .disabled(true)
    .child(TreeItem::new("protected/secret.txt", "secret.txt"))
```

Disabled nodes remain visible and are exposed as disabled to assistive technologies. Pointer and keyboard selection skip them.

## Context menu

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

## Programmatic control

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

Invalid indexes and disabled nodes clear selection. Selecting a hidden item expands its ancestors. If a selected descendant becomes hidden by collapsing an ancestor, selection moves to that ancestor.

## Keyboard behavior

| Key | Behavior |
| --- | --- |
| `Up` | Select the previous enabled visible node |
| `Down` | Select the next enabled visible node |
| `Left` | Collapse an expanded node, otherwise move to its parent |
| `Right` | Expand a collapsed node, otherwise move to its first enabled child |
| `Enter` | Toggle the selected folder |

Navigation wraps at the visible-list boundary and remains unchanged when no enabled node exists.

## Accessibility

- The root is exposed as `Tree` and accepts a custom accessible name through `aria_label`.
- Visible nodes are exposed as `TreeItem` with stable IDs.
- Level, sibling position, sibling count, selected, expanded, and disabled states are exposed.
- Keyboard focus remains on the composite Tree while the selected node is its active descendant.

## API reference

### Tree

| Method | Description |
| --- | --- |
| `aria_label(label)` | Set the accessible Tree name |
| `with_size(size)` / `small()` / `large()` | Set semantic row size |
| `context_menu(builder)` | Build the right-click menu |

### TreeState

| Method | Description |
| --- | --- |
| `items(items)` | Set initial items |
| `set_items(items, cx)` | Replace items and clear selection |
| `selected_index()` | Get the selected visible index |
| `selected_item()` / `selected_entry()` | Get selected data |
| `set_selected_index(index, cx)` | Select a valid enabled visible index |
| `set_selected_item(item, cx)` | Select an item and reveal its ancestors |
| `reveal_item(id, strategy, cx)` | Expand ancestors and scroll to an item |
| `scroll_to_item(index, strategy)` | Scroll to a visible index |
| `focus(window, cx)` | Focus the composite Tree |

### TreeEntry

| Method | Description |
| --- | --- |
| `item()` | Get the source `TreeItem` |
| `depth()` | Get zero-based hierarchy depth |
| `content_inset(cx)` | Resolve semantic left inset from the Tree size |
| `content_gap(cx)` | Resolve semantic row-content gap from the Tree size |
| `is_folder()` | Whether the item has children |
| `is_expanded()` | Whether children are visible |
| `is_disabled()` | Whether interaction is disabled |
