use std::rc::Rc;

use gpui::{
    AnyElement, App, Entity, InteractiveElement as _, IntoElement, ListAlignment, ListState,
    ParentElement as _, SharedString, StyleRefinement, Styled, Window, div, list,
    prelude::FluentBuilder as _, px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme, Icon, IconName, Sizable, StyledExt,
    button::Button,
    h_flex,
    label::Label,
    scroll::ScrollableElement,
    setting::{
        RenderOptions, SettingGroup,
        settings::{SettingsMetrics, SettingsState},
    },
    v_flex,
};

/// A setting page that can contain multiple setting groups.
#[derive(Clone)]
pub struct SettingPage {
    pub(super) icon: Option<Icon>,
    resettable: bool,
    pub(super) default_open: bool,
    pub(super) title: SharedString,
    pub(super) title_suffix: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>>,
    pub(super) description: Option<SharedString>,
    pub(super) groups: Vec<SettingGroup>,
    pub(super) header_style: StyleRefinement,
}

impl SettingPage {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            icon: None,
            resettable: true,
            default_open: false,
            title: title.into(),
            title_suffix: None,
            description: None,
            groups: Vec::new(),
            header_style: StyleRefinement::default(),
        }
    }

    /// Set the title of the setting page.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set a custom element to render after the title in the page header.
    ///
    /// For example, an info icon button that opens the help documentation.
    pub fn title_suffix<F, E>(mut self, suffix: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.title_suffix = Some(Rc::new(move |window, cx| {
            suffix(window, cx).into_any_element()
        }));
        self
    }

    /// Set the icon of the setting page.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the description of the setting page, default is None.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the default open state of the setting page, default is false.
    pub fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }

    /// Set whether the setting page is resettable, default is true.
    ///
    /// If true and the items in this page has changed, the reset button will appear.
    pub fn resettable(mut self, resettable: bool) -> Self {
        self.resettable = resettable;
        self
    }

    /// Add a setting group to the page.
    pub fn group(mut self, group: SettingGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// Add multiple setting groups to the page.
    pub fn groups(mut self, groups: impl IntoIterator<Item = SettingGroup>) -> Self {
        self.groups.extend(groups);
        self
    }

    /// Set the style refinement for the header of the setting page.
    pub fn header_style(mut self, style: &StyleRefinement) -> Self {
        self.header_style = style.clone();
        self
    }

    fn is_resettable(&self, cx: &App) -> bool {
        self.resettable && self.groups.iter().any(|group| group.is_resettable(cx))
    }

    fn reset_all(&self, window: &mut Window, cx: &mut App) {
        for group in &self.groups {
            group.reset(window, cx);
        }
    }

    pub(super) fn render(
        &self,
        ix: usize,
        visible_group_indices: &[usize],
        state: &Entity<SettingsState>,
        options: &RenderOptions,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let query = state.read(cx).search_input.read(cx).value();
        let metrics = SettingsMetrics::for_density(cx.theme().style.density);
        let groups_count = visible_group_indices.len();

        let list_state = window
            .use_keyed_state(
                SharedString::from(format!("list-state:{}", ix)),
                cx,
                |_, _| ListState::new(groups_count, ListAlignment::Top, px(100.)),
            )
            .read(cx)
            .clone();

        if list_state.item_count() != groups_count {
            list_state.reset(groups_count);
        }

        let deferred_scroll_group_ix = state.read(cx).deferred_scroll_group_ix;
        if let Some(source_group_ix) = deferred_scroll_group_ix {
            state.update(cx, |state, _| {
                state.deferred_scroll_group_ix = None;
            });
            if let Some(visible_ix) = visible_group_indices
                .iter()
                .position(|group_ix| *group_ix == source_group_ix)
            {
                list_state.scroll_to_reveal_item(visible_ix);
            }
        }

        v_flex()
            .id(ix)
            .size_full()
            .child(
                v_flex()
                    .px(metrics.page_padding_x)
                    .py(metrics.page_padding_y)
                    .gap(metrics.item_gap)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .refine_style(&self.header_style)
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap(metrics.text_gap)
                                    .child(Label::new(self.title.clone()).text_lg().font_semibold())
                                    .when_some(self.title_suffix.clone(), |this, suffix| {
                                        this.child(suffix(window, cx))
                                    }),
                            )
                            .when(self.is_resettable(cx), |this| {
                                this.child(
                                    Button::new("reset")
                                        .icon(IconName::Undo2)
                                        .ghost()
                                        .with_size(options.size)
                                        .tooltip(t!("Settings.Reset All"))
                                        .on_click({
                                            let page = self.clone();
                                            move |_, window, cx| {
                                                page.reset_all(window, cx);
                                            }
                                        }),
                                )
                            }),
                    )
                    .when_some(self.description.clone(), |this, description| {
                        this.child(
                            Label::new(description)
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                        )
                    }),
            )
            .child(
                div()
                    .px(metrics.page_padding_x)
                    .relative()
                    .flex_1()
                    .w_full()
                    .child(
                        list(list_state.clone(), {
                            let query = query.to_string();
                            let visible_group_indices = visible_group_indices.to_vec();
                            let groups = self.groups.clone();
                            let options = *options;
                            move |visible_group_ix, window, cx| {
                                let group_ix = visible_group_indices[visible_group_ix];
                                let group = groups[group_ix].clone();
                                group
                                    .py(metrics.group_padding_y)
                                    .render(
                                        &query,
                                        &RenderOptions {
                                            page_ix: ix,
                                            group_ix,
                                            ..options
                                        },
                                        metrics,
                                        window,
                                        cx,
                                    )
                                    .into_any_element()
                            }
                        })
                        .size_full(),
                    )
                    .vertical_scrollbar(&list_state),
            )
    }
}
