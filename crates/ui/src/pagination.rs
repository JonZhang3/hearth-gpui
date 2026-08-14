// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `aria_label`.
// - Added or exposed behavior through `aria_label`, `normalize_current_page`, `pagination_gap`,
//   `page_range_respects_visible_limit_and_normalizes_current_page`,
//   `pagination_gap_matches_style_density`.
// - Reworked Pagination around accessibility semantics and ARIA state, semantic Style Preset
//   geometry and density.
use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, prelude::FluentBuilder, px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Disableable, Sizable, Size, StyledExt,
    accessibility::accessibility_state_with_current,
    button::Button,
    h_flex,
    icon::IconName,
    menu::{DropdownMenu as _, PopupMenuItem},
    theme::Density,
};

/// Pagination with page navigation, next and previous links.
#[derive(IntoElement)]
pub struct Pagination {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    current_page: usize,
    total_pages: usize,
    disabled: bool,
    compact: bool,
    visible_pages: usize,
    aria_label: Option<SharedString>,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum PageItem {
    Page(usize),
    Ellipsis(Range<usize>),
}

impl Pagination {
    /// Create a new Pagination component with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            size: Size::default(),
            current_page: 1,
            total_pages: 1,
            visible_pages: 5,
            disabled: false,
            compact: false,
            aria_label: None,
            on_click: None,
        }
    }

    /// Set the accessible name announced for the pagination navigation region.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set the current page number (1-based).
    ///
    /// The value will be clamped between 1 and total_pages when total_pages is set.
    pub fn current_page(mut self, page: usize) -> Self {
        self.current_page = page.max(1);
        self
    }

    /// Set the total number of pages.
    pub fn total_pages(mut self, pages: usize) -> Self {
        self.total_pages = pages.max(1);
        if self.current_page > self.total_pages {
            self.current_page = self.total_pages;
        }
        self
    }

    /// Set the handler for page change (when clicking on page numbers, prev, or next).
    ///
    /// This handler receives the new page number to navigate to.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// Pagination::new("my-pagination")
    ///     .current_page(current_page)
    ///     .total_pages(total_pages)
    ///     .on_click(|page, _, cx| {
    ///         // Handle page change
    ///     })
    /// ```
    pub fn on_click(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set to display as compact style.
    ///
    /// If true, only the prev, next buttons with only icon.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Set the maximum number of visible page and ellipsis items.
    ///
    /// Values below five are normalized to five so the first page, current
    /// range, last page, and ellipsis controls remain representable.
    pub fn visible_pages(mut self, max: usize) -> Self {
        self.visible_pages = max;
        self
    }

    fn render_nav_button(&self, is_prev: bool) -> Button {
        let (id, label, aria_label, icon, disabled) = if is_prev {
            (
                "prev",
                t!("Pagination.previous"),
                t!("Pagination.previous_aria"),
                IconName::ChevronLeft,
                self.current_page <= 1,
            )
        } else {
            (
                "next",
                t!("Pagination.next"),
                t!("Pagination.next_aria"),
                IconName::ChevronRight,
                self.current_page >= self.total_pages,
            )
        };

        let target_page = if is_prev {
            self.current_page.saturating_sub(1)
        } else {
            self.current_page.saturating_add(1)
        };

        Button::new(id)
            .ghost()
            .with_size(self.size)
            .disabled(self.disabled || disabled)
            .aria_label(aria_label)
            .when(self.compact, |this| this.tooltip(label.clone()))
            .when(self.compact, |this| this.icon(icon.clone()))
            .when(!self.compact && is_prev, |this| {
                this.icon(icon.clone()).label(label.clone())
            })
            .when(!self.compact && !is_prev, |this| {
                this.label(label.clone()).trailing_icon(icon.clone())
            })
            .when_some(self.on_click.clone(), |this, handler| {
                this.on_click(move |_, window, cx| {
                    handler(&target_page, window, cx);
                })
            })
    }
}

impl Disableable for Pagination {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Sizable for Pagination {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Pagination {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Pagination {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.current_page = normalize_current_page(self.current_page, self.total_pages);
        let page_numbers = if !self.compact {
            calculate_page_range(self.current_page, self.total_pages, self.visible_pages)
        } else {
            vec![]
        };

        let current_page = self.current_page;
        let is_disabled = self.disabled;
        let on_click = self.on_click.clone();
        let gap = pagination_gap(cx.theme().style.density);
        let page_size = cx.theme().style.controls.for_size(self.size).height;
        let aria_label = self
            .aria_label
            .clone()
            .unwrap_or_else(|| t!("Pagination.label").into());

        h_flex()
            .id(self.id.clone())
            .role(Role::Navigation)
            .aria_label(aria_label)
            .w_full()
            .justify_center()
            .gap(gap)
            .items_center()
            .refine_style(&self.style)
            .child(self.render_nav_button(true))
            .children({
                page_numbers.into_iter().map(|item| match item {
                    PageItem::Page(page) => {
                        let is_selected = page == current_page;

                        let button = Button::new(page)
                            .with_size(self.size)
                            .map(|this| {
                                if is_selected {
                                    // Pagination's active link uses the Outline border without
                                    // inheriting the standalone Button elevation.
                                    this.outline().shadow_none()
                                } else {
                                    this.ghost()
                                }
                            })
                            .label(page.to_string())
                            .size(page_size)
                            .disabled(is_disabled)
                            .when_some(on_click.clone(), |this, handler| {
                                this.on_click(move |_, window, cx| {
                                    handler(&page, window, cx);
                                })
                            });

                        if is_selected {
                            accessibility_state_with_current(
                                button,
                                false,
                                false,
                                is_disabled,
                                Some(gpui::accesskit::AriaCurrent::Page),
                            )
                            .into_any_element()
                        } else {
                            button.into_any_element()
                        }
                    }
                    PageItem::Ellipsis(range) => Button::new(SharedString::from(format!(
                        "ellipsis-{}-{}",
                        range.start, range.end
                    )))
                    .ghost()
                    .with_size(self.size)
                    .disabled(self.disabled)
                    .aria_label(t!("Pagination.more_pages"))
                    .tooltip(t!("Pagination.more_pages"))
                    .icon(IconName::Ellipsis)
                    .dropdown_menu({
                        let on_click = on_click.clone();
                        move |mut menu, _, _| {
                            for page in range.clone() {
                                menu = menu.item(
                                    PopupMenuItem::new(format!("{}", page))
                                        .checked(page == current_page)
                                        .on_click({
                                            let on_click = on_click.clone();
                                            move |_, window, cx| {
                                                if let Some(handler) = &on_click {
                                                    handler(&page, window, cx);
                                                }
                                            }
                                        }),
                                )
                            }

                            menu.min_w(px(55.)).max_h(px(240.)).scrollable(true)
                        }
                    })
                    .into_any_element(),
                })
            })
            .child(self.render_nav_button(false))
    }
}

fn normalize_current_page(current: usize, total: usize) -> usize {
    current.clamp(1, total.max(1))
}

fn pagination_gap(density: Density) -> gpui::Pixels {
    match density {
        Density::Compact => px(2.),
        Density::Standard | Density::Comfortable => px(4.),
    }
}

fn calculate_page_range(current: usize, total: usize, max_visible: usize) -> Vec<PageItem> {
    if total <= 1 {
        return vec![];
    }

    let max_visible = max_visible.max(5);

    if total <= max_visible {
        return (1..=total).map(PageItem::Page).collect();
    }

    let current = normalize_current_page(current, total);
    let edge_pages = max_visible - 2;
    let edge_threshold = edge_pages - 1;

    if current <= edge_threshold {
        return (1..=edge_pages)
            .map(PageItem::Page)
            .chain([
                PageItem::Ellipsis(edge_pages + 1..total),
                PageItem::Page(total),
            ])
            .collect();
    }

    if current > total - edge_threshold {
        return [
            PageItem::Page(1),
            PageItem::Ellipsis(2..total - edge_pages + 1),
        ]
        .into_iter()
        .chain((total - edge_pages + 1..=total).map(PageItem::Page))
        .collect();
    }

    let center_pages = max_visible - 4;
    let pages_before = (center_pages - 1) / 2;
    let pages_after = center_pages - pages_before - 1;
    let start = current - pages_before;
    let end = current + pages_after;

    [PageItem::Page(1), PageItem::Ellipsis(2..start)]
        .into_iter()
        .chain((start..=end).map(PageItem::Page))
        .chain([PageItem::Ellipsis(end + 1..total), PageItem::Page(total)])
        .collect()
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use crate::theme::Density;

    #[test]
    fn test_calculate_page_range() {
        use super::{PageItem, calculate_page_range};

        let result = calculate_page_range(1, 10, 7);
        let expected = vec![
            PageItem::Page(1),
            PageItem::Page(2),
            PageItem::Page(3),
            PageItem::Page(4),
            PageItem::Page(5),
            PageItem::Ellipsis(6..10),
            PageItem::Page(10),
        ];
        assert_eq!(result, expected);

        let result = calculate_page_range(5, 10, 7);
        let expected = vec![
            PageItem::Page(1),
            PageItem::Ellipsis(2..4),
            PageItem::Page(4),
            PageItem::Page(5),
            PageItem::Page(6),
            PageItem::Ellipsis(7..10),
            PageItem::Page(10),
        ];
        assert_eq!(result, expected);

        let result = calculate_page_range(10, 10, 7);
        let expected = vec![
            PageItem::Page(1),
            PageItem::Ellipsis(2..6),
            PageItem::Page(6),
            PageItem::Page(7),
            PageItem::Page(8),
            PageItem::Page(9),
            PageItem::Page(10),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn page_range_respects_visible_limit_and_normalizes_current_page() {
        use super::{PageItem, calculate_page_range, normalize_current_page};

        for current in 0..=12 {
            let result = calculate_page_range(current, 10, 5);
            assert!(result.len() <= 5);
            assert_eq!(result.first(), Some(&PageItem::Page(1)));
            assert_eq!(result.last(), Some(&PageItem::Page(10)));
        }

        assert_eq!(normalize_current_page(0, 10), 1);
        assert_eq!(normalize_current_page(12, 10), 10);
        assert_eq!(normalize_current_page(3, 0), 1);
    }

    #[test]
    fn pagination_gap_matches_style_density() {
        use super::pagination_gap;

        assert_eq!(pagination_gap(Density::Compact), px(2.));
        assert_eq!(pagination_gap(Density::Standard), px(4.));
        assert_eq!(pagination_gap(Density::Comfortable), px(4.));
    }
}
