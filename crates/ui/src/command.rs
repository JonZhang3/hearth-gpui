use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement,
    Styled, Task, WeakEntity, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{
    ActiveTheme as _, Disableable, Icon, IconName, IndexPath, Selectable, Sizable, Size,
    StyledExt as _, WindowExt as _,
    button::Button,
    dialog::Dialog,
    h_flex,
    input::{Input, InputGroup, InputGroupAddon},
    list::{List, ListDelegate, ListState},
    theme::{Density, StylePreset},
    v_flex,
};

type CommandItemRenderer = Rc<dyn Fn(bool, bool, &mut Window, &mut App) -> AnyElement + 'static>;
type CommandSelectHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// Events emitted by [`CommandState`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandEvent {
    /// A command item was activated by pointer or keyboard.
    Select(SharedString),
    /// Escape requested cancellation.
    Cancel,
}

/// Component-local Command geometry derived from semantic StylePreset values.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CommandMetrics {
    root_radius: gpui::Pixels,
    input_height: gpui::Pixels,
    input_radius: gpui::Pixels,
    item_radius: gpui::Pixels,
    item_padding_x: gpui::Pixels,
    item_padding_y: gpui::Pixels,
    heading_padding_x: gpui::Pixels,
    heading_padding_y: gpui::Pixels,
}

impl CommandMetrics {
    /// Resolves Vega, Nova, and Maia intent without branching on preset identity.
    fn resolve(style: &StylePreset) -> Self {
        match style.density {
            Density::Compact | Density::Standard => Self {
                root_radius: style.radii.xl,
                input_height: px(32.),
                input_radius: style.radii.lg,
                item_radius: style.radii.sm,
                item_padding_x: px(8.),
                item_padding_y: px(6.),
                heading_padding_x: px(8.),
                heading_padding_y: px(6.),
            },
            Density::Comfortable => Self {
                root_radius: style.radii.xl,
                input_height: px(36.),
                input_radius: style.radii.xl,
                item_radius: style.radii.lg,
                item_padding_x: px(12.),
                item_padding_y: px(8.),
                heading_padding_x: px(12.),
                heading_padding_y: px(8.),
            },
        }
    }
}

/// Trailing keyboard shortcut text shown by a command item.
#[derive(Clone, IntoElement)]
pub struct CommandShortcut {
    text: SharedString,
    style: StyleRefinement,
}

impl CommandShortcut {
    /// Creates shortcut text such as `⌘K` or `Ctrl P`.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            style: StyleRefinement::default(),
        }
    }
}

impl Styled for CommandShortcut {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CommandShortcut {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .ml_auto()
            .flex_shrink_0()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .child(self.text)
    }
}

/// A selectable command with optional icon, keywords, shortcut, and checked state.
#[derive(Clone)]
pub struct CommandItem {
    id: SharedString,
    label: SharedString,
    keywords: Vec<SharedString>,
    icon: Option<Icon>,
    shortcut: Option<CommandShortcut>,
    checked: bool,
    disabled: bool,
    style: StyleRefinement,
    renderer: Option<CommandItemRenderer>,
    on_select: Option<CommandSelectHandler>,
}

impl CommandItem {
    /// Creates a command with stable identity and searchable label.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            keywords: Vec::new(),
            icon: None,
            shortcut: None,
            checked: false,
            disabled: false,
            style: StyleRefinement::default(),
            renderer: None,
            on_select: None,
        }
    }

    /// Adds a searchable alias without changing the visible label.
    pub fn keyword(mut self, keyword: impl Into<SharedString>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    /// Adds multiple searchable aliases.
    pub fn keywords(mut self, keywords: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.keywords.extend(keywords.into_iter().map(Into::into));
        self
    }

    /// Adds a leading decorative icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Adds a trailing shortcut. Shortcut presence replaces the check indicator.
    pub fn shortcut(mut self, shortcut: CommandShortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Sets an independent checked state without changing keyboard activity.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Prevents pointer and keyboard activation.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Replaces the leading and label region while retaining trailing semantics.
    pub fn renderer(
        mut self,
        renderer: impl Fn(bool, bool, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.renderer = Some(Rc::new(renderer));
        self
    }

    /// Runs when the item is confirmed by pointer or Enter.
    pub fn on_select(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Returns the stable command identifier.
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    fn matches(&self, normalized_query: &str) -> bool {
        normalized_query.is_empty()
            || self.label.to_lowercase().contains(normalized_query)
            || self
                .keywords
                .iter()
                .any(|keyword| keyword.to_lowercase().contains(normalized_query))
    }
}

impl Styled for CommandItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// A labeled collection of command items.
#[derive(Clone)]
pub struct CommandGroup {
    id: SharedString,
    heading: Option<SharedString>,
    items: Vec<CommandItem>,
}

impl CommandGroup {
    /// Creates an empty group with stable identity.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            heading: None,
            items: Vec::new(),
        }
    }

    /// Sets the visible group heading.
    pub fn heading(mut self, heading: impl Into<SharedString>) -> Self {
        self.heading = Some(heading.into());
        self
    }

    /// Appends a command item.
    pub fn item(mut self, item: CommandItem) -> Self {
        self.items.push(item);
        self
    }

    /// Appends multiple command items.
    pub fn items(mut self, items: impl IntoIterator<Item = CommandItem>) -> Self {
        self.items.extend(items);
        self
    }
}

/// Empty result content rendered below the search input.
#[derive(Clone)]
pub struct CommandEmpty {
    text: SharedString,
    style: StyleRefinement,
}

impl CommandEmpty {
    /// Creates a text empty state.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            style: StyleRefinement::default(),
        }
    }
}

impl Styled for CommandEmpty {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// Visual divider placed before the next visible command group.
#[derive(Clone, Default)]
pub struct CommandSeparator;

impl CommandSeparator {
    /// Creates a group separator marker.
    pub fn new() -> Self {
        Self
    }
}

#[derive(Clone)]
struct CommandSection {
    group: CommandGroup,
    separator_before: bool,
}

/// Compositional data source for a Command surface.
#[derive(Clone)]
pub struct CommandList {
    sections: Vec<CommandSection>,
    empty: CommandEmpty,
    pending_separator: bool,
}

impl CommandList {
    /// Creates an empty command list.
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
            empty: CommandEmpty::new("No results found."),
            pending_separator: false,
        }
    }

    /// Sets the no-result presentation.
    pub fn empty(mut self, empty: CommandEmpty) -> Self {
        self.empty = empty;
        self
    }

    /// Appends a group and applies any pending separator before it.
    pub fn group(mut self, group: CommandGroup) -> Self {
        self.sections.push(CommandSection {
            group,
            separator_before: self.pending_separator,
        });
        self.pending_separator = false;
        self
    }

    /// Places a separator before the next group.
    pub fn separator(mut self, _: CommandSeparator) -> Self {
        self.pending_separator = !self.sections.is_empty();
        self
    }
}

impl Default for CommandList {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct VisibleCommandSection {
    id: SharedString,
    heading: Option<SharedString>,
    items: Vec<CommandItem>,
    separator_before: bool,
}

struct CommandDelegate {
    source: CommandList,
    visible: Vec<VisibleCommandSection>,
    selected_index: Option<IndexPath>,
    owner: WeakEntity<CommandState>,
}

impl CommandDelegate {
    fn new(source: CommandList, owner: WeakEntity<CommandState>) -> Self {
        let mut this = Self {
            source,
            visible: Vec::new(),
            selected_index: None,
            owner,
        };
        this.filter("");
        this
    }

    /// Rebuilds visible sections with stable source ordering.
    fn filter(&mut self, query: &str) {
        let query = query.trim().to_lowercase();
        let visible_indices = self
            .source
            .sections
            .iter()
            .enumerate()
            .filter_map(|(index, section)| {
                let items = section
                    .group
                    .items
                    .iter()
                    .filter(|item| item.matches(&query))
                    .cloned()
                    .collect::<Vec<_>>();
                (!items.is_empty()).then_some((index, section, items))
            })
            .collect::<Vec<_>>();

        self.visible = visible_indices
            .into_iter()
            .enumerate()
            .map(
                |(visible_index, (_, section, items))| VisibleCommandSection {
                    id: section.group.id.clone(),
                    heading: section.group.heading.clone(),
                    items,
                    separator_before: visible_index > 0 && section.separator_before,
                },
            )
            .collect();
    }

    fn item(&self, ix: IndexPath) -> Option<&CommandItem> {
        self.visible
            .get(ix.section)
            .and_then(|section| section.items.get(ix.row))
    }
}

#[derive(IntoElement)]
struct CommandRow {
    item: CommandItem,
    selected: bool,
    disabled: bool,
    size: Size,
}

impl Disableable for CommandRow {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for CommandRow {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Sizable for CommandRow {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for CommandRow {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = CommandMetrics::resolve(&cx.theme().style);
        let custom = self
            .item
            .renderer
            .as_ref()
            .map(|renderer| renderer(self.selected, self.item.checked, window, cx));
        let has_shortcut = self.item.shortcut.is_some();

        h_flex()
            .w_full()
            .min_w_0()
            .gap_2()
            .px(metrics.item_padding_x)
            .py(metrics.item_padding_y)
            .rounded(metrics.item_radius)
            .text_sm()
            .when(self.selected, |this| {
                this.bg(cx.theme().muted).text_color(cx.theme().foreground)
            })
            .when(self.disabled, |this| this.opacity(0.5))
            .refine_style(&self.item.style)
            .children(custom)
            .when(self.item.renderer.is_none(), |this| {
                this.children(self.item.icon.map(|icon| icon.small().flex_shrink_0()))
                    .child(div().min_w_0().flex_1().truncate().child(self.item.label))
            })
            .when_some(self.item.shortcut, |this, shortcut| this.child(shortcut))
            .when(!has_shortcut, |this| {
                this.child(
                    Icon::new(IconName::Check)
                        .small()
                        .ml_auto()
                        .flex_shrink_0()
                        .opacity(if self.item.checked { 1. } else { 0. }),
                )
            })
    }
}

impl ListDelegate for CommandDelegate {
    type Item = CommandRow;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.filter(query);
        Task::ready(())
    }

    fn sections_count(&self, _: &App) -> usize {
        self.visible.len().max(1)
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        self.visible
            .get(section)
            .map_or(0, |section| section.items.len())
    }

    fn item_label(&self, ix: IndexPath, _: &App) -> SharedString {
        self.item(ix)
            .map(|item| item.label.clone())
            .unwrap_or_default()
    }

    fn is_item_enabled(&self, ix: IndexPath, _: &App) -> bool {
        self.item(ix).is_some_and(|item| !item.disabled)
    }

    fn item_toggled(&self, ix: IndexPath, _: &App) -> Option<bool> {
        self.item(ix).map(|item| item.checked)
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Self::Item {
        CommandRow {
            item: self.item(ix).cloned().expect("valid command index"),
            selected: false,
            disabled: false,
            size: Size::Medium,
        }
    }

    fn render_section_header(
        &mut self,
        section: usize,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        let section = self.visible.get(section)?.clone();
        let metrics = CommandMetrics::resolve(&cx.theme().style);

        Some(
            v_flex()
                .id(format!("command-group-{}", section.id))
                .relative()
                .w_full()
                .when(section.separator_before, |this| {
                    this.child(
                        div()
                            .absolute()
                            .top_0()
                            .left(px(-4.))
                            .right(px(-4.))
                            .h_px()
                            .bg(cx.theme().border),
                    )
                })
                .when_some(section.heading, |this, heading| {
                    this.child(
                        div()
                            .px(metrics.heading_padding_x)
                            .py(metrics.heading_padding_y)
                            .text_xs()
                            .font_medium()
                            .text_color(cx.theme().muted_foreground)
                            .child(heading),
                    )
                }),
        )
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        div()
            .w_full()
            .py_6()
            .text_center()
            .text_sm()
            .refine_style(&self.source.empty.style)
            .child(self.source.empty.text.clone())
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn confirm(&mut self, _: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let Some(item) = self.selected_index.and_then(|ix| self.item(ix)).cloned() else {
            return;
        };
        let owner = self.owner.clone();
        cx.defer_in(window, move |_, window, cx| {
            if let Some(handler) = item.on_select {
                handler(window, cx);
            }
            _ = owner.update(cx, |state, cx| {
                state.selected_id = Some(item.id.clone());
                cx.emit(CommandEvent::Select(item.id));
                cx.notify();
            });
        });
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let owner = self.owner.clone();
        cx.defer_in(window, move |_, window, cx| {
            let dismiss_dialog = owner
                .update(cx, |state, cx| {
                    cx.emit(CommandEvent::Cancel);
                    state.dismiss_dialog_on_cancel
                })
                .unwrap_or(false);
            if dismiss_dialog {
                window.close_dialog(cx);
            }
        });
    }
}

/// Entity-backed state for query, filtering, keyboard selection, and activation.
pub struct CommandState {
    list: Entity<ListState<CommandDelegate>>,
    selected_id: Option<SharedString>,
    dismiss_dialog_on_cancel: bool,
}

impl CommandState {
    /// Creates Command state from a compositional list.
    pub fn new(list: CommandList, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let owner = cx.entity().downgrade();
        let delegate = CommandDelegate::new(list, owner);
        let list = cx.new(|cx| {
            ListState::new(delegate, window, cx)
                .searchable(true)
                .initial_selected_index(Some(IndexPath::default()))
        });
        Self {
            list,
            selected_id: None,
            dismiss_dialog_on_cancel: false,
        }
    }

    /// Returns the current query string.
    pub fn query(&self, cx: &App) -> SharedString {
        self.list.read(cx).query_input().read(cx).value().into()
    }

    /// Replaces the current query and triggers filtering.
    pub fn set_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.list
            .update(cx, |list, cx| list.set_query(query, window, cx));
    }

    /// Returns the most recently confirmed command identifier.
    pub fn selected_id(&self) -> Option<&SharedString> {
        self.selected_id.as_ref()
    }

    /// Replaces all groups and reapplies the current query.
    pub fn set_items(&mut self, items: CommandList, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.query(cx).to_string();
        self.list.update(cx, |list, cx| {
            let delegate = list.delegate_mut();
            delegate.source = items;
            delegate.filter(&query);
            list.set_selected_index(None, window, cx);
            cx.notify();
        });
        self.selected_id = None;
    }

    /// Focuses the search field.
    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.list.update(cx, |list, cx| list.focus(window, cx));
    }

    /// Clears the query and committed selection.
    pub fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_id = None;
        self.list.update(cx, |list, cx| {
            list.set_query("", window, cx);
            list.set_selected_index(None, window, cx);
        });
    }
}

impl Focusable for CommandState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.list.focus_handle(cx)
    }
}

impl EventEmitter<CommandEvent> for CommandState {}

/// Search input configuration used by Command.
#[derive(Clone)]
pub struct CommandInput {
    placeholder: SharedString,
    aria_label: SharedString,
    disabled: bool,
    style: StyleRefinement,
}

impl CommandInput {
    /// Creates a search input with the default Command placeholder.
    pub fn new() -> Self {
        Self {
            placeholder: "Type a command or search...".into(),
            aria_label: "Search commands".into(),
            disabled: false,
            style: StyleRefinement::default(),
        }
    }

    /// Sets placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the accessible input name.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = label.into();
        self
    }

    /// Sets whether search editing is unavailable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Default for CommandInput {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for CommandInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// Inline Command surface backed by [`CommandState`].
#[derive(IntoElement)]
pub struct Command {
    state: Entity<CommandState>,
    input: CommandInput,
    style: StyleRefinement,
}

impl Command {
    /// Creates an inline Command surface.
    pub fn new(state: &Entity<CommandState>) -> Self {
        Self {
            state: state.clone(),
            input: CommandInput::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Replaces the default search input configuration.
    pub fn input(mut self, input: CommandInput) -> Self {
        self.input = input;
        self
    }
}

impl Styled for Command {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Command {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = CommandMetrics::resolve(&cx.theme().style);
        let input = self.input;
        let placeholder = input.placeholder.clone();
        let list = self.state.read(cx).list.clone();

        v_flex()
            .size_full()
            .overflow_hidden()
            .rounded(metrics.root_radius)
            .bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground)
            .p_1()
            .refine_style(&self.style)
            .child(
                List::new(&list)
                    .aria_label("Commands")
                    .search_placeholder(placeholder)
                    .scrollbar_visible(false)
                    .p_1()
                    .max_h(px(288.))
                    .search_renderer(move |state, active_label, _window, cx| {
                        let metrics = CommandMetrics::resolve(&cx.theme().style);
                        div()
                            .p_1()
                            .pb_0()
                            .child(
                                InputGroup::new(("command-input", state.entity_id()))
                                    .disabled(input.disabled)
                                    .bg(cx.theme().input.opacity(0.3))
                                    .border_color(cx.theme().input.opacity(0.3))
                                    .h(metrics.input_height)
                                    .rounded(metrics.input_radius)
                                    .shadow_none()
                                    .refine_style(&input.style)
                                    .input(
                                        Input::new(&state)
                                            .aria_label(input.aria_label.clone())
                                            .when_some(active_label, |this, label| {
                                                this.aria_description(label)
                                            })
                                            .disabled(input.disabled),
                                    )
                                    .addon(
                                        InputGroupAddon::new().pl_2().child(
                                            Icon::new(IconName::Search).small().opacity(0.5),
                                        ),
                                    ),
                            )
                            .into_any_element()
                    }),
            )
    }
}

/// Dialog adapter for a Command surface. The Dialog owns modal motion and dismissal.
#[derive(IntoElement)]
pub struct CommandDialog {
    state: Entity<CommandState>,
    input: CommandInput,
    trigger: Option<Button>,
    title: SharedString,
    description: SharedString,
    show_close_button: bool,
    style: StyleRefinement,
}

impl CommandDialog {
    /// Creates a Command dialog adapter around existing state.
    pub fn new(state: &Entity<CommandState>) -> Self {
        Self {
            state: state.clone(),
            input: CommandInput::new(),
            trigger: None,
            title: "Command Palette".into(),
            description: "Search for a command to run...".into(),
            show_close_button: false,
            style: StyleRefinement::default(),
        }
    }

    /// Sets a Button trigger without replacing its existing click handler.
    pub fn trigger(mut self, trigger: Button) -> Self {
        self.trigger = Some(trigger);
        self
    }

    /// Replaces the default search input configuration.
    pub fn input(mut self, input: CommandInput) -> Self {
        self.input = input;
        self
    }

    /// Sets the hidden semantic dialog title.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the hidden semantic dialog description.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }

    /// Controls the optional top-right close button.
    pub fn show_close_button(mut self, show: bool) -> Self {
        self.show_close_button = show;
        self
    }

    /// Opens a configured Command dialog programmatically.
    pub fn open(
        state: &Entity<CommandState>,
        input: CommandInput,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = state.clone();
        let focus = state.focus_handle(cx);
        state.update(cx, |state, _| state.dismiss_dialog_on_cancel = true);
        let margin_top = window.viewport_size().height / 3.;
        window.open_dialog(cx, move |dialog, _, _| {
            let state = state.clone();
            let input = input.clone();
            dialog
                .aria_label("Command Palette")
                .aria_description("Search for a command to run...")
                .show_close_button(false)
                .confirm_on_enter(false)
                .margin_top(margin_top)
                .initial_focus(focus.clone())
                .p_0()
                .content(move |content, _, _| {
                    content
                        .p_0()
                        .child(Command::new(&state).input(input.clone()).h(px(336.)))
                })
        });
    }
}

impl Styled for CommandDialog {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CommandDialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state
            .update(cx, |state, _| state.dismiss_dialog_on_cancel = true);
        let focus = self.state.focus_handle(cx);
        let state = self.state;
        let input = self.input;
        let metrics = CommandMetrics::resolve(&cx.theme().style);

        Dialog::new(cx)
            .when_some(self.trigger, |dialog, trigger| dialog.trigger(trigger))
            .aria_label(self.title)
            .aria_description(self.description)
            .show_close_button(self.show_close_button)
            .confirm_on_enter(false)
            .margin_top(window.viewport_size().height / 3.)
            .initial_focus(focus)
            .rounded(metrics.root_radius)
            .p_0()
            .refine_style(&self.style)
            .content(move |content, _, _| {
                content
                    .p_0()
                    .child(Command::new(&state).input(input.clone()).h(px(336.)))
            })
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    fn fixture() -> CommandList {
        CommandList::new()
            .group(
                CommandGroup::new("suggestions")
                    .heading("Suggestions")
                    .item(CommandItem::new("calendar", "Calendar").keyword("date"))
                    .item(CommandItem::new("disabled", "Disabled").disabled(true)),
            )
            .separator(CommandSeparator::new())
            .group(CommandGroup::new("settings").item(CommandItem::new("profile", "Profile")))
    }

    #[test]
    fn stable_filter_preserves_group_and_item_order() {
        let owner = WeakEntity::new_invalid();
        let mut delegate = CommandDelegate::new(fixture(), owner);
        delegate.filter("date");

        assert_eq!(delegate.visible.len(), 1);
        assert_eq!(delegate.visible[0].id.as_ref(), "suggestions");
        assert_eq!(delegate.visible[0].items[0].id.as_ref(), "calendar");
    }

    #[test]
    fn command_builders_preserve_item_and_input_contracts() {
        let item = CommandItem::new("theme", "Theme")
            .keyword("appearance")
            .checked(true)
            .disabled(true)
            .shortcut(CommandShortcut::new("⌘T"));
        let input = CommandInput::new()
            .placeholder("Search actions")
            .aria_label("Action search")
            .disabled(true);

        assert_eq!(item.id(), "theme");
        assert!(item.matches("appearance"));
        assert!(item.checked);
        assert!(item.disabled);
        assert!(item.shortcut.is_some());
        assert_eq!(input.placeholder, "Search actions");
        assert_eq!(input.aria_label, "Action search");
        assert!(input.disabled);
    }

    #[gpui::test]
    fn state_builder_and_query_round_trip(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let state = cx.update(|window, cx| cx.new(|cx| CommandState::new(fixture(), window, cx)));

        cx.update(|window, cx| {
            state.update(cx, |state, cx| state.set_query("profile", window, cx));
        });

        assert_eq!(cx.update(|_, cx| state.read(cx).query(cx)), "profile");
    }
}
