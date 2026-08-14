// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added `SelectPosition` with ItemAligned and Popper positioning modes.
// - Added `position`, `aria_label`, `aria_description`, `invalid`, and `group_separators` builder
//   APIs.
// - Added printable-key typeahead, keyboard-open guarding, blur reconciliation, and focus
//   restoration.
// - Added interruptible overlay lifecycle transitions and density-specific Select metrics.
use std::time::{Duration, Instant};

use gpui::{
    AbsoluteLength, AnchoredPositionMode, Animation, AnimationExt as _, AnyElement, App,
    ClickEvent, Context, DismissEvent, Edges, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, Hsla, InteractiveElement, IntoElement, KeyBinding, KeyDownEvent, KeyUpEvent, Length,
    ParentElement, Pixels, Render, RenderOnce, Role, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Task, Window, anchored, deferred, div, point, prelude::FluentBuilder,
    px, rems,
};
use rust_i18n::t;

use crate::{
    ActiveTheme, Density, Disableable, ElementExt as _, Icon, IconName, IndexPath, Sizable, Size,
    StyleSized, StyledExt,
    actions::{Cancel, Confirm, SelectDown, SelectUp},
    animation::{Lerp, OverlayLifecycle, OverlayPhase, OverlayTransition, Transition},
    geometry::LengthExt as _,
    global_state::GlobalState,
    h_flex,
    input::{
        Input, InputMotionKind, InputMotionState, InputPaintState, clear_button, input_child_id,
        input_metrics, input_motion_timing, input_uses_semantic_color_motion,
    },
    list::List,
    searchable_list::{
        SearchableListChange, SearchableListDelegate, SearchableListItem, SearchableListState,
    },
    v_flex,
};

// MARK: Public re-exports for back-compat

/// Re-exported for backward compatibility. New code should prefer [`SearchableGroup`].
pub use crate::searchable_list::SearchableGroup as SelectGroup;
/// Re-exported for backward compatibility. New code should prefer [`SearchableListDelegate`].
pub use crate::searchable_list::SearchableListDelegate as SelectDelegate;
/// Re-exported for backward compatibility. New code should prefer [`SearchableListItem`].
pub use crate::searchable_list::SearchableListItem as SelectItem;
/// Re-exported for backward compatibility. New code should prefer [`SearchableListItemElement`].
pub use crate::searchable_list::SearchableListItemElement as SelectListItem;
/// Re-exported for backward compatibility.
pub use crate::searchable_list::SearchableVec;

/// Controls how Select content is positioned relative to its trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectPosition {
    /// Aligns the selected option vertically with the trigger, matching the
    /// canonical shadcn/Radix Select behavior.
    #[default]
    ItemAligned,
    /// Positions the content below the trigger with a 4 px side offset.
    Popper,
}

/// Select-only presentation derived from semantic Style Preset values.
///
/// These values stay local until another menu family has the same geometry contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SelectMetrics {
    pub(crate) trigger_radius: Pixels,
    pub(crate) trigger_padding_left: Pixels,
    pub(crate) trigger_padding_right: Pixels,
    pub(crate) trigger_gap: Pixels,
    pub(crate) content_radius: Pixels,
    pub(crate) content_ring_opacity: f32,
    pub(crate) content_shadow: SelectShadow,
    pub(crate) item_height: Pixels,
    pub(crate) item_padding_left: Pixels,
    pub(crate) item_padding_y: Pixels,
    pub(crate) item_gap: Pixels,
    pub(crate) item_radius: Pixels,
    pub(crate) label_padding_x: Pixels,
    pub(crate) label_padding_y: Pixels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectShadow {
    Medium,
    ExtraLarge,
}

impl SelectMetrics {
    /// Resolves pinned shadcn geometry without branching on preset identifiers.
    pub(crate) fn resolve(size: Size, cx: &App) -> Self {
        Self::from_style(size, &cx.theme().style)
    }

    /// Resolves Select geometry from a semantic Style Preset.
    fn from_style(size: Size, style: &crate::theme::StylePreset) -> Self {
        let control = style.controls.for_size(size);
        match style.density {
            Density::Standard => Self {
                trigger_radius: style.radii.md,
                trigger_padding_left: if matches!(size, Size::Small | Size::Medium) {
                    px(10.)
                } else {
                    control.padding_x
                },
                trigger_padding_right: if matches!(size, Size::Small | Size::Medium) {
                    px(8.)
                } else {
                    control.padding_x
                },
                trigger_gap: px(6.),
                content_radius: style.radii.md,
                content_ring_opacity: 0.1,
                content_shadow: SelectShadow::Medium,
                item_height: px(32.),
                item_padding_left: px(8.),
                item_padding_y: px(6.),
                item_gap: px(8.),
                item_radius: style.radii.sm,
                label_padding_x: px(8.),
                label_padding_y: px(6.),
            },
            Density::Compact => Self {
                trigger_radius: if matches!(size, Size::XSmall | Size::Small) {
                    style.radii.md
                } else {
                    style.radii.lg
                },
                trigger_padding_left: if matches!(size, Size::Small | Size::Medium) {
                    px(10.)
                } else {
                    control.padding_x
                },
                trigger_padding_right: if matches!(size, Size::Small | Size::Medium) {
                    px(8.)
                } else {
                    control.padding_x
                },
                trigger_gap: px(6.),
                content_radius: style.radii.lg,
                content_ring_opacity: 0.1,
                content_shadow: SelectShadow::Medium,
                item_height: px(28.),
                item_padding_left: px(6.),
                item_padding_y: px(4.),
                item_gap: px(6.),
                item_radius: style.radii.md,
                label_padding_x: px(6.),
                label_padding_y: px(4.),
            },
            Density::Comfortable => Self {
                trigger_radius: style.radii.xl,
                trigger_padding_left: if matches!(size, Size::Small | Size::Medium) {
                    px(12.)
                } else {
                    control.padding_x
                },
                trigger_padding_right: if matches!(size, Size::Small | Size::Medium) {
                    px(12.)
                } else {
                    control.padding_x
                },
                trigger_gap: px(6.),
                content_radius: style.radii.lg,
                content_ring_opacity: 0.05,
                content_shadow: SelectShadow::ExtraLarge,
                item_height: px(36.),
                item_padding_left: px(12.),
                item_padding_y: px(8.),
                item_gap: px(10.),
                item_radius: style.radii.lg,
                label_padding_x: px(12.),
                label_padding_y: px(10.),
            },
        }
    }
}

#[derive(IntoElement)]
pub struct Caret {
    size: Size,
    color: Option<Hsla>,
}

impl Caret {
    /// Create a select caret sized for its trigger.
    pub fn new(size: Size) -> Self {
        Self { size, color: None }
    }

    /// Set the caret color.
    pub fn text_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Caret {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::new(IconName::ChevronDown)
            .size(if self.size == Size::XSmall {
                px(12.)
            } else {
                px(16.)
            })
            .when_some(self.color, |this, color| this.text_color(color))
    }
}

const CONTEXT: &str = "Select";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("space", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new(
            "secondary-enter",
            Confirm { secondary: true },
            Some(CONTEXT),
        ),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ])
}

/// Events emitted by [`SelectState`].
pub enum SelectEvent<D: SearchableListDelegate + 'static>
where
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    Confirm(Option<<D::Item as SearchableListItem>::Value>),
}

// MARK: SelectOptions (builder only — applied to SearchableListState during render)

struct SelectOptions {
    style: StyleRefinement,
    size: Size,
    icon: Option<Icon>,
    cleanable: bool,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
    placeholder: Option<SharedString>,
    title_prefix: Option<SharedString>,
    search_placeholder: Option<SharedString>,
    menu_width: Length,
    menu_max_h: Length,
    position: Option<SelectPosition>,
    disabled: bool,
    invalid: bool,
    group_separators: bool,
    appearance: bool,
}

impl Default for SelectOptions {
    fn default() -> Self {
        Self {
            style: StyleRefinement::default(),
            size: Size::default(),
            icon: None,
            cleanable: false,
            aria_label: None,
            aria_description: None,
            placeholder: None,
            title_prefix: None,
            menu_width: Length::Auto,
            menu_max_h: rems(20.).into(),
            position: None,
            disabled: false,
            invalid: false,
            group_separators: false,
            appearance: true,
            search_placeholder: None,
        }
    }
}

// MARK: SelectState

/// State of the [`Select`] component.
pub struct SelectState<D: SearchableListDelegate + 'static>
where
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    pub(crate) state: SearchableListState<D>,

    // Select-specific fields
    searchable: bool,
    icon: Option<Icon>,
    title_prefix: Option<SharedString>,
    invalid: bool,
    group_separators: bool,
    position: Option<SelectPosition>,
    item_aligned_selected_center: Option<Pixels>,
    lifecycle: OverlayLifecycle,
    restore_focus_after_close: bool,
    typeahead_query: String,
    typeahead_revision: u64,
    typeahead_task: Task<()>,
    keyboard_open_guard: bool,
}

/// A Select element.
#[derive(IntoElement)]
pub struct Select<D: SearchableListDelegate + 'static>
where
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    id: ElementId,
    state: Entity<SelectState<D>>,
    options: SelectOptions,
    empty: Option<Box<dyn Fn(&mut Window, &App) -> AnyElement + 'static>>,
}

impl<D> SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    /// Create a new Select state.
    pub fn new(
        delegate: D,
        selected_index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let weak = cx.entity().downgrade();
        let weak_confirm = weak.clone();
        let weak_cancel = weak.clone();
        let weak_empty = weak;

        let selected_indices = selected_index.into_iter().collect::<Vec<_>>();

        let state = SearchableListState::new(
            delegate,
            selected_indices,
            // on_confirm — commit the selection
            move |selected_index, _secondary, window, cx| {
                cx.defer_in(window, {
                    let weak_confirm = weak_confirm.clone();
                    move |list_state, window, cx| {
                        if weak_confirm
                            .upgrade()
                            .is_some_and(|state| state.read(cx).keyboard_open_guard)
                        {
                            return;
                        }

                        let mut selection = weak_confirm
                            .upgrade()
                            .map(|e| e.read(cx).state.selection.clone())
                            .unwrap_or_default();

                        let changes = {
                            let mut changes: Vec<SearchableListChange> = selection
                                .iter()
                                .map(|(ix, _)| SearchableListChange::Deselect { index: *ix })
                                .collect();

                            if let Some(ix) = selected_index {
                                changes.push(SearchableListChange::Select { index: ix });
                            }

                            changes
                        };

                        // on_will_change is called directly — entity-handle access would
                        // re-enter the ListState lock that defer_in holds for this callback.
                        list_state
                            .delegate_mut()
                            .delegate
                            .on_will_change(&mut selection, &changes);

                        let new_selection = weak_confirm.update(cx, |this, cx| {
                            this.state.selection = selection;

                            let final_value =
                                this.state.selection.first().map(|(_, i)| i.value().clone());

                            cx.emit(SelectEvent::Confirm(final_value));
                            cx.notify();
                            this.set_open(false, window, cx);

                            this.state.selection.clone()
                        });

                        // Sync snapshot and fire on_confirm directly — same re-entrancy guard.
                        if let Ok(new_selection) = new_selection {
                            list_state
                                .delegate_mut()
                                .update_selection_snapshot(new_selection.clone());
                            list_state
                                .delegate_mut()
                                .delegate
                                .on_confirm(&new_selection);
                        }
                    }
                });
            },
            // on_cancel — restore cursor to committed index, close
            move |_final_selected_index, window, cx| {
                cx.defer_in(window, {
                    let weak_cancel = weak_cancel.clone();
                    move |list_state, window, cx| {
                        let committed_ix = weak_cancel
                            .upgrade()
                            .and_then(|e| e.read(cx).state.selection.first().map(|(ix, _)| *ix));

                        list_state.set_selected_index(committed_ix, window, cx);

                        _ = weak_cancel.update(cx, |this, cx| {
                            this.set_open(false, window, cx);
                        });
                    }
                });
            },
            // on_render_empty
            move |window, cx| {
                if let Some(empty) = weak_empty
                    .upgrade()
                    .and_then(|e| e.read(cx).state.empty.as_ref().map(|f| f(window, cx)))
                {
                    empty
                } else {
                    h_flex()
                        .justify_center()
                        .w_full()
                        .py_2()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t!("Select.empty"))
                        .into_any_element()
                }
            },
            Self::on_blur,
            window,
            cx,
        );

        Self {
            state,
            searchable: false,
            icon: None,
            title_prefix: None,
            invalid: false,
            group_separators: false,
            position: None,
            item_aligned_selected_center: None,
            lifecycle: OverlayLifecycle::default(),
            restore_focus_after_close: false,
            typeahead_query: String::new(),
            typeahead_revision: 0,
            typeahead_task: Task::ready(()),
            keyboard_open_guard: false,
        }
    }

    /// Sets whether the dropdown menu is searchable, default is `false`.
    ///
    /// When `true`, a search input appears at the top of the dropdown menu.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// Set the selected index for the select.
    pub fn set_selected_index(
        &mut self,
        selected_index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.list.update(cx, |list, cx| {
            list._set_selected_index(selected_index, window, cx);
        });

        let item = selected_index
            .and_then(|ix| self.state.list.read(cx).delegate().delegate.item(ix))
            .map(|i| i.clone());

        self.state.selection = match (selected_index, item) {
            (Some(ix), Some(item)) => vec![(ix, item)],
            _ => vec![],
        };
        self.state.sync_snapshot(cx);
    }

    /// Set selected value for the select.
    ///
    /// Looks up the position from the delegate and sets the selected index accordingly.
    /// Passes `None` when the value is not found.
    pub fn set_selected_value(
        &mut self,
        selected_value: &<D::Item as SearchableListItem>::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected_index = self
            .state
            .list
            .read(cx)
            .delegate()
            .delegate
            .position(selected_value);

        self.set_selected_index(selected_index, window, cx);
    }

    /// Replace the delegate (item data) for the select state.
    pub fn set_items(&mut self, items: D, _: &mut Window, cx: &mut Context<Self>)
    where
        D: SearchableListDelegate + 'static,
    {
        self.state.list.update(cx, |list, _| {
            list.delegate_mut().delegate = items;
        });
    }

    /// Get the current selected index.
    pub fn selected_index(&self, cx: &App) -> Option<IndexPath> {
        self.state.list.read(cx).selected_index()
    }

    /// Get the current selected value.
    pub fn selected_value(&self) -> Option<&<D::Item as SearchableListItem>::Value> {
        self.state.selection.first().map(|(_, i)| i.value())
    }

    /// Resolves the explicit positioning mode or the mode implied by search.
    fn effective_position(&self) -> SelectPosition {
        if self.searchable {
            SelectPosition::Popper
        } else {
            self.position.unwrap_or(SelectPosition::ItemAligned)
        }
    }

    /// Focus the select trigger input.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.state.focus_handle.focus(window, cx);
    }

    /// Defers blur reconciliation until the currently updating child ListState is released.
    fn on_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.defer_in(window, |this, window, cx| {
            this.reconcile_blur(window, cx);
        });
    }

    /// Closes the popup only after focus has left both the trigger and its list.
    fn reconcile_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.list_is_focused(window) || self.state.focus_handle.is_focused(window) {
            return;
        }

        let committed_ix = self.state.selection.first().map(|(ix, _)| *ix);
        if self.selected_index(cx) != committed_ix {
            self.state.list.update(cx, |list, cx| {
                list.set_selected_index(committed_ix, window, cx);
            });
        }

        self.set_open(false, window, cx);
        cx.notify();
    }

    fn up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        if !self.state.open {
            self.set_open(true, window, cx);
        }

        self.state
            .active_list_focus_handle(self.searchable)
            .focus(window, cx);
        cx.propagate();
    }

    fn down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        if !self.state.open {
            self.set_open(true, window, cx);
        }

        self.state
            .active_list_focus_handle(self.searchable)
            .focus(window, cx);
        cx.propagate();
    }

    fn enter(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.open {
            cx.propagate();
            return;
        }

        // The opening key press must not reach the newly focused list and
        // immediately confirm its temporary first-item cursor.
        cx.stop_propagation();
        self.keyboard_open_guard = true;
        self.set_open(true, window, cx);
        self.state
            .active_list_focus_handle(self.searchable)
            .focus(window, cx);
        cx.notify();
    }

    /// Releases the guard that keeps the opening key press and its repeats
    /// from confirming the temporary first-item cursor.
    fn release_open_key(&mut self, event: &KeyUpEvent, _: &mut Window, _: &mut Context<Self>) {
        if matches!(event.keystroke.key.as_str(), "space" | "enter") {
            self.keyboard_open_guard = false;
        }
    }

    fn toggle_menu(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();

        self.keyboard_open_guard = false;
        self.set_open(!self.state.open, window, cx);

        if self.state.open {
            self.state
                .active_list_focus_handle(self.searchable)
                .focus(window, cx);
        }

        cx.notify();
    }

    fn escape(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if !self.state.open {
            cx.propagate();
            return;
        }

        cx.stop_propagation();
        self.set_open(false, window, cx);
        cx.notify();
    }

    /// Handles printable-character typeahead for non-searchable Selects.
    fn typeahead_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.is_held && matches!(event.keystroke.key.as_str(), "space" | "enter") {
            window.prevent_default();
            cx.stop_propagation();
            return;
        }

        if self.searchable
            || self.state.disabled
            || event.is_held
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

        self.typeahead_query.push_str(&character.to_lowercase());
        let repeated_character = self
            .typeahead_query
            .chars()
            .next()
            .filter(|first| self.typeahead_query.chars().all(|value| value == *first))
            .map(|value| value.to_string());
        let query = repeated_character
            .as_deref()
            .unwrap_or(&self.typeahead_query);

        let current = self.selected_index(cx);
        let matching_index = {
            let list = self.state.list.read(cx);
            let delegate = &list.delegate().delegate;
            let mut matches = Vec::new();
            for section in 0..delegate.sections_count(cx) {
                for row in 0..delegate.items_count(section) {
                    let index = IndexPath::default().section(section).row(row);
                    let Some(item) = delegate.item(index) else {
                        continue;
                    };
                    if delegate.is_item_enabled(index, item, cx)
                        && item.title().to_lowercase().starts_with(query)
                    {
                        matches.push(index);
                    }
                }
            }

            let current_position =
                current.and_then(|index| matches.iter().position(|candidate| *candidate == index));
            current_position
                .and_then(|position| matches.get((position + 1) % matches.len()).copied())
                .or_else(|| matches.first().copied())
        };

        if let Some(index) = matching_index {
            let open = self.state.open;
            self.state.list.update(cx, |list, cx| {
                list._set_selected_index(Some(index), window, cx);
                if !open {
                    list.on_action_confirm(&Confirm { secondary: false }, window, cx);
                }
            });
        }

        self.typeahead_revision = self.typeahead_revision.wrapping_add(1);
        let revision = self.typeahead_revision;
        self.typeahead_task = cx.spawn_in(window, async move |state, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(700))
                .await;
            let _ = state.update(cx, |state, _| {
                if state.typeahead_revision == revision {
                    state.typeahead_query.clear();
                }
            });
        });
        window.prevent_default();
        cx.stop_propagation();
    }

    fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        let transition = if open {
            self.lifecycle.begin_open()
        } else {
            self.lifecycle.begin_close()
        };
        let Some(_transition) = transition else {
            return;
        };

        self.state.open = open;
        if open {
            self.item_aligned_selected_center = None;
            GlobalState::global_mut(cx).register_deferred_popover(&self.state.focus_handle);
        } else {
            self.keyboard_open_guard = false;
            self.restore_focus_after_close = self.state.list_is_focused(window);
        }
        cx.notify();
    }

    /// Completes the active overlay motion and performs close-only ownership work.
    fn complete_motion(
        &mut self,
        opening: bool,
        transition: OverlayTransition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let completed = if opening {
            self.lifecycle.complete_open(transition)
        } else {
            self.lifecycle.complete_close(transition)
        };
        if !completed {
            return;
        }

        if !opening {
            GlobalState::global_mut(cx).unregister_deferred_popover(&self.state.focus_handle);
            if self.restore_focus_after_close {
                self.state.focus_handle.focus(window, cx);
            }
            self.restore_focus_after_close = false;
        }
        cx.notify();
    }

    fn clean(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        self.set_selected_index(None, window, cx);
        cx.emit(SelectEvent::Confirm(None));
    }

    fn display_title(&mut self, _: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let default_title = div().text_color(cx.theme().muted_foreground).child(
            self.state
                .placeholder
                .clone()
                .unwrap_or_else(|| t!("Select.placeholder").into()),
        );

        let Some(selected_index) = self.selected_index(cx) else {
            return default_title;
        };

        let Some(title) = self
            .state
            .list
            .read(cx)
            .delegate()
            .delegate
            .item(selected_index)
            .map(|item| {
                if let Some(el) = item.display_title() {
                    el
                } else if let Some(prefix) = self.title_prefix.as_ref() {
                    format!("{}{}", prefix, item.title()).into_any_element()
                } else {
                    item.title().into_any_element()
                }
            })
        else {
            return default_title;
        };

        div()
            .when(self.state.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .child(title)
    }
}

impl<D> Render for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let searchable = self.searchable;
        let is_focused = self.state.focus_handle.is_focused(window);
        let focus_visible = is_focused && !self.state.disabled && window.last_input_was_keyboard();
        let show_clean = self.state.cleanable && self.selected_index(cx).is_some();
        let bounds = self.state.bounds;
        let phase = self.lifecycle.phase();
        let active_transition = self.lifecycle.active_transition();
        let mounted = self.lifecycle.is_mounted();
        let closing = phase == OverlayPhase::Closing;
        let allow_open = !(self.state.open || self.state.disabled);
        let opening = phase == OverlayPhase::Opening;
        let metrics = SelectMetrics::resolve(self.state.size, cx);
        let position = self.effective_position();
        let input_metrics = input_metrics(&cx.theme().style);
        let control_metrics = cx.theme().style.controls.for_size(self.state.size);
        let invalid_border =
            cx.theme()
                .danger
                .opacity(if cx.theme().is_dark() { 0.5 } else { 1.0 });
        let border = if self.invalid {
            invalid_border
        } else if focus_visible {
            cx.theme().ring
        } else {
            cx.theme().input
        };
        let ring_visible = self.state.appearance && (self.invalid || focus_visible);
        let ring_color = if self.invalid {
            cx.theme()
                .danger
                .opacity(if cx.theme().is_dark() { 0.4 } else { 0.2 })
        } else {
            cx.theme().ring.opacity(0.5)
        };
        let ring_color = if self.state.disabled {
            ring_color.opacity(0.5)
        } else {
            ring_color
        };
        let paint = InputPaintState {
            background: Input::surface_background(input_metrics, false, cx),
            border,
            ring: if ring_visible {
                ring_color
            } else {
                ring_color.opacity(0.)
            },
        };
        let uses_semantic_color_motion = input_uses_semantic_color_motion(&self.state.style);
        let root_id: ElementId = ("select-trigger", cx.entity().entity_id()).into();

        self.state.list.update(cx, |list, cx| {
            list.set_searchable(searchable, cx);
            let delegate = list.delegate_mut();
            delegate.size = Size::Medium;
            delegate.select_style = true;
            delegate.section_separators = self.group_separators;
        });

        if self.item_aligned_selected_center.is_none()
            && position == SelectPosition::ItemAligned
            && mounted
        {
            let max_height = self
                .state
                .menu_max_h
                .to_pixels(
                    AbsoluteLength::Pixels(window.viewport_size().height),
                    window.rem_size(),
                )
                .unwrap_or(window.viewport_size().height);
            let selected_index = self.selected_index(cx);
            self.item_aligned_selected_center = self.state.list.update(cx, |list, cx| {
                list.prepare_item_alignment(selected_index, max_height, px(4.), px(4.), window, cx)
            });
        }
        let popup_position = if position == SelectPosition::ItemAligned {
            self.item_aligned_selected_center.map(|selected_center| {
                point(px(0.), -bounds.size.height / 2. - selected_center - px(1.))
            })
        } else {
            None
        }
        .unwrap_or_else(|| point(px(0.), px(4.)));

        let mut trigger = div()
            .id("input")
            .relative()
            .flex()
            .items_center()
            .justify_between()
            .h(control_metrics.height)
            .pl(metrics.trigger_padding_left)
            .pr(metrics.trigger_padding_right)
            .border_1()
            .border_color(cx.theme().transparent)
            .when(self.state.disabled, |this| this.opacity(0.5))
            .when(self.state.appearance, |this| {
                this.bg(paint.background)
                    .text_color(cx.theme().foreground)
                    .border_color(paint.border)
                    .rounded(metrics.trigger_radius)
                    .when(input_metrics.shadow, |this| this.shadow_xs())
            })
            .when(
                self.state.appearance
                    && uses_semantic_color_motion
                    && !self.state.disabled
                    && cx.theme().is_dark(),
                |this| this.hover(|this| this.bg(cx.theme().input.opacity(0.5))),
            )
            .overflow_hidden()
            .input_text_size(self.state.size)
            .refine_style(&self.state.style)
            .when(allow_open, |this| {
                this.on_click(cx.listener(Self::toggle_menu))
            })
            .child(
                h_flex()
                    .id("inner")
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap(metrics.trigger_gap)
                    .child(
                        div()
                            .id("title")
                            .w_full()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .truncate()
                            .child(self.display_title(window, cx)),
                    )
                    .when(show_clean, |this| {
                        this.child(clear_button(cx).map(|this| {
                            if self.state.disabled {
                                this.disabled(true)
                            } else {
                                this.on_click(cx.listener(Self::clean))
                            }
                        }))
                    })
                    .when(!show_clean, |this| {
                        let icon = match self.icon.clone() {
                            Some(icon) => icon
                                .size(if self.state.size == Size::XSmall {
                                    px(12.)
                                } else {
                                    px(16.)
                                })
                                .text_color(cx.theme().muted_foreground)
                                .into_any_element(),
                            None => Caret::new(self.state.size)
                                .text_color(cx.theme().muted_foreground)
                                .into_any_element(),
                        };

                        this.child(icon)
                    }),
            )
            .on_prepaint({
                let state = cx.entity();
                move |bounds, _, cx| state.update(cx, |r, _| r.state.bounds = bounds)
            });

        let motion_state =
            window.use_keyed_state(input_child_id(&root_id, "motion-state"), cx, |_, _| {
                InputMotionState::new(paint)
            });
        let (motion_duration, motion_easing) = input_motion_timing(ring_visible, cx);
        let transition = motion_state.update(cx, |state, _| {
            state.transition_to(
                paint,
                Instant::now(),
                motion_duration,
                motion_easing,
                input_metrics.motion_kind,
            )
        });

        let ring_transition =
            transition.filter(|transition| transition.from.ring != transition.to.ring);
        let ring_geometry = (self.state.appearance && (ring_visible || ring_transition.is_some()))
            .then(|| {
                let ring_width = cx.theme().style.focus.ring_width;
                let ring_outset = ring_width + cx.theme().style.focus.ring_offset;
                (
                    ring_width,
                    ring_outset,
                    Input::outer_ring_geometry(trigger.style(), ring_outset, window),
                )
            });

        let trigger = if self.state.appearance
            && uses_semantic_color_motion
            && let Some(transition) = transition.filter(|transition| {
                transition.from.background != transition.to.background
                    || transition.from.border != transition.to.border
            }) {
            let from = transition.from;
            let to = transition.to;
            let motion_kind = input_metrics.motion_kind;
            trigger
                .with_animation(
                    input_child_id(&root_id, format!("surface-{}", transition.epoch)),
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
                    input_child_id(&root_id, format!("ring-{}", transition.epoch)),
                    Animation::new(transition.duration)
                        .with_easing(move |delta| motion_easing.sample(delta)),
                    move |this, delta| this.border_color(Lerp::lerp(&from.ring, &to.ring, delta)),
                )
                .into_any_element()
            } else {
                ring.into_any_element()
            }
        });

        div()
            .size_full()
            .relative()
            .children(ring)
            .child(trigger)
            .when(mounted, |this| {
                this.child(
                    deferred(
                        anchored()
                            .position_mode(AnchoredPositionMode::Local)
                            .position(popup_position)
                            .when(position == SelectPosition::ItemAligned, |this| {
                                // Item-aligned content must preserve the Trigger's horizontal
                                // edge. A window margin would shift the content inward whenever
                                // its width reaches the available window boundary.
                                this.snap_to_window()
                            })
                            .when(position == SelectPosition::Popper, |this| {
                                this.snap_to_window_with_margin(px(8.))
                            })
                            .child(
                                div()
                                    .occlude()
                                    .map(|this| match self.state.menu_width {
                                        Length::Auto => this.w(bounds.size.width).min_w(rems(9.)),
                                        Length::Definite(w) => this.w(w),
                                    })
                                    .child({
                                        let motion = cx.theme().style.motion;
                                        let popup = v_flex()
                                            .relative()
                                            .occlude()
                                            .bg(cx.theme().tokens.popover)
                                            .text_color(cx.theme().tokens.popover_foreground)
                                            .border_1()
                                            .border_color(
                                                cx.theme()
                                                    .foreground
                                                    .opacity(metrics.content_ring_opacity),
                                            )
                                            .rounded(metrics.content_radius)
                                            .when(cx.theme().style.elevation.enabled, |this| {
                                                match metrics.content_shadow {
                                                    SelectShadow::Medium => this.shadow_md(),
                                                    SelectShadow::ExtraLarge => this.shadow_2xl(),
                                                }
                                            })
                                            .when(!closing, |this| {
                                                this.on_key_down(
                                                    cx.listener(Self::typeahead_key_down),
                                                )
                                            })
                                            .child(
                                                List::new(&self.state.list)
                                                    .when_some(
                                                        self.state.search_placeholder.clone(),
                                                        |this, placeholder| {
                                                            this.search_placeholder(placeholder)
                                                        },
                                                    )
                                                    .with_size(Size::Medium)
                                                    .max_h(self.state.menu_max_h)
                                                    .paddings(Edges::all(px(4.))),
                                            )
                                            .when(closing, |this| {
                                                this.child(
                                                    div()
                                                        .absolute()
                                                        .top_0()
                                                        .left_0()
                                                        .size_full()
                                                        .occlude(),
                                                )
                                            });
                                        let state = cx.entity();
                                        Transition::new(motion.fast())
                                            .ease_token(if closing {
                                                motion.exit_easing
                                            } else {
                                                motion.enter_easing
                                            })
                                            .slide_y(
                                                if closing { px(0.) } else { px(-8.) },
                                                if closing { px(-8.) } else { px(0.) },
                                            )
                                            .when_some(active_transition, |this, transition| {
                                                this.on_complete(move |window, cx| {
                                                    state.update(cx, |state, cx| {
                                                        state.complete_motion(
                                                            opening, transition, window, cx,
                                                        );
                                                    });
                                                })
                                            })
                                            .apply(popup, "select-motion")
                                    })
                                    .when(!closing, |this| {
                                        this.on_mouse_down_out(cx.listener(
                                            |this, _, window, cx| {
                                                this.escape(&Cancel, window, cx);
                                            },
                                        ))
                                    }),
                            ),
                    )
                    .with_priority(1),
                )
            })
    }
}

impl<D> Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    pub fn new(state: &Entity<SelectState<D>>) -> Self {
        Self {
            id: ("select", state.entity_id()).into(),
            state: state.clone(),
            options: SelectOptions::default(),
            empty: None,
        }
    }

    /// Set the width of the dropdown menu, default: `Length::Auto`.
    pub fn menu_width(mut self, width: impl Into<Length>) -> Self {
        self.options.menu_width = width.into();
        self
    }

    /// Set the max height of the dropdown menu, default: 20rem.
    pub fn menu_max_h(mut self, max_h: impl Into<Length>) -> Self {
        self.options.menu_max_h = max_h.into();
        self
    }

    /// Set the content positioning mode.
    ///
    /// Non-searchable Selects default to [`SelectPosition::ItemAligned`]. Searchable Selects
    /// always use [`SelectPosition::Popper`] because the search field is not an option row.
    pub fn position(mut self, position: SelectPosition) -> Self {
        self.options.position = Some(position);
        self
    }

    /// Set the placeholder shown when no value is selected.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.options.placeholder = Some(placeholder.into());
        self
    }

    /// Set the accessible name for the Select trigger.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.options.aria_label = Some(label.into());
        self
    }

    /// Set the accessible description for the Select trigger.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.options.aria_description = Some(description.into());
        self
    }

    /// Override the trailing icon, replacing the default chevron.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.options.icon = Some(icon.into());
        self
    }

    /// Set a label prefix shown before the selected title in the trigger.
    ///
    /// e.g. `title_prefix("Country: ")` → "Country: United States"
    pub fn title_prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.options.title_prefix = Some(prefix.into());
        self
    }

    /// Show a clear button when a value is selected.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.options.cleanable = cleanable;
        self
    }

    /// Set the placeholder text for the search input.
    pub fn search_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.options.search_placeholder = Some(placeholder.into());
        self
    }

    /// Set the disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.options.disabled = disabled;
        self
    }

    /// Set the invalid state and destructive feedback treatment.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.options.invalid = invalid;
        self
    }

    /// Show separators between grouped sections.
    pub fn group_separators(mut self, separators: bool) -> Self {
        self.options.group_separators = separators;
        self
    }

    /// Set a custom closure that renders the empty-state element.
    pub fn empty<E: IntoElement + 'static>(
        mut self,
        builder: impl Fn(&mut Window, &App) -> E + 'static,
    ) -> Self {
        self.empty = Some(Box::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }

    /// Control whether the trigger shows a border and background (`true` by default).
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.options.appearance = appearance;
        self
    }
}

impl<D> Sizable for Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.options.size = size.into();
        self
    }
}

impl<D> EventEmitter<SelectEvent<D>> for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
}

impl<D> EventEmitter<DismissEvent> for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
}

impl<D> Focusable for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        if self.state.open {
            self.state.active_list_focus_handle(self.searchable)
        } else {
            self.state.focus_handle.clone()
        }
    }
}

impl<D> Styled for Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.options.style
    }
}

impl<D> RenderOnce for Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let disabled = self.options.disabled;
        let invalid = self.options.invalid;
        let aria_label = self.options.aria_label.clone();
        let aria_description = self.options.aria_description.clone();
        let is_open = self.state.read(cx).state.open;
        let selected_value = self
            .state
            .read(cx)
            .state
            .selection
            .first()
            .map(|(_, item)| item.title());
        let focus_handle = self.state.focus_handle(cx);
        let empty = self.empty;
        let opts = self.options;

        self.state.update(cx, |this, _| {
            this.state.style = opts.style;
            this.state.size = opts.size;
            this.state.cleanable = opts.cleanable;
            this.state.placeholder = opts.placeholder;
            this.state.search_placeholder = opts.search_placeholder;
            this.state.menu_width = opts.menu_width;
            this.state.menu_max_h = opts.menu_max_h;
            if this.position != opts.position {
                this.item_aligned_selected_center = None;
                this.position = opts.position;
            }
            this.state.disabled = opts.disabled;
            this.state.appearance = opts.appearance;
            this.invalid = opts.invalid;
            this.group_separators = opts.group_separators;
            this.icon = opts.icon;
            this.title_prefix = opts.title_prefix;

            if let Some(empty) = empty {
                this.state.empty = Some(empty);
            }
        });

        let element = div()
            .id(self.id.clone())
            .role(Role::ComboBox)
            .aria_expanded(is_open)
            .when_some(aria_label, |this, label| this.aria_label(label))
            .when_some(aria_description, |this, description| {
                this.aria_description(description)
            })
            .when_some(selected_value, |this, value| this.aria_value(value))
            .key_context(CONTEXT)
            .when(!disabled, |this| {
                this.track_focus(&focus_handle.tab_stop(true))
            })
            .on_action(window.listener_for(&self.state, SelectState::up))
            .on_action(window.listener_for(&self.state, SelectState::down))
            .on_action(window.listener_for(&self.state, SelectState::enter))
            .on_action(window.listener_for(&self.state, SelectState::escape))
            .on_key_down(window.listener_for(&self.state, SelectState::typeahead_key_down))
            .on_key_up(window.listener_for(&self.state, SelectState::release_open_key))
            .size_full()
            .child(self.state);

        crate::accessibility::accessibility_state(element, invalid, false, disabled)
    }
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use gpui::{
        AppContext as _, Context, Element as _, Entity, IntoElement, KeyDownEvent, Keystroke,
        Render, RenderOnce as _, Role, TestAppContext, VisualTestContext, Window,
    };

    use crate::{
        IndexPath,
        animation::OverlayPhase,
        searchable_list::SearchableVec,
        select::{Select, SelectGroup, SelectMetrics, SelectPosition, SelectShadow, SelectState},
        theme::StylePreset,
    };

    struct KeyboardFixture {
        state: Entity<SelectState<Vec<&'static str>>>,
    }

    #[derive(Clone, PartialEq)]
    struct DisabledTestOption {
        title: &'static str,
        disabled: bool,
    }

    impl crate::searchable_list::SearchableListItem for DisabledTestOption {
        type Value = &'static str;

        fn title(&self) -> gpui::SharedString {
            self.title.into()
        }

        fn value(&self) -> &Self::Value {
            &self.title
        }

        fn disabled(&self) -> bool {
            self.disabled
        }
    }

    struct DisabledFixture {
        state: Entity<SelectState<Vec<DisabledTestOption>>>,
    }

    impl Render for DisabledFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Select::new(&self.state).aria_label("Framework")
        }
    }

    impl Render for KeyboardFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Select::new(&self.state).aria_label("Fruit")
        }
    }

    fn keyboard_fixture(
        cx: &mut TestAppContext,
    ) -> (
        Entity<SelectState<Vec<&'static str>>>,
        &mut VisualTestContext,
    ) {
        let state_slot = std::rc::Rc::new(std::cell::RefCell::new(None));
        let captured_state = state_slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let state = cx
                .new(|cx| SelectState::new(vec!["Apple", "Banana", "Blueberry"], None, window, cx));
            *captured_state.borrow_mut() = Some(state.clone());
            let fixture = cx.new(|_| KeyboardFixture { state });
            crate::Root::new(fixture, window, cx)
        });
        let state = state_slot
            .borrow_mut()
            .take()
            .expect("fixture must expose Select state");
        (state, cx)
    }

    #[gpui::test]
    fn test_select_initial_selection_seeds_cursor(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let items = SearchableVec::new(vec!["Rust", "Go", "C++"]);
            let state = cx.new(|cx| SelectState::new(items, Some(IndexPath::new(1)), window, cx));

            assert_eq!(
                state.read(cx).selected_index(cx),
                Some(IndexPath::new(1)),
                "initial cursor should be seeded on ListState so display_title can read it",
            );
            assert_eq!(state.read(cx).selected_value(), Some(&"Go"));
        });
    }

    #[test]
    fn select_metrics_match_builtin_shadcn_presets() {
        let vega = SelectMetrics::from_style(crate::Size::Medium, &StylePreset::vega());
        let nova = SelectMetrics::from_style(crate::Size::Medium, &StylePreset::nova());
        let maia = SelectMetrics::from_style(crate::Size::Medium, &StylePreset::maia());

        assert_eq!(vega.item_height, gpui::px(32.));
        assert_eq!(nova.item_height, gpui::px(28.));
        assert_eq!(maia.item_height, gpui::px(36.));
        assert_eq!(vega.content_shadow, SelectShadow::Medium);
        assert_eq!(nova.content_shadow, SelectShadow::Medium);
        assert_eq!(maia.content_shadow, SelectShadow::ExtraLarge);
        assert_eq!(vega.content_radius, StylePreset::vega().radii.md);
        assert_eq!(nova.content_radius, StylePreset::nova().radii.lg);
        assert_eq!(maia.content_radius, StylePreset::maia().radii.lg);
    }

    #[gpui::test]
    fn select_position_follows_searchability_and_safe_overrides(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let plain = cx.new(|cx| SelectState::new(vec!["One"], None, window, cx));
            let searchable =
                cx.new(|cx| SelectState::new(vec!["One"], None, window, cx).searchable(true));

            assert_eq!(
                plain.read(cx).effective_position(),
                SelectPosition::ItemAligned
            );
            assert_eq!(
                searchable.read(cx).effective_position(),
                SelectPosition::Popper
            );

            Select::new(&plain)
                .position(SelectPosition::Popper)
                .render(window, cx);
            assert_eq!(plain.read(cx).effective_position(), SelectPosition::Popper);

            Select::new(&searchable)
                .position(SelectPosition::ItemAligned)
                .render(window, cx);
            assert_eq!(
                searchable.read(cx).effective_position(),
                SelectPosition::Popper
            );
        });
    }

    #[gpui::test]
    fn select_exposes_name_value_expansion_and_disabled_state(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let items = SearchableVec::new(vec!["Rust", "Go", "C++"]);
            let state = cx.new(|cx| SelectState::new(items, Some(IndexPath::new(1)), window, cx));
            let mut node = gpui::accesskit::Node::new(Role::ComboBox);

            Select::new(&state)
                .aria_label("Language")
                .aria_description("Choose a language")
                .invalid(true)
                .disabled(true)
                .render(window, cx)
                .into_element()
                .write_a11y_info(&mut node);

            assert_eq!(node.label(), Some("Language"));
            assert_eq!(node.description(), Some("Choose a language"));
            assert_eq!(node.value(), Some("Go"));
            assert_eq!(node.is_expanded(), Some(false));
            assert_eq!(node.invalid(), Some(gpui::accesskit::Invalid::True));
            assert!(node.is_disabled());
        });
    }

    #[gpui::test]
    fn test_select_initial_grouped_selection_seeds_cursor(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let mut groups: SearchableVec<SelectGroup<&'static str>> = SearchableVec::new(vec![]);
            groups.push(SelectGroup::new("A").items(["Apple", "Avocado"]));
            groups.push(SelectGroup::new("B").items(["Banana", "Blueberry", "Blackberry"]));

            let initial = IndexPath::new(1).section(1);
            let state = cx.new(|cx| SelectState::new(groups, Some(initial), window, cx));

            assert_eq!(state.read(cx).selected_index(cx), Some(initial));
            assert_eq!(state.read(cx).selected_value(), Some(&"Blueberry"));
        });
    }

    #[gpui::test]
    fn test_select_reopen_invalidates_pending_close(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let state = cx.update(|window, cx| {
            let items = SearchableVec::new(vec!["Rust", "Go", "C++"]);
            cx.new(|cx| SelectState::new(items, None, window, cx))
        });

        let opening = cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_open(true, window, cx);
                state.lifecycle.active_transition().unwrap()
            })
        });

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.complete_motion(true, opening, window, cx);
                assert_eq!(state.lifecycle.phase(), OverlayPhase::Open);
            });
        });

        let (closing, reopening) = cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_open(false, window, cx);
                assert_eq!(state.lifecycle.phase(), OverlayPhase::Closing);
                let closing = state.lifecycle.active_transition().unwrap();
                state.set_open(true, window, cx);
                assert_eq!(state.lifecycle.phase(), OverlayPhase::Opening);
                let reopening = state.lifecycle.active_transition().unwrap();
                (closing, reopening)
            })
        });

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.complete_motion(false, closing, window, cx);
                assert_eq!(state.lifecycle.phase(), OverlayPhase::Opening);
                state.complete_motion(true, reopening, window, cx);
                assert_eq!(state.lifecycle.phase(), OverlayPhase::Open);
                assert!(state.state.open);
            });
        });
    }

    #[gpui::test]
    fn space_opens_select_and_key_repeat_does_not_toggle_it_closed(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (state, cx) = keyboard_fixture(cx);
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
            keystroke: space,
            is_held: true,
            prefer_character_input: false,
        });
        cx.run_until_parked();

        assert!(cx.update(|_, cx| state.read(cx).state.open));
        assert_eq!(
            cx.update(|_, cx| state.read(cx).selected_index(cx)),
            Some(IndexPath::new(0))
        );
        assert!(cx.update(|_, cx| state.read(cx).state.selection.is_empty()));
    }

    #[gpui::test]
    fn item_aligned_select_uses_first_enabled_item_as_temporary_cursor(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state_slot = std::rc::Rc::new(std::cell::RefCell::new(None));
        let captured_state = state_slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let state = cx.new(|cx| {
                SelectState::new(
                    vec![
                        DisabledTestOption {
                            title: "Disabled",
                            disabled: true,
                        },
                        DisabledTestOption {
                            title: "Enabled",
                            disabled: false,
                        },
                    ],
                    None,
                    window,
                    cx,
                )
            });
            *captured_state.borrow_mut() = Some(state.clone());
            let fixture = cx.new(|_| DisabledFixture { state });
            crate::Root::new(fixture, window, cx)
        });
        let state = state_slot
            .borrow_mut()
            .take()
            .expect("fixture must expose Select state");

        cx.update(|window, cx| {
            _ = window.draw(cx);
            state.update(cx, |state, cx| state.set_open(true, window, cx));
            _ = window.draw(cx);
        });

        assert_eq!(
            cx.update(|_, cx| state.read(cx).selected_index(cx)),
            Some(IndexPath::new(1))
        );
        assert!(cx.update(|_, cx| state.read(cx).state.selection.is_empty()));
    }

    #[gpui::test]
    fn blur_reconciliation_does_not_reenter_updating_list_state(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let state = cx.update(|window, cx| {
            let mut groups = SearchableVec::new(Vec::new());
            groups.push(SelectGroup::new("A").items(["Australia", "Austria"]));
            groups.push(SelectGroup::new("B").items(["Brazil", "Belgium"]));
            cx.new(|cx| SelectState::new(groups, None, window, cx).searchable(true))
        });
        let list = cx.update(|_, cx| state.read(cx).state.list.clone());

        cx.update(|window, cx| {
            list.update(cx, |_, cx| {
                state.update(cx, |state, cx| {
                    state.on_blur(window, cx);
                });
            });
        });
        cx.run_until_parked();

        assert!(!cx.update(|_, cx| state.read(cx).state.open));
    }

    #[gpui::test]
    fn close_requested_from_list_callback_does_not_read_list_state(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let state = cx.update(|window, cx| {
            cx.new(|cx| SelectState::new(vec!["Apple", "Banana"], None, window, cx))
        });
        let list = cx.update(|_, cx| state.read(cx).state.list.clone());

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_open(true, window, cx);
            });
            list.update(cx, |_, cx| {
                state.update(cx, |state, cx| {
                    state.set_open(false, window, cx);
                });
            });
        });

        assert_eq!(
            cx.update(|_, cx| state.read(cx).lifecycle.phase()),
            OverlayPhase::Closing
        );
    }

    #[gpui::test]
    fn printable_character_selects_the_next_matching_option(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (state, cx) = keyboard_fixture(cx);
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
        });

        cx.simulate_event(KeyDownEvent {
            keystroke: Keystroke {
                key: "b".into(),
                key_char: Some("b".into()),
                ..Default::default()
            },
            is_held: false,
            prefer_character_input: true,
        });
        cx.run_until_parked();

        assert_eq!(
            cx.update(|_, cx| state.read(cx).selected_value().copied()),
            Some("Banana")
        );
    }
}
