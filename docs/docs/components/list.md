---
title: List
description: A virtualized list with sections, search, selection, disabled items, and incremental loading.
---

# List

`List` renders equal-height rows through a `ListDelegate`. `items_count` is a strict contract: every reported index must have a renderable item and an accessible label.

## Basic usage

```rust
use hearth_gpui::{
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

List::new(&state).aria_label("Files")
```

## Search and keyboard behavior

Enable the built-in single-line search field with `.searchable(true)` and implement `perform_search`. The latest search task wins; stale tasks are cancelled when the query changes.

```rust
let state = cx.new(|cx| ListState::new(delegate, window, cx).searchable(true));
```

Keyboard behavior:

- Up/Down selects the previous or next enabled item and wraps across sections.
- Home/End selects the first or last enabled item when the list surface has focus. In the search field they retain native text-cursor behavior.
- Enter confirms the active item.
- Escape clears the selection when `reset_on_cancel` is enabled.

Use `is_item_enabled` to exclude an item from pointer and keyboard interaction:

```rust
fn is_item_enabled(&self, ix: IndexPath, _: &App) -> bool {
    !self.items[ix.row].archived
}
```

## Sections

Return the number of sections from `sections_count`. Sections whose `items_count` is zero are omitted together with their header and footer. The measurement row may be in any non-empty section.

## Incremental loading

`load_more` returns the task representing the complete request. `List` allows only one request at a time and will not immediately retry when a completed request adds no rows.

```rust
fn has_more(&self, _: &App) -> bool {
    self.has_more
}

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

## Events and accessibility

`ListEvent::Select` is emitted when the active item changes. A pointer activation emits `Select` once, followed by `Confirm`. Selectable lists expose `ListBox`/`ListBoxOption`; read-only lists expose `List`/`ListItem`. `item_label` and `List::aria_label` provide their accessible names.
