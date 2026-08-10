use std::{cell::RefCell, rc::Rc};

use gpui::{
    Anchor, AnyElement, App, Background, Bounds, ClickEvent, Div, Edges, ElementId,
    InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce, Role, ScrollHandle,
    SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement, Styled, Window,
    accesskit, div, prelude::FluentBuilder as _, px,
};
use rust_i18n::t;
use smallvec::SmallVec;

use super::{Tab, TabVariant, tab::tab_child_id};
use crate::animation::Transition;
use crate::button::Button;
use crate::menu::{DropdownMenu as _, PopupMenuItem};
use crate::{ActiveTheme, ElementExt, Icon, Selectable, Sizable, Size, StyledExt, h_flex};

struct TabIndicatorBounds {
    container: Bounds<Pixels>,
    tabs: Vec<Bounds<Pixels>>,
}

type TabClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

struct TabMenuItemMeta {
    label: Option<SharedString>,
    icon: Option<Icon>,
    disabled: bool,
    on_click: Option<TabClickHandler>,
}

/// Resolves horizontal roving-focus navigation for an enabled tab sequence.
fn tab_bar_focus_target(key: &str, position: usize, item_count: usize) -> Option<usize> {
    if item_count == 0 {
        return None;
    }

    match key {
        "left" => Some(position.checked_sub(1).unwrap_or(item_count - 1)),
        "right" => Some((position + 1) % item_count),
        "home" => Some(0),
        "end" => Some(item_count - 1),
        _ => None,
    }
}

impl TabIndicatorBounds {
    fn new(num_tabs: usize) -> Self {
        Self {
            container: Bounds::default(),
            tabs: vec![Bounds::default(); num_tabs],
        }
    }

    fn resize(&mut self, num_tabs: usize) {
        self.tabs.resize(num_tabs, Bounds::default());
    }
}

/// A TabBar element that contains multiple [`Tab`] items.
#[derive(IntoElement)]
pub struct TabBar {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    scroll_handle: Option<ScrollHandle>,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    children: SmallVec<[Tab; 2]>,
    last_empty_space: Option<AnyElement>,
    selected_index: Option<usize>,
    variant: TabVariant,
    size: Size,
    menu: bool,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
}

impl TabBar {
    /// Create a new TabBar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            base: div().id(id),
            style: StyleRefinement::default(),
            children: SmallVec::new(),
            scroll_handle: None,
            prefix: None,
            suffix: None,
            variant: TabVariant::default(),
            size: Size::default(),
            last_empty_space: None,
            selected_index: None,
            on_click: None,
            menu: false,
        }
    }

    /// Set the Tab variant, all children will inherit the variant.
    pub fn with_variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the Tab variant to Pill, all children will inherit the variant.
    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    /// Set the Tab variant to Outline, all children will inherit the variant.
    pub fn outline(mut self) -> Self {
        self.variant = TabVariant::Outline;
        self
    }

    /// Set the Tab variant to Segmented, all children will inherit the variant.
    pub fn segmented(mut self) -> Self {
        self.variant = TabVariant::Segmented;
        self
    }

    /// Set the Tab variant to Underline, all children will inherit the variant.
    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
    }

    /// Set whether to keep the all-tabs menu button visible, default is false.
    pub fn menu(mut self, menu: bool) -> Self {
        self.menu = menu;
        self
    }

    /// Track the scroll of the TabBar.
    pub fn track_scroll(mut self, scroll_handle: &ScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle.clone());
        self
    }

    /// Set the prefix element of the TabBar
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set the suffix element of the TabBar
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Add children of the TabBar, all children will inherit the variant.
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Tab>>) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }

    /// Add child of the TabBar, tab will inherit the variant.
    pub fn child(mut self, child: impl Into<Tab>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Set the selected index of the TabBar.
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = Some(index);
        self
    }

    /// Set the last empty space element of the TabBar.
    pub fn last_empty_space(mut self, last_empty_space: impl IntoElement) -> Self {
        self.last_empty_space = Some(last_empty_space.into_any_element());
        self
    }

    /// Set the on_click callback of the TabBar, the first parameter is the index of the clicked tab.
    ///
    /// When this is set, the children's on_click will be ignored.
    pub fn on_click<F>(mut self, on_click: F) -> Self
    where
        F: Fn(&usize, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Renders an interruptible sliding indicator for the selected tab.
    fn render_indicator(
        &self,
        bounds_rc: &Option<Rc<RefCell<TabIndicatorBounds>>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let has_indicator = matches!(
            self.variant,
            TabVariant::Segmented | TabVariant::Pill | TabVariant::Underline
        );
        let num_tabs = self.children.len();
        let selected_ix = self.selected_index.unwrap_or(usize::MAX);

        if !(has_indicator && num_tabs > 0 && selected_ix < num_tabs) {
            return None;
        }

        let prev_key = tab_child_id(&self.id, "indicator-previous");
        let anim_key = tab_child_id(&self.id, "indicator-geometry");
        let init_key = tab_child_id(&self.id, "indicator-initialized");

        let prev_selected = window.use_keyed_state(prev_key, cx, |_, _| selected_ix);
        // Geometry is kept separate from the motion adapter so prepaint can
        // update target bounds without mutating render-time element state.
        let anim_params =
            window.use_keyed_state(anim_key, cx, |_, _| (px(0.), px(0.), px(0.), px(0.)));
        let initialized = window.use_keyed_state(init_key, cx, |_, _| false);

        // First frame: trigger re-render to capture bounds via on_prepaint
        if !*initialized.read(cx) {
            initialized.update(cx, |v, _| *v = true);
        }

        self.update_anim_params(selected_ix, bounds_rc, &prev_selected, &anim_params, cx);

        let (from_left, from_width, to_left, to_width) = *anim_params.read(cx);
        if to_width <= px(0.) {
            return None;
        }

        let variant = self.variant;
        let size = self.size;
        let inner_height = variant.inner_height(size, cx);
        let inner_radius = variant.inner_radius(size, cx);

        let easing = cx.theme().style.motion.move_easing;
        let indicator = div()
            .absolute()
            .top_0()
            .bottom_0()
            .map(|el| match variant {
                TabVariant::Segmented => el.flex().items_center().child(
                    div()
                        .w_full()
                        .h(inner_height)
                        .bg(cx.theme().tokens.background)
                        .rounded(inner_radius)
                        .when(cx.theme().style.elevation.enabled, |this| this.shadow_sm()),
                ),
                TabVariant::Pill => el.flex().items_center().child(
                    div()
                        .size_full()
                        .bg(cx.theme().tokens.primary)
                        .rounded(px(99.)),
                ),
                TabVariant::Underline => el.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(px(2.))
                        .bg(cx.theme().tokens.primary),
                ),
                _ => el,
            })
            .left(to_left)
            .w(to_width);

        Some(
            Transition::new(cx.theme().style.motion.slow())
                .ease_token(easing)
                .slide_x(from_left, to_left)
                .width(from_width, to_width)
                .apply(indicator, tab_child_id(&self.id, "indicator-motion"))
                .into_any_element(),
        )
    }

    /// Update animation parameters based on current and previous selection.
    fn update_anim_params(
        &self,
        selected_ix: usize,
        bounds_rc: &Option<Rc<RefCell<TabIndicatorBounds>>>,
        prev_selected: &gpui::Entity<usize>,
        anim_params: &gpui::Entity<(Pixels, Pixels, Pixels, Pixels)>,
        cx: &mut App,
    ) {
        let rc = match bounds_rc {
            Some(rc) => rc,
            None => return,
        };

        let prev_ix = *prev_selected.read(cx);
        let bounds = rc.borrow();
        let container = bounds.container;

        if container.size.width == px(0.) {
            if prev_ix != selected_ix {
                prev_selected.update(cx, |v, _| *v = selected_ix);
            }
            return;
        }

        if prev_ix != selected_ix {
            let from_b = bounds.tabs.get(prev_ix);
            let to_b = bounds.tabs.get(selected_ix);
            match (from_b, to_b) {
                (Some(from_b), Some(to_b)) => {
                    let from_left = from_b.origin.x - container.origin.x;
                    let from_width = from_b.size.width;
                    let to_left = to_b.origin.x - container.origin.x;
                    let to_width = to_b.size.width;
                    anim_params.update(cx, |v, _| *v = (from_left, from_width, to_left, to_width));
                }
                (None, Some(to_b)) => {
                    let left = to_b.origin.x - container.origin.x;
                    let width = to_b.size.width;
                    anim_params.update(cx, |v, _| *v = (left, width, left, width));
                }
                _ => {}
            }
            drop(bounds);
            prev_selected.update(cx, |v, _| *v = selected_ix);
            return;
        }

        if let Some(to_b) = bounds.tabs.get(selected_ix) {
            let left = to_b.origin.x - container.origin.x;
            let width = to_b.size.width;
            let (_, _, to_left, to_width) = *anim_params.read(cx);

            if to_width == px(0.) {
                anim_params.update(cx, |v, _| *v = (left, width, left, width));
                return;
            }

            if left != to_left || width != to_width {
                anim_params.update(cx, |v, _| *v = (left, width, left, width));
            }
        }
    }
}

impl Styled for TabBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for TabBar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for TabBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let controls = cx.theme().style.controls.for_size(self.size);
        let default_gap = controls.gap
            + match self.size {
                Size::XSmall | Size::Small => px(4.),
                Size::Large => px(10.),
                Size::Medium | Size::Size(_) => px(6.),
            };
        let (bg, paddings, gap): (Background, _, _) = match self.variant {
            TabVariant::Tab => {
                let padding = Edges::all(px(0.));
                (cx.theme().tokens.tab_bar.into(), padding, px(0.))
            }
            TabVariant::Outline => {
                let padding = Edges::all(px(0.));
                (cx.theme().transparent.into(), padding, default_gap)
            }
            TabVariant::Pill => {
                let padding = Edges::all(px(0.));
                (cx.theme().transparent.into(), padding, controls.gap)
            }
            TabVariant::Segmented => {
                let padding_x = (controls.icon_edge_padding / 2.).max(px(2.));
                let padding = Edges {
                    left: padding_x,
                    right: padding_x,
                    ..Default::default()
                };

                (cx.theme().tokens.tab_bar_segmented.into(), padding, px(2.))
            }
            TabVariant::Underline => {
                let gap = controls.padding_x
                    + match self.size {
                        Size::XSmall | Size::Small => px(2.),
                        Size::Medium | Size::Large | Size::Size(_) => px(4.),
                    };

                (cx.theme().transparent.into(), Edges::all(px(0.)), gap)
            }
        };

        let has_indicator = matches!(
            self.variant,
            TabVariant::Segmented | TabVariant::Pill | TabVariant::Underline
        );
        let num_tabs = self.children.len();
        let item_ids = (0..num_tabs)
            .map(|index| tab_child_id(&self.id, format!("item-{index}")))
            .collect::<Vec<_>>();
        let item_focus_handles = item_ids
            .iter()
            .map(|id| {
                window
                    .use_keyed_state(tab_child_id(id, "focus"), cx, |_, cx| cx.focus_handle())
                    .read(cx)
                    .clone()
            })
            .collect::<Vec<_>>();
        let enabled_indexes = self
            .children
            .iter()
            .enumerate()
            .filter_map(|(index, child)| (!child.disabled).then_some(index))
            .collect::<Vec<_>>();
        let mut enabled_positions = vec![None; num_tabs];
        for (position, index) in enabled_indexes.iter().copied().enumerate() {
            enabled_positions[index] = Some(position);
        }
        let preferred_tab_index = enabled_indexes
            .iter()
            .copied()
            .find(|index| self.selected_index == Some(*index))
            .or_else(|| enabled_indexes.first().copied());
        let enabled_indexes = Rc::new(enabled_indexes);
        let enabled_focus_handles = Rc::new(
            enabled_indexes
                .iter()
                .map(|index| item_focus_handles[*index].clone())
                .collect::<Vec<_>>(),
        );

        // A TabBar handler remains authoritative. Without one, each Tab keeps
        // its own callback so keyboard navigation and the menu share behavior.
        let effective_handlers = Rc::new(
            self.children
                .iter()
                .enumerate()
                .map(|(index, child)| {
                    if let Some(group_handler) = self.on_click.clone() {
                        let handler: TabClickHandler =
                            Rc::new(move |_, window, cx| group_handler(&index, window, cx));
                        Some(handler)
                    } else {
                        child.on_click.clone()
                    }
                })
                .collect::<Vec<_>>(),
        );
        let item_metas = self
            .children
            .iter()
            .enumerate()
            .map(|(index, child)| TabMenuItemMeta {
                label: child.label.clone(),
                icon: child.icon.clone(),
                disabled: child.disabled,
                on_click: effective_handlers[index].clone(),
            })
            .collect::<Vec<_>>();

        // Bounds tracking for tab indicator animation.
        // Uses Rc<RefCell> to avoid triggering re-renders from prepaint writes.
        let bounds_rc = if has_indicator && num_tabs > 0 {
            let rc: Rc<RefCell<TabIndicatorBounds>> = window
                .use_keyed_state(tab_child_id(&self.id, "indicator-bounds"), cx, |_, _| {
                    Rc::new(RefCell::new(TabIndicatorBounds::new(num_tabs)))
                })
                .read(cx)
                .clone();
            rc.borrow_mut().resize(num_tabs);
            Some(rc)
        } else {
            None
        };

        let indicator = self.render_indicator(&bounds_rc, window, cx);
        let indicator_ready = indicator.is_some();

        let has_suffix_or_menu = self.suffix.is_some() || self.menu;
        let selected_index = self.selected_index;
        let tab_height = self.variant.height(self.size, cx);
        let trailing_space = self
            .last_empty_space
            .unwrap_or_else(|| div().w(controls.gap * 2.).into_any_element());

        self.base
            .role(Role::TabList)
            .aria_orientation(accesskit::Orientation::Horizontal)
            .group("tab-bar")
            .relative()
            .flex()
            .items_center()
            .bg(bg)
            .text_color(cx.theme().tab_foreground)
            .when(
                self.variant == TabVariant::Underline || self.variant == TabVariant::Tab,
                |this| {
                    this.child(
                        div()
                            .id("border-b")
                            .absolute()
                            .left_0()
                            .bottom_0()
                            .size_full()
                            .border_b_1()
                            .border_color(cx.theme().border),
                    )
                },
            )
            .rounded(self.variant.tab_bar_radius(self.size, cx))
            .paddings(paddings)
            .when(self.variant == TabVariant::Tab, |this| this.px(px(-1.)))
            .refine_style(&self.style)
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(
                h_flex().id("tabs").flex_1().overflow_x_hidden().child(
                    h_flex()
                        .id("tabs-inner")
                        .relative()
                        .gap(gap)
                        .overflow_x_scroll()
                        .restrict_scroll_to_axis()
                        .when_some(self.scroll_handle, |this, scroll_handle| {
                            this.track_scroll(&scroll_handle)
                        })
                        .when_some(bounds_rc.clone(), |this, rc| {
                            this.on_prepaint(move |bounds, _, _| {
                                rc.borrow_mut().container = bounds;
                            })
                        })
                        .when_some(indicator, |this, ind| this.child(ind))
                        .children(self.children.into_iter().enumerate().map(|(ix, child)| {
                            let tab_bar_prefix = child.tab_bar_prefix.unwrap_or(true);
                            let focus_handle = item_focus_handles[ix].clone();
                            let enabled_position = enabled_positions[ix];
                            let keyboard_enabled_indexes = enabled_indexes.clone();
                            let keyboard_focus_handles = enabled_focus_handles.clone();
                            let keyboard_handlers = effective_handlers.clone();
                            let current_handler = effective_handlers[ix].clone();
                            let current_selected = selected_index;
                            let mut tab = child
                                .ix(ix)
                                .tab_bar_prefix(tab_bar_prefix)
                                .with_variant(self.variant)
                                .with_size(self.size)
                                .state_id(item_ids[ix].clone())
                                .focus_handle(focus_handle, preferred_tab_index == Some(ix))
                                .on_group_key_down(move |event, window, cx| {
                                    if event.keystroke.modifiers.modified() || event.is_held {
                                        return;
                                    }

                                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                        window.prevent_default();
                                        cx.stop_propagation();
                                        if let Some(handler) = current_handler.as_ref() {
                                            handler(&ClickEvent::default(), window, cx);
                                        }
                                        return;
                                    }

                                    let Some(position) = enabled_position else {
                                        return;
                                    };
                                    let Some(target_position) = tab_bar_focus_target(
                                        event.keystroke.key.as_str(),
                                        position,
                                        keyboard_focus_handles.len(),
                                    ) else {
                                        return;
                                    };
                                    let Some(target_index) =
                                        keyboard_enabled_indexes.get(target_position).copied()
                                    else {
                                        return;
                                    };
                                    let Some(target_focus) =
                                        keyboard_focus_handles.get(target_position)
                                    else {
                                        return;
                                    };

                                    window.prevent_default();
                                    cx.stop_propagation();
                                    target_focus.focus(window, cx);
                                    if current_selected != Some(target_index)
                                        && let Some(handler) =
                                            keyboard_handlers[target_index].as_ref()
                                    {
                                        handler(&ClickEvent::default(), window, cx);
                                    }
                                });
                            tab.indicator_active = has_indicator;
                            tab.indicator_ready = indicator_ready;
                            let tab = tab
                                .when_some(self.selected_index, |this, selected_ix| {
                                    this.selected(selected_ix == ix)
                                })
                                .when_some(effective_handlers[ix].clone(), |this, handler| {
                                    this.on_click(move |event, window, cx| {
                                        handler(event, window, cx)
                                    })
                                });

                            if let Some(ref rc) = bounds_rc {
                                let rc = rc.clone();
                                div()
                                    .flex_shrink_0()
                                    .on_prepaint(move |bounds, _, _| {
                                        if let Some(slot) = rc.borrow_mut().tabs.get_mut(ix) {
                                            *slot = bounds;
                                        }
                                    })
                                    .child(tab)
                                    .into_any_element()
                            } else {
                                tab.into_any_element()
                            }
                        }))
                        .when(has_suffix_or_menu, |this| this.child(trailing_space)),
                ),
            )
            .when(self.menu, |this| {
                this.child(
                    Button::new("more")
                        .with_size(self.size)
                        .h(tab_height)
                        .ghost()
                        .icon(crate::IconName::ChevronDown)
                        .aria_label("More tabs")
                        .dropdown_menu(move |mut this, _, _| {
                            this = this.scrollable(true);
                            for (ix, meta) in item_metas.iter().enumerate() {
                                let base = if let Some(label) = meta.label.clone() {
                                    PopupMenuItem::new(label)
                                } else if let Some(icon) = meta.icon.clone() {
                                    PopupMenuItem::element(move |_, _| icon.clone())
                                } else {
                                    PopupMenuItem::new(t!("Dock.Unnamed"))
                                };
                                this = this.item(
                                    base.checked(selected_index == Some(ix))
                                        .disabled(meta.disabled)
                                        .when_some(meta.on_click.clone(), |this, handler| {
                                            this.on_click(move |event, window, cx| {
                                                handler(event, window, cx)
                                            })
                                        }),
                                );
                            }

                            this
                        })
                        .anchor(Anchor::TopRight),
                )
            })
            .when_some(self.suffix, |this, suffix| this.child(suffix))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_navigation_wraps_and_supports_boundaries() {
        assert_eq!(tab_bar_focus_target("left", 0, 3), Some(2));
        assert_eq!(tab_bar_focus_target("right", 2, 3), Some(0));
        assert_eq!(tab_bar_focus_target("home", 2, 3), Some(0));
        assert_eq!(tab_bar_focus_target("end", 0, 3), Some(2));
        assert_eq!(tab_bar_focus_target("up", 0, 3), None);
    }

    #[test]
    fn empty_navigation_has_no_target() {
        assert_eq!(tab_bar_focus_target("left", 0, 0), None);
    }
}
