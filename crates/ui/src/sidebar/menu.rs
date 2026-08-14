// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added or exposed behavior through `resolve`, `from_style`, `render_at_depth`,
//   `sidebar_menu_metrics_follow_semantic_density`.
// - Reworked Menu around accessibility semantics and ARIA state, semantic Style Preset geometry and
//   density, keyboard navigation and activation behavior, focus-visible and focus restoration
//   behavior.
// - Replaced legacy radius access with `Theme.style.radii.md`.
use crate::{
    ActiveTheme as _, Collapsible, Density, FocusableExt as _, Icon, IconName, Sizable as _,
    StyledExt,
    accessibility::accessibility_state,
    button::Button,
    h_flex,
    menu::{ContextMenuExt, PopupMenu},
    sidebar::SidebarItem,
    v_flex,
};
use gpui::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement as _, IntoElement,
    ParentElement as _, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, percentage, prelude::FluentBuilder, px,
};
use std::{rc::Rc, sync::Arc};

/// Sidebar-menu geometry resolved from semantic preset density.
#[derive(Clone, Copy)]
struct SidebarMenuMetrics {
    menu_gap: gpui::Pixels,
    item_height: gpui::Pixels,
    item_padding_x: gpui::Pixels,
    item_gap: gpui::Pixels,
    item_radius: gpui::Pixels,
    collapsed_edge: gpui::Pixels,
    subitem_height: gpui::Pixels,
}

impl SidebarMenuMetrics {
    /// Resolves Vega, Nova, and Maia geometry without branching on preset identifiers.
    fn resolve(cx: &App) -> Self {
        Self::from_style(&cx.theme().style)
    }

    /// Resolves menu geometry from a semantic Style Preset.
    fn from_style(style: &crate::StylePreset) -> Self {
        match style.density {
            Density::Standard => Self {
                menu_gap: px(4.),
                item_height: px(32.),
                item_padding_x: px(8.),
                item_gap: px(8.),
                item_radius: style.radii.md,
                collapsed_edge: px(32.),
                subitem_height: px(28.),
            },
            Density::Compact => Self {
                menu_gap: px(0.),
                item_height: px(32.),
                item_padding_x: px(8.),
                item_gap: px(8.),
                item_radius: style.radii.md,
                collapsed_edge: px(32.),
                subitem_height: px(28.),
            },
            Density::Comfortable => Self {
                menu_gap: px(4.),
                item_height: px(36.),
                item_padding_x: px(12.),
                item_gap: px(10.),
                item_radius: style.radii.lg,
                collapsed_edge: px(36.),
                subitem_height: px(28.),
            },
        }
    }
}

/// Menu for the [`super::Sidebar`]
#[derive(Clone)]
pub struct SidebarMenu {
    style: StyleRefinement,
    collapsed: bool,
    items: Vec<SidebarMenuItem>,
}

impl SidebarMenu {
    /// Create a new SidebarMenu
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            items: Vec::new(),
            collapsed: false,
        }
    }

    /// Add a [`SidebarMenuItem`] child menu item to the sidebar menu.
    ///
    /// See also [`SidebarMenu::children`].
    pub fn child(mut self, child: impl Into<SidebarMenuItem>) -> Self {
        self.items.push(child.into());
        self
    }

    /// Add multiple [`SidebarMenuItem`] child menu items to the sidebar menu.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = impl Into<SidebarMenuItem>>,
    ) -> Self {
        self.items = children.into_iter().map(Into::into).collect();
        self
    }
}

impl Collapsible for SidebarMenu {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

impl SidebarItem for SidebarMenu {
    fn render(
        self,
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let id = id.into();

        let metrics = SidebarMenuMetrics::resolve(cx);

        v_flex()
            .gap(metrics.menu_gap)
            .refine_style(&self.style)
            .children(self.items.into_iter().enumerate().map(|(ix, item)| {
                let id = ElementId::NamedChild(Arc::new(id.clone()), ix.to_string().into());
                item.collapsed(self.collapsed)
                    .render(id, window, cx)
                    .into_any_element()
            }))
    }
}

impl Styled for SidebarMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// Menu item for the [`SidebarMenu`]
#[derive(Clone)]
pub struct SidebarMenuItem {
    icon: Option<Icon>,
    label: SharedString,
    handler: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
    active: bool,
    default_open: bool,
    click_to_open: bool,
    collapsed: bool,
    click_to_toggle: bool,
    children: Vec<Self>,
    suffix: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>>,
    disabled: bool,
    context_menu: Option<Rc<dyn Fn(PopupMenu, &mut Window, &mut App) -> PopupMenu + 'static>>,
}

impl SidebarMenuItem {
    /// Create a new [`SidebarMenuItem`] with a label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            icon: None,
            label: label.into(),
            handler: Rc::new(|_, _, _| {}),
            active: false,
            collapsed: false,
            default_open: false,
            click_to_open: false,
            click_to_toggle: false,
            children: Vec::new(),
            suffix: None,
            disabled: false,
            context_menu: None,
        }
    }

    /// Set the icon for the menu item
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the active state of the menu item
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Add a click handler to the menu item
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.handler = Rc::new(handler);
        self
    }

    /// Set the collapsed state of the menu item
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Set the default open state of the Submenu, default is `false`.
    ///
    /// This only used on initial render, the internal state will be used afterwards.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Set whether clicking the menu item open the submenu.
    ///
    /// Default is `false`.
    ///
    /// If `false` we only handle open/close via the caret button.
    pub fn click_to_open(mut self, click_to_open: bool) -> Self {
        self.click_to_open = click_to_open;
        self
    }

    /// Set whether clicking the menu item toggles the submenu.
    ///
    /// If click_to_open is `true`, this has no effect.
    ///
    /// Default is `false`.
    pub fn click_to_toggle(mut self, click_to_toggle: bool) -> Self {
        self.click_to_toggle = click_to_toggle;
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Self>>) -> Self {
        self.children = children.into_iter().map(Into::into).collect();
        self
    }

    /// Set the suffix for the menu item.
    pub fn suffix<F, E>(mut self, builder: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.suffix = Some(Rc::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }

    /// Set disabled flat for menu item.
    pub fn disable(mut self, disable: bool) -> Self {
        self.disabled = disable;
        self
    }

    fn is_submenu(&self) -> bool {
        !self.children.is_empty()
    }

    /// Renders a root or nested item using the matching sidebar geometry contract.
    fn render_at_depth(
        self,
        id: ElementId,
        depth: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let click_to_open = self.click_to_open;
        let click_to_toggle = self.click_to_toggle;
        let default_open = self.default_open;
        let is_submenu = self.is_submenu();
        let open_state = if is_submenu {
            let state_id = ElementId::NamedChild(Arc::new(id.clone()), "open-state".into());
            Some(window.use_keyed_state(state_id, cx, |_, _| default_open))
        } else {
            None
        };
        let focus_id = ElementId::NamedChild(Arc::new(id.clone()), "focus-state".into());
        let focus_handle = window
            .use_keyed_state(focus_id, cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let handler = self.handler.clone();
        let is_collapsed = self.collapsed;
        let is_active = self.active;
        let is_disabled = self.disabled;
        let is_open = open_state
            .as_ref()
            .is_some_and(|state| !is_collapsed && *state.read(cx));
        let metrics = SidebarMenuMetrics::resolve(cx);
        let nested = depth > 0;
        let item_height = if nested {
            metrics.subitem_height
        } else {
            metrics.item_height
        };
        let padding_x = if nested {
            px(8.)
        } else {
            metrics.item_padding_x
        };
        let item_gap = if nested { px(8.) } else { metrics.item_gap };
        let item_radius = if nested {
            cx.theme().style.radii.md
        } else {
            metrics.item_radius
        };
        let focus_visible = focus_handle.is_focused(window) && window.last_input_was_keyboard();

        let item = h_flex()
            .id("item")
            .role(Role::Button)
            .when_some(open_state.as_ref(), |this, _| this.aria_expanded(is_open))
            .aria_label(self.label.clone())
            .w_full()
            .h(item_height)
            .min_w(px(0.))
            .overflow_x_hidden()
            .flex_shrink_0()
            .px(padding_x)
            .gap(item_gap)
            .rounded(item_radius)
            .text_sm()
            .when(!is_disabled, |this| {
                this.track_focus(&focus_handle.tab_stop(true))
                    .cursor_pointer()
                    .hover(|this| {
                        this.bg(cx.theme().sidebar_accent)
                            .text_color(cx.theme().sidebar_accent_foreground)
                    })
            })
            .when(is_active, |this| {
                this.font_medium()
                    .bg(cx.theme().tokens.sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .when_some(self.icon.clone(), |this, icon| {
                this.child(icon.size_4().flex_none())
            })
            .when(is_collapsed, |this| {
                this.size(metrics.collapsed_edge).p_0().justify_center()
            })
            .when(!is_collapsed, |this| {
                this.child(
                    h_flex()
                        .flex_1()
                        .min_w(px(0.))
                        .gap(item_gap)
                        .justify_between()
                        .overflow_x_hidden()
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .overflow_x_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(self.label.clone()),
                        )
                        .when_some(self.suffix.clone(), |this, suffix| {
                            this.child(div().flex_none().child(suffix(window, cx)))
                        }),
                )
                .when_some(open_state.clone(), |this, open_state| {
                    this.child(
                        Button::new("caret")
                            .xsmall()
                            .ghost()
                            .aria_label(if is_open {
                                "Collapse submenu"
                            } else {
                                "Expand submenu"
                            })
                            .icon(
                                Icon::new(IconName::ChevronRight)
                                    .size_4()
                                    .when(is_open, |this| this.rotate(percentage(90. / 360.))),
                            )
                            .on_click(move |_, _, cx| {
                                cx.stop_propagation();
                                open_state.update(cx, |is_open, cx| {
                                    *is_open = !*is_open;
                                    cx.notify();
                                });
                            }),
                    )
                })
            })
            .when(is_disabled, |this| this.opacity(0.5))
            .when(!is_disabled, |this| {
                this.on_click({
                    let open_state = open_state.clone();
                    move |event, window, cx| {
                        if click_to_open {
                            if let Some(state) = &open_state {
                                state.update(cx, |is_open, cx| {
                                    *is_open = true;
                                    cx.notify();
                                });
                            }
                        } else if click_to_toggle {
                            if let Some(state) = &open_state {
                                state.update(cx, |is_open, cx| {
                                    *is_open = !*is_open;
                                    cx.notify();
                                });
                            }
                        }
                        handler(event, window, cx);
                    }
                })
            })
            .focus_ring(focus_visible, px(0.), window, cx);

        let item = if let Some(context_menu) = self.context_menu {
            item.context_menu(move |menu, window, cx| context_menu(menu, window, cx))
                .into_any_element()
        } else {
            item.into_any_element()
        };
        let item = accessibility_state(item, false, false, is_disabled);

        div()
            .id(id.clone())
            .w_full()
            .child(item)
            .when(is_open, |this| {
                this.child(
                    v_flex()
                        .id("submenu")
                        .border_l_1()
                        .border_color(cx.theme().sidebar_border)
                        .gap_1()
                        .ml_3p5()
                        .pl_2p5()
                        .py_0p5()
                        .children(self.children.into_iter().enumerate().map(|(index, item)| {
                            let child_id = ElementId::NamedChild(
                                Arc::new(id.clone()),
                                index.to_string().into(),
                            );
                            item.render_at_depth(child_id, depth + 1, window, cx)
                        })),
                )
            })
            .into_any_element()
    }

    /// Set the context menu for the menu item.
    pub fn context_menu(
        mut self,
        f: impl Fn(PopupMenu, &mut Window, &mut App) -> PopupMenu + 'static,
    ) -> Self {
        self.context_menu = Some(Rc::new(f));
        self
    }
}

impl FluentBuilder for SidebarMenuItem {}

impl Collapsible for SidebarMenuItem {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

impl SidebarItem for SidebarMenuItem {
    fn render(
        self,
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        self.render_at_depth(id.into(), 0, window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_menu_metrics_follow_semantic_density() {
        let vega = SidebarMenuMetrics::from_style(&crate::StylePreset::vega());
        let nova = SidebarMenuMetrics::from_style(&crate::StylePreset::nova());
        let maia = SidebarMenuMetrics::from_style(&crate::StylePreset::maia());

        assert_eq!(vega.menu_gap, px(4.));
        assert_eq!(vega.item_height, px(32.));
        assert_eq!(nova.menu_gap, px(0.));
        assert_eq!(nova.item_height, px(32.));
        assert_eq!(maia.menu_gap, px(4.));
        assert_eq!(maia.item_height, px(36.));
        assert_eq!(maia.item_padding_x, px(12.));
    }
}
