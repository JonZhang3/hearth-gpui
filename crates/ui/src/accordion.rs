use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::Arc};

use gpui::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, Styled, Window, div,
    prelude::FluentBuilder as _, rems,
};

use crate::{
    ActiveTheme as _, FocusableExt as _, Icon, IconName, Sizable, Size, collapsible::Collapsible,
    h_flex, v_flex,
};

/// Returns whether the keyboard event should activate an accordion trigger.
fn is_toggle_key(event: &KeyDownEvent) -> bool {
    is_toggle_key_name(
        event.keystroke.key.as_str(),
        event.keystroke.modifiers.modified(),
    )
}

/// Matches the platform-independent activation keys for a button-like trigger.
fn is_toggle_key_name(key: &str, modified: bool) -> bool {
    !modified && matches!(key, "enter" | "space")
}

/// Accordion element.
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    multiple: bool,
    size: Size,
    bordered: bool,
    disabled: bool,
    children: Vec<AccordionItem>,
    on_toggle_click: Option<Arc<dyn Fn(&[usize], &mut Window, &mut App) + Send + Sync>>,
}

impl Accordion {
    /// Create a new Accordion with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            multiple: false,
            size: Size::default(),
            bordered: true,
            children: Vec::new(),
            disabled: false,
            on_toggle_click: None,
        }
    }

    /// Set whether multiple accordion items can be opened simultaneously, default: false
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Set whether the accordion items have borders, default: true
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set whether the accordion is disabled, default: false
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Adds an AccordionItem to the Accordion.
    pub fn item<F>(mut self, child: F) -> Self
    where
        F: FnOnce(AccordionItem) -> AccordionItem,
    {
        let item = child(AccordionItem::new());
        self.children.push(item);
        self
    }

    /// Sets the on_toggle_click callback for the AccordionGroup.
    ///
    /// The first argument `Vec<usize>` is the indices of the open accordions.
    pub fn on_toggle_click(
        mut self,
        on_toggle_click: impl Fn(&[usize], &mut Window, &mut App) + Send + Sync + 'static,
    ) -> Self {
        self.on_toggle_click = Some(Arc::new(on_toggle_click));
        self
    }
}

impl Sizable for Accordion {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Accordion {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let open_ixs = Rc::new(RefCell::new(HashSet::new()));
        let is_multiple = self.multiple;
        let accordion_id = self.id.clone();
        let on_group_toggle = self.on_toggle_click.filter(|_| !self.disabled);

        v_flex()
            .id(self.id)
            .size_full()
            .when(self.bordered, |this| this.gap_y_2())
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(ix, accordion)| {
                        if accordion.open {
                            open_ixs.borrow_mut().insert(ix);
                        }

                        accordion
                            .index(ix)
                            .motion_id(format!("{accordion_id}-item-{ix}"))
                            .with_size(self.size)
                            .bordered(self.bordered)
                            .disabled(self.disabled)
                            .on_toggle_click({
                                let open_ixs = Rc::clone(&open_ixs);
                                let on_group_toggle = on_group_toggle.clone();
                                move |open, window, cx| {
                                    let open_indices = {
                                        let mut open_ixs = open_ixs.borrow_mut();
                                        if *open {
                                            if !is_multiple {
                                                open_ixs.clear();
                                            }
                                            open_ixs.insert(ix);
                                        } else {
                                            open_ixs.remove(&ix);
                                        }

                                        let mut open_ixs: Vec<usize> =
                                            open_ixs.iter().copied().collect();
                                        open_ixs.sort_unstable();
                                        open_ixs
                                    };

                                    if let Some(on_group_toggle) = &on_group_toggle {
                                        on_group_toggle(&open_indices, window, cx);
                                    }
                                }
                            })
                    }),
            )
    }
}

/// An Accordion is a vertically stacked list of items, each of which can be expanded to reveal the content associated with it.
#[derive(IntoElement)]
pub struct AccordionItem {
    index: usize,
    motion_id: Option<ElementId>,
    icon: Option<Icon>,
    title: AnyElement,
    aria_label: Option<SharedString>,
    children: Vec<AnyElement>,
    open: bool,
    size: Size,
    bordered: bool,
    disabled: bool,
    on_toggle_click: Option<Arc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl AccordionItem {
    /// Create a new AccordionItem.
    pub fn new() -> Self {
        Self {
            index: 0,
            motion_id: None,
            icon: None,
            title: SharedString::default().into_any_element(),
            aria_label: None,
            children: Vec::new(),
            open: false,
            disabled: false,
            on_toggle_click: None,
            size: Size::default(),
            bordered: true,
        }
    }

    /// Set the icon for the accordion item.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the title for the accordion item.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = title.into_any_element();
        self
    }

    /// Set the accessible name for the accordion trigger.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    fn motion_id(mut self, id: impl Into<ElementId>) -> Self {
        self.motion_id = Some(id.into());
        self
    }

    fn on_toggle_click(
        mut self,
        on_toggle_click: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_click = Some(Arc::new(on_toggle_click));
        self
    }
}

impl ParentElement for AccordionItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for AccordionItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for AccordionItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_size = match self.size {
            Size::XSmall => rems(0.875),
            Size::Small => rems(0.875),
            _ => rems(1.0),
        };
        let disabled = self.disabled;
        let open = self.open;
        let focus_key = self
            .motion_id
            .clone()
            .unwrap_or_else(|| format!("accordion-item-{}", self.index).into());
        let focus_handle = window
            .use_keyed_state(format!("{focus_key}-focus"), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);
        let keyboard_toggle = self.on_toggle_click.clone();

        div().flex_1().child(
            v_flex()
                .w_full()
                .bg(cx.theme().tokens.accordion)
                .overflow_hidden()
                .when(self.bordered, |this| {
                    this.border_1()
                        .rounded(cx.theme().style.radii.md)
                        .border_color(cx.theme().border)
                })
                .text_size(text_size)
                .child(
                    h_flex()
                        .id(self.index)
                        .role(Role::Button)
                        .aria_expanded(open)
                        .when_some(self.aria_label, |this, label| this.aria_label(label))
                        .when(!disabled, |this| {
                            this.track_focus(&focus_handle.tab_stop(true))
                        })
                        .focus_ring_color(
                            is_focused,
                            cx.theme().style.focus.ring_offset,
                            cx.theme().ring,
                            window,
                            cx,
                        )
                        .justify_between()
                        .gap_3()
                        .map(|this| match self.size {
                            Size::XSmall => this.py_0().px_1p5(),
                            Size::Small => this.py_0p5().px_2(),
                            Size::Large => this.py_1p5().px_4(),
                            _ => this.py_1().px_3(),
                        })
                        .when(open, |this| {
                            this.when(self.bordered, |this| {
                                this.text_color(cx.theme().foreground)
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                            })
                        })
                        .when(!self.bordered, |this| {
                            this.border_b_1().border_color(cx.theme().border)
                        })
                        .child(
                            h_flex()
                                .items_center()
                                .map(|this| match self.size {
                                    Size::XSmall => this.gap_1(),
                                    Size::Small => this.gap_1(),
                                    _ => this.gap_2(),
                                })
                                .when_some(self.icon, |this, icon| {
                                    this.child(
                                        icon.with_size(self.size)
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                })
                                .child(self.title),
                        )
                        .when(!disabled, |this| {
                            this.hover(|this| this.bg(cx.theme().tokens.accordion_hover))
                                .child(
                                    Icon::new(if open {
                                        IconName::ChevronUp
                                    } else {
                                        IconName::ChevronDown
                                    })
                                    .xsmall()
                                    .text_color(cx.theme().muted_foreground),
                                )
                                .when_some(self.on_toggle_click, |this, on_toggle_click| {
                                    this.on_click({
                                        move |_, window, cx| {
                                            on_toggle_click(&!open, window, cx);
                                        }
                                    })
                                })
                        })
                        .when_some(keyboard_toggle.filter(|_| !disabled), |this, toggle| {
                            this.on_key_down(move |event, window, cx| {
                                if is_toggle_key(event) {
                                    window.prevent_default();
                                    cx.stop_propagation();
                                    toggle(&!open, window, cx);
                                }
                            })
                        }),
                )
                .child(
                    Collapsible::new()
                        .when_some(self.motion_id, |this, id| this.id(id))
                        .open(open)
                        .content(
                            div()
                                .map(|this| match self.size {
                                    Size::XSmall => this.p_1p5(),
                                    Size::Small => this.p_2(),
                                    Size::Large => this.p_4(),
                                    _ => this.p_3(),
                                })
                                .children(self.children),
                        ),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;
    use gpui::{AppContext as _, Context, Render, TestAppContext, VisualTestContext};

    struct KeyboardFixture {
        calls: Arc<AtomicUsize>,
        open_indices: Arc<Mutex<Vec<usize>>>,
    }

    impl Render for KeyboardFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let calls = Arc::clone(&self.calls);
            let open_indices = Arc::clone(&self.open_indices);

            Accordion::new("keyboard-accordion")
                .on_toggle_click(move |indices, _, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    *open_indices.lock().unwrap() = indices.to_vec();
                })
                .item(|item| {
                    item.title("Keyboard section")
                        .aria_label("Keyboard section")
                        .child("Keyboard content")
                })
        }
    }

    #[test]
    fn toggle_key_matcher_accepts_unmodified_enter_and_space() {
        assert!(is_toggle_key_name("enter", false));
        assert!(is_toggle_key_name("space", false));
        assert!(!is_toggle_key_name("escape", false));
        assert!(!is_toggle_key_name("enter", true));
        assert!(!is_toggle_key_name("space", true));
    }

    #[gpui::test]
    fn focus_navigation_and_space_activate_the_trigger_once(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let open_indices = Arc::new(Mutex::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let calls = Arc::clone(&calls);
            let open_indices = Arc::clone(&open_indices);
            move |window, cx| {
                let fixture = cx.new(|_| KeyboardFixture {
                    calls,
                    open_indices,
                });
                crate::Root::new(fixture, window, cx)
            }
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
        });
        cx.simulate_keystrokes("space");
        cx.run_until_parked();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(*open_indices.lock().unwrap(), vec![0]);
    }
}
