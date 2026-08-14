use std::ops::Range;

use crate::actions::{Cancel, Confirm, SelectDown, SelectFirst, SelectLast, SelectUp};
use crate::input::{InputState, MoveDown, MoveUp};
use crate::list::cache::{MeasuredEntrySize, RowEntry, RowsCache};
use crate::{
    ActiveTheme, IconName, Size,
    input::{Input, InputEvent},
    scroll::Scrollbar,
    v_flex,
};
use crate::{Disableable, Icon, IndexPath, Selectable, Sizable, StyledExt};
use crate::{VirtualListScrollHandle, list::ListDelegate, v_virtual_list};
use gpui::{
    AnyElement, App, AvailableSpace, ClickEvent, Context, DefiniteLength, EdgesRefinement,
    EventEmitter, ListSizingBehavior, Pixels, RenderOnce, Role, ScrollStrategy, SharedString,
    StatefulInteractiveElement, StyleRefinement, Subscription, px, size,
};
use gpui::{
    AppContext, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, KeyBinding,
    Length, MouseButton, ParentElement, Render, Styled, Task, Window, div, prelude::FluentBuilder,
};
use rust_i18n::t;
use std::rc::Rc;

pub(crate) fn init(cx: &mut App) {
    let context: Option<&str> = Some("List");
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, context),
        KeyBinding::new("enter", Confirm { secondary: false }, context),
        KeyBinding::new("secondary-enter", Confirm { secondary: true }, context),
        KeyBinding::new("up", SelectUp, context),
        KeyBinding::new("down", SelectDown, context),
        KeyBinding::new("home", SelectFirst, context),
        KeyBinding::new("end", SelectLast, context),
    ]);
}

#[derive(Clone)]
pub enum ListEvent {
    /// Move to select item.
    Select(IndexPath),
    /// Click on item or pressed Enter.
    Confirm(IndexPath),
    /// Pressed ESC to deselect the item.
    Cancel,
}

struct ListOptions {
    size: Size,
    scrollbar_visible: bool,
    search_placeholder: Option<SharedString>,
    max_height: Option<Length>,
    paddings: EdgesRefinement<DefiniteLength>,
    aria_label: SharedString,
    search_renderer: Option<SearchRenderer>,
}

type SearchRenderer =
    Rc<dyn Fn(Entity<InputState>, Option<SharedString>, &mut Window, &mut App) -> AnyElement>;

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            size: Size::default(),
            scrollbar_visible: true,
            max_height: None,
            search_placeholder: None,
            paddings: EdgesRefinement::default(),
            aria_label: t!("List.label").into(),
            search_renderer: None,
        }
    }
}

/// The state for List.
///
/// List required all items has the same height.
pub struct ListState<D: ListDelegate> {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) query_input: Entity<InputState>,
    options: ListOptions,
    delegate: D,
    last_query: Option<String>,
    scroll_handle: VirtualListScrollHandle,
    rows_cache: RowsCache,
    selected_index: Option<IndexPath>,
    selection_needs_sync: bool,
    item_to_measure_index: Option<IndexPath>,
    deferred_scroll_to_index: Option<(IndexPath, ScrollStrategy)>,
    mouse_right_clicked_index: Option<IndexPath>,
    reset_on_cancel: bool,
    searchable: bool,
    selectable: bool,
    _search_task: Task<()>,
    _load_more_task: Task<()>,
    search_revision: u64,
    select_first_after_search: bool,
    loading_more: bool,
    last_load_more_entities_count: Option<usize>,
    _query_input_subscription: Subscription,
}

impl<D> ListState<D>
where
    D: ListDelegate,
{
    pub fn new(delegate: D, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("List.search_placeholder")));

        let _query_input_subscription =
            cx.subscribe_in(&query_input, window, Self::on_query_input_event);

        Self {
            focus_handle: cx.focus_handle(),
            options: ListOptions::default(),
            delegate,
            rows_cache: RowsCache::default(),
            query_input,
            last_query: None,
            selected_index: None,
            selection_needs_sync: false,
            selectable: true,
            searchable: false,
            item_to_measure_index: None,
            deferred_scroll_to_index: None,
            mouse_right_clicked_index: None,
            scroll_handle: VirtualListScrollHandle::new(),
            reset_on_cancel: true,
            _search_task: Task::ready(()),
            _load_more_task: Task::ready(()),
            search_revision: 0,
            select_first_after_search: false,
            loading_more: false,
            last_load_more_entities_count: None,
            _query_input_subscription,
        }
    }

    /// Sets whether the list is searchable, default is `false`.
    ///
    /// When `true`, there will be a search input at the top of the list.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    pub fn set_searchable(&mut self, searchable: bool, cx: &mut Context<Self>) {
        self.searchable = searchable;
        cx.notify();
    }

    /// Sets whether the list is selectable, default is true.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Sets the initial keyboard selection before the first render.
    pub fn initial_selected_index(mut self, ix: Option<IndexPath>) -> Self {
        self.selected_index = ix;
        self.selection_needs_sync = true;
        self
    }

    /// Sets whether the list is selectable, default is true.
    pub fn set_selectable(&mut self, selectable: bool, cx: &mut Context<Self>) {
        self.selectable = selectable;
        cx.notify();
    }

    pub fn delegate(&self) -> &D {
        &self.delegate
    }

    pub fn delegate_mut(&mut self) -> &mut D {
        &mut self.delegate
    }

    /// Focus the list, if the list is searchable, focus the search input.
    pub fn focus(&mut self, window: &mut Window, cx: &mut App) {
        self.focus_handle(cx).focus(window, cx);
    }

    /// Set the selected index of the list,
    /// this will also scroll to the selected item.
    pub(crate) fn _set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.selectable {
            return;
        }

        let ix = ix.filter(|ix| self.is_valid_enabled_index(*ix, cx));
        self.selected_index = ix;
        self.selection_needs_sync = false;
        self.delegate.set_selected_index(ix, window, cx);
        self.scroll_to_selected_item(window, cx);
    }

    /// Set the selected index of the list,
    /// this method will not scroll to the selected item.
    pub fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ix = ix.filter(|ix| self.is_valid_enabled_index(*ix, cx));
        self.selected_index = ix;
        self.selection_needs_sync = false;
        self.delegate.set_selected_index(ix, window, cx);
    }

    pub fn selected_index(&self) -> Option<IndexPath> {
        self.selected_index
    }

    /// Set the index of the item that has been right clicked.
    pub fn set_right_clicked_index(
        &mut self,
        ix: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mouse_right_clicked_index = ix;
        self.delegate.set_right_clicked_index(ix, window, cx);
    }

    /// Returns the index of the item that has been right clicked.
    pub fn right_clicked_index(&self) -> Option<IndexPath> {
        self.mouse_right_clicked_index
    }

    /// Set the query text of the search input, this will trigger a search.
    pub fn set_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        let query = query.to_string();
        self.query_input.update(cx, |input, cx| {
            input.set_value(query, window, cx);
        });
    }

    /// Returns the InputState used by the searchable list header.
    pub(crate) fn query_input(&self) -> Entity<InputState> {
        self.query_input.clone()
    }

    /// Set a specific list item for measurement.
    pub fn set_item_to_measure_index(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.item_to_measure_index = Some(ix);
        cx.notify();
    }

    /// Scroll to the item at the given index.
    pub fn scroll_to_item(
        &mut self,
        ix: IndexPath,
        strategy: ScrollStrategy,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if ix.section == 0 && ix.row == 0 {
            // If the item is the first item, scroll to the top.
            let mut offset = self.scroll_handle.base_handle().offset();
            offset.y = px(0.);
            self.scroll_handle.base_handle().set_offset(offset);
            cx.notify();
            return;
        }
        self.deferred_scroll_to_index = Some((ix, strategy));
        cx.notify();
    }

    /// Get scroll handle
    pub fn scroll_handle(&self) -> &VirtualListScrollHandle {
        &self.scroll_handle
    }

    pub fn scroll_to_selected_item(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.selected_index {
            self.deferred_scroll_to_index = Some((ix, ScrollStrategy::Top));
            cx.notify();
        }
    }

    /// Prepares the virtual rows and returns the selected row center within a
    /// clamped viewport. The scroll offset is applied before layout so an
    /// item-aligned popup is stable on its first painted frame.
    pub(crate) fn prepare_item_alignment(
        &mut self,
        ix: Option<IndexPath>,
        max_height: Pixels,
        padding_top: Pixels,
        padding_bottom: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Pixels> {
        self.prepare_items_if_needed(window, cx);

        let ix = ix
            .filter(|ix| self.is_valid_enabled_index(*ix, cx))
            .or_else(|| self.first_enabled_index(cx))?;
        if self.selected_index != Some(ix) {
            self.selected_index = Some(ix);
            self.selection_needs_sync = false;
            self.delegate.set_selected_index(Some(ix), window, cx);
        }

        let position = self.rows_cache.position_of(&ix)?;
        let item_size = self.rows_cache.entries_sizes.get(position)?.height;
        let item_top = padding_top
            + self
                .rows_cache
                .entries_sizes
                .iter()
                .take(position)
                .map(|size| size.height)
                .sum::<Pixels>();
        let item_center = item_top + item_size / 2.;
        let content_height = padding_top
            + self
                .rows_cache
                .entries_sizes
                .iter()
                .map(|size| size.height)
                .sum::<Pixels>()
            + padding_bottom;
        let viewport_height = content_height.min(max_height);

        let minimum_offset = (viewport_height - content_height).min(px(0.));
        let desired_offset = viewport_height / 2. - item_center;
        let offset = desired_offset.max(minimum_offset).min(px(0.));
        let mut scroll_offset = self.scroll_handle.base_handle().offset();
        scroll_offset.y = offset;
        self.scroll_handle.base_handle().set_offset(scroll_offset);
        self.deferred_scroll_to_index = None;

        Some(item_center + offset)
    }

    fn on_query_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let text = state.read(cx).value();
                let text = text.trim().to_string();
                if Some(&text) == self.last_query.as_ref() {
                    return;
                }

                self.search_revision = self.search_revision.wrapping_add(1);
                let revision = self.search_revision;
                self.select_first_after_search = false;
                self.last_query = Some(text.clone());
                self.last_load_more_entities_count = None;
                self.set_searching(true, window, cx);
                let search = self.delegate.perform_search(&text, window, cx);

                self._search_task = cx.spawn_in(window, async move |this, window| {
                    search.await;
                    _ = this.update_in(window, |this, window, cx| {
                        if this.search_revision != revision {
                            return;
                        }
                        this.set_searching(false, window, cx);
                        // Measuring list elements is only valid during layout/render. Defer
                        // rebuilding the cursor until the next render pass has refreshed rows.
                        this.select_first_after_search = true;
                        cx.notify();
                    });
                });
            }
            _ => {}
        }
    }

    fn set_searching(&mut self, searching: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.query_input
            .update(cx, |input, cx| input.set_loading(searching, window, cx));
    }

    fn is_valid_index(&self, ix: IndexPath, cx: &App) -> bool {
        ix.section < self.delegate.sections_count(cx).max(1)
            && ix.row < self.delegate.items_count(ix.section, cx)
    }

    fn is_valid_enabled_index(&self, ix: IndexPath, cx: &App) -> bool {
        self.is_valid_index(ix, cx) && self.delegate.is_item_enabled(ix, cx)
    }

    fn first_enabled_index(&self, cx: &App) -> Option<IndexPath> {
        self.rows_cache
            .entities
            .iter()
            .filter_map(|entry| match entry {
                RowEntry::Entry(ix) => Some(*ix),
                _ => None,
            })
            .find(|ix| self.delegate.is_item_enabled(*ix, cx))
    }

    fn last_enabled_index(&self, cx: &App) -> Option<IndexPath> {
        self.rows_cache
            .entities
            .iter()
            .rev()
            .filter_map(|entry| match entry {
                RowEntry::Entry(ix) => Some(*ix),
                _ => None,
            })
            .find(|ix| self.delegate.is_item_enabled(*ix, cx))
    }

    fn next_enabled_index(&self, forward: bool, cx: &App) -> Option<IndexPath> {
        let entities_count = self.rows_cache.len();
        if entities_count == 0 {
            return None;
        }

        let start = self
            .selected_index
            .and_then(|ix| self.rows_cache.position_of(&ix))
            .unwrap_or_else(|| if forward { entities_count - 1 } else { 0 });

        // Traverse the flattened cache once so disabled runs remain linear.
        for offset in 1..=entities_count {
            let flatten_ix = if forward {
                start.wrapping_add(offset) % entities_count
            } else {
                let step = offset % entities_count;
                start.wrapping_add(entities_count).wrapping_sub(step) % entities_count
            };

            let Some(RowEntry::Entry(ix)) = self.rows_cache.get(flatten_ix) else {
                continue;
            };
            if self.delegate.is_item_enabled(ix, cx) {
                return Some(ix);
            }
        }
        None
    }

    fn select_previous(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.next_enabled_index(false, cx) {
            self.select_item(ix, window, cx);
        } else {
            cx.propagate();
        }
    }

    fn select_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.next_enabled_index(true, cx) {
            self.select_item(ix, window, cx);
        } else {
            cx.propagate();
        }
    }

    /// Dispatch delegate's `load_more` method when the
    /// visible range is near the end.
    fn load_more_if_need(
        &mut self,
        entities_count: usize,
        visible_end: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let threshold = self.delegate.load_more_threshold();
        // Securely handle subtract logic to prevent attempt
        // to subtract with overflow
        if visible_end >= entities_count.saturating_sub(threshold) {
            if !self.delegate.has_more(cx)
                || self.loading_more
                || self.last_load_more_entities_count == Some(entities_count)
            {
                return;
            }

            self.loading_more = true;
            self.last_load_more_entities_count = Some(entities_count);
            let task = self.delegate.load_more(window, cx);
            self._load_more_task = cx.spawn_in(window, async move |view, cx| {
                task.await;
                _ = view.update_in(cx, |view, _, cx| {
                    view.loading_more = false;
                    cx.notify();
                });
            });
        }
    }

    pub(crate) fn reset_on_cancel(mut self, reset: bool) -> Self {
        self.reset_on_cancel = reset;
        self
    }

    fn on_action_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selectable {
            cx.propagate();
            return;
        }
        if self.reset_on_cancel {
            self._set_selected_index(None, window, cx);
        }

        self.delegate.cancel(window, cx);
        cx.emit(ListEvent::Cancel);
        cx.notify();
    }

    pub(crate) fn on_action_confirm(
        &mut self,
        confirm: &Confirm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.selectable {
            cx.propagate();
            return;
        }

        let Some(ix) = self
            .selected_index
            .filter(|ix| self.is_valid_enabled_index(*ix, cx))
        else {
            cx.propagate();
            return;
        };

        self.delegate
            .set_selected_index(self.selected_index, window, cx);
        self.delegate.confirm(confirm.secondary, window, cx);
        cx.emit(ListEvent::Confirm(ix));
        cx.notify();
    }

    fn select_item(&mut self, ix: IndexPath, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selectable || !self.is_valid_enabled_index(ix, cx) {
            return;
        }

        self.selected_index = Some(ix);
        self.selection_needs_sync = false;
        self.delegate.set_selected_index(Some(ix), window, cx);
        self.scroll_to_selected_item(window, cx);
        cx.emit(ListEvent::Select(ix));
        cx.notify();
    }

    pub(crate) fn on_action_select_prev(
        &mut self,
        _: &SelectUp,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_previous(window, cx);
    }

    pub(crate) fn on_action_select_next(
        &mut self,
        _: &SelectDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_next(window, cx);
    }

    fn on_action_move_up(&mut self, _: &MoveUp, window: &mut Window, cx: &mut Context<Self>) {
        self.select_previous(window, cx);
    }

    fn on_action_move_down(&mut self, _: &MoveDown, window: &mut Window, cx: &mut Context<Self>) {
        self.select_next(window, cx);
    }

    fn on_action_select_first(
        &mut self,
        _: &SelectFirst,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(ix) = self.first_enabled_index(cx) {
            self.select_item(ix, window, cx);
        } else {
            cx.propagate();
        }
    }

    fn on_action_select_last(
        &mut self,
        _: &SelectLast,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(ix) = self.last_enabled_index(cx) {
            self.select_item(ix, window, cx);
        } else {
            cx.propagate();
        }
    }

    fn prepare_items_if_needed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sections_count = self.delegate.sections_count(cx).max(1);
        let mut measured_size = MeasuredEntrySize::default();

        let first_item = (0..sections_count).find_map(|section| {
            (self.delegate.items_count(section, cx) > 0)
                .then_some(IndexPath::default().section(section))
        });
        let measurement_item = self
            .item_to_measure_index
            .filter(|ix| self.is_valid_index(*ix, cx))
            .or(first_item);

        // Items share one height, while section chrome is measured independently so virtual
        // offsets remain correct when only some groups render separators.
        let available_space = size(AvailableSpace::MinContent, AvailableSpace::MinContent);
        if let Some(ix) = measurement_item {
            measured_size.item_size = self
                .render_list_item(ix, window, cx)
                .into_any_element()
                .layout_as_root(available_space, window, cx);
        }

        measured_size.section_header_sizes = vec![Default::default(); sections_count];
        measured_size.section_footer_sizes = vec![Default::default(); sections_count];
        for section in 0..sections_count {
            if self.delegate.items_count(section, cx) == 0 {
                continue;
            }

            if let Some(mut el) = self
                .delegate
                .render_section_header(section, window, cx)
                .map(|r| r.into_any_element())
            {
                measured_size.section_header_sizes[section] =
                    el.layout_as_root(available_space, window, cx);
            }
            if let Some(mut el) = self
                .delegate
                .render_section_footer(section, window, cx)
                .map(|r| r.into_any_element())
            {
                measured_size.section_footer_sizes[section] =
                    el.layout_as_root(available_space, window, cx);
            }
        }

        self.rows_cache
            .prepare_if_needed(sections_count, measured_size, cx, |section_ix, cx| {
                self.delegate.items_count(section_ix, cx)
            });

        if self
            .selected_index
            .is_some_and(|ix| !self.is_valid_enabled_index(ix, cx))
        {
            self.selected_index = None;
            self.selection_needs_sync = true;
        }
    }

    fn render_list_item(
        &mut self,
        ix: IndexPath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selectable = self.selectable;
        let enabled = self.delegate.is_item_enabled(ix, cx);
        let toggled = self.delegate.item_toggled(ix, cx);
        let selected = self.selected_index.map(|s| s.eq_row(ix)).unwrap_or(false);
        let mouse_right_clicked = self
            .mouse_right_clicked_index
            .map(|s| s.eq_row(ix))
            .unwrap_or(false);
        let id = SharedString::from(format!("list-{}-item-{}", cx.entity().entity_id(), ix));

        let total_items = self.rows_cache.items_count();
        let position = self.rows_cache.item_ordinal(ix).unwrap_or(1);
        let label = self.delegate.item_label(ix, cx);
        let item = self
            .delegate
            .render_item(ix, window, cx)
            .with_size(self.options.size)
            .disabled(!enabled)
            .selected(selected)
            .secondary_selected(mouse_right_clicked);

        let element = div()
            .id(id)
            .role(if selectable {
                Role::ListBoxOption
            } else {
                Role::ListItem
            })
            .aria_label(label)
            .aria_position_in_set(position)
            .aria_size_of_set(total_items)
            .when(selectable, |this| this.aria_selected(selected))
            .when_some(toggled, |this, toggled| this.aria_toggled(toggled.into()))
            .when(selected, |this| this.aria_active_descendant())
            .w_full()
            .relative()
            .overflow_hidden()
            .child(item)
            .when(selectable && enabled, |this| {
                this.on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
                    this.set_right_clicked_index(None, window, cx);
                    this.select_item(ix, window, cx);
                    this.on_action_confirm(
                        &Confirm {
                            secondary: e.modifiers().secondary(),
                        },
                        window,
                        cx,
                    );
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, _, window, cx| {
                        this.set_right_clicked_index(Some(ix), window, cx);
                        cx.notify();
                    }),
                )
            });

        crate::accessibility::accessibility_state(element, false, false, !enabled)
    }

    fn render_items(
        &mut self,
        items_count: usize,
        entities_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let rows_cache = self.rows_cache.clone();
        let scrollbar_visible = self.options.scrollbar_visible;
        let scroll_handle = self.scroll_handle.clone();
        let item_to_measure_index = self
            .item_to_measure_index
            .and_then(|ix| rows_cache.position_of(&ix))
            .or_else(|| rows_cache.first_entry_position())
            .unwrap_or(0);

        v_flex()
            .flex_grow_1()
            .relative()
            .size_full()
            .when_some(self.options.max_height, |this, h| this.max_h(h))
            .overflow_hidden()
            .when(items_count == 0, |this| {
                this.child(self.delegate.render_empty(window, cx))
            })
            .when(items_count > 0, {
                |this| {
                    this.child(
                        v_virtual_list(
                            cx.entity(),
                            "virtual-list",
                            rows_cache.entries_sizes.clone(),
                            move |list, visible_range: Range<usize>, window, cx| {
                                list.load_more_if_need(
                                    entities_count,
                                    visible_range.end,
                                    window,
                                    cx,
                                );

                                // NOTE: Here the v_virtual_list would not able to have gap_y,
                                // because the section header, footer is always have rendered as a empty child item,
                                // even the delegate give a None result.

                                visible_range
                                    .map(|ix| {
                                        let Some(entry) = rows_cache.get(ix) else {
                                            return div();
                                        };

                                        div().children(match entry {
                                            RowEntry::Entry(index) => Some(
                                                list.render_list_item(index, window, cx)
                                                    .into_any_element(),
                                            ),
                                            RowEntry::SectionHeader(section_ix) => list
                                                .delegate_mut()
                                                .render_section_header(section_ix, window, cx)
                                                .map(|r| r.into_any_element()),
                                            RowEntry::SectionFooter(section_ix) => list
                                                .delegate_mut()
                                                .render_section_footer(section_ix, window, cx)
                                                .map(|r| r.into_any_element()),
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            },
                        )
                        .with_item_to_measure_index(item_to_measure_index)
                        .paddings(self.options.paddings.clone())
                        .when(self.options.max_height.is_some(), |this| {
                            this.with_sizing_behavior(ListSizingBehavior::Infer)
                        })
                        .track_scroll(&scroll_handle)
                        .into_any_element(),
                    )
                }
            })
            .when(scrollbar_visible, |this| {
                this.child(Scrollbar::vertical(&scroll_handle))
            })
    }
}

impl<D> Focusable for ListState<D>
where
    D: ListDelegate,
{
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        if self.searchable {
            self.query_input.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}
impl<D> EventEmitter<ListEvent> for ListState<D> where D: ListDelegate {}
impl<D> Render for ListState<D>
where
    D: ListDelegate,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.prepare_items_if_needed(window, cx);

        if self.select_first_after_search {
            let first = self.first_enabled_index(cx);
            self._set_selected_index(first, window, cx);
            self.scroll_handle.scroll_to_item(0, ScrollStrategy::Top);
            self.select_first_after_search = false;
        }

        if self.selection_needs_sync {
            self.delegate
                .set_selected_index(self.selected_index, window, cx);
            self.selection_needs_sync = false;
        }

        // Scroll to the selected item if it is set.
        if let Some((ix, strategy)) = self.deferred_scroll_to_index.take() {
            if let Some(item_ix) = self.rows_cache.position_of(&ix) {
                self.scroll_handle.scroll_to_item(item_ix, strategy);
            }
        }

        let loading = self.delegate().loading(cx);
        let query_input = if self.searchable {
            // sync placeholder
            if let Some(placeholder) = &self.options.search_placeholder {
                self.query_input.update(cx, |input, cx| {
                    input.set_placeholder(placeholder.clone(), window, cx);
                });
            }
            Some(self.query_input.clone())
        } else {
            None
        };

        let loading_view = if loading {
            Some(self.delegate.render_loading(window, cx).into_any_element())
        } else {
            None
        };
        let initial_view = if let Some(input) = &query_input {
            if input.read(cx).value().is_empty() {
                self.delegate.render_initial(window, cx)
            } else {
                None
            }
        } else {
            None
        };
        let items_count = self.rows_cache.items_count();
        let entities_count = self.rows_cache.len();
        let mouse_right_clicked_index = self.mouse_right_clicked_index;
        let search_padding_x = self.options.size.table_cell_padding(cx).left;
        let active_item_label = self
            .selected_index
            .filter(|ix| self.is_valid_index(*ix, cx))
            .map(|ix| self.delegate.item_label(ix, cx));

        v_flex()
            .key_context("List")
            .id(("list-state", cx.entity().entity_id()))
            .track_focus(&self.focus_handle)
            .role(if self.selectable {
                Role::ListBox
            } else {
                Role::List
            })
            .aria_label(self.options.aria_label.clone())
            .size_full()
            .relative()
            .overflow_hidden()
            .when_some(query_input, |this, input| {
                let custom_search = self.options.search_renderer.clone();
                this.child(if let Some(renderer) = custom_search {
                    renderer(input, active_item_label.clone(), window, cx)
                } else {
                    div()
                        .px(search_padding_x)
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            Input::new(&input)
                                .with_size(self.options.size)
                                .aria_label(
                                    self.options
                                        .search_placeholder
                                        .clone()
                                        .unwrap_or_else(|| t!("List.search_placeholder").into()),
                                )
                                .when_some(active_item_label.clone(), |this, label| {
                                    this.aria_description(label)
                                })
                                .prefix(
                                    Icon::new(IconName::Search)
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .cleanable(true)
                                .p_0()
                                .appearance(false),
                        )
                        .into_any_element()
                })
            })
            .when(!loading, |this| {
                this.on_action(cx.listener(Self::on_action_cancel))
                    .on_action(cx.listener(Self::on_action_confirm))
                    .on_action(cx.listener(Self::on_action_select_next))
                    .on_action(cx.listener(Self::on_action_select_prev))
                    .on_action(cx.listener(Self::on_action_select_first))
                    .on_action(cx.listener(Self::on_action_select_last))
                    .on_action(cx.listener(Self::on_action_move_up))
                    .on_action(cx.listener(Self::on_action_move_down))
                    .map(|this| {
                        if let Some(view) = initial_view {
                            this.child(view)
                        } else {
                            this.child(self.render_items(items_count, entities_count, window, cx))
                        }
                    })
                    // Click out to cancel right clicked row
                    .when(mouse_right_clicked_index.is_some(), |this| {
                        this.on_mouse_down_out(cx.listener(|this, _, window, cx| {
                            this.set_right_clicked_index(None, window, cx);
                            cx.notify();
                        }))
                    })
            })
            .children(loading_view)
    }
}

/// The List element.
#[derive(IntoElement)]
pub struct List<D: ListDelegate + 'static> {
    state: Entity<ListState<D>>,
    style: StyleRefinement,
    options: ListOptions,
}

impl<D> List<D>
where
    D: ListDelegate + 'static,
{
    /// Create a new List element with the given ListState entity.
    pub fn new(state: &Entity<ListState<D>>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            options: ListOptions::default(),
        }
    }

    /// Set whether the scrollbar is visible, default is `true`.
    pub fn scrollbar_visible(mut self, visible: bool) -> Self {
        self.options.scrollbar_visible = visible;
        self
    }

    /// Sets the placeholder text for the search input.
    pub fn search_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.options.search_placeholder = Some(placeholder.into());
        self
    }

    /// Sets the accessible name announced for the list.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.options.aria_label = label.into();
        self
    }

    /// Replaces the built-in searchable header while retaining ListState behavior.
    pub(crate) fn search_renderer(
        mut self,
        renderer: impl Fn(Entity<InputState>, Option<SharedString>, &mut Window, &mut App) -> AnyElement
        + 'static,
    ) -> Self {
        self.options.search_renderer = Some(Rc::new(renderer));
        self
    }
}

impl<D> Styled for List<D>
where
    D: ListDelegate + 'static,
{
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl<D> Sizable for List<D>
where
    D: ListDelegate + 'static,
{
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.options.size = size.into();
        self
    }
}

impl<D> RenderOnce for List<D>
where
    D: ListDelegate + 'static,
{
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // Take paddings, max_height to options, and clear them from style,
        // because they would be applied to the inner virtual list.
        self.options.paddings = self.style.padding.clone();
        self.options.max_height = self.style.max_size.height;
        self.style.padding = EdgesRefinement::default();
        self.style.max_size.height = None;

        self.state.update(cx, |state, _| {
            state.options = self.options;
        });

        div()
            .id(("list", self.state.entity_id()))
            .size_full()
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;
    use crate::list::ListItem;

    struct SearchDelegate;

    impl ListDelegate for SearchDelegate {
        type Item = ListItem;

        fn perform_search(
            &mut self,
            _: &str,
            _: &mut Window,
            _: &mut Context<ListState<Self>>,
        ) -> Task<()> {
            Task::ready(())
        }

        fn items_count(&self, _: usize, _: &App) -> usize {
            1
        }

        fn item_label(&self, _: IndexPath, _: &App) -> SharedString {
            "Result".into()
        }

        fn render_item(
            &mut self,
            ix: IndexPath,
            _: &mut Window,
            _: &mut Context<ListState<Self>>,
        ) -> Self::Item {
            ListItem::new(("search-result", ix.row)).child("Result")
        }

        fn set_selected_index(
            &mut self,
            _: Option<IndexPath>,
            _: &mut Window,
            _: &mut Context<ListState<Self>>,
        ) {
        }
    }

    #[gpui::test]
    fn search_completion_defers_measurement_until_render(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let list = cx.update(|window, cx| {
            cx.new(|cx| ListState::new(SearchDelegate, window, cx).searchable(true))
        });
        let input = cx.update(|_, cx| list.read(cx).query_input.clone());

        cx.update(|window, cx| {
            input.update(cx, |input, cx| input.set_value("result", window, cx));
            list.update(cx, |list, cx| {
                list.on_query_input_event(&input, &InputEvent::Change, window, cx);
            });
        });
        cx.run_until_parked();

        assert!(cx.update(|_, cx| list.read(cx).select_first_after_search));
        assert_eq!(cx.update(|_, cx| list.read(cx).rows_cache.items_count()), 0);
    }
}
