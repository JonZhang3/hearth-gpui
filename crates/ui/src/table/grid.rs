use gpui::{
    AnyElement, App, Edges, ElementId, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, Pixels, Refineable as _, RenderOnce, Role, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, TextAlign, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::{
    ActiveTheme as _, ChildElement, Sizable, Size,
    scroll::{ScrollableElement as _, horizontal_scroll_area},
};

const DEFAULT_MIN_COLUMN_WIDTH: Pixels = px(100.);
const FRAME_BORDER_WIDTH: Pixels = px(1.5);
const CELL_BORDER_WIDTH: Pixels = px(1.);
const MAX_GRID_TRACKS: usize = 1_000;

/// Controls whether table tracks preserve unwrapped content or may shrink.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableGridSizing {
    #[default]
    MaxContent,
    MinContent,
}

/// A cell in a [`TableGrid`].
///
/// `intrinsic_width` is the unwrapped width of the cell content. `TableGrid`
/// owns padding, borders, spans, and conversion to the content-box width
/// required by the currently pinned GPUI/Taffy layout engine.
pub struct TableGridCell {
    children: Vec<AnyElement>,
    intrinsic_width: Pixels,
    col_span: usize,
    row_span: usize,
    header: bool,
    align: TextAlign,
    id: Option<ElementId>,
    debug_selector: Option<SharedString>,
    aria_label: Option<SharedString>,
}

impl TableGridCell {
    /// Creates an empty cell with one-row and one-column spans.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            intrinsic_width: Pixels::ZERO,
            col_span: 1,
            row_span: 1,
            header: false,
            align: TextAlign::Left,
            id: None,
            debug_selector: None,
            aria_label: None,
        }
    }

    /// Adds one rendered child to this cell.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    /// Adds rendered children to this cell in source order.
    pub fn children(mut self, children: impl IntoIterator<Item = AnyElement>) -> Self {
        self.children.extend(children);
        self
    }

    /// Sets the unwrapped content width used to derive the shared column tracks.
    pub fn intrinsic_width(mut self, width: Pixels) -> Self {
        self.intrinsic_width = width.max(Pixels::ZERO);
        self
    }

    /// Marks this cell as a column header.
    pub fn header(mut self, header: bool) -> Self {
        self.header = header;
        self
    }

    /// Sets the number of columns occupied by this cell.
    pub fn col_span(mut self, span: usize) -> Self {
        self.col_span = span.max(1);
        self
    }

    /// Sets the number of rows occupied by this cell.
    pub fn row_span(mut self, span: usize) -> Self {
        self.row_span = span.max(1);
        self
    }

    /// Sets the content alignment inside this cell.
    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Sets a stable element identifier for interaction and persisted state.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the selector exposed by GPUI visual tests and the inspector.
    pub fn debug_selector(mut self, selector: impl Into<SharedString>) -> Self {
        self.debug_selector = Some(selector.into());
        self
    }

    /// Sets the accessible text label for this cell.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }
}

impl Default for TableGridCell {
    fn default() -> Self {
        Self::new()
    }
}

/// A logical row retained by [`TableGrid`] until all cells can be placed into
/// one shared grid.
pub struct TableGridRow {
    cells: Vec<TableGridCell>,
}

impl TableGridRow {
    /// Creates an empty logical row.
    pub fn new() -> Self {
        Self { cells: Vec::new() }
    }

    /// Appends one logical cell to this row.
    pub fn child(mut self, cell: TableGridCell) -> Self {
        self.cells.push(cell);
        self
    }

    /// Appends logical cells to this row in source order.
    pub fn children(mut self, cells: impl IntoIterator<Item = TableGridCell>) -> Self {
        self.cells.extend(cells);
        self
    }
}

impl Default for TableGridRow {
    fn default() -> Self {
        Self::new()
    }
}

/// A stateless table module with shared column tracks and span-aware layout.
///
/// The caller renders cell content and provides each cell's intrinsic width;
/// this module owns all table geometry, scrolling, and table accessibility.
#[derive(IntoElement)]
pub struct TableGrid {
    id: ElementId,
    ix: usize,
    column_count: usize,
    rows: Vec<TableGridRow>,
    style: StyleRefinement,
    size: Size,
    cell_padding: Option<Edges<Pixels>>,
    min_column_width: Pixels,
    sizing: TableGridSizing,
    border_color: Option<Hsla>,
    header_background: Option<Hsla>,
    stripe_background: Option<Hsla>,
    striped: bool,
    scroll_handle: ScrollHandle,
    debug_selector: Option<SharedString>,
}

impl TableGrid {
    /// Creates a table with a caller-owned persistent horizontal scroll handle.
    pub fn new(
        id: impl Into<ElementId>,
        column_count: usize,
        scroll_handle: &ScrollHandle,
    ) -> Self {
        Self {
            id: id.into(),
            ix: 0,
            column_count: column_count.min(MAX_GRID_TRACKS),
            rows: Vec::new(),
            style: StyleRefinement::default(),
            size: Size::default(),
            cell_padding: None,
            min_column_width: DEFAULT_MIN_COLUMN_WIDTH,
            sizing: TableGridSizing::default(),
            border_color: None,
            header_background: None,
            stripe_background: None,
            striped: false,
            scroll_handle: scroll_handle.clone(),
            debug_selector: None,
        }
    }

    /// Appends one logical row to this table.
    pub fn child(mut self, row: TableGridRow) -> Self {
        self.rows.push(row);
        self
    }

    /// Appends logical rows to this table in source order.
    pub fn children(mut self, rows: impl IntoIterator<Item = TableGridRow>) -> Self {
        self.rows.extend(rows);
        self
    }

    /// Overrides the cell padding used for both measurement and rendering.
    pub fn cell_padding(mut self, padding: Edges<Pixels>) -> Self {
        self.cell_padding = Some(padding);
        self
    }

    /// Sets the minimum outer width of every logical column.
    pub fn min_column_width(mut self, width: Pixels) -> Self {
        self.min_column_width = width.max(Pixels::ZERO);
        self
    }

    /// Selects the table track sizing policy.
    pub fn sizing(mut self, sizing: TableGridSizing) -> Self {
        self.sizing = sizing;
        self
    }

    /// Sets the frame and internal separator color.
    pub fn border_color(mut self, color: Hsla) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Sets the background used by header cells.
    pub fn header_background(mut self, color: Hsla) -> Self {
        self.header_background = Some(color);
        self
    }

    /// Enables alternating body rows with the supplied background color.
    pub fn stripe_background(mut self, color: Hsla) -> Self {
        self.striped = true;
        self.stripe_background = Some(color);
        self
    }

    /// Sets the selector exposed by GPUI visual tests and the inspector.
    pub fn debug_selector(mut self, selector: impl Into<SharedString>) -> Self {
        self.debug_selector = Some(selector.into());
        self
    }
}

impl Styled for TableGrid {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for TableGrid {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ChildElement for TableGrid {
    fn with_ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self
    }
}

/// Returns the next unoccupied logical column in a row.
fn next_column(occupied: &[Vec<bool>], row: usize, mut column: usize) -> usize {
    while column < occupied[row].len() && occupied[row][column] {
        column += 1;
    }
    column
}

/// Marks every logical slot covered by one span.
fn occupy(occupied: &mut [Vec<bool>], row: usize, column: usize, row_span: usize, col_span: usize) {
    for row_offset in 0..row_span {
        for column_offset in 0..col_span {
            occupied[row + row_offset][column + column_offset] = true;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CellPlacement {
    cell_index: usize,
    row: usize,
    column: usize,
    row_span: usize,
    col_span: usize,
}

/// Resolves every cell to an explicit, non-overlapping logical grid position.
fn resolve_placements(
    rows: &[TableGridRow],
    row_count: usize,
    column_count: usize,
) -> Vec<Vec<CellPlacement>> {
    let mut occupied = vec![vec![false; column_count]; row_count];
    let mut placements = vec![Vec::new(); row_count];
    for (row_index, row) in rows.iter().take(row_count).enumerate() {
        let mut column_index = 0;
        for (cell_index, cell) in row.cells.iter().enumerate() {
            column_index = next_column(&occupied, row_index, column_index);
            if column_index >= column_count {
                break;
            }
            let col_span = cell.col_span.min(column_count - column_index).max(1);
            let row_span = cell.row_span.min(row_count - row_index).max(1);
            occupy(&mut occupied, row_index, column_index, row_span, col_span);
            placements[row_index].push(CellPlacement {
                cell_index,
                row: row_index,
                column: column_index,
                row_span,
                col_span,
            });
            column_index += col_span;
        }
    }
    placements
}

/// Computes the authoritative outer width of every logical column.
fn column_widths(
    rows: &[TableGridRow],
    placements: &[Vec<CellPlacement>],
    column_count: usize,
    padding: Edges<Pixels>,
    min_column_width: Pixels,
) -> Vec<Pixels> {
    let mut widths = vec![min_column_width; column_count];
    for (row, row_placements) in rows.iter().zip(placements) {
        for placement in row_placements {
            let cell = &row.cells[placement.cell_index];
            let leading_border = if placement.column > 0 {
                CELL_BORDER_WIDTH
            } else {
                Pixels::ZERO
            };
            let required = cell.intrinsic_width + padding.left + padding.right + leading_border;
            let current = widths[placement.column..placement.column + placement.col_span]
                .iter()
                .copied()
                .sum::<Pixels>();
            if required > current {
                let addition = (required - current) / placement.col_span as f32;
                for width in &mut widths[placement.column..placement.column + placement.col_span] {
                    *width += addition;
                }
            }
        }
    }
    widths
}

struct CellLayout {
    row: usize,
    column: usize,
    row_span: usize,
    col_span: usize,
    track_width: Pixels,
}

/// Resolves one cell's span and border-box width from the shared tracks.
fn cell_layout(
    row: usize,
    column: usize,
    row_span: usize,
    col_span: usize,
    widths: &[Pixels],
) -> CellLayout {
    let track_width = widths[column..column + col_span]
        .iter()
        .copied()
        .sum::<Pixels>();
    CellLayout {
        row,
        column,
        row_span,
        col_span,
        track_width,
    }
}

struct CellVisuals {
    id: ElementId,
    debug_selector: Option<SharedString>,
    aria_label: Option<SharedString>,
    header: bool,
    align: TextAlign,
    children: Vec<AnyElement>,
}

#[derive(Clone, Copy)]
struct CellRenderStyle {
    padding: Edges<Pixels>,
    border_color: Hsla,
    header_background: Hsla,
    stripe_background: Hsla,
    striped: bool,
    sizing: TableGridSizing,
}

/// Builds one positioned visual and semantic cell.
fn render_cell(
    layout: CellLayout,
    visuals: CellVisuals,
    style: CellRenderStyle,
) -> impl IntoElement {
    let row_span = layout.row_span;
    let col_span = layout.col_span;
    div()
        .id(visuals.id)
        .when_some(visuals.debug_selector, |cell, selector| {
            cell.debug_selector(move || selector.to_string())
        })
        .role(if visuals.header {
            Role::ColumnHeader
        } else {
            Role::Cell
        })
        .aria_row_index(layout.row + 1)
        .aria_column_index(layout.column + 1)
        .when_some(visuals.aria_label, |cell, label| cell.aria_label(label))
        .a11y_synthetic_children(move |builder| {
            builder.parent_node().set_row_span(row_span);
            builder.parent_node().set_column_span(col_span);
        })
        .col_span(layout.col_span as u16)
        .row_span(layout.row_span as u16)
        // GPUI's span builders replace the complete placement range, so the
        // explicit start lines must be applied after the spans.
        .col_start((layout.column + 1) as i16)
        .row_start((layout.row + 1) as i16)
        .flex()
        .flex_col()
        .when(style.sizing == TableGridSizing::MaxContent, |cell| {
            // GPUI maps to Taffy's BorderBox sizing, so this width already
            // includes the cell padding and internal separator border.
            cell.w(layout.track_width)
                .min_w(layout.track_width)
                .max_w(layout.track_width)
        })
        .h_full()
        .px(style.padding.left)
        .py(style.padding.top)
        .when(layout.column > 0, |cell| cell.border_l(CELL_BORDER_WIDTH))
        .when(layout.row > 0, |cell| cell.border_t(CELL_BORDER_WIDTH))
        .border_color(style.border_color)
        .when(visuals.header, |cell| cell.bg(style.header_background))
        .when(
            !visuals.header && style.striped && layout.row % 2 == 1,
            |cell| cell.bg(style.stripe_background),
        )
        .when(visuals.align == TextAlign::Center, |cell| {
            cell.items_center()
        })
        .when(visuals.align == TextAlign::Right, |cell| cell.items_end())
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .w_full()
                .justify_center()
                .text_align(visuals.align)
                .children(visuals.children),
        )
}

impl RenderOnce for TableGrid {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.column_count == 0 || self.rows.is_empty() {
            return div().into_any_element();
        }

        let padding = self
            .cell_padding
            .unwrap_or_else(|| self.size.table_cell_padding(cx));
        let border_color = self.border_color.unwrap_or(cx.theme().border);
        let header_background = self.header_background.unwrap_or(cx.theme().table_head);
        let stripe_background = self.stripe_background.unwrap_or(cx.theme().table_even);
        let cell_render_style = CellRenderStyle {
            padding,
            border_color,
            header_background,
            stripe_background,
            striped: self.striped,
            sizing: self.sizing,
        };
        let row_count = self.rows.len().min(i16::MAX as usize);
        let placements = resolve_placements(&self.rows, row_count, self.column_count);
        let widths = column_widths(
            &self.rows,
            &placements,
            self.column_count,
            padding,
            self.min_column_width,
        );
        let table_width = widths.iter().copied().sum::<Pixels>() + FRAME_BORDER_WIDTH * 2.;
        let mut cells = Vec::new();
        let generated_id_prefix = self.id.clone();

        for (row, row_placements) in self.rows.into_iter().zip(placements) {
            let mut row_cells = row.cells.into_iter().map(Some).collect::<Vec<_>>();
            for placement in row_placements {
                let cell = row_cells[placement.cell_index]
                    .take()
                    .expect("resolved table cell must exist");
                let layout = cell_layout(
                    placement.row,
                    placement.column,
                    placement.row_span,
                    placement.col_span,
                    &widths,
                );
                let id = cell.id.unwrap_or_else(|| {
                    (
                        generated_id_prefix.clone(),
                        format!("cell-{}-{}-{}", self.ix, placement.row, placement.column),
                    )
                        .into()
                });
                cells.push(
                    render_cell(
                        layout,
                        CellVisuals {
                            id,
                            debug_selector: cell.debug_selector,
                            aria_label: cell.aria_label,
                            header: cell.header,
                            align: cell.align,
                            children: cell.children,
                        },
                        cell_render_style,
                    )
                    .into_any_element(),
                );
            }
        }

        let scroll_handle = self.scroll_handle;
        let table_id = self.id;
        let mut grid = div()
            .id(table_id.clone())
            .when_some(self.debug_selector, |grid, selector| {
                grid.debug_selector(move || selector.to_string())
            })
            .role(Role::Table)
            .aria_row_count(row_count)
            .aria_column_count(self.column_count)
            .min_w_0()
            .flex_none()
            .grid()
            .when(self.sizing == TableGridSizing::MaxContent, |grid| {
                // Taffy 0.12 redistributes explicit repeated max-content
                // tracks independently from definite cell widths. Implicit
                // tracks preserve the exact border-box widths computed above.
                grid.w(table_width)
            })
            .when(self.sizing == TableGridSizing::MinContent, |grid| {
                grid.w_full()
                    .grid_cols_min_content(self.column_count as u16)
            })
            .border(FRAME_BORDER_WIDTH)
            .border_color(border_color)
            .rounded_sm()
            .children(cells);
        grid.style().refine(&self.style);

        let viewport_style = StyleRefinement::default();
        let viewport = horizontal_scroll_area(
            (table_id, "viewport"),
            &scroll_handle,
            &viewport_style,
            grid,
        );
        div()
            .w_full()
            .min_w_0()
            .child(viewport)
            .horizontal_scrollbar(&scroll_handle)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_are_never_zero() {
        let cell = TableGridCell::new().col_span(0).row_span(0);
        assert_eq!(cell.col_span, 1);
        assert_eq!(cell.row_span, 1);
    }

    #[test]
    fn column_widths_share_span_deficits() {
        let rows = vec![
            TableGridRow::new()
                .child(TableGridCell::new().intrinsic_width(px(40.)))
                .child(TableGridCell::new().intrinsic_width(px(60.))),
            TableGridRow::new().child(TableGridCell::new().intrinsic_width(px(140.)).col_span(2)),
        ];
        let placements = resolve_placements(&rows, rows.len(), 2);
        let widths = column_widths(&rows, &placements, 2, Edges::all(px(0.)), px(0.));
        assert_eq!(widths, vec![px(59.5), px(80.5)]);
    }

    #[test]
    fn cell_layout_uses_the_complete_border_box_track_width() {
        let layout = cell_layout(0, 0, 1, 2, &[px(60.), px(80.)]);
        assert_eq!(layout.track_width, px(140.));
    }

    #[test]
    fn rowspan_reserves_the_same_column_in_following_rows() {
        let rows = vec![
            TableGridRow::new()
                .child(TableGridCell::new().row_span(2))
                .child(TableGridCell::new()),
            TableGridRow::new().child(TableGridCell::new()),
        ];
        let placements = resolve_placements(&rows, rows.len(), 2);
        assert_eq!(placements[0][0].column, 0);
        assert_eq!(placements[0][1].column, 1);
        assert_eq!(placements[1][0].column, 1);
    }

    #[test]
    fn spans_are_clamped_to_remaining_grid_space() {
        let rows = vec![
            TableGridRow::new().child(
                TableGridCell::new()
                    .col_span(usize::MAX)
                    .row_span(usize::MAX),
            ),
        ];
        let placements = resolve_placements(&rows, rows.len(), 3);
        assert_eq!(placements[0][0].col_span, 3);
        assert_eq!(placements[0][0].row_span, 1);
    }
}
