// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added or exposed behavior through `select_style`.
// - Added Select-specific item geometry, colors, check placement, truncation, and disabled opacity.
// - Migrated regular item rounding and sizing to semantic Style Preset metrics.
use gpui::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::{
    ActiveTheme, Disableable, Icon, IconName, Selectable, Sizable, Size, StyleSized, StyledExt,
    h_flex,
};

/// A single row element used inside searchable-list dropdowns (Select, ComboBox, MultiComboBox).
///
/// - `selected` — controls the cursor-highlight background (the `List` overwrites this field via
///   `Selectable::selected` to match the keyboard cursor position).
/// - `checked` — controls the visibility of the trailing check icon; set by the adapter based on
///   the current selection state and NOT overwritten by the `List`.
#[derive(IntoElement)]
pub struct SearchableListItemElement {
    id: ElementId,
    size: Size,
    style: StyleRefinement,
    /// Cursor/highlight background (overridden by `List` to the keyboard cursor row).
    selected: bool,
    /// Whether the trailing check icon is shown.
    checked: bool,
    disabled: bool,
    select_style: bool,
    children: Vec<AnyElement>,
    /// The icon drawn at the trailing edge when `checked` is `true`.
    check_icon: Option<Icon>,
}

impl SearchableListItemElement {
    pub fn new(ix: usize) -> Self {
        Self {
            id: ("searchable-list-item", ix).into(),
            size: Size::default(),
            style: StyleRefinement::default(),
            selected: false,
            checked: false,
            disabled: false,
            select_style: false,
            children: Vec::new(),
            check_icon: Some(Icon::new(IconName::Check)),
        }
    }

    /// Set whether the trailing check icon is visible.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Override the default check icon.
    pub fn check_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.check_icon = Some(icon.into());
        self
    }

    /// Apply Select-specific item geometry while retaining the shared item protocol.
    pub(crate) fn select_style(mut self, select_style: bool) -> Self {
        self.select_style = select_style;
        self
    }
}

impl ParentElement for SearchableListItemElement {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Disableable for SearchableListItemElement {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for SearchableListItemElement {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Sizable for SearchableListItemElement {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for SearchableListItemElement {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for SearchableListItemElement {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = crate::select::SelectMetrics::resolve(Size::Medium, cx);
        let item = h_flex()
            .id(self.id)
            .relative()
            .w_full()
            .overflow_hidden()
            .text_color(if self.select_style {
                cx.theme().tokens.popover_foreground.color
            } else {
                cx.theme().foreground
            })
            .items_center()
            .input_text_size(self.size)
            .when(self.select_style, |this| {
                this.min_h(metrics.item_height)
                    .pl(metrics.item_padding_left)
                    .pr(px(32.))
                    .py(metrics.item_padding_y)
                    .rounded(metrics.item_radius)
            })
            .when(!self.select_style, |this| {
                this.gap_x_1()
                    .rounded(cx.theme().style.radii.md)
                    .list_size(self.size, cx)
            })
            .refine_style(&self.style)
            .when(
                !self.disabled && !self.selected && self.select_style,
                |this| {
                    this.hover(|this| {
                        this.bg(cx.theme().tokens.accent)
                            .text_color(cx.theme().accent_foreground)
                    })
                },
            )
            .when(
                !self.disabled && !self.selected && !self.select_style,
                |this| this.hover(|this| this.bg(cx.theme().accent.opacity(0.7))),
            )
            .when(self.selected, |this| {
                this.bg(cx.theme().tokens.accent)
                    .when(self.select_style, |this| {
                        this.text_color(cx.theme().accent_foreground)
                    })
            })
            .when(self.disabled, |this| this.cursor_not_allowed().opacity(0.5))
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .items_center()
                    .gap(metrics.item_gap)
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .items_center()
                            .gap(metrics.item_gap)
                            .children(self.children),
                    )
                    .when_some(
                        self.check_icon.clone().filter(|_| !self.select_style),
                        |this, icon| {
                            this.child(
                                icon.xsmall()
                                    .text_color(cx.theme().foreground)
                                    .when(!self.checked, |this| this.invisible()),
                            )
                        },
                    ),
            );

        item.when(self.select_style, |this| {
            this.when_some(self.check_icon, |this, icon| {
                this.child(
                    div()
                        .absolute()
                        .right(px(8.))
                        .top_0()
                        .bottom_0()
                        .w(px(16.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            icon.size(px(16.))
                                .text_color(if self.selected {
                                    cx.theme().accent_foreground
                                } else {
                                    cx.theme().foreground
                                })
                                .when(!self.checked, |this| this.invisible()),
                        ),
                )
            })
        })
    }
}
