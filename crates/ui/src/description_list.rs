use gpui::{
    AnyElement, App, Axis, DefiniteLength, ElementId, InteractiveElement as _, IntoElement,
    ParentElement, Pixels, RenderOnce, Role, SharedString, Stateful,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative,
};

use crate::{
    ActiveTheme as _, AxisExt, Sizable, Size, StylePreset, StyleSized as _, StyledExt as _, h_flex,
    text::Text, v_flex,
};

/// DescriptionList geometry resolved from shared data-surface metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
struct DescriptionListMetrics {
    padding_x: Pixels,
    padding_y: Pixels,
    gap: Pixels,
    separator_height: Pixels,
    radius: Pixels,
}

/// Radius refinements for surfaces that paint against the outer DescriptionList edge.
struct DescriptionListEdgeStyles {
    top_left: StyleRefinement,
    top_right: StyleRefinement,
    bottom_left: StyleRefinement,
    top: StyleRefinement,
    bottom: StyleRefinement,
}

/// Resolves data density without branching on a built-in Style Preset identity.
fn description_list_metrics(size: Size, style: &StylePreset) -> DescriptionListMetrics {
    let index = match size {
        Size::XSmall => 0,
        Size::Small => 1,
        Size::Medium | Size::Size(_) => 2,
        Size::Large => 3,
    };
    let padding_x = style.data.cell_padding_x[index];
    let padding_y = style.data.cell_padding_y[index];

    DescriptionListMetrics {
        padding_x,
        padding_y,
        gap: padding_y,
        separator_height: padding_y * 2.,
        radius: style.radii.md,
    }
}

/// Mirrors the resolved outer radius onto colored edge children because GPUI's
/// overflow mask does not clip descendants to a rounded content shape.
fn description_list_edge_styles(
    metrics: DescriptionListMetrics,
    root_style: &StyleRefinement,
) -> DescriptionListEdgeStyles {
    let top_left = root_style
        .corner_radii
        .top_left
        .or_else(|| Some(metrics.radius.into()));
    let top_right = root_style
        .corner_radii
        .top_right
        .or_else(|| Some(metrics.radius.into()));
    let bottom_right = root_style
        .corner_radii
        .bottom_right
        .or_else(|| Some(metrics.radius.into()));
    let bottom_left = root_style
        .corner_radii
        .bottom_left
        .or_else(|| Some(metrics.radius.into()));

    let mut top_left_style = StyleRefinement::default();
    top_left_style.corner_radii.top_left = top_left;
    let mut top_right_style = StyleRefinement::default();
    top_right_style.corner_radii.top_right = top_right;
    let mut bottom_left_style = StyleRefinement::default();
    bottom_left_style.corner_radii.bottom_left = bottom_left;
    let mut top_style = StyleRefinement::default();
    top_style.corner_radii.top_left = top_left;
    top_style.corner_radii.top_right = top_right;
    let mut bottom_style = StyleRefinement::default();
    bottom_style.corner_radii.bottom_right = bottom_right;
    bottom_style.corner_radii.bottom_left = bottom_left;

    DescriptionListEdgeStyles {
        top_left: top_left_style,
        top_right: top_right_style,
        bottom_left: bottom_left_style,
        top: top_style,
        bottom: bottom_style,
    }
}

/// A description list.
#[derive(IntoElement)]
pub struct DescriptionList {
    id: Option<ElementId>,
    style: StyleRefinement,
    items: Vec<DescriptionItem>,
    size: Size,
    layout: Axis,
    label_width: DefiniteLength,
    bordered: bool,
    columns: usize,
}

/// Item for the [`DescriptionList`].
pub enum DescriptionItem {
    Item {
        label: DescriptionText,
        value: DescriptionText,
        span: usize,
    },
    Separator,
}

/// Text for the label or value in the [`DescriptionList`].
#[derive(IntoElement)]
pub enum DescriptionText {
    String(SharedString),
    Text(Text),
    AnyElement(AnyElement),
}

impl DescriptionText {
    /// Returns text that can name a semantic Term or Definition node.
    fn accessibility_label(&self) -> Option<SharedString> {
        match self {
            Self::String(text) => Some(text.clone()),
            Self::Text(_) | Self::AnyElement(_) => None,
        }
    }
}

impl From<&str> for DescriptionText {
    fn from(text: &str) -> Self {
        DescriptionText::String(SharedString::from(text.to_string()))
    }
}

impl From<Text> for DescriptionText {
    fn from(text: Text) -> Self {
        DescriptionText::Text(text)
    }
}

impl From<AnyElement> for DescriptionText {
    fn from(element: AnyElement) -> Self {
        DescriptionText::AnyElement(element)
    }
}

impl From<SharedString> for DescriptionText {
    fn from(text: SharedString) -> Self {
        DescriptionText::String(text)
    }
}

impl From<String> for DescriptionText {
    fn from(text: String) -> Self {
        DescriptionText::String(SharedString::from(text))
    }
}

impl RenderOnce for DescriptionText {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        match self {
            DescriptionText::String(text) => div().child(text).into_any_element(),
            DescriptionText::Text(text) => text.into_any_element(),
            DescriptionText::AnyElement(element) => element,
        }
    }
}

impl DescriptionItem {
    /// Create a new description item, with a label.
    ///
    /// The value is an empty element.
    pub fn new(label: impl Into<DescriptionText>) -> Self {
        DescriptionItem::Item {
            label: label.into(),
            value: "".into(),
            span: 1,
        }
    }

    /// Set the element value of the item.
    pub fn value(mut self, value: impl Into<DescriptionText>) -> Self {
        let new_value = value.into();
        if let DescriptionItem::Item { value, .. } = &mut self {
            *value = new_value;
        }
        self
    }

    /// Set the span of the item.
    ///
    /// This method only works for [`DescriptionItem::Item`].
    pub fn span(mut self, span: usize) -> Self {
        let val = span;
        if let DescriptionItem::Item { span, .. } = &mut self {
            *span = val;
        }
        self
    }
}

impl DescriptionList {
    /// Create a new description list with the default layout (Horizontal).
    pub fn new() -> Self {
        Self {
            id: None,
            style: StyleRefinement::default(),
            items: Vec::new(),
            layout: Axis::Horizontal,
            label_width: px(120.).into(),
            size: Size::default(),
            bordered: true,
            columns: 3,
        }
    }

    /// Sets a stable identity and enables DescriptionList accessibility semantics.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Create a vertical description list.
    pub fn vertical() -> Self {
        Self::new().layout(Axis::Vertical)
    }

    /// Create a horizontal description list, the default.
    pub fn horizontal() -> Self {
        Self::new().layout(Axis::Horizontal)
    }

    /// Set the width of the label, only works for horizontal layout.
    ///
    /// Default is `120px`.
    pub fn label_width(mut self, label_width: impl Into<DefiniteLength>) -> Self {
        self.label_width = label_width.into();
        self
    }

    /// Set the layout of the description list.
    pub fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    /// Set the border of the description list, default is `true`.
    /// Both horizontal and vertical layouts support the bordered presentation.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set the number of columns in the description list, default is `3`.
    ///
    /// A value between `1` and `10` is allowed.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.clamp(1, 10);
        self
    }

    /// Add a [`DescriptionItem::Item`] to the list.
    pub fn item(
        mut self,
        label: impl Into<DescriptionText>,
        value: impl Into<DescriptionText>,
        span: usize,
    ) -> Self {
        self.items.push(DescriptionItem::Item {
            label: label.into(),
            value: value.into(),
            span,
        });
        self
    }

    /// Add a child to the list.
    pub fn child(mut self, child: impl Into<DescriptionItem>) -> Self {
        self.items.push(child.into());
        self
    }

    /// Add children to the list.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = impl Into<DescriptionItem>>,
    ) -> Self {
        self.items
            .extend(children.into_iter().map(Into::into).collect::<Vec<_>>());
        self
    }

    /// Add a separator to the list.
    pub fn separator(mut self) -> Self {
        self.items.push(DescriptionItem::Separator);
        self
    }

    fn group_item_rows(items: Vec<DescriptionItem>, columns: usize) -> Vec<Vec<DescriptionItem>> {
        let mut rows = vec![];
        let mut current_span = 0;
        for mut item in items {
            let span = match &mut item {
                DescriptionItem::Item { span, .. } => {
                    *span = (*span).clamp(1, columns);
                    *span
                }
                DescriptionItem::Separator => columns,
            };
            if rows.is_empty() {
                rows.push(vec![]);
            }
            if current_span + span > columns {
                rows.push(vec![]);
                current_span = 0;
            }
            let last_group = rows.last_mut().unwrap();
            last_group.push(item);
            current_span += span;
        }
        // Remove last empty rows if it exists
        while let Some(last_group) = rows.last() {
            if !last_group.is_empty() {
                break;
            }

            rows.pop();
        }

        rows
    }
}

impl Default for DescriptionList {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for DescriptionList {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for DescriptionList {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

/// Exposes a visual container only when stable semantic identity is available.
fn description_semantic_container(
    element: gpui::Div,
    id: ElementId,
    role: Role,
    label: Option<SharedString>,
) -> Stateful<gpui::Div> {
    element
        .id(id)
        .role(role)
        .when_some(label, |this, label| this.aria_label(label))
}

/// Keeps anonymous instances visual-only so multiple lists cannot collide in the accessibility tree.
fn description_semantic_element(
    element: gpui::Div,
    semantics: Option<(ElementId, Role, Option<SharedString>)>,
) -> AnyElement {
    match semantics {
        Some((id, role, label)) => {
            description_semantic_container(element, id, role, label).into_any_element()
        }
        None => element.into_any_element(),
    }
}

impl RenderOnce for DescriptionList {
    fn render(self, _: &mut Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let Self {
            id,
            style,
            items,
            size,
            layout,
            label_width,
            bordered,
            columns,
        } = self;
        let metrics = description_list_metrics(size, &cx.theme().style);
        let edge_styles = description_list_edge_styles(metrics, &style);
        let padding_x = if bordered { metrics.padding_x } else { px(0.) };
        let padding_y = if bordered { metrics.padding_y } else { px(0.) };
        let gap = if bordered { px(0.) } else { metrics.gap };
        let label_width = layout.is_horizontal().then_some(label_width);
        let root_id = id.clone();

        // Rows are packed after spans have been normalized to the configured grid.
        let rows = Self::group_item_rows(items, columns);
        let rows_len = rows.len();
        let root = v_flex()
            .w_full()
            .gap(gap)
            .overflow_hidden()
            .input_text_size(size)
            .when(bordered, |this| {
                this.rounded(metrics.radius)
                    .border_1()
                    .border_color(cx.theme().border)
            })
            .refine_style(&style)
            .children(rows.into_iter().enumerate().map(|(ix, items)| {
                let is_first = ix == 0;
                let is_last = ix == rows_len - 1;
                let mut consumed_columns = 0;
                h_flex()
                    .items_stretch()
                    .when(bordered && !is_last, |this| {
                        this.border_b_1().border_color(cx.theme().border)
                    })
                    .children(items.into_iter().enumerate().map(|(item_ix, item)| {
                        let is_first_col = item_ix == 0;

                        match item {
                            DescriptionItem::Item { label, value, span } => {
                                consumed_columns += span;
                                let reaches_last_column = consumed_columns == columns;
                                let label_a11y = label.accessibility_label();
                                let value_a11y = value.accessibility_label();
                                let label_semantics = root_id.clone().map(|id| {
                                    (
                                        (id, format!("term-{ix}-{item_ix}")).into(),
                                        Role::Term,
                                        label_a11y,
                                    )
                                });
                                let value_semantics = root_id.clone().map(|id| {
                                    (
                                        (id, format!("definition-{ix}-{item_ix}")).into(),
                                        Role::Definition,
                                        value_a11y,
                                    )
                                });
                                let item = if layout.is_vertical() {
                                    v_flex()
                                } else {
                                    div().flex().flex_row().h_full()
                                };
                                let label = div()
                                    .min_w_0()
                                    .when(layout.is_horizontal(), |this| this.h_full())
                                    .text_color(cx.theme().description_list_label_foreground)
                                    .px(padding_x)
                                    .py(padding_y)
                                    .when(bordered, |this| {
                                        this.when(layout.is_horizontal(), |this| {
                                            this.border_r_1()
                                                .when(!is_first_col, |this| this.border_l_1())
                                        })
                                        .when(layout.is_vertical(), |this| this.border_b_1())
                                        .border_color(cx.theme().border)
                                        .bg(cx.theme().tokens.description_list_label)
                                        .when(is_first && is_first_col, |this| {
                                            this.refine_style(&edge_styles.top_left)
                                        })
                                        .when(
                                            is_first && layout.is_vertical() && reaches_last_column,
                                            |this| this.refine_style(&edge_styles.top_right),
                                        )
                                        .when(
                                            is_last && layout.is_horizontal() && is_first_col,
                                            |this| this.refine_style(&edge_styles.bottom_left),
                                        )
                                    })
                                    .map(|this| match label_width {
                                        Some(label_width) => this.w(label_width).flex_shrink_0(),
                                        None => this,
                                    })
                                    .child(label);
                                let value = div()
                                    .flex_1()
                                    .min_w_0()
                                    .px(padding_x)
                                    .py(padding_y)
                                    .overflow_x_hidden()
                                    .child(value);

                                item.flex_grow_0()
                                    .flex_shrink_1()
                                    .flex_basis(relative((span as f32) / (columns as f32)))
                                    .min_w_0()
                                    .overflow_x_hidden()
                                    .child(description_semantic_element(label, label_semantics))
                                    .child(description_semantic_element(value, value_semantics))
                            }
                            DescriptionItem::Separator => div()
                                .h(metrics.separator_height)
                                .w_full()
                                .when(bordered, |this| {
                                    this.bg(cx.theme().tokens.description_list_label)
                                        .when(is_first, |this| this.refine_style(&edge_styles.top))
                                        .when(is_last, |this| {
                                            this.refine_style(&edge_styles.bottom)
                                        })
                                }),
                        }
                    }))
            }));

        description_semantic_element(root, id.map(|id| (id, Role::DescriptionList, None)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Element as _, accesskit, px};

    fn item_span(item: &DescriptionItem) -> Option<usize> {
        match item {
            DescriptionItem::Item { span, .. } => Some(*span),
            DescriptionItem::Separator => None,
        }
    }

    #[test]
    fn test_group_item_rows() {
        let items = vec![
            DescriptionItem::new("test1"),
            DescriptionItem::new("test2").span(2),
            DescriptionItem::new("test3"),
            DescriptionItem::new("test4"),
            DescriptionItem::new("test5"),
            DescriptionItem::new("test6").span(3),
            DescriptionItem::new("test7"),
        ];
        let rows = super::DescriptionList::group_item_rows(items, 3);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].len(), 2);
        assert_eq!(rows[1].len(), 3);
        assert_eq!(rows[2].len(), 1);
        assert_eq!(rows[3].len(), 1);
    }

    #[test]
    fn spans_are_normalized_and_separators_own_their_rows() {
        let rows = DescriptionList::group_item_rows(
            vec![
                DescriptionItem::new("zero").span(0),
                DescriptionItem::new("oversized").span(9),
                DescriptionItem::Separator,
                DescriptionItem::Separator,
                DescriptionItem::new("tail"),
            ],
            3,
        );

        assert_eq!(rows.len(), 5);
        assert_eq!(item_span(&rows[0][0]), Some(1));
        assert_eq!(item_span(&rows[1][0]), Some(3));
        assert!(matches!(rows[2][0], DescriptionItem::Separator));
        assert!(matches!(rows[3][0], DescriptionItem::Separator));
        assert_eq!(item_span(&rows[4][0]), Some(1));
    }

    #[test]
    fn metrics_follow_shared_data_density() {
        let vega = description_list_metrics(Size::Medium, &StylePreset::vega());
        let nova = description_list_metrics(Size::Medium, &StylePreset::nova());
        let maia = description_list_metrics(Size::Medium, &StylePreset::maia());

        assert_eq!(vega.padding_x, px(8.));
        assert_eq!(vega.padding_y, px(4.));
        assert_eq!(vega.separator_height, px(8.));
        assert_eq!(vega.radius, StylePreset::vega().radii.md);
        assert!(nova.padding_y < vega.padding_y);
        assert!(maia.padding_x > vega.padding_x);
        assert!(maia.radius > vega.radius);
    }

    #[test]
    fn stable_ids_expose_description_roles_and_string_labels() {
        let term = description_semantic_container(
            div(),
            "description-term".into(),
            Role::Term,
            Some("Name".into()),
        );
        let mut node = accesskit::Node::new(Role::Term);
        term.write_a11y_info(&mut node);

        assert_eq!(term.a11y_role(), Some(Role::Term));
        assert_eq!(node.label(), Some("Name"));
        assert_eq!(
            DescriptionText::from("Version").accessibility_label(),
            Some("Version".into())
        );
    }
}
