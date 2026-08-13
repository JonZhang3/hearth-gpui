use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    Action, Animation, AnimationExt as _, App, Bounds, ElementId, InteractiveElement, IntoElement,
    KeyBinding, KeyDownEvent, ParentElement as _, Pixels, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, actions, div,
    prelude::FluentBuilder as _, px,
};

use crate::{
    ActiveTheme as _, Density, Disableable, ElementExt as _, Icon, IconName, Sizable, Size,
    StyleSized as _, StyledExt as _,
    accessibility::accessibility_state,
    animation::Lerp,
    input::{
        Input, InputMotionKind, InputMotionState, InputPaintState, input_child_id, input_metrics,
        input_motion_timing, input_uses_semantic_color_motion,
    },
    native_menu::NativeMenu,
};

const CONTEXT: &str = "NativeSelect";

actions!(native_select, [Open, Previous, Next, First, Last]);

/// Registers NativeSelect keyboard bindings.
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Open, Some(CONTEXT)),
        KeyBinding::new("space", Open, Some(CONTEXT)),
        KeyBinding::new("up", Previous, Some(CONTEXT)),
        KeyBinding::new("down", Next, Some(CONTEXT)),
        KeyBinding::new("home", First, Some(CONTEXT)),
        KeyBinding::new("end", Last, Some(CONTEXT)),
    ]);
}

type ChangeHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A selectable value displayed by [`NativeSelect`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSelectOption {
    value: SharedString,
    label: SharedString,
    disabled: bool,
}

impl NativeSelectOption {
    /// Creates an option with a submitted value and visible label.
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    /// Returns the option value.
    pub fn value(&self) -> &SharedString {
        &self.value
    }

    /// Returns the visible option label.
    pub fn label(&self) -> &SharedString {
        &self.label
    }
}

impl Disableable for NativeSelectOption {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A labeled group of native select options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSelectOptGroup {
    label: SharedString,
    options: Vec<NativeSelectOption>,
}

impl NativeSelectOptGroup {
    /// Creates an empty option group.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            options: Vec::new(),
        }
    }

    /// Appends an option to the group.
    pub fn child(mut self, option: NativeSelectOption) -> Self {
        self.options.push(option);
        self
    }

    /// Appends an option to the group.
    pub fn option(self, option: NativeSelectOption) -> Self {
        self.child(option)
    }
}

/// A typed child accepted by [`NativeSelect::child`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeSelectChild {
    /// A top-level option.
    Option(NativeSelectOption),
    /// A labeled option group.
    Group(NativeSelectOptGroup),
}

impl From<NativeSelectOption> for NativeSelectChild {
    fn from(value: NativeSelectOption) -> Self {
        Self::Option(value)
    }
}

impl From<NativeSelectOptGroup> for NativeSelectChild {
    fn from(value: NativeSelectOptGroup) -> Self {
        Self::Group(value)
    }
}

#[derive(Action, Clone, PartialEq)]
#[action(namespace = native_select, no_json)]
struct ChooseOption {
    owner: ElementId,
    index: usize,
}

struct NativeSelectState {
    value: Option<SharedString>,
    typeahead_query: String,
    last_typeahead_at: Option<Instant>,
}

/// A compact select trigger whose option menu is rendered by the operating system.
#[derive(IntoElement)]
pub struct NativeSelect {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    children: Vec<NativeSelectChild>,
    value: Option<SharedString>,
    default_value: Option<SharedString>,
    disabled: bool,
    invalid: bool,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
    on_change: Option<ChangeHandler>,
}

impl NativeSelect {
    /// Creates a NativeSelect with stable identity.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            size: Size::Medium,
            children: Vec::new(),
            value: None,
            default_value: None,
            disabled: false,
            invalid: false,
            aria_label: None,
            aria_description: None,
            on_change: None,
        }
    }

    /// Appends an option or option group.
    pub fn child(mut self, child: impl Into<NativeSelectChild>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Appends a top-level option.
    pub fn option(self, option: NativeSelectOption) -> Self {
        self.child(option)
    }

    /// Appends an option group.
    pub fn group(self, group: NativeSelectOptGroup) -> Self {
        self.child(group)
    }

    /// Sets the controlled selected value.
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Sets the initial value used when the component is uncontrolled.
    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    /// Sets the accessible name of the control.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets the accessible description of the control.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Sets whether the current value is invalid.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Handles a value selected by pointer, keyboard, or assistive technology.
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    fn flattened_options(children: &[NativeSelectChild]) -> Vec<NativeSelectOption> {
        children
            .iter()
            .flat_map(|child| match child {
                NativeSelectChild::Option(option) => std::slice::from_ref(option),
                NativeSelectChild::Group(group) => group.options.as_slice(),
            })
            .cloned()
            .collect()
    }

    fn selected_index(
        options: &[NativeSelectOption],
        value: Option<&SharedString>,
    ) -> Option<usize> {
        value
            .and_then(|value| options.iter().position(|option| option.value == *value))
            .or_else(|| (!options.is_empty()).then_some(0))
    }

    fn adjacent_enabled(
        options: &[NativeSelectOption],
        selected: Option<usize>,
        direction: isize,
    ) -> Option<usize> {
        let mut index = selected.map(|index| index as isize).unwrap_or_else(|| {
            if direction > 0 {
                -1
            } else {
                options.len() as isize
            }
        });
        loop {
            index += direction;
            if index < 0 || index >= options.len() as isize {
                return None;
            }
            if !options[index as usize].disabled {
                return Some(index as usize);
            }
        }
    }

    fn edge_enabled(options: &[NativeSelectOption], reverse: bool) -> Option<usize> {
        if reverse {
            options.iter().rposition(|option| !option.disabled)
        } else {
            options.iter().position(|option| !option.disabled)
        }
    }

    /// Finds the next enabled option whose label begins with the typeahead query.
    fn typeahead_match(
        options: &[NativeSelectOption],
        selected: Option<usize>,
        query: &str,
    ) -> Option<usize> {
        let matching = options
            .iter()
            .enumerate()
            .filter(|(_, option)| {
                !option.disabled && option.label.to_lowercase().starts_with(query)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let current = selected
            .and_then(|selected| matching.iter().position(|candidate| *candidate == selected));
        current
            .and_then(|position| matching.get((position + 1) % matching.len()).copied())
            .or_else(|| matching.first().copied())
    }

    fn native_menu(
        children: &[NativeSelectChild],
        owner: &ElementId,
        selected_index: Option<usize>,
    ) -> NativeMenu {
        let mut menu = NativeMenu::new();
        let mut index = 0;
        for child in children {
            match child {
                NativeSelectChild::Option(option) => {
                    menu = menu.menu_with_check_and_disabled(
                        option.label.clone(),
                        selected_index == Some(index),
                        option.disabled,
                        Box::new(ChooseOption {
                            owner: owner.clone(),
                            index,
                        }),
                    );
                    index += 1;
                }
                NativeSelectChild::Group(group) => {
                    menu = menu.label(group.label.clone());
                    for option in &group.options {
                        menu = menu.menu_with_check_and_disabled(
                            option.label.clone(),
                            selected_index == Some(index),
                            option.disabled,
                            Box::new(ChooseOption {
                                owner: owner.clone(),
                                index,
                            }),
                        );
                        index += 1;
                    }
                }
            }
        }
        menu
    }

    /// Maps a flattened option index to its top-level native menu item index.
    ///
    /// OptGroup labels occupy native menu rows, so their positions must be
    /// included before asking AppKit to align the selected item to the trigger.
    fn native_menu_item_index(
        children: &[NativeSelectChild],
        selected_index: Option<usize>,
    ) -> Option<usize> {
        let selected_index = selected_index?;
        let mut option_index = 0;
        let mut menu_item_index = 0;

        for child in children {
            match child {
                NativeSelectChild::Option(_) => {
                    if option_index == selected_index {
                        return Some(menu_item_index);
                    }
                    option_index += 1;
                    menu_item_index += 1;
                }
                NativeSelectChild::Group(group) => {
                    menu_item_index += 1;
                    for _ in &group.options {
                        if option_index == selected_index {
                            return Some(menu_item_index);
                        }
                        option_index += 1;
                        menu_item_index += 1;
                    }
                }
            }
        }
        None
    }
}

impl Disableable for NativeSelect {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Sizable for NativeSelect {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for NativeSelect {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for NativeSelect {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let options = Self::flattened_options(&self.children);
        let initial_value = self
            .default_value
            .clone()
            .or_else(|| options.first().map(|option| option.value.clone()));
        let state = window.use_keyed_state(input_child_id(&self.id, "state"), cx, |_, _| {
            NativeSelectState {
                value: initial_value,
                typeahead_query: String::new(),
                last_typeahead_at: None,
            }
        });
        let selected_value = self.value.clone().or_else(|| state.read(cx).value.clone());
        let selected_index = Self::selected_index(&options, selected_value.as_ref());
        let selected_option = selected_index.and_then(|index| options.get(index));
        let selected_label = selected_option
            .map(|option| option.label.clone())
            .unwrap_or_default();

        let focus_handle = window
            .use_keyed_state(input_child_id(&self.id, "focus"), cx, |_, cx| {
                cx.focus_handle()
            })
            .read(cx)
            .clone();
        let focused = focus_handle.is_focused(window) && !self.disabled;
        let focus_visible = focused && window.last_input_was_keyboard();
        let metrics = input_metrics(&cx.theme().style);
        let control = cx.theme().style.controls.for_size(self.size);
        let invalid_border = cx
            .theme()
            .danger
            .opacity(if cx.theme().is_dark() { 0.5 } else { 1. });
        let border = if self.invalid {
            invalid_border
        } else if focus_visible {
            cx.theme().ring
        } else {
            cx.theme().input
        };
        let ring_visible = self.invalid || focus_visible;
        let ring_color = if self.invalid {
            cx.theme()
                .danger
                .opacity(if cx.theme().is_dark() { 0.4 } else { 0.2 })
        } else {
            cx.theme().ring.opacity(0.5)
        };
        let paint = InputPaintState {
            background: Input::surface_background(metrics, self.disabled, cx),
            border,
            ring: if ring_visible {
                ring_color
            } else {
                ring_color.opacity(0.)
            },
        };
        let uses_semantic_motion = input_uses_semantic_color_motion(&self.style);
        let bounds = Rc::new(Cell::new(Bounds::<Pixels>::default()));

        let open_menu: Rc<dyn Fn(&mut Window, &mut App)> = {
            let children = self.children.clone();
            let owner = self.id.clone();
            let bounds = bounds.clone();
            let focus_handle = focus_handle.clone();
            let selected_menu_item_index = Self::native_menu_item_index(&children, selected_index);
            Rc::new(move |window, cx| {
                focus_handle.focus(window, cx);
                let menu = Self::native_menu(&children, &owner, selected_index);
                #[cfg(target_os = "macos")]
                if let Some(selected_menu_item_index) = selected_menu_item_index {
                    menu.show_selected_at(
                        bounds.get().origin,
                        selected_menu_item_index,
                        window,
                        cx,
                    );
                    return;
                }
                #[cfg(not(target_os = "macos"))]
                let _ = selected_menu_item_index;
                menu.show(bounds.get().bottom_left(), window, cx);
            })
        };

        let commit: Rc<dyn Fn(usize, &mut Window, &mut App)> = {
            let options = options.clone();
            let state = state.clone();
            let on_change = self.on_change.clone();
            Rc::new(move |index, window, cx| {
                let Some(option) = options.get(index).filter(|option| !option.disabled) else {
                    return;
                };
                state.update(cx, |state, cx| {
                    state.value = Some(option.value.clone());
                    cx.notify();
                });
                if let Some(on_change) = &on_change {
                    on_change(&option.value, window, cx);
                }
            })
        };

        let mut trigger = div()
            .id(self.id.clone())
            .role(Role::ComboBox)
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .when_some(self.aria_description, |this, description| {
                this.aria_description(description)
            })
            .aria_value(selected_label.clone())
            .key_context(CONTEXT)
            .relative()
            .flex()
            .min_w_0()
            .items_center()
            .justify_between()
            .gap(control.gap)
            .h(control.height)
            .pl(match cx.theme().style.density {
                Density::Comfortable => px(12.),
                _ => px(10.),
            })
            .pr(match cx.theme().style.density {
                Density::Comfortable => px(14.),
                _ => px(12.),
            })
            .rounded(metrics.radius)
            .border_1()
            .border_color(paint.border)
            .bg(paint.background)
            .when(metrics.shadow, |this| this.shadow_xs())
            .when(
                !self.disabled && cx.theme().is_dark() && uses_semantic_motion,
                |this| this.hover(|this| this.bg(cx.theme().input.opacity(0.5))),
            )
            .input_text_size(self.size)
            .refine_style(&self.style)
            .when(!self.disabled, |this| {
                let open_menu = open_menu.clone();
                this.track_focus(&focus_handle.tab_stop(true))
                    .on_click(move |_, window, cx| open_menu(window, cx))
            })
            .on_action({
                let open_menu = open_menu.clone();
                move |_: &Open, window, cx| open_menu(window, cx)
            })
            .on_action({
                let commit = commit.clone();
                let owner = self.id.clone();
                move |action: &ChooseOption, window, cx| {
                    if action.owner == owner {
                        commit(action.index, window, cx);
                    }
                }
            })
            .on_action({
                let commit = commit.clone();
                let options = options.clone();
                move |_: &Previous, window, cx| {
                    if let Some(index) = Self::adjacent_enabled(&options, selected_index, -1) {
                        commit(index, window, cx);
                    }
                }
            })
            .on_action({
                let commit = commit.clone();
                let options = options.clone();
                move |_: &Next, window, cx| {
                    if let Some(index) = Self::adjacent_enabled(&options, selected_index, 1) {
                        commit(index, window, cx);
                    }
                }
            })
            .on_action({
                let commit = commit.clone();
                let options = options.clone();
                move |_: &First, window, cx| {
                    if let Some(index) = Self::edge_enabled(&options, false) {
                        commit(index, window, cx);
                    }
                }
            })
            .on_action({
                let commit = commit.clone();
                let options = options.clone();
                move |_: &Last, window, cx| {
                    if let Some(index) = Self::edge_enabled(&options, true) {
                        commit(index, window, cx);
                    }
                }
            })
            .on_key_down({
                let commit = commit.clone();
                let options = options.clone();
                let state = state.clone();
                move |event: &KeyDownEvent, window, cx| {
                    if event.is_held
                        || event.keystroke.modifiers.control
                        || event.keystroke.modifiers.alt
                        || event.keystroke.modifiers.platform
                        || event.keystroke.modifiers.function
                    {
                        return;
                    }
                    let Some(character) = event
                        .keystroke
                        .key_char
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                    else {
                        return;
                    };
                    if character.chars().count() != 1 {
                        return;
                    }

                    let now = Instant::now();
                    let query = state.update(cx, |state, _| {
                        if state.last_typeahead_at.is_none_or(|last| {
                            now.duration_since(last) > Duration::from_millis(700)
                        }) {
                            state.typeahead_query.clear();
                        }
                        state.typeahead_query.push_str(&character.to_lowercase());
                        state.last_typeahead_at = Some(now);
                        let repeated = state
                            .typeahead_query
                            .chars()
                            .next()
                            .filter(|first| {
                                state.typeahead_query.chars().all(|value| value == *first)
                            })
                            .map(|value| value.to_string());
                        repeated.unwrap_or_else(|| state.typeahead_query.clone())
                    });
                    if let Some(index) = Self::typeahead_match(&options, selected_index, &query) {
                        commit(index, window, cx);
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                }
            })
            .on_prepaint({
                let bounds_state = bounds.clone();
                move |bounds, _, _| bounds_state.set(bounds)
            })
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .truncate()
                    .child(selected_label),
            )
            .child(
                Icon::new(IconName::ChevronDown)
                    .size(px(16.))
                    .text_color(cx.theme().muted_foreground.opacity(0.5)),
            );

        let motion_state =
            window.use_keyed_state(input_child_id(&self.id, "motion-state"), cx, |_, _| {
                InputMotionState::new(paint)
            });
        let (motion_duration, motion_easing) = input_motion_timing(ring_visible, cx);
        let transition = motion_state.update(cx, |state, _| {
            state.transition_to(
                paint,
                Instant::now(),
                motion_duration,
                motion_easing,
                metrics.motion_kind,
            )
        });
        let ring_transition =
            transition.filter(|transition| transition.from.ring != transition.to.ring);
        let ring_geometry = (ring_visible || ring_transition.is_some()).then(|| {
            let ring_width = cx.theme().style.focus.ring_width;
            let ring_outset = ring_width + cx.theme().style.focus.ring_offset;
            (
                ring_width,
                ring_outset,
                Input::outer_ring_geometry(trigger.style(), ring_outset, window),
            )
        });

        let trigger = if uses_semantic_motion
            && let Some(transition) = transition.filter(|transition| {
                transition.from.background != transition.to.background
                    || transition.from.border != transition.to.border
            }) {
            let from = transition.from;
            let to = transition.to;
            let motion_kind = metrics.motion_kind;
            trigger
                .with_animation(
                    input_child_id(&self.id, format!("surface-{}", transition.epoch)),
                    Animation::new(transition.duration)
                        .with_easing(move |delta| motion_easing.sample(delta)),
                    move |this, delta| match motion_kind {
                        InputMotionKind::Colors | InputMotionKind::ColorsAndShadow => this
                            .bg(Lerp::lerp(&from.background, &to.background, delta))
                            .border_color(Lerp::lerp(&from.border, &to.border, delta)),
                        InputMotionKind::Shadow => this.bg(to.background).border_color(to.border),
                    },
                )
                .into_any_element()
        } else {
            trigger.into_any_element()
        };

        let ring = ring_geometry.map(|(ring_width, ring_outset, ring_style)| {
            let ring = div()
                .absolute()
                .top(-ring_outset)
                .right(-ring_outset)
                .bottom(-ring_outset)
                .left(-ring_outset)
                .border(ring_width)
                .border_color(paint.ring)
                .refine_style(&ring_style);
            if let Some(transition) = ring_transition {
                let from = transition.from;
                let to = transition.to;
                ring.with_animation(
                    input_child_id(&self.id, format!("ring-{}", transition.epoch)),
                    Animation::new(transition.duration)
                        .with_easing(move |delta| motion_easing.sample(delta)),
                    move |this, delta| this.border_color(Lerp::lerp(&from.ring, &to.ring, delta)),
                )
                .into_any_element()
            } else {
                ring.into_any_element()
            }
        });

        let trigger = accessibility_state(trigger, self.invalid, false, self.disabled);
        div()
            .relative()
            .when(self.disabled, |this| this.opacity(0.5))
            .when_some(ring, |this, ring| this.child(ring))
            .child(trigger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_navigation_skips_disabled_values() {
        let options = vec![
            NativeSelectOption::new("a", "A"),
            NativeSelectOption::new("b", "B").disabled(true),
            NativeSelectOption::new("c", "C"),
        ];

        assert_eq!(
            NativeSelect::adjacent_enabled(&options, Some(0), 1),
            Some(2)
        );
        assert_eq!(
            NativeSelect::adjacent_enabled(&options, Some(2), -1),
            Some(0)
        );
        assert_eq!(NativeSelect::edge_enabled(&options, false), Some(0));
        assert_eq!(NativeSelect::edge_enabled(&options, true), Some(2));
    }

    #[test]
    fn typeahead_cycles_enabled_matching_options() {
        let options = vec![
            NativeSelectOption::new("apple", "Apple"),
            NativeSelectOption::new("apricot", "Apricot").disabled(true),
            NativeSelectOption::new("avocado", "Avocado"),
        ];

        assert_eq!(
            NativeSelect::typeahead_match(&options, Some(0), "a"),
            Some(2)
        );
        assert_eq!(
            NativeSelect::typeahead_match(&options, Some(2), "a"),
            Some(0)
        );
        assert_eq!(NativeSelect::typeahead_match(&options, None, "av"), Some(2));
    }

    #[test]
    fn native_menu_item_index_includes_group_labels() {
        let children = vec![
            NativeSelectOption::new("none", "Select a language").into(),
            NativeSelectOptGroup::new("Compiled")
                .child(NativeSelectOption::new("rust", "Rust"))
                .child(NativeSelectOption::new("go", "Go"))
                .into(),
            NativeSelectOption::new("python", "Python").into(),
        ];

        assert_eq!(
            NativeSelect::native_menu_item_index(&children, Some(0)),
            Some(0)
        );
        assert_eq!(
            NativeSelect::native_menu_item_index(&children, Some(1)),
            Some(2)
        );
        assert_eq!(
            NativeSelect::native_menu_item_index(&children, Some(2)),
            Some(3)
        );
        assert_eq!(
            NativeSelect::native_menu_item_index(&children, Some(3)),
            Some(4)
        );
    }

    #[test]
    fn native_select_builds_controlled_grouped_state() {
        let component = NativeSelect::new("language")
            .value("rust")
            .aria_label("Language")
            .invalid(true)
            .disabled(true)
            .child(NativeSelectOption::new("rust", "Rust"))
            .child(NativeSelectOptGroup::new("Other").child(NativeSelectOption::new("go", "Go")));
        let options = NativeSelect::flattened_options(&component.children);

        assert_eq!(
            NativeSelect::selected_index(&options, component.value.as_ref()),
            Some(0)
        );
        assert!(component.disabled);
        assert!(component.invalid);
    }
}
