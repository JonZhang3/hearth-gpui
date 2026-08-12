use std::ops::Range;

use crate::{
    ActiveTheme, Density, IconName, Sizable, Size, StyledExt,
    group_box::GroupBoxVariant,
    input::{Input, InputEvent, InputState},
    resizable::{h_resizable, resizable_panel},
    setting::SettingPage,
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
};
use gpui::{
    App, AppContext as _, Axis, ElementId, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Pixels, RenderOnce, Role, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Subscription, Window, container_query, div, prelude::FluentBuilder as _, px, relative,
};
use rust_i18n::t;

const STACKED_LAYOUT_MAX_WIDTH: Pixels = px(480.);

/// Component-local geometry derived from semantic Style Preset density.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SettingsMetrics {
    pub(super) page_padding_x: Pixels,
    pub(super) page_padding_y: Pixels,
    pub(super) group_padding_y: Pixels,
    pub(super) section_gap: Pixels,
    pub(super) item_gap: Pixels,
    pub(super) text_gap: Pixels,
}

impl SettingsMetrics {
    /// Resolves Settings geometry without branching on concrete preset IDs.
    pub(super) fn for_density(density: Density) -> Self {
        match density {
            Density::Compact => Self {
                page_padding_x: px(12.),
                page_padding_y: px(12.),
                group_padding_y: px(12.),
                section_gap: px(12.),
                item_gap: px(8.),
                text_gap: px(2.),
            },
            Density::Standard => Self {
                page_padding_x: px(16.),
                page_padding_y: px(16.),
                group_padding_y: px(16.),
                section_gap: px(16.),
                item_gap: px(12.),
                text_gap: px(4.),
            },
            Density::Comfortable => Self {
                page_padding_x: px(20.),
                page_padding_y: px(20.),
                group_padding_y: px(20.),
                section_gap: px(20.),
                item_gap: px(16.),
                text_gap: px(6.),
            },
        }
    }
}

/// A filtered page keeps source indices stable while exposing only matching groups.
#[derive(Debug, Clone)]
struct VisiblePage {
    page_ix: usize,
    group_indices: Vec<usize>,
}

/// The settings structure containing multiple pages for app settings.
///
/// The hierarchy of settings is as follows:
///
/// ```ignore
/// Settings
///   SettingPage     <- The single active page displayed
///     SettingGroup
///       SettingItem
///         Label
///         SettingField (e.g., Switch, Dropdown, Input)
/// ```
#[derive(IntoElement)]
pub struct Settings {
    id: ElementId,
    style: StyleRefinement,
    pages: Vec<SettingPage>,
    group_variant: GroupBoxVariant,
    size: Size,
    sidebar_width: Pixels,
    sidebar_size_range: Range<Pixels>,
    sidebar_style: StyleRefinement,
    default_selected_index: SelectIndex,
    header_style: StyleRefinement,
}

impl Settings {
    /// Create a new settings with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            pages: vec![],
            group_variant: GroupBoxVariant::default(),
            size: Size::default(),
            sidebar_width: px(250.0),
            sidebar_size_range: px(160.0)..px(360.0),
            sidebar_style: StyleRefinement::default(),
            default_selected_index: SelectIndex::default(),
            header_style: StyleRefinement::default(),
        }
    }

    /// Set the width of the sidebar, default is `250px`.
    pub fn sidebar_width(mut self, width: impl Into<Pixels>) -> Self {
        self.sidebar_width = width.into();
        self
    }

    /// Set the resize range of the sidebar, default is `160px..360px`.
    pub fn sidebar_size_range(mut self, range: impl Into<Range<Pixels>>) -> Self {
        self.sidebar_size_range = range.into();
        self
    }

    /// Add a page to the settings.
    pub fn page(mut self, page: SettingPage) -> Self {
        self.pages.push(page);
        self
    }

    /// Add pages to the settings.
    pub fn pages(mut self, pages: impl IntoIterator<Item = SettingPage>) -> Self {
        self.pages.extend(pages);
        self
    }

    /// Set the default variant for all setting groups.
    ///
    /// All setting groups will use this variant unless overridden individually.
    pub fn with_group_variant(mut self, variant: GroupBoxVariant) -> Self {
        self.group_variant = variant;
        self
    }

    /// Set the style refinement for the sidebar.
    pub fn sidebar_style(mut self, style: &StyleRefinement) -> Self {
        self.sidebar_style = style.clone();
        self
    }

    /// Set the default index of the page to be selected.
    pub fn default_selected_index(mut self, index: SelectIndex) -> Self {
        self.default_selected_index = index;
        self
    }

    /// Set the style refinement for the header.
    pub fn header_style(mut self, style: &StyleRefinement) -> Self {
        self.header_style = style.clone();
        self
    }

    fn visible_pages(&self, query: &str, cx: &App) -> Vec<VisiblePage> {
        self.pages
            .iter()
            .enumerate()
            .filter_map(|(page_ix, page)| {
                let group_indices = page
                    .groups
                    .iter()
                    .enumerate()
                    .filter_map(|(group_ix, group)| group.is_match(query, cx).then_some(group_ix))
                    .collect::<Vec<_>>();
                if group_indices.is_empty() {
                    None
                } else {
                    Some(VisiblePage {
                        page_ix,
                        group_indices,
                    })
                }
            })
            .collect()
    }

    /// Resolves the displayed page without overwriting the persisted selection during search.
    fn active_page_ix(selected_page_ix: usize, pages: &[VisiblePage]) -> Option<usize> {
        pages
            .iter()
            .find(|page| page.page_ix == selected_page_ix)
            .or_else(|| pages.first())
            .map(|page| page.page_ix)
    }

    fn render_active_page(
        &self,
        state: &Entity<SettingsState>,
        pages: &[VisiblePage],
        options: &RenderOptions,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::AnyElement {
        let selected_index = state.read(cx).selected_index;
        let Some(page_ix) = Self::active_page_ix(selected_index.page_ix, pages) else {
            let message = t!("Settings.no_results");
            return div()
                .id("settings-no-results")
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .role(Role::Status)
                .aria_label(message.clone())
                .child(message)
                .into_any_element();
        };
        let visible_page = pages
            .iter()
            .find(|page| page.page_ix == page_ix)
            .expect("active page must be visible");

        self.pages[page_ix]
            .render(
                page_ix,
                &visible_page.group_indices,
                state,
                options,
                window,
                cx,
            )
            .into_any_element()
    }

    fn render_sidebar(
        &self,
        state: &Entity<SettingsState>,
        pages: &[VisiblePage],
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let selected_index = state.read(cx).selected_index;
        let active_page_ix = Self::active_page_ix(selected_index.page_ix, pages);
        let search_input = state.read(cx).search_input.clone();

        Sidebar::new("settings-sidebar")
            .w(relative(1.))
            .border_0()
            .refine_style(&self.sidebar_style)
            .collapsible(false)
            .collapsed(false)
            .header(
                div().w_full().refine_style(&self.header_style).child(
                    Input::new(&search_input)
                        .prefix(IconName::Search)
                        .with_size(self.size)
                        .aria_label(t!("Settings.search_placeholder")),
                ),
            )
            .child(
                SidebarMenu::new().children(pages.iter().map(|visible_page| {
                    let page_ix = visible_page.page_ix;
                    let page = &self.pages[page_ix];
                    let selected_group_is_visible = visible_page.group_indices.len() > 1
                        && selected_index.page_ix == page_ix
                        && selected_index.group_ix.is_some_and(|group_ix| {
                            visible_page.group_indices.contains(&group_ix)
                                && page.groups[group_ix].title.is_some()
                        });
                    let is_page_active =
                        active_page_ix == Some(page_ix) && !selected_group_is_visible;
                    SidebarMenuItem::new(page.title.clone())
                        .click_to_open(true)
                        .when_some(page.icon.clone(), |this, icon| this.icon(icon))
                        .default_open(page.default_open)
                        .active(is_page_active)
                        .on_click({
                            let state = state.clone();
                            move |_, _, cx| {
                                state.update(cx, |state, cx| {
                                    state.selected_index = SelectIndex {
                                        page_ix,
                                        ..Default::default()
                                    };
                                    cx.notify();
                                })
                            }
                        })
                        .when(visible_page.group_indices.len() > 1, |this| {
                            this.children(visible_page.group_indices.iter().filter_map(
                                |group_ix| {
                                    let group = &page.groups[*group_ix];
                                    let title = group.title.clone()?;
                                    let is_active = selected_index.page_ix == page_ix
                                        && selected_index.group_ix == Some(*group_ix);
                                    let group_ix = *group_ix;

                                    Some(SidebarMenuItem::new(title).active(is_active).on_click({
                                        let state = state.clone();
                                        move |_, _, cx| {
                                            state.update(cx, |state, cx| {
                                                state.selected_index = SelectIndex {
                                                    page_ix,
                                                    group_ix: Some(group_ix),
                                                };
                                                state.deferred_scroll_group_ix = Some(group_ix);
                                                cx.notify();
                                            })
                                        }
                                    }))
                                },
                            ))
                        })
                })),
            )
    }
}

impl Sizable for Settings {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Settings {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

pub(super) struct SettingsState {
    pub(super) selected_index: SelectIndex,
    /// If set, defer scrolling to this group index after rendering.
    pub(super) deferred_scroll_group_ix: Option<usize>,
    pub(super) search_input: Entity<InputState>,
    _search_subscription: Subscription,
}

/// Options for rendering setting item.
#[derive(Clone, Copy)]
pub struct RenderOptions {
    pub page_ix: usize,
    pub group_ix: usize,
    pub item_ix: usize,
    pub size: Size,
    pub group_variant: GroupBoxVariant,
    pub layout: Axis,
    pub disabled: bool,
}

#[derive(Clone, Copy, Default)]
pub struct SelectIndex {
    pub page_ix: usize,
    pub group_ix: Option<usize>,
}

impl RenderOnce for Settings {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |window, cx| {
            let search_input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t!("Settings.search_placeholder"))
                    .default_value("")
            });
            let search_subscription = cx.subscribe(&search_input, {
                move |state: &mut SettingsState, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        state.deferred_scroll_group_ix = None;
                        cx.notify();
                    }
                }
            });

            SettingsState {
                search_input,
                selected_index: self.default_selected_index,
                deferred_scroll_group_ix: None,
                _search_subscription: search_subscription,
            }
        });

        let query = state.read(cx).search_input.read(cx).value();
        let visible_pages = self.visible_pages(&query, cx);
        let options = RenderOptions {
            page_ix: 0,
            group_ix: 0,
            item_ix: 0,
            size: self.size,
            group_variant: self.group_variant,
            layout: Axis::Horizontal,
            disabled: false,
        };
        let sidebar_size_range = self.sidebar_size_range.clone();
        let sidebar = self
            .render_sidebar(&state, &visible_pages, window, cx)
            .into_any_element();

        div().size_full().refine_style(&self.style).child(
            h_resizable(self.id.clone())
                .child(
                    resizable_panel()
                        .size(self.sidebar_width)
                        .size_range(sidebar_size_range)
                        .child(sidebar),
                )
                .child(
                    resizable_panel().child(container_query(move |size, window, cx| {
                        let options = RenderOptions {
                            layout: if size.width <= STACKED_LAYOUT_MAX_WIDTH {
                                Axis::Vertical
                            } else {
                                Axis::Horizontal
                            },
                            ..options
                        };
                        self.render_active_page(&state, &visible_pages, &options, window, cx)
                    })),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        setting::{SettingGroup, SettingItem},
        v_flex,
    };
    use gpui::TestAppContext;

    #[test]
    fn density_metrics_expand_monotonically() {
        let compact = SettingsMetrics::for_density(Density::Compact);
        let standard = SettingsMetrics::for_density(Density::Standard);
        let comfortable = SettingsMetrics::for_density(Density::Comfortable);

        assert!(compact.page_padding_x < standard.page_padding_x);
        assert!(standard.page_padding_x < comfortable.page_padding_x);
        assert!(compact.item_gap < standard.item_gap);
        assert!(standard.item_gap < comfortable.item_gap);
    }

    #[test]
    fn active_page_preserves_source_indices() {
        let pages = vec![
            VisiblePage {
                page_ix: 2,
                group_indices: vec![1],
            },
            VisiblePage {
                page_ix: 5,
                group_indices: vec![0],
            },
        ];

        assert_eq!(Settings::active_page_ix(5, &pages), Some(5));
        assert_eq!(Settings::active_page_ix(3, &pages), Some(2));
        assert_eq!(Settings::active_page_ix(0, &[]), None);
    }

    #[gpui::test]
    fn filtering_preserves_source_page_and_group_indices(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|_, cx| {
            let settings = Settings::new("settings")
                .page(
                    SettingPage::new("Hidden").group(
                        SettingGroup::new()
                            .item(SettingItem::render(|_, _, _| v_flex()).keywords(["other"])),
                    ),
                )
                .page(
                    SettingPage::new("Visible").groups([
                        SettingGroup::new()
                            .item(SettingItem::render(|_, _, _| v_flex()).keywords(["other"])),
                        SettingGroup::new()
                            .title("Match")
                            .item(SettingItem::render(|_, _, _| v_flex()).keywords(["needle"])),
                    ]),
                );

            let pages = settings.visible_pages("needle", cx);
            assert_eq!(pages.len(), 1);
            assert_eq!(pages[0].page_ix, 1);
            assert_eq!(pages[0].group_indices, vec![1]);
        });
    }
}
