// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `single`, `collapsible`, `framed`, `default_open_values`, `open_values`,
//   `on_open_change`, `aria_label`.
// - Removed public methods: `new`, `bordered`, `on_toggle_click`, `open`.
// - Added or exposed behavior through `normalize`, `is_toggle_key`, `is_toggle_key_name`,
//   `toggled_values`, `ordered_values`, `validate_configuration`, `single`, `collapsible` and 13
//   more.
// - Removed or replaced `bordered`, `on_toggle_click`, `with_size`, `open`, `index`.
// - Reworked Accordion around accessibility semantics and ARIA state, interruptible and
//   reduced-motion-aware transitions, semantic Style Preset geometry and density, keyboard
//   navigation and activation behavior, focus-visible and focus restoration behavior, invalid and
//   validation state handling.
use std::{collections::HashSet, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, FocusHandle, InteractiveElement as _, IntoElement, KeyDownEvent,
    MouseButton, ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    Styled, Window, div, prelude::FluentBuilder as _,
};

use crate::{
    ActiveTheme as _, FocusableExt as _, Icon, IconName, Sizable as _, Size, StyledExt as _,
    collapsible::Collapsible, h_flex, v_flex,
};

/// The selection behavior owned by an Accordion group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccordionMode {
    Single,
    Multiple,
}

type OpenChangeCallback = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;

/// Persistent state for an uncontrolled Accordion.
#[derive(Clone, Debug)]
struct AccordionState {
    open_values: Vec<SharedString>,
}

impl AccordionState {
    /// Removes deleted items and restores declaration order after the item set changes.
    fn normalize(&mut self, item_values: &[SharedString], mode: AccordionMode) {
        self.open_values = ordered_values(item_values, &self.open_values);
        if mode == AccordionMode::Single {
            self.open_values.truncate(1);
        }
    }
}

/// Returns whether the keyboard event should activate an Accordion trigger.
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

/// Resolves the next open values for one user-triggered state change.
fn toggled_values(
    current: &[SharedString],
    value: &SharedString,
    item_values: &[SharedString],
    mode: AccordionMode,
    collapsible: bool,
) -> Vec<SharedString> {
    let is_open = current.contains(value);
    match mode {
        AccordionMode::Single if is_open && collapsible => Vec::new(),
        AccordionMode::Single if is_open => current.to_vec(),
        AccordionMode::Single => vec![value.clone()],
        AccordionMode::Multiple => {
            let mut proposed = current.to_vec();
            if is_open {
                proposed.retain(|candidate| candidate != value);
            } else {
                proposed.push(value.clone());
            }
            ordered_values(item_values, &proposed)
        }
    }
}

/// Orders a value set by item declaration order and removes duplicates.
fn ordered_values(item_values: &[SharedString], values: &[SharedString]) -> Vec<SharedString> {
    item_values
        .iter()
        .filter(|value| values.contains(value))
        .cloned()
        .collect()
}

/// Validates the static item identity and configured open-value contract.
fn validate_configuration(
    item_values: &[SharedString],
    open_values: &[SharedString],
    mode: AccordionMode,
) {
    let mut unique_items = HashSet::new();
    for value in item_values {
        assert!(
            !value.trim().is_empty(),
            "Accordion item values cannot be empty"
        );
        assert!(
            unique_items.insert(value.clone()),
            "Accordion item value '{value}' is duplicated"
        );
    }

    let mut unique_open_values = HashSet::new();
    for value in open_values {
        assert!(
            unique_open_values.insert(value.clone()),
            "Accordion open value '{value}' is duplicated"
        );
        assert!(
            unique_items.contains(value),
            "Accordion open value '{value}' does not match an item"
        );
    }

    assert!(
        mode == AccordionMode::Multiple || open_values.len() <= 1,
        "A single Accordion accepts at most one open value"
    );
}

/// A vertically stacked group of disclosure items.
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    mode: AccordionMode,
    collapsible: bool,
    framed: Option<bool>,
    disabled: bool,
    default_open_values: Vec<SharedString>,
    open_values: Option<Vec<SharedString>>,
    children: Vec<AccordionItem>,
    on_open_change: Option<OpenChangeCallback>,
}

impl Accordion {
    /// Creates a single-selection Accordion.
    pub fn single(id: impl Into<ElementId>) -> Self {
        Self::new(id, AccordionMode::Single)
    }

    /// Creates a multiple-selection Accordion.
    pub fn multiple(id: impl Into<ElementId>) -> Self {
        Self::new(id, AccordionMode::Multiple)
    }

    fn new(id: impl Into<ElementId>, mode: AccordionMode) -> Self {
        Self {
            id: id.into(),
            mode,
            collapsible: true,
            framed: None,
            disabled: false,
            default_open_values: Vec::new(),
            open_values: None,
            children: Vec::new(),
            on_open_change: None,
        }
    }

    /// Sets whether an open item in a single Accordion may be closed.
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// Overrides the Style Preset's default framed appearance.
    pub fn framed(mut self, framed: bool) -> Self {
        self.framed = Some(framed);
        self
    }

    /// Sets whether every item in the Accordion is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the initial values for an uncontrolled Accordion.
    pub fn default_open_values(
        mut self,
        values: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.default_open_values = values.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the values for a controlled Accordion.
    pub fn open_values(
        mut self,
        values: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.open_values = Some(values.into_iter().map(Into::into).collect());
        self
    }

    /// Adds an item with a stable value that identifies it across reordering.
    pub fn item(
        mut self,
        value: impl Into<SharedString>,
        builder: impl FnOnce(AccordionItem) -> AccordionItem,
    ) -> Self {
        self.children
            .push(builder(AccordionItem::new(value.into())));
        self
    }

    /// Sets the callback invoked with the proposed open values after user input.
    pub fn on_open_change(
        mut self,
        callback: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(callback));
        self
    }
}

impl RenderOnce for Accordion {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let item_values = self
            .children
            .iter()
            .map(|item| item.value.clone())
            .collect::<Vec<_>>();
        let configured_values = self
            .open_values
            .as_ref()
            .unwrap_or(&self.default_open_values);
        validate_configuration(&item_values, configured_values, self.mode);

        let state_key = format!("{}-accordion-state", self.id);
        let default_open_values = self.default_open_values.clone();
        let state = window.use_keyed_state(state_key, cx, |_, _| AccordionState {
            open_values: default_open_values,
        });
        state.update(cx, |state, _| state.normalize(&item_values, self.mode));

        let visual_open_values = self
            .open_values
            .clone()
            .unwrap_or_else(|| state.read(cx).open_values.clone());
        let metrics = cx.theme().style.disclosure;
        let framed = self.framed.unwrap_or(metrics.framed_by_default);
        let parent_view_id = window.current_view();

        let focus_entries = self
            .children
            .iter()
            .map(|item| {
                let focus_key = format!("{}-accordion-item-{}-focus", self.id, item.value);
                let handle = window
                    .use_keyed_state(focus_key, cx, |_, cx| cx.focus_handle())
                    .read(cx)
                    .clone();
                (item.value.clone(), handle, self.disabled || item.disabled)
            })
            .collect::<Vec<_>>();
        let enabled_focus_handles = focus_entries
            .iter()
            .filter(|(_, _, disabled)| !disabled)
            .map(|(_, handle, _)| handle.clone())
            .collect::<Vec<_>>();

        let item_count = self.children.len();
        let accordion_id = self.id.clone();
        let group = v_flex()
            .id(self.id.clone())
            .w_full()
            .bg(cx.theme().tokens.accordion)
            .when(framed, |this| {
                this.border_1()
                    .border_color(cx.theme().border)
                    .rounded(metrics.frame_radius)
            })
            .children(self.children.into_iter().enumerate().map(|(index, item)| {
                let value = item.value.clone();
                let open = visual_open_values.contains(&value);
                let disabled = self.disabled || item.disabled;
                let focus_handle = focus_entries[index].1.clone();
                let enabled_position = enabled_focus_handles
                    .iter()
                    .position(|handle| handle == &focus_handle);
                let controlled_values = self.open_values.clone();
                let state = state.clone();
                let item_values = item_values.clone();
                let callback = self.on_open_change.clone();
                let mode = self.mode;
                let collapsible = self.collapsible;
                let item_id = format!("{accordion_id}-accordion-item-{value}");

                item.render(
                    item_id,
                    index == 0,
                    index + 1 == item_count,
                    open,
                    disabled,
                    framed,
                    metrics,
                    focus_handle,
                    enabled_focus_handles.clone(),
                    enabled_position,
                    move |window, cx| {
                        let proposed = if let Some(current) = &controlled_values {
                            let proposed =
                                toggled_values(current, &value, &item_values, mode, collapsible);
                            (proposed != *current).then_some(proposed)
                        } else {
                            state.update(cx, |state, _| {
                                let proposed = toggled_values(
                                    &state.open_values,
                                    &value,
                                    &item_values,
                                    mode,
                                    collapsible,
                                );
                                if proposed == state.open_values {
                                    return None;
                                }
                                state.open_values = proposed.clone();
                                Some(proposed)
                            })
                        };

                        let Some(proposed) = proposed else {
                            return;
                        };

                        if let Some(callback) = &callback {
                            callback(&proposed, window, cx);
                        }
                        cx.notify(parent_view_id);
                    },
                    window,
                    cx,
                )
            }));

        group
    }
}

/// One item contained by an [`Accordion`].
pub struct AccordionItem {
    value: SharedString,
    icon: Option<Icon>,
    title: AnyElement,
    aria_label: Option<SharedString>,
    children: Vec<AnyElement>,
    disabled: bool,
}

impl AccordionItem {
    fn new(value: SharedString) -> Self {
        Self {
            value,
            icon: None,
            title: SharedString::default().into_any_element(),
            aria_label: None,
            children: Vec::new(),
            disabled: false,
        }
    }

    /// Sets the optional leading icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the trigger title.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = title.into_any_element();
        self
    }

    /// Sets an explicit accessible name for a custom trigger title.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets whether this item is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[allow(clippy::too_many_arguments)]
    fn render(
        self,
        item_id: impl Into<ElementId>,
        first: bool,
        last: bool,
        open: bool,
        disabled: bool,
        framed: bool,
        metrics: crate::theme::DisclosureMetrics,
        focus_handle: FocusHandle,
        enabled_focus_handles: Vec<FocusHandle>,
        enabled_position: Option<usize>,
        on_toggle: impl Fn(&mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let item_id = item_id.into();
        let motion_id = format!("{item_id}-motion");
        let is_focused = focus_handle.is_focused(window);
        let keyboard_toggle = Rc::new(on_toggle);
        let click_toggle = Rc::clone(&keyboard_toggle);
        let indicator_size = Size::Size(metrics.indicator_size);
        let trigger_id = format!("{item_id}-trigger");

        let trigger = h_flex()
            .id(trigger_id.clone())
            .debug_selector(move || trigger_id)
            .relative()
            .w_full()
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
            .gap(metrics.title_gap)
            .px(metrics.trigger_padding_x)
            .py(metrics.trigger_padding_y)
            .rounded(metrics.trigger_radius)
            .text_sm()
            .font_medium()
            .when(!disabled, |this| {
                this.cursor_pointer()
                    .hover(|this| this.text_decoration_1())
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        // Pointer activation must not produce a keyboard focus-visible ring.
                        window.prevent_default();
                        crate::global_state::GlobalState::suppress_text_selection(cx);
                    })
                    .on_click(move |_, window, cx| click_toggle(window, cx))
            })
            .when(disabled, |this| this.opacity(0.5))
            .child(
                h_flex()
                    .items_center()
                    .gap(metrics.title_gap)
                    .when_some(self.icon, |this, icon| {
                        this.child(
                            icon.with_size(indicator_size)
                                .text_color(cx.theme().muted_foreground),
                        )
                    })
                    .child(self.title),
            )
            .child(
                Icon::new(if open {
                    IconName::ChevronUp
                } else {
                    IconName::ChevronDown
                })
                .with_size(indicator_size)
                .text_color(cx.theme().muted_foreground),
            )
            .when(!disabled, |this| {
                this.on_key_down(move |event, window, cx| {
                    if is_toggle_key(event) {
                        window.prevent_default();
                        cx.stop_propagation();
                        keyboard_toggle(window, cx);
                        return;
                    }
                    if event.keystroke.modifiers.modified() {
                        return;
                    }

                    let Some(position) = enabled_position else {
                        return;
                    };
                    let target = match event.keystroke.key.as_str() {
                        "down" => {
                            enabled_focus_handles.get((position + 1) % enabled_focus_handles.len())
                        }
                        "up" => enabled_focus_handles.get(
                            (position + enabled_focus_handles.len() - 1)
                                % enabled_focus_handles.len(),
                        ),
                        "home" => enabled_focus_handles.first(),
                        "end" => enabled_focus_handles.last(),
                        _ => None,
                    };
                    if let Some(target) = target {
                        window.prevent_default();
                        cx.stop_propagation();
                        target.focus(window, cx);
                    }
                })
            });
        let trigger = crate::accessibility::accessibility_state(trigger, false, false, disabled);

        v_flex()
            .w_full()
            .when(!last, |this| {
                this.border_b_1().border_color(cx.theme().border)
            })
            .when(framed && open && metrics.open_tint, |this| {
                this.bg(cx.theme().muted.opacity(0.5))
            })
            .when(framed && first, |this| {
                this.rounded_tl(metrics.frame_radius)
                    .rounded_tr(metrics.frame_radius)
            })
            .when(framed && last, |this| {
                this.rounded_bl(metrics.frame_radius)
                    .rounded_br(metrics.frame_radius)
            })
            .child(trigger)
            .child(
                Collapsible::new().id(motion_id).open(open).content(
                    div()
                        .w_full()
                        .px(metrics.content_padding_x)
                        .pb(metrics.content_padding_bottom)
                        .text_sm()
                        .when(disabled, |this| this.opacity(0.5))
                        .children(self.children),
                ),
            )
            .into_any_element()
    }
}

impl ParentElement for AccordionItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;
    use gpui::{AppContext as _, Context, Modifiers, Render, TestAppContext, VisualTestContext};

    #[test]
    fn toggle_key_matcher_accepts_unmodified_enter_and_space() {
        assert!(is_toggle_key_name("enter", false));
        assert!(is_toggle_key_name("space", false));
        assert!(!is_toggle_key_name("escape", false));
        assert!(!is_toggle_key_name("enter", true));
    }

    #[test]
    fn single_and_multiple_state_transitions_preserve_item_order() {
        let values = vec!["shipping".into(), "returns".into(), "support".into()];

        assert_eq!(
            toggled_values(&[], &"returns".into(), &values, AccordionMode::Single, true),
            vec![SharedString::from("returns")]
        );
        assert_eq!(
            toggled_values(
                &["returns".into()],
                &"returns".into(),
                &values,
                AccordionMode::Single,
                false,
            ),
            vec![SharedString::from("returns")]
        );
        assert_eq!(
            toggled_values(
                &["support".into()],
                &"shipping".into(),
                &values,
                AccordionMode::Multiple,
                true,
            ),
            vec![
                SharedString::from("shipping"),
                SharedString::from("support")
            ]
        );
    }

    #[test]
    #[should_panic(expected = "is duplicated")]
    fn duplicate_item_values_are_rejected() {
        validate_configuration(
            &["item".into(), "item".into()],
            &[],
            AccordionMode::Multiple,
        );
    }

    #[test]
    #[should_panic(expected = "at most one open value")]
    fn single_accordion_rejects_multiple_open_values() {
        validate_configuration(
            &["first".into(), "second".into()],
            &["first".into(), "second".into()],
            AccordionMode::Single,
        );
    }

    struct KeyboardFixture {
        calls: Arc<AtomicUsize>,
        open_values: Arc<Mutex<Vec<SharedString>>>,
        controlled: bool,
    }

    impl Render for KeyboardFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let calls = Arc::clone(&self.calls);
            let open_values = Arc::clone(&self.open_values);

            let accordion = Accordion::single("keyboard-accordion")
                .on_open_change(move |values, _, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    *open_values.lock().unwrap() = values.to_vec();
                })
                .item("section", |item| {
                    item.title("Keyboard section")
                        .aria_label("Keyboard section")
                        .child("Keyboard content")
                });

            if self.controlled {
                accordion.open_values(std::iter::empty::<&str>())
            } else {
                accordion
            }
        }
    }

    #[gpui::test]
    fn focus_navigation_and_space_activate_the_trigger_once(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let open_values = Arc::new(Mutex::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let calls = Arc::clone(&calls);
            let open_values = Arc::clone(&open_values);
            move |window, cx| {
                let fixture = cx.new(|_| KeyboardFixture {
                    calls,
                    open_values,
                    controlled: true,
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
        assert_eq!(
            *open_values.lock().unwrap(),
            vec![SharedString::from("section")]
        );
    }

    #[gpui::test]
    fn pointer_activation_does_not_create_a_focus_ring(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let open_values = Arc::new(Mutex::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let calls = Arc::clone(&calls);
            let open_values = Arc::clone(&open_values);
            move |window, cx| {
                let fixture = cx.new(|_| KeyboardFixture {
                    calls,
                    open_values,
                    controlled: true,
                });
                crate::Root::new(fixture, window, cx)
            }
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        let bounds = cx
            .debug_bounds("keyboard-accordion-accordion-item-section-trigger")
            .expect("Accordion trigger bounds should be available after drawing");
        cx.simulate_click(bounds.center(), Modifiers::default());
        cx.run_until_parked();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(cx.update(|window, cx| window.focused(cx).is_none()));
    }

    #[gpui::test]
    fn uncontrolled_state_persists_across_renders(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let open_values = Arc::new(Mutex::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let calls = Arc::clone(&calls);
            let open_values = Arc::clone(&open_values);
            move |window, cx| {
                let fixture = cx.new(|_| KeyboardFixture {
                    calls,
                    open_values,
                    controlled: false,
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
        cx.simulate_keystrokes("space");
        cx.run_until_parked();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(open_values.lock().unwrap().is_empty());
    }

    struct NavigationFixture {
        open_values: Arc<Mutex<Vec<SharedString>>>,
    }

    impl Render for NavigationFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let open_values = Arc::clone(&self.open_values);
            Accordion::single("navigation-accordion")
                .on_open_change(move |values, _, _| {
                    *open_values.lock().unwrap() = values.to_vec();
                })
                .item("first", |item| item.title("First").child("First content"))
                .item("disabled", |item| {
                    item.disabled(true)
                        .title("Disabled")
                        .child("Disabled content")
                })
                .item("third", |item| item.title("Third").child("Third content"))
        }
    }

    #[gpui::test]
    fn group_navigation_wraps_and_skips_disabled_items(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let open_values = Arc::new(Mutex::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let open_values = Arc::clone(&open_values);
            move |window, cx| {
                let fixture = cx.new(|_| NavigationFixture { open_values });
                crate::Root::new(fixture, window, cx)
            }
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
        });
        cx.simulate_keystrokes("down space");
        cx.run_until_parked();

        assert_eq!(
            *open_values.lock().unwrap(),
            vec![SharedString::from("third")]
        );

        cx.simulate_keystrokes("home space");
        cx.run_until_parked();
        assert_eq!(
            *open_values.lock().unwrap(),
            vec![SharedString::from("first")]
        );

        cx.simulate_keystrokes("up space");
        cx.run_until_parked();
        assert_eq!(
            *open_values.lock().unwrap(),
            vec![SharedString::from("third")]
        );

        cx.simulate_keystrokes("home space");
        cx.run_until_parked();
        assert_eq!(
            *open_values.lock().unwrap(),
            vec![SharedString::from("first")]
        );

        cx.simulate_keystrokes("end space");
        cx.run_until_parked();
        assert_eq!(
            *open_values.lock().unwrap(),
            vec![SharedString::from("third")]
        );
    }
}
