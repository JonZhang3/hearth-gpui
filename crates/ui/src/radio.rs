use std::{rc::Rc, sync::Arc};

use crate::{
    ActiveTheme, Disableable, Sizable, Size, StyledExt, text::Text, theme::Density,
    tooltip::ComponentTooltip, v_flex,
};
use gpui::{
    AnyElement, App, Axis, Div, ElementId, FocusHandle, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Toggled, Window, accesskit, div, prelude::FluentBuilder, px, relative,
};

/// Creates a structural child ID without flattening the caller's [`ElementId`].
fn radio_child_id(id: &ElementId, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), name.into())
}

/// Resolves the next enabled item for the RadioGroup roving-focus model.
fn radio_group_focus_target(
    key: &str,
    orientation: Axis,
    position: usize,
    len: usize,
) -> Option<usize> {
    if len == 0 || position >= len {
        return None;
    }

    match key {
        "home" => Some(0),
        "end" => Some(len - 1),
        "left" if orientation == Axis::Horizontal => Some((position + len - 1) % len),
        "right" if orientation == Axis::Horizontal => Some((position + 1) % len),
        "up" if orientation == Axis::Vertical => Some((position + len - 1) % len),
        "down" if orientation == Axis::Vertical => Some((position + 1) % len),
        _ => None,
    }
}

/// Returns the shadcn-aligned Radio geometry for the optional project size extension.
fn radio_edge(size: Size) -> gpui::Pixels {
    match size {
        Size::XSmall => px(12.),
        Size::Small => px(14.),
        Size::Medium => px(16.),
        Size::Large => px(18.),
        Size::Size(edge) => edge,
    }
}

/// A controlled Radio control with optional integrated label and description content.
///
/// Use [`RadioGroupItem`] inside a [`RadioGroup`] when options are mutually exclusive.
#[derive(IntoElement)]
pub struct Radio {
    base: Div,
    style: StyleRefinement,
    id: ElementId,
    label: Option<Text>,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
    children: Vec<AnyElement>,
    checked: bool,
    invalid: bool,
    disabled: bool,
    tab_stop: bool,
    tab_index: isize,
    size: Size,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    on_key_down: Option<Rc<dyn Fn(&KeyDownEvent, &mut Window, &mut App) + 'static>>,
    focus_handle: Option<FocusHandle>,
    tooltip: ComponentTooltip,
    position_in_set: Option<usize>,
    size_of_set: Option<usize>,
}

impl Radio {
    /// Creates a controlled Radio with a stable element ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style: StyleRefinement::default(),
            label: None,
            aria_label: None,
            aria_description: None,
            children: Vec::new(),
            checked: false,
            invalid: false,
            disabled: false,
            tab_index: 0,
            tab_stop: true,
            size: Size::default(),
            on_click: None,
            on_key_down: None,
            focus_handle: None,
            tooltip: ComponentTooltip::default(),
            position_in_set: None,
            size_of_set: None,
        }
    }

    /// Sets tooltip text for the Radio.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip.text = Some((tooltip.into(), None));
        self
    }

    /// Sets the visible label and uses its text as the accessible name.
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets an accessible name independently from visible content.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets an accessible description for supplemental option content.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Sets the controlled selected state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Sets the invalid accessibility and destructive-ring state.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets whether the Radio is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the keyboard tab index.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Sets whether this Radio participates in sequential keyboard focus.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Sets the activation handler. Radio activation always requests `true`.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Injects the focus handle owned by a RadioGroup.
    fn focus_handle(mut self, focus_handle: FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle);
        self
    }

    /// Injects RadioGroup roving-focus keyboard behavior.
    fn on_group_key_down(
        mut self,
        handler: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_key_down = Some(Rc::new(handler));
        self
    }
}

impl Sizable for Radio {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Radio {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for Radio {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for Radio {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Radio {}

impl ParentElement for Radio {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Radio {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let disabled = self.disabled;
        let invalid = self.invalid;
        let has_content = self.label.is_some() || !self.children.is_empty();
        let focus_handle = self.focus_handle.clone().unwrap_or_else(|| {
            window
                .use_keyed_state(radio_child_id(&self.id, "focus"), cx, |_, cx| {
                    cx.focus_handle()
                })
                .read(cx)
                .clone()
        });
        let focus_visible = focus_handle.is_focused(window) && window.last_input_was_keyboard();
        let tracking_focus_handle = focus_handle.clone();
        let edge = radio_edge(self.size);
        let indicator_edge = edge * 0.5;
        let ring_width = cx.theme().style.focus.ring_width;
        let ring_inset = ring_width + cx.theme().style.focus.ring_offset;
        let invalid_border = cx
            .theme()
            .danger
            .opacity(if cx.theme().is_dark() { 0.5 } else { 1. });
        let border = if invalid && !checked {
            invalid_border
        } else if focus_visible {
            cx.theme().ring
        } else if checked {
            cx.theme().primary
        } else {
            cx.theme().input
        };
        let ring_color = if invalid {
            cx.theme()
                .danger
                .opacity(if cx.theme().is_dark() { 0.4 } else { 0.2 })
        } else {
            cx.theme().ring.opacity(0.5)
        };
        let background = if checked {
            cx.theme().tokens.primary.background
        } else if cx.theme().is_dark() {
            cx.theme().input_background().into()
        } else {
            cx.theme().transparent.into()
        };
        let accessible_label = self
            .aria_label
            .clone()
            .or_else(|| self.label.as_ref().map(|label| label.get_text(cx)))
            .or_else(|| self.tooltip.text.as_ref().map(|(text, _)| text.clone()));
        let on_click = self.on_click.clone();

        let ring = (invalid || focus_visible).then(|| {
            div()
                .absolute()
                .top(-ring_inset)
                .right(-ring_inset)
                .bottom(-ring_inset)
                .left(-ring_inset)
                .border(ring_width)
                .border_color(ring_color)
                .rounded_full()
        });
        let control = div()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .size(edge)
            .flex_shrink_0()
            .border_1()
            .border_color(border)
            .bg(background)
            .rounded_full()
            .when(has_content, |this| this.mt_px())
            .when_some(ring, |this, ring| this.child(ring))
            // The pinned shadcn source mounts the indicator atomically without a transition.
            .when(checked, |this| {
                this.child(
                    div()
                        .size(indicator_edge)
                        .rounded_full()
                        .bg(cx.theme().primary_foreground),
                )
            });

        let element = self
            .base
            .id(self.id.clone())
            .role(Role::RadioButton)
            .aria_toggled(if checked {
                Toggled::True
            } else {
                Toggled::False
            })
            .when_some(accessible_label, |this, label| this.aria_label(label))
            .when_some(self.aria_description, |this, description| {
                this.aria_description(description)
            })
            .when_some(self.position_in_set, |this, position| {
                this.aria_position_in_set(position)
            })
            .when_some(self.size_of_set, |this, size| this.aria_size_of_set(size))
            .when(!disabled, |this| {
                this.track_focus(
                    &tracking_focus_handle
                        .tab_stop(self.tab_stop)
                        .tab_index(self.tab_index),
                )
            })
            .h_flex()
            .gap_3()
            .when(has_content, |this| this.items_start())
            .when(!has_content, |this| this.items_center())
            .text_sm()
            .line_height(relative(1.))
            .text_color(cx.theme().foreground)
            .when(disabled, |this| this.opacity(0.5))
            .refine_style(&self.style)
            .child(control)
            .when(has_content, |this| {
                this.child(
                    v_flex()
                        .flex_1()
                        .overflow_hidden()
                        .line_height(relative(1.2))
                        .gap_1()
                        .when_some(self.label, |this, label| {
                            this.child(
                                div()
                                    .size_full()
                                    .font_medium()
                                    .line_height(relative(1.))
                                    .child(label),
                            )
                        })
                        .children(self.children),
                )
            })
            .when(!disabled, |this| {
                let focus_handle = focus_handle.clone();
                this.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    crate::global_state::GlobalState::suppress_text_selection(cx);
                    focus_handle.focus(window, cx);
                })
            })
            .when(!disabled, |this| {
                this.on_click(move |_, window, cx| {
                    window.prevent_default();
                    if let Some(on_click) = on_click.as_ref() {
                        on_click(&true, window, cx);
                    }
                })
            })
            .when_some(self.on_key_down, |this, on_key_down| {
                this.on_key_down(move |event, window, cx| on_key_down(event, window, cx))
            })
            .map(|this| self.tooltip.apply(&self.id, this));

        crate::accessibility::accessibility_state(element, invalid, false, disabled)
    }
}

/// A typed, value-bearing item owned by a [`RadioGroup`].
pub struct RadioGroupItem {
    value: SharedString,
    radio: Radio,
}

impl RadioGroupItem {
    /// Creates an item whose stable value is also used as its default element ID.
    pub fn new(value: impl Into<SharedString>) -> Self {
        let value = value.into();
        Self {
            radio: Radio::new(value.clone()),
            value,
        }
    }

    /// Sets a visible label and accessible name.
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.radio = self.radio.label(label);
        self
    }

    /// Sets an accessible name independently from visible content.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.radio = self.radio.aria_label(label);
        self
    }

    /// Sets an accessible description for supplemental content.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.radio = self.radio.aria_description(description);
        self
    }

    /// Sets tooltip text for the item.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.radio = self.radio.tooltip(tooltip);
        self
    }

    /// Sets the invalid state for this item.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.radio = self.radio.invalid(invalid);
        self
    }

    /// Adds an item-level activation side effect without replacing group selection handling.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.radio = self.radio.on_click(handler);
        self
    }
}

impl ParentElement for RadioGroupItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.radio.extend(elements);
    }
}

impl Disableable for RadioGroupItem {
    fn disabled(mut self, disabled: bool) -> Self {
        self.radio = self.radio.disabled(disabled);
        self
    }
}

impl Sizable for RadioGroupItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.radio = self.radio.with_size(size);
        self
    }
}

impl Styled for RadioGroupItem {
    fn style(&mut self) -> &mut StyleRefinement {
        self.radio.style()
    }
}

impl From<&'static str> for RadioGroupItem {
    fn from(value: &'static str) -> Self {
        Self::new(value).label(value)
    }
}

impl From<SharedString> for RadioGroupItem {
    fn from(value: SharedString) -> Self {
        Self::new(value.clone()).label(value)
    }
}

impl From<String> for RadioGroupItem {
    fn from(value: String) -> Self {
        let value = SharedString::from(value);
        Self::new(value.clone()).label(value)
    }
}

/// A controlled, mutually exclusive group of [`RadioGroupItem`] values.
#[derive(IntoElement)]
pub struct RadioGroup {
    id: ElementId,
    style: StyleRefinement,
    items: Vec<RadioGroupItem>,
    orientation: Axis,
    value: Option<SharedString>,
    disabled: bool,
    aria_label: Option<SharedString>,
    on_change: Option<Rc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
}

impl RadioGroup {
    /// Creates a vertical RadioGroup.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            items: Vec::new(),
            orientation: Axis::Vertical,
            value: None,
            disabled: false,
            aria_label: None,
            on_change: None,
        }
    }

    /// Creates a vertical RadioGroup.
    pub fn vertical(id: impl Into<ElementId>) -> Self {
        Self::new(id)
    }

    /// Creates a horizontal RadioGroup.
    pub fn horizontal(id: impl Into<ElementId>) -> Self {
        Self::new(id).orientation(Axis::Horizontal)
    }

    /// Sets the visual and keyboard orientation.
    pub fn orientation(mut self, orientation: Axis) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets the controlled selected value.
    pub fn value(mut self, value: Option<impl Into<SharedString>>) -> Self {
        self.value = value.map(Into::into);
        self
    }

    /// Sets the accessible name of the RadioGroup.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets the callback invoked when a different enabled value is selected.
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Adds a typed RadioGroup item.
    pub fn child(mut self, item: impl Into<RadioGroupItem>) -> Self {
        self.items.push(item.into());
        self
    }

    /// Adds typed RadioGroup items.
    pub fn children(mut self, items: impl IntoIterator<Item = impl Into<RadioGroupItem>>) -> Self {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }
}

impl Disableable for RadioGroup {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for RadioGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for RadioGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let item_count = self.items.len();
        let item_ids = self
            .items
            .iter()
            .map(|item| radio_child_id(&self.id, item.value.clone()))
            .collect::<Vec<_>>();
        let item_focus_handles = self
            .items
            .iter()
            .enumerate()
            .map(|(index, _)| {
                window
                    .use_keyed_state(radio_child_id(&item_ids[index], "focus"), cx, |_, cx| {
                        cx.focus_handle()
                    })
                    .read(cx)
                    .clone()
            })
            .collect::<Vec<_>>();
        let enabled_indexes = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (!self.disabled && !item.radio.disabled).then_some(index))
            .collect::<Vec<_>>();
        let mut enabled_positions = vec![None; item_count];
        for (position, index) in enabled_indexes.iter().copied().enumerate() {
            enabled_positions[index] = Some(position);
        }
        let preferred_tab_index = enabled_indexes
            .iter()
            .copied()
            .find(|index| self.value.as_ref() == Some(&self.items[*index].value))
            .or_else(|| enabled_indexes.first().copied());
        let enabled_focus_handles = Rc::new(
            enabled_indexes
                .iter()
                .map(|index| item_focus_handles[*index].clone())
                .collect::<Vec<_>>(),
        );
        let enabled_values = Rc::new(
            enabled_indexes
                .iter()
                .map(|index| self.items[*index].value.clone())
                .collect::<Vec<_>>(),
        );
        let gap = match cx.theme().style.density {
            Density::Compact => px(8.),
            Density::Standard | Density::Comfortable => px(12.),
        };
        let orientation = self.orientation;
        let disabled = self.disabled;
        let selected_value = self.value.clone();
        let on_change = self.on_change.clone();
        let mut group = div()
            .id(self.id.clone())
            .role(Role::RadioGroup)
            .aria_orientation(if orientation == Axis::Horizontal {
                accesskit::Orientation::Horizontal
            } else {
                accesskit::Orientation::Vertical
            })
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .w_full()
            .flex()
            .when(orientation == Axis::Horizontal, |this| {
                this.flex_row().items_center().flex_wrap()
            })
            .when(orientation == Axis::Vertical, |this| this.flex_col())
            .gap(gap)
            .refine_style(&self.style);

        group = group.children(self.items.into_iter().enumerate().map(|(index, mut item)| {
            let value = item.value.clone();
            let checked = selected_value.as_ref() == Some(&value);
            let item_disabled = disabled || item.radio.disabled;
            item.radio.id = item_ids[index].clone();
            let item_handler = item.radio.on_click.take();
            let keyboard_group_handler = on_change.clone();
            let click_group_handler = on_change.clone();
            let keyboard_current_value = selected_value.clone();
            let click_current_value = selected_value.clone();
            let focus_handle = item_focus_handles[index].clone();
            let enabled_focus_handles = enabled_focus_handles.clone();
            let enabled_values = enabled_values.clone();
            let enabled_position = enabled_positions[index];

            item.radio
                .checked(checked)
                .disabled(item_disabled)
                .tab_stop(preferred_tab_index == Some(index))
                .focus_handle(focus_handle)
                .on_group_key_down(move |event, window, cx| {
                    if event.keystroke.modifiers.modified() {
                        return;
                    }
                    let Some(position) = enabled_position else {
                        return;
                    };
                    let Some(target_position) = radio_group_focus_target(
                        event.keystroke.key.as_str(),
                        orientation,
                        position,
                        enabled_focus_handles.len(),
                    ) else {
                        return;
                    };
                    let Some(target_focus) = enabled_focus_handles.get(target_position) else {
                        return;
                    };
                    let Some(target_value) = enabled_values.get(target_position) else {
                        return;
                    };

                    window.prevent_default();
                    cx.stop_propagation();
                    target_focus.focus(window, cx);
                    if keyboard_current_value.as_ref() != Some(target_value)
                        && let Some(group_handler) = keyboard_group_handler.as_ref()
                    {
                        group_handler(target_value, window, cx);
                    }
                })
                .on_click(move |selected, window, cx| {
                    if let Some(item_handler) = item_handler.as_ref() {
                        item_handler(selected, window, cx);
                    }
                    if click_current_value.as_ref() != Some(&value)
                        && let Some(group_handler) = click_group_handler.as_ref()
                    {
                        group_handler(&value, window, cx);
                    }
                })
                .map(|mut radio| {
                    radio.position_in_set = Some(index + 1);
                    radio.size_of_set = Some(item_count);
                    radio
                })
        }));

        crate::accessibility::accessibility_state(group, false, false, disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementExt as _;
    use gpui::{
        AppContext as _, Element as _, KeyDownEvent, KeyUpEvent, Keystroke, Render, TestAppContext,
        VisualTestContext,
    };
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn group_roving_focus_is_axis_aware_and_wraps() {
        assert_eq!(
            radio_group_focus_target("left", Axis::Horizontal, 0, 3),
            Some(2)
        );
        assert_eq!(
            radio_group_focus_target("right", Axis::Horizontal, 2, 3),
            Some(0)
        );
        assert_eq!(
            radio_group_focus_target("up", Axis::Vertical, 0, 3),
            Some(2)
        );
        assert_eq!(
            radio_group_focus_target("down", Axis::Vertical, 2, 3),
            Some(0)
        );
        assert_eq!(
            radio_group_focus_target("down", Axis::Horizontal, 1, 3),
            None
        );
        assert_eq!(
            radio_group_focus_target("home", Axis::Vertical, 2, 3),
            Some(0)
        );
        assert_eq!(
            radio_group_focus_target("end", Axis::Vertical, 0, 3),
            Some(2)
        );
        assert_eq!(radio_group_focus_target("end", Axis::Vertical, 0, 0), None);
    }

    #[gpui::test]
    fn builders_preserve_item_and_group_semantics(_cx: &mut TestAppContext) {
        let item = RadioGroupItem::new("pro")
            .label("Pro")
            .aria_description("For growing businesses")
            .invalid(true)
            .disabled(true);
        assert_eq!(item.value.as_ref(), "pro");
        assert!(item.radio.label.is_some());
        assert_eq!(
            item.radio.aria_description.as_deref(),
            Some("For growing businesses")
        );
        assert!(item.radio.invalid);
        assert!(item.radio.disabled);

        let group = RadioGroup::horizontal("plans")
            .aria_label("Plans")
            .value(Some("pro"))
            .child(item);
        assert_eq!(group.orientation, Axis::Horizontal);
        assert_eq!(group.value.as_deref(), Some("pro"));
        assert_eq!(group.aria_label.as_deref(), Some("Plans"));
        assert_eq!(group.items.len(), 1);
    }

    struct KeyboardGroupFixture {
        selected: Arc<Mutex<Option<SharedString>>>,
        calls: Arc<AtomicUsize>,
    }

    impl Render for KeyboardGroupFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let selected_value = self.selected.lock().unwrap().clone();
            let selected = self.selected.clone();
            let calls = self.calls.clone();
            div().child(
                RadioGroup::horizontal("keyboard-radio-group")
                    .aria_label("Keyboard group")
                    .value(selected_value)
                    .child(RadioGroupItem::new("one").label("One"))
                    .child(RadioGroupItem::new("two").label("Two").disabled(true))
                    .child(RadioGroupItem::new("three").label("Three"))
                    .on_change(move |value, _, _| {
                        calls.fetch_add(1, Ordering::SeqCst);
                        *selected.lock().unwrap() = Some(value.clone());
                    }),
            )
        }
    }

    #[gpui::test]
    fn arrow_navigation_skips_disabled_items_and_selects_target(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let selected = Arc::new(Mutex::new(Some(SharedString::from("one"))));
        let calls = Arc::new(AtomicUsize::new(0));
        let captured_selected = selected.clone();
        let captured_calls = calls.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| KeyboardGroupFixture { selected, calls });
            crate::Root::new(fixture, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
        });
        cx.simulate_event(KeyDownEvent {
            keystroke: Keystroke::parse("right").expect("right must be a valid keystroke"),
            is_held: false,
            prefer_character_input: false,
        });
        cx.run_until_parked();

        assert_eq!(captured_calls.load(Ordering::SeqCst), 1);
        assert_eq!(captured_selected.lock().unwrap().as_deref(), Some("three"));
    }

    struct StandaloneFixture {
        checked: bool,
        calls: Arc<AtomicUsize>,
    }

    impl Render for StandaloneFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            div().child(
                Radio::new("selected-radio")
                    .label("Selected")
                    .checked(self.checked)
                    .on_click(move |_, _, _| {
                        calls.fetch_add(1, Ordering::SeqCst);
                    }),
            )
        }
    }

    #[gpui::test]
    fn activating_selected_radio_keeps_requesting_selected_state(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let captured_calls = calls.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| StandaloneFixture {
                checked: true,
                calls,
            });
            crate::Root::new(fixture, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
        });
        let space = Keystroke::parse("space").expect("space must be a valid keystroke");
        cx.simulate_event(KeyDownEvent {
            keystroke: space.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke: space });
        cx.run_until_parked();

        assert_eq!(captured_calls.load(Ordering::SeqCst), 1);
    }

    #[gpui::test]
    fn space_selects_radio_once_and_ignores_key_repeat(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let captured_calls = calls.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| StandaloneFixture {
                checked: false,
                calls,
            });
            crate::Root::new(fixture, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
        });
        let space = Keystroke::parse("space").expect("space must be a valid keystroke");
        cx.simulate_event(KeyDownEvent {
            keystroke: space.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyDownEvent {
            keystroke: space.clone(),
            is_held: true,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke: space });
        cx.run_until_parked();

        assert_eq!(captured_calls.load(Ordering::SeqCst), 1);
    }

    struct AccessibilityProbe {
        result: Arc<Mutex<Option<bool>>>,
    }

    impl Render for AccessibilityProbe {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let result = self.result.clone();
            div().on_prepaint(move |_, window, cx| {
                let mut node = accesskit::Node::new(Role::RadioButton);
                Radio::new("accessible-radio")
                    .label("Email")
                    .checked(true)
                    .disabled(true)
                    .render(window, cx)
                    .into_element()
                    .write_a11y_info(&mut node);
                *result.lock().unwrap() = Some(node.is_disabled());
            })
        }
    }

    #[gpui::test]
    fn radio_exposes_disabled_accessibility_state(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let result = Arc::new(Mutex::new(None));
        let captured = result.clone();
        let (_, cx) = cx.add_window_view(move |_, _| AccessibilityProbe { result });
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        assert_eq!(*captured.lock().unwrap(), Some(true));
    }
}
