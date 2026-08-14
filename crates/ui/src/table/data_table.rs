// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `aria_label`.
// - Added or exposed behavior through `aria_label`, `style`.
// - Reworked Data Table around accessibility semantics and ARIA state, semantic Style Preset
//   geometry and density.
// - Replaced legacy radius access with `Theme.style.radii.md`.
use crate::{
    ActiveTheme, Sizable, Size, StyledExt as _,
    actions::{
        Cancel, SelectDown, SelectFirst, SelectLast, SelectNextColumn, SelectPageDown,
        SelectPageUp, SelectPrevColumn, SelectUp,
    },
    table::{TableDelegate, TableState},
};
use gpui::{
    App, Edges, Entity, Focusable, InteractiveElement, IntoElement, KeyBinding, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Window, div, prelude::FluentBuilder,
};

const CONTEXT: &'static str = "DataTable";
pub(super) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("left", SelectPrevColumn, Some(CONTEXT)),
        KeyBinding::new("right", SelectNextColumn, Some(CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
        KeyBinding::new("pageup", SelectPageUp, Some(CONTEXT)),
        KeyBinding::new("pagedown", SelectPageDown, Some(CONTEXT)),
        KeyBinding::new("tab", SelectNextColumn, Some(CONTEXT)),
        KeyBinding::new("shift-tab", SelectPrevColumn, Some(CONTEXT)),
    ]);
}

pub(super) struct TableOptions {
    pub(super) scrollbar_visible: Edges<bool>,
    /// Set stripe style of the table.
    pub(super) stripe: bool,
    /// Set to use border style of the table.
    pub(super) bordered: bool,
    /// The cell size of the table.
    pub(super) size: Size,
}

impl Default for TableOptions {
    fn default() -> Self {
        Self {
            scrollbar_visible: Edges::all(true),
            stripe: false,
            bordered: true,
            size: Size::default(),
        }
    }
}

/// A table element with support for row, column, and cell selection.
///
/// # Features
///
/// - **Multiple Selection Modes**: Support for row, column, and cell selection
/// - **Cell Selection**: Click to select individual cells, with keyboard navigation
/// - **Virtual Scrolling**: Efficient rendering of large datasets
/// - **Resizable Columns**: Drag column borders to resize
/// - **Movable Columns**: Drag column headers to reorder
/// - **Fixed Columns**: Pin columns to the left side
/// - **Sortable Columns**: Click column headers to sort
/// - **Context Menus**: Right-click support for rows and cells
///
/// # Cell Selection Mode
///
/// When cell selection is enabled via [`TableState::cell_selectable()`]:
/// - Click on cells to select them
/// - A row header column appears on the left for selecting entire rows
///   (use [`TableState::row_header()`] to hide it)
/// - Keyboard navigation (arrow keys, Tab, Home, End, PageUp, PageDown) works at cell level
/// - Right-click and double-click events are supported
///
/// See [`TableState`] for more details on cell selection.
///
/// # Example
///
/// ```rust,ignore
/// let table_state = cx.new(|cx| {
///     TableState::new(delegate, cx)
///         .cell_selectable(true)
///         .row_selectable(true)
/// });
///
/// DataTable::new(&table_state)
///     .stripe(true)
///     .bordered(true)
/// ```
#[derive(IntoElement)]
pub struct DataTable<D: TableDelegate> {
    state: Entity<TableState<D>>,
    options: TableOptions,
    aria_label: Option<SharedString>,
    style: StyleRefinement,
}

impl<D> DataTable<D>
where
    D: TableDelegate,
{
    /// Create a new DataTable element with the given [`TableState`].
    pub fn new(state: &Entity<TableState<D>>) -> Self {
        Self {
            state: state.clone(),
            options: TableOptions::default(),
            aria_label: None,
            style: StyleRefinement::default(),
        }
    }

    /// Set the accessible name announced for the interactive data grid.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set to use stripe style of the table, default to false.
    pub fn stripe(mut self, stripe: bool) -> Self {
        self.options.stripe = stripe;
        self
    }

    /// Set to use border style of the table, default to true.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.options.bordered = bordered;
        self
    }

    /// Set scrollbar visibility.
    pub fn scrollbar_visible(mut self, vertical: bool, horizontal: bool) -> Self {
        self.options.scrollbar_visible = Edges {
            right: vertical,
            bottom: horizontal,
            ..Default::default()
        };
        self
    }
}

impl<D> Styled for DataTable<D>
where
    D: TableDelegate,
{
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl<D> Sizable for DataTable<D>
where
    D: TableDelegate,
{
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.options.size = size.into();
        self
    }
}

impl<D> RenderOnce for DataTable<D>
where
    D: TableDelegate,
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let bordered = self.options.bordered;
        let focus_handle = self.state.focus_handle(cx);
        let (rows_count, columns_count) = {
            let state = self.state.read(cx);
            (
                state.accessibility_row_count(cx),
                state.delegate().columns_count(cx),
            )
        };
        self.state.update(cx, |state, _| {
            state.options = self.options;
        });

        div()
            .id(("data-table", self.state.entity_id()))
            .role(Role::Grid)
            .aria_row_count(rows_count)
            .aria_column_count(columns_count)
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .size_full()
            .text_sm()
            .key_context(CONTEXT)
            .track_focus(&focus_handle)
            .on_action(window.listener_for(&self.state, TableState::action_cancel))
            .on_action(window.listener_for(&self.state, TableState::action_select_next))
            .on_action(window.listener_for(&self.state, TableState::action_select_prev))
            .on_action(window.listener_for(&self.state, TableState::action_select_next_col))
            .on_action(window.listener_for(&self.state, TableState::action_select_prev_col))
            .on_action(window.listener_for(&self.state, TableState::action_select_first_column))
            .on_action(window.listener_for(&self.state, TableState::action_select_last_column))
            .on_action(window.listener_for(&self.state, TableState::action_select_page_up))
            .on_action(window.listener_for(&self.state, TableState::action_select_page_down))
            .bg(cx.theme().tokens.table)
            .when(bordered, |this| {
                this.rounded(cx.theme().style.radii.md)
                    .border_1()
                    .border_color(cx.theme().border)
            })
            .refine_style(&self.style)
            .child(self.state)
    }
}
