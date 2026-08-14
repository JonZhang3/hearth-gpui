use std::{cell::RefCell, ops::Range, rc::Rc, sync::Arc};

use gpui::{
    App, Context, ElementId, Entity, EventEmitter, FocusHandle, InteractiveElement as _,
    IntoElement, KeyBinding, ListSizingBehavior, MouseButton, ParentElement, Pixels, Render,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    UniformListScrollHandle, Window, div, prelude::FluentBuilder as _, px, uniform_list,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Selectable as _, Sizable, Size, StyledExt,
    accessibility::accessibility_state,
    actions::{Confirm, SelectDown, SelectLeft, SelectRight, SelectUp},
    list::ListItem,
    menu::{ContextMenuExt as _, PopupMenu},
    scroll::ScrollableElement,
    styled::FocusableExt as _,
};

const CONTEXT: &str = "Tree";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
    ]);
}

/// Create a [`Tree`].
///
/// # Arguments
///
/// * `state` - The shared state managing the tree items.
/// * `render_item` - A closure to render each tree item.
///
/// ```ignore
/// let state = cx.new(|cx| {
///     TreeState::new(cx).items(vec![
///         TreeItem::new("src", "src")
///             .child(TreeItem::new("src/lib.rs", "lib.rs")),
///         TreeItem::new("Cargo.toml", "Cargo.toml"),
///         TreeItem::new("README.md", "README.md"),
///     ])
/// });
///
/// tree(&state, |ix, entry, selected, window, cx| {
///     let item = entry.item();
///     ListItem::new(ix)
///         .pl(entry.content_inset(cx))
///         .child(item.label.clone())
/// })
/// ```
pub fn tree<R>(state: &Entity<TreeState>, render_item: R) -> Tree
where
    R: Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem + 'static,
{
    Tree::new(state, render_item)
}

struct TreeItemState {
    expanded: bool,
    disabled: bool,
}

/// A tree item with a label, children, and an expanded state.
#[derive(Clone)]
pub struct TreeItem {
    pub id: SharedString,
    pub label: SharedString,
    pub children: Vec<TreeItem>,
    state: Rc<RefCell<TreeItemState>>,
}

/// A flat representation of a tree item with its depth.
#[derive(Clone)]
pub struct TreeEntry {
    item: TreeItem,
    depth: usize,
    parent_id: Option<SharedString>,
    position_in_set: usize,
    size_of_set: usize,
    size: Size,
}

impl TreeEntry {
    /// Get the source tree item.
    #[inline]
    pub fn item(&self) -> &TreeItem {
        &self.item
    }

    /// The depth of this item in the tree.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the semantic left inset for this hierarchy level.
    ///
    /// The inset follows the active Style Preset's control geometry without
    /// branching on a preset identifier.
    pub fn content_inset(&self, cx: &App) -> Pixels {
        let metrics = cx.theme().style.controls.for_size(self.size);
        metrics.padding_x + (metrics.icon_size + metrics.gap) * self.depth as f32
    }

    /// Returns the semantic gap used between row content elements.
    pub fn content_gap(&self, cx: &App) -> Pixels {
        cx.theme().style.controls.for_size(self.size).gap
    }

    #[inline]
    fn is_root(&self) -> bool {
        self.depth == 0
    }

    /// Whether this item is a folder (has children).
    #[inline]
    pub fn is_folder(&self) -> bool {
        self.item.is_folder()
    }

    /// Return true if the item is expanded.
    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.item.is_expanded()
    }

    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.item.is_disabled()
    }
}

/// Event emitted by [`TreeState`] when user-visible state changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeEvent {
    /// A tree node was expanded.
    Expanded(SharedString),
    /// A tree node was collapsed.
    Collapsed(SharedString),
}

impl TreeItem {
    /// Create a new tree item with the given label.
    ///
    /// - The `id` for you to uniquely identify this item, then later you can use it for selection or other purposes.
    /// - The `label` is the text to display for this item.
    ///
    /// For example, the `id` is the full file path, and the `label` is the file name.
    ///
    /// ```ignore
    /// TreeItem::new("src/ui/button.rs", "button.rs")
    /// ```
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            state: Rc::new(RefCell::new(TreeItemState {
                expanded: false,
                disabled: false,
            })),
        }
    }

    /// Add a child item to this tree item.
    pub fn child(mut self, child: TreeItem) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple child items to this tree item.
    pub fn children(mut self, children: impl IntoIterator<Item = TreeItem>) -> Self {
        self.children.extend(children);
        self
    }

    /// Set expanded state for this tree item.
    pub fn expanded(self, expanded: bool) -> Self {
        self.state.borrow_mut().expanded = expanded;
        self
    }

    /// Set disabled state for this tree item.
    pub fn disabled(self, disabled: bool) -> Self {
        self.state.borrow_mut().disabled = disabled;
        self
    }

    /// Whether this item is a folder (has children).
    #[inline]
    pub fn is_folder(&self) -> bool {
        self.children.len() > 0
    }

    /// Return true if the item is disabled.
    pub fn is_disabled(&self) -> bool {
        self.state.borrow().disabled
    }

    /// Return true if the item is expanded.
    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.state.borrow().expanded
    }

    fn find_ancestors(&self, target_id: &SharedString) -> Option<Vec<TreeItem>> {
        if self.id == *target_id {
            return Some(vec![]);
        }

        for child in &self.children {
            if let Some(mut path) = child.find_ancestors(target_id) {
                path.push(self.clone());
                return Some(path);
            }
        }

        None
    }
}

/// State for managing tree items.
pub struct TreeState {
    focus_handle: FocusHandle,
    entries: Vec<TreeEntry>,
    scroll_handle: UniformListScrollHandle,
    selected_ix: Option<usize>,
    selected_id: Option<SharedString>,
    right_clicked_ix: Option<usize>,
    render_item: Rc<dyn Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem>,
    context_menu_builder: Option<
        Rc<dyn Fn(usize, &TreeEntry, PopupMenu, &mut Window, &mut Context<TreeState>) -> PopupMenu>,
    >,
    owner_id: ElementId,
    size: Size,
}

impl EventEmitter<TreeEvent> for TreeState {}

impl TreeState {
    /// Create a new empty tree state.
    pub fn new(cx: &mut App) -> Self {
        Self {
            selected_ix: None,
            selected_id: None,
            right_clicked_ix: None,
            focus_handle: cx.focus_handle(),
            scroll_handle: UniformListScrollHandle::default(),
            entries: Vec::new(),
            render_item: Rc::new(|_, _, _, _, _| ListItem::new(0)),
            context_menu_builder: None,
            owner_id: "tree".into(),
            size: Size::default(),
        }
    }

    /// Set the tree items.
    pub fn items(mut self, items: impl Into<Vec<TreeItem>>) -> Self {
        let items = items.into();
        self.entries.clear();
        self.add_entries(&items, 0, None);
        self
    }

    /// Set the tree items.
    pub fn set_items(&mut self, items: impl Into<Vec<TreeItem>>, cx: &mut Context<Self>) {
        let items = items.into();
        self.entries.clear();
        self.add_entries(&items, 0, None);
        self.selected_ix = None;
        self.selected_id = None;
        self.right_clicked_ix = None;
        cx.notify();
    }

    /// Get the currently selected index, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_ix
    }

    /// Set the selected index, or `None` to clear selection.
    pub fn set_selected_index(&mut self, ix: Option<usize>, cx: &mut Context<Self>) {
        self.select_index(ix);
        cx.notify();
    }

    /// Set the selected index by tree item, or `None` to clear selection.
    pub fn set_selected_item(&mut self, item: Option<&TreeItem>, cx: &mut Context<Self>) {
        if let Some(item) = item {
            let ix = self
                .entries
                .iter()
                .position(|entry| entry.item.id == item.id);
            if ix.is_some() {
                self.select_index(ix);
            } else {
                self.expand_ancestors(item.id.clone(), cx);
                let ix = self
                    .entries
                    .iter()
                    .position(|entry| entry.item.id == item.id);
                self.select_index(ix);
            }
        } else {
            self.select_index(None);
        }
        cx.notify();
    }

    /// Get the currently selected tree item, if any.
    pub fn selected_item(&self) -> Option<&TreeItem> {
        self.selected_ix
            .and_then(|ix| self.entries.get(ix).map(|entry| &entry.item))
    }

    pub fn scroll_to_item(&mut self, ix: usize, strategy: gpui::ScrollStrategy) {
        self.scroll_handle.scroll_to_item(ix, strategy);
    }

    /// Find the flat index of the entry whose `item.id` matches, if present.
    pub(crate) fn index_of(&self, id: &SharedString) -> Option<usize> {
        self.entries.iter().position(|e| &e.item.id == id)
    }

    /// Expand all ancestors of the node with `id` and scroll it into view.
    /// No-op if `id` is not found. Does not change the selected index.
    pub fn reveal_item(
        &mut self,
        id: &SharedString,
        strategy: gpui::ScrollStrategy,
        cx: &mut Context<Self>,
    ) {
        self.expand_ancestors(id.clone(), cx);
        if let Some(ix) = self.index_of(id) {
            self.scroll_to_item(ix, strategy);
        }
    }

    /// Get the currently selected entry, if any.
    pub fn selected_entry(&self) -> Option<&TreeEntry> {
        self.selected_ix.and_then(|ix| self.entries.get(ix))
    }

    fn expand_ancestors(&mut self, target_id: SharedString, cx: &mut Context<Self>) {
        let mut ancestors = Vec::new();

        for entry in &self.entries {
            if let Some(found_ancestors) = entry.item.find_ancestors(&target_id) {
                ancestors = found_ancestors;
                break;
            }
        }

        if ancestors.is_empty() {
            return;
        }

        for ancestor in ancestors.into_iter().rev() {
            if !ancestor.is_expanded() {
                ancestor.state.borrow_mut().expanded = true;
                cx.emit(TreeEvent::Expanded(ancestor.id.clone()));
            }
        }

        self.rebuild_entries(None);
    }

    /// Adds visible siblings while retaining hierarchy metadata.
    fn add_entries(&mut self, items: &[TreeItem], depth: usize, parent_id: Option<SharedString>) {
        let size_of_set = items.len();
        for (position, item) in items.iter().enumerate() {
            self.entries.push(TreeEntry {
                item: item.clone(),
                depth,
                parent_id: parent_id.clone(),
                position_in_set: position + 1,
                size_of_set,
                size: self.size,
            });
            if item.is_expanded() {
                self.add_entries(&item.children, depth + 1, Some(item.id.clone()));
            }
        }
    }

    fn toggle_expand(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(entry) = self.entries.get_mut(ix) else {
            return;
        };
        if !entry.is_folder() {
            return;
        }

        let expanded = !entry.is_expanded();
        let id = entry.item.id.clone();
        entry.item.state.borrow_mut().expanded = expanded;

        if expanded {
            cx.emit(TreeEvent::Expanded(id.clone()));
        } else {
            cx.emit(TreeEvent::Collapsed(id.clone()));
        }

        self.right_clicked_ix = None;
        self.rebuild_entries((!expanded).then_some(id));
    }

    /// Rebuilds visible entries and restores selection by stable item ID.
    fn rebuild_entries(&mut self, hidden_selection_fallback: Option<SharedString>) {
        let root_items: Vec<TreeItem> = self
            .entries
            .iter()
            .filter(|e| e.is_root())
            .map(|e| e.item.clone())
            .collect();
        self.entries.clear();
        self.add_entries(&root_items, 0, None);

        let selected_ix = self.selected_id.as_ref().and_then(|id| self.index_of(id));
        if selected_ix.is_some() {
            self.selected_ix = selected_ix;
        } else if let Some(fallback_id) = hidden_selection_fallback {
            self.select_index(self.index_of(&fallback_id));
        } else {
            self.select_index(None);
        }
    }

    /// Updates index and stable-ID selection as a single invariant.
    fn select_index(&mut self, ix: Option<usize>) {
        let ix = ix.filter(|ix| {
            self.entries
                .get(*ix)
                .is_some_and(|entry| !entry.is_disabled())
        });
        self.selected_id = ix.map(|ix| self.entries[ix].item.id.clone());
        self.selected_ix = ix;
    }

    /// Finds the next enabled visible node, wrapping at the list boundary.
    fn adjacent_enabled_index(&self, direction: isize) -> Option<usize> {
        let len = self.entries.len();
        if len == 0 {
            return None;
        }

        let start = match (self.selected_ix, direction.is_positive()) {
            (Some(ix), _) => ix,
            (None, true) => len - 1,
            (None, false) => 0,
        };
        for offset in 1..=len {
            let ix =
                (start as isize + direction * offset as isize).rem_euclid(len as isize) as usize;
            if !self.entries[ix].is_disabled() {
                return Some(ix);
            }
        }
        None
    }

    /// Selects an enabled node and keeps it visible in the virtualized list.
    fn select_and_scroll(&mut self, ix: usize, strategy: gpui::ScrollStrategy) {
        self.select_index(Some(ix));
        if self.selected_ix == Some(ix) {
            self.scroll_handle.scroll_to_item(ix, strategy);
        }
    }

    /// Propagates the composite Tree size to every entry's render metrics.
    fn set_size(&mut self, size: Size) {
        if self.size == size {
            return;
        }

        self.size = size;
        for entry in &mut self.entries {
            entry.size = size;
        }
    }

    pub fn focus(&mut self, window: &mut Window, cx: &mut App) {
        self.focus_handle.focus(window, cx);
    }

    fn on_action_confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.selected_ix {
            if let Some(entry) = self.entries.get(selected_ix) {
                if entry.is_folder() {
                    self.toggle_expand(selected_ix, cx);
                    cx.notify();
                }
            }
        }
    }

    fn on_action_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        let Some(selected_ix) = self.selected_ix else {
            return;
        };
        let Some(entry) = self.entries.get(selected_ix) else {
            return;
        };
        if entry.is_folder() && entry.is_expanded() {
            self.toggle_expand(selected_ix, cx);
        } else if let Some(parent_id) = entry.parent_id.clone()
            && let Some(parent_ix) = self.index_of(&parent_id)
            && !self.entries[parent_ix].is_disabled()
        {
            self.select_and_scroll(parent_ix, gpui::ScrollStrategy::Center);
        }
        cx.notify();
    }

    fn on_action_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        let Some(selected_ix) = self.selected_ix else {
            return;
        };
        let Some(entry) = self.entries.get(selected_ix) else {
            return;
        };
        if entry.is_folder() && !entry.is_expanded() {
            self.toggle_expand(selected_ix, cx);
        } else if entry.is_folder() {
            let parent_id = entry.item.id.clone();
            if let Some(child_ix) = self.entries.iter().position(|entry| {
                entry.parent_id.as_ref() == Some(&parent_id) && !entry.is_disabled()
            }) {
                self.select_and_scroll(child_ix, gpui::ScrollStrategy::Center);
            }
        }
        cx.notify();
    }

    fn on_action_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.adjacent_enabled_index(-1) {
            self.select_and_scroll(selected_ix, gpui::ScrollStrategy::Top);
            cx.notify();
        }
    }

    fn on_action_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.adjacent_enabled_index(1) {
            self.select_and_scroll(selected_ix, gpui::ScrollStrategy::Bottom);
            cx.notify();
        }
    }

    fn on_entry_click(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window, cx);
        self.select_index(Some(ix));
        self.toggle_expand(ix, cx);
        cx.notify();
    }
}

impl Render for TreeState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let render_item = self.render_item.clone();
        let state = cx.entity().clone();

        div()
            .id("tree-state")
            .size_full()
            .relative()
            .context_menu({
                let state = state.clone();
                move |menu, window, cx: &mut Context<PopupMenu>| {
                    if state.read(cx).context_menu_builder.is_none() {
                        return menu;
                    }

                    let (ix, entry) = {
                        let state = state.read(cx);
                        let entry = state
                            .right_clicked_ix
                            .and_then(|ix| state.entries.get(ix).cloned());
                        (state.right_clicked_ix, entry)
                    };

                    if let (Some(ix), Some(entry)) = (ix, entry) {
                        state.update(cx, |state, cx| {
                            if let Some(build) = state.context_menu_builder.clone() {
                                build(ix, &entry, menu, window, cx)
                            } else {
                                menu
                            }
                        })
                    } else {
                        menu
                    }
                }
            })
            .child(
                uniform_list("entries", self.entries.len(), {
                    cx.processor(move |state, visible_range: Range<usize>, window, cx| {
                        let mut items = Vec::with_capacity(visible_range.len());
                        for ix in visible_range {
                            let entry = &state.entries[ix];
                            let selected = Some(ix) == state.selected_ix;
                            let right_clicked = Some(ix) == state.right_clicked_ix;
                            let item = (render_item)(ix, entry, selected, window, cx)
                                .with_size(state.size);
                            let entry_id = ElementId::NamedChild(
                                Arc::new(state.owner_id.clone()),
                                entry.item.id.clone(),
                            );

                            let el = div()
                                .id(entry_id)
                                .role(Role::TreeItem)
                                .aria_label(entry.item.label.clone())
                                .aria_level(entry.depth + 1)
                                .aria_position_in_set(entry.position_in_set)
                                .aria_size_of_set(entry.size_of_set)
                                .aria_selected(selected)
                                .when(entry.is_folder(), |this| {
                                    this.aria_expanded(entry.is_expanded())
                                })
                                .when(selected, |this| this.aria_active_descendant())
                                .w_full()
                                .relative()
                                .overflow_hidden()
                                .child(
                                    item.disabled(entry.item().is_disabled())
                                        .selected(selected)
                                        .secondary_selected(right_clicked),
                                )
                                .when(!entry.item().is_disabled(), |this| {
                                    this.on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener({
                                            move |this, _, window, cx| {
                                                this.on_entry_click(ix, window, cx);
                                            }
                                        }),
                                    )
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        cx.listener(move |this, _, window, cx| {
                                            this.focus_handle.focus(window, cx);
                                            this.right_clicked_ix = Some(ix);
                                            cx.notify();
                                        }),
                                    )
                                });

                            items.push(accessibility_state(
                                el,
                                false,
                                false,
                                entry.item().is_disabled(),
                            ))
                        }

                        items
                    })
                })
                .flex_grow_1()
                .size_full()
                .track_scroll(&self.scroll_handle)
                .with_sizing_behavior(ListSizingBehavior::Auto)
                .into_any_element(),
            )
    }
}

/// A tree view element that displays hierarchical data.
#[derive(IntoElement)]
pub struct Tree {
    id: ElementId,
    state: Entity<TreeState>,
    style: StyleRefinement,
    size: Size,
    aria_label: Option<SharedString>,
    render_item: Rc<dyn Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem>,
    context_menu_builder: Option<
        Rc<dyn Fn(usize, &TreeEntry, PopupMenu, &mut Window, &mut Context<TreeState>) -> PopupMenu>,
    >,
}

impl Tree {
    pub fn new<R>(state: &Entity<TreeState>, render_item: R) -> Self
    where
        R: Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem + 'static,
    {
        Self {
            id: ElementId::Name(format!("tree-{}", state.entity_id()).into()),
            state: state.clone(),
            style: StyleRefinement::default(),
            size: Size::default(),
            aria_label: None,
            render_item: Rc::new(move |ix, item, selected, window, app| {
                render_item(ix, item, selected, window, app)
            }),
            context_menu_builder: None,
        }
    }

    /// Add a context menu to the tree.
    ///
    /// The closure receives:
    /// - `ix`: the index of the right-clicked entry
    /// - `entry`: the right-clicked tree entry
    /// - `menu`: the popup menu builder
    pub fn context_menu<F>(mut self, f: F) -> Self
    where
        F: Fn(usize, &TreeEntry, PopupMenu, &mut Window, &mut Context<TreeState>) -> PopupMenu
            + 'static,
    {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }

    /// Sets the accessible name announced for the composite tree widget.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }
}

impl Sizable for Tree {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Tree {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Tree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.state.read(cx).focus_handle.clone();
        let scroll_handle = self.state.read(cx).scroll_handle.clone();
        let focus_visible = focus_handle.is_focused(window) && window.last_input_was_keyboard();
        let owner_id = self.id.clone();
        let render_item = self.render_item.clone();
        let context_menu_builder = self.context_menu_builder.clone();

        self.state.update(cx, |state, _| {
            state.render_item = render_item;
            state.context_menu_builder = context_menu_builder;
            state.owner_id = owner_id;
            state.set_size(self.size);
        });

        div()
            .id(self.id)
            .role(Role::Tree)
            .aria_label(
                self.aria_label
                    .unwrap_or_else(|| t!("Tree.label").to_string().into()),
            )
            .key_context(CONTEXT)
            .track_focus(&focus_handle)
            .on_action(window.listener_for(&self.state, TreeState::on_action_confirm))
            .on_action(window.listener_for(&self.state, TreeState::on_action_left))
            .on_action(window.listener_for(&self.state, TreeState::on_action_right))
            .on_action(window.listener_for(&self.state, TreeState::on_action_up))
            .on_action(window.listener_for(&self.state, TreeState::on_action_down))
            .relative()
            .size_full()
            .child(self.state)
            .refine_style(&self.style)
            .focus_ring(focus_visible, px(0.), window, cx)
            .vertical_scrollbar(&scroll_handle)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use indoc::indoc;

    use super::{Tree, TreeEvent, TreeItem, TreeState};
    use gpui::{AppContext as _, Render, Subscription};

    struct TestCollector {
        _state: gpui::Entity<TreeState>,
        events: Rc<RefCell<Vec<TreeEvent>>>,
        _subscription: Subscription,
    }

    impl TestCollector {
        fn new(state: &gpui::Entity<TreeState>, cx: &mut gpui::Context<Self>) -> Self {
            let events = Rc::new(RefCell::new(Vec::new()));
            let events_clone = events.clone();
            let _subscription = cx.subscribe(state, move |_, _, ev: &TreeEvent, _| {
                events_clone.borrow_mut().push(ev.clone());
            });
            Self {
                _state: state.clone(),
                events,
                _subscription,
            }
        }
    }

    impl Render for TestCollector {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }

    fn assert_entries(entries: &Vec<super::TreeEntry>, expected: &str) {
        let actual: Vec<String> = entries
            .iter()
            .map(|e| {
                let mut s = String::new();
                s.push_str(&"    ".repeat(e.depth));
                s.push_str(e.item().label.as_str());
                s
            })
            .collect();
        let actual = actual.join("\n");
        assert_eq!(actual.trim(), expected.trim());
    }

    #[gpui::test]
    fn test_tree_builder(cx: &mut gpui::TestAppContext) {
        use crate::{Sizable as _, Size};

        let state = cx.new(|cx| TreeState::new(cx));
        let tree = Tree::new(&state, |ix, _, _, _, _| crate::list::ListItem::new(ix))
            .small()
            .aria_label("Project files");

        assert_eq!(tree.size, Size::Small);
        assert_eq!(tree.aria_label.as_deref(), Some("Project files"));
    }

    #[gpui::test]
    fn test_selection_rejects_empty_disabled_and_out_of_bounds(cx: &mut gpui::TestAppContext) {
        let empty = cx.new(|cx| TreeState::new(cx));
        empty.update(cx, |state, _| {
            state.select_index(Some(0));
            assert_eq!(state.selected_index(), None);
            assert_eq!(state.adjacent_enabled_index(1), None);
            assert_eq!(state.adjacent_enabled_index(-1), None);
        });

        let state = cx.new(|cx| {
            TreeState::new(cx).items(vec![
                TreeItem::new("disabled-a", "Disabled A").disabled(true),
                TreeItem::new("enabled", "Enabled"),
                TreeItem::new("disabled-b", "Disabled B").disabled(true),
            ])
        });
        state.update(cx, |state, _| {
            state.select_index(Some(99));
            assert_eq!(state.selected_index(), None);
            state.select_index(Some(0));
            assert_eq!(state.selected_index(), None);
            assert_eq!(state.adjacent_enabled_index(1), Some(1));
            assert_eq!(state.adjacent_enabled_index(-1), Some(1));
        });

        let all_disabled = cx.new(|cx| {
            TreeState::new(cx).items(vec![
                TreeItem::new("disabled-a", "Disabled A").disabled(true),
                TreeItem::new("disabled-b", "Disabled B").disabled(true),
            ])
        });
        all_disabled.read_with(cx, |state, _| {
            assert_eq!(state.adjacent_enabled_index(1), None);
            assert_eq!(state.adjacent_enabled_index(-1), None);
        });
    }

    #[gpui::test]
    fn test_collapsing_selected_descendant_selects_parent(cx: &mut gpui::TestAppContext) {
        let state = cx.new(|cx| {
            TreeState::new(cx).items(vec![
                TreeItem::new("src", "src")
                    .expanded(true)
                    .child(TreeItem::new("src/lib.rs", "lib.rs")),
            ])
        });

        state.update(cx, |state, cx| {
            state.select_index(Some(1));
            state.toggle_expand(0, cx);

            assert_eq!(state.selected_index(), Some(0));
            assert_eq!(
                state.selected_item().map(|item| item.id.as_ref()),
                Some("src")
            );
        });
    }

    #[gpui::test]
    fn test_entries_retain_hierarchy_metadata(cx: &mut gpui::TestAppContext) {
        let state = cx.new(|cx| {
            TreeState::new(cx).items(vec![
                TreeItem::new("src", "src").expanded(true).children([
                    TreeItem::new("src/a.rs", "a.rs"),
                    TreeItem::new("src/b.rs", "b.rs"),
                ]),
                TreeItem::new("README.md", "README.md"),
            ])
        });

        state.read_with(cx, |state, _| {
            let child = &state.entries[1];
            assert_eq!(child.parent_id.as_deref(), Some("src"));
            assert_eq!(child.depth, 1);
            assert_eq!(child.position_in_set, 1);
            assert_eq!(child.size_of_set, 2);
            assert_eq!(state.entries[3].position_in_set, 2);
            assert_eq!(state.entries[3].size_of_set, 2);
        });
    }

    #[gpui::test]
    fn test_tree_size_propagates_to_entry_metrics(cx: &mut gpui::TestAppContext) {
        use crate::Size;

        let state = cx.new(|cx| TreeState::new(cx).items(vec![TreeItem::new("src", "src")]));
        state.update(cx, |state, _| {
            state.set_size(Size::Small);
            assert_eq!(state.size, Size::Small);
            assert_eq!(state.entries[0].size, Size::Small);
        });
    }

    #[gpui::test]
    fn test_tree_entry(cx: &mut gpui::TestAppContext) {
        let items = vec![
            TreeItem::new("src", "src")
                .expanded(true)
                .child(
                    TreeItem::new("src/ui", "ui")
                        .expanded(true)
                        .child(TreeItem::new("src/ui/button.rs", "button.rs"))
                        .child(TreeItem::new("src/ui/icon.rs", "icon.rs"))
                        .child(TreeItem::new("src/ui/mod.rs", "mod.rs")),
                )
                .child(TreeItem::new("src/lib.rs", "lib.rs")),
            TreeItem::new("Cargo.toml", "Cargo.toml"),
            TreeItem::new("Cargo.lock", "Cargo.lock").disabled(true),
            TreeItem::new("README.md", "README.md"),
        ];

        let state = cx.new(|cx| TreeState::new(cx).items(items));
        state.update(cx, |state, cx| {
            assert_entries(
                &state.entries,
                indoc! {
                    r#"
                src
                    ui
                        button.rs
                        icon.rs
                        mod.rs
                    lib.rs
                Cargo.toml
                Cargo.lock
                README.md
                "#
                },
            );

            let entry = state.entries.get(0).unwrap();
            assert_eq!(entry.depth(), 0);
            assert_eq!(entry.is_root(), true);
            assert_eq!(entry.is_folder(), true);
            assert_eq!(entry.is_expanded(), true);

            let entry = state.entries.get(1).unwrap();
            assert_eq!(entry.depth(), 1);
            assert_eq!(entry.is_root(), false);
            assert_eq!(entry.is_folder(), true);
            assert_eq!(entry.is_expanded(), true);
            assert_eq!(entry.item().label.as_str(), "ui");

            state.toggle_expand(1, cx);
            let entry = state.entries.get(1).unwrap();
            assert_eq!(entry.is_expanded(), false);
            assert_entries(
                &state.entries,
                indoc! {
                    r#"
                src
                    ui
                    lib.rs
                Cargo.toml
                Cargo.lock
                README.md
                "#
                },
            );
        })
    }

    #[gpui::test]
    fn test_emits_expanded_event(cx: &mut gpui::TestAppContext) {
        let items = vec![
            super::TreeItem::new("src", "src").child(super::TreeItem::new("src/lib.rs", "lib.rs")),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));
        let collector = cx.new(|cx| TestCollector::new(&state, cx));

        state.update(cx, |state, cx| {
            state.toggle_expand(0, cx);
        });

        let events = collector.read_with(cx, |c, _| c.events.borrow().clone());
        assert_eq!(events, vec![TreeEvent::Expanded("src".into())]);
    }

    #[gpui::test]
    fn test_emits_collapsed_event(cx: &mut gpui::TestAppContext) {
        let items = vec![
            super::TreeItem::new("src", "src")
                .expanded(true)
                .child(super::TreeItem::new("src/lib.rs", "lib.rs")),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));
        let collector = cx.new(|cx| TestCollector::new(&state, cx));

        state.update(cx, |state, cx| {
            state.toggle_expand(0, cx);
        });

        let events = collector.read_with(cx, |c, _| c.events.borrow().clone());
        assert_eq!(events, vec![TreeEvent::Collapsed("src".into())]);
    }

    #[gpui::test]
    fn test_set_items_does_not_emit_expansion_events(cx: &mut gpui::TestAppContext) {
        let items = vec![
            super::TreeItem::new("src", "src")
                .expanded(true)
                .child(super::TreeItem::new("src/lib.rs", "lib.rs")),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));
        let collector = cx.new(|cx| TestCollector::new(&state, cx));

        let new_items = vec![
            super::TreeItem::new("docs", "docs")
                .expanded(true)
                .child(super::TreeItem::new("docs/readme.md", "readme.md")),
        ];
        state.update(cx, |state, cx| {
            state.set_items(new_items, cx);
        });

        let events = collector.read_with(cx, |c, _| c.events.borrow().clone());
        assert!(
            events.is_empty(),
            "set_items should not emit Expanded/Collapsed events"
        );
    }

    #[gpui::test]
    fn test_event_carries_item_id(cx: &mut gpui::TestAppContext) {
        let items = vec![
            super::TreeItem::new("src", "src").expanded(true).child(
                super::TreeItem::new("src/ui", "ui")
                    .child(super::TreeItem::new("src/ui/button.rs", "button.rs")),
            ),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));
        let collector = cx.new(|cx| TestCollector::new(&state, cx));

        // Toggle the child at index 1 ("src/ui"), event payload should be the id not the index.
        state.update(cx, |state, cx| {
            state.toggle_expand(1, cx);
        });

        let events = collector.read_with(cx, |c, _| c.events.borrow().clone());
        assert_eq!(events, vec![TreeEvent::Expanded("src/ui".into())]);
    }

    #[gpui::test]
    fn test_set_selected_item_emits_expanded_events_for_hidden_ancestors(
        cx: &mut gpui::TestAppContext,
    ) {
        let target = super::TreeItem::new("src/ui/button.rs", "button.rs");
        let items = vec![
            super::TreeItem::new("src", "src")
                .child(super::TreeItem::new("src/ui", "ui").child(target.clone())),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));
        let collector = cx.new(|cx| TestCollector::new(&state, cx));

        state.update(cx, |state, cx| {
            state.set_selected_item(Some(&target), cx);
        });

        let events = collector.read_with(cx, |c, _| c.events.borrow().clone());
        assert_eq!(
            events,
            vec![
                TreeEvent::Expanded("src".into()),
                TreeEvent::Expanded("src/ui".into())
            ]
        );
    }
}
