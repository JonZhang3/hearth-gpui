use futures::Stream as _;
use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
    time::Duration,
};

use gpui::{
    App, AppContext as _, Bounds, Context, FocusHandle, IntoElement, KeyBinding,
    ParentElement as _, Pixels, Point, Render, ScrollAnchor, ScrollHandle, SharedString,
    Styled as _, Task, Window, prelude::FluentBuilder as _, px,
};

use crate::{
    ElementExt,
    async_util::{Receiver, Sender, unbounded},
    input::{self, SelectAll},
    scroll::AutoScroll,
    text::{
        CodeBlockActionsFn, LegacyMarkdownStyle, MarkdownBlockRenderers, MarkdownExtensions,
        MarkdownLinkClickFn, MarkdownLinkHandler, MarkdownOptions, MarkdownResourcePolicy,
        MarkdownStyleProfile, TextViewStyle,
        document::ParsedDocument,
        format,
        node::{self, NodeContext},
        streaming::close_hanging_markdown,
    },
    v_flex,
};

const CONTEXT: &'static str = "TextView";
// Keep coalescing bounded so sustained streams still render intermediate updates.
const MAX_COALESCED_UPDATES_PER_PARSE: usize = 64;

/// Controls how provider deltas are batched and when an implicit stream settles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StreamingMarkdownConfig {
    /// Minimum delay between progressive Markdown parses during an explicit stream.
    pub parse_interval: Duration,
    /// Idle delay used by the backwards-compatible implicit streaming path.
    pub idle_settle_delay: Duration,
}

impl Default for StreamingMarkdownConfig {
    fn default() -> Self {
        Self {
            parse_interval: Duration::from_millis(24),
            idle_settle_delay: Duration::from_millis(300),
        }
    }
}

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys(vec![
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", input::Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", input::Copy, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", input::SelectAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", input::SelectAll, Some(CONTEXT)),
    ]);
}

/// The content format of the text view.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TextViewFormat {
    /// Markdown view
    Markdown,
    /// HTML view
    Html,
}

/// The state of a TextView.
pub struct TextViewState {
    pub(super) focus_handle: FocusHandle,
    pub(super) entity_id: gpui::EntityId,

    /// The bounds of the text view
    bounds: Bounds<Pixels>,

    pub(super) selectable: bool,
    scroll_handle: Option<ScrollHandle>,
    root_scroll_anchors: Vec<ScrollAnchor>,
    pending_scroll_block: Option<usize>,
    pub(super) text_view_style: TextViewStyle,
    pub(super) code_block_actions: Option<std::sync::Arc<CodeBlockActionsFn>>,
    pub(super) markdown_extensions: Arc<MarkdownExtensions>,
    pub(super) markdown_style: Arc<LegacyMarkdownStyle>,
    pub(super) markdown_block_renderers: Arc<MarkdownBlockRenderers>,
    pub(super) markdown_resource_policy: MarkdownResourcePolicy,
    markdown_options: MarkdownOptions,
    markdown_style_profile: MarkdownStyleProfile,
    search_matches: Arc<Vec<std::ops::Range<usize>>>,
    active_search_match: Option<usize>,
    pub(super) on_link_click: Option<Arc<MarkdownLinkClickFn>>,

    pub(super) is_selecting: bool,
    multi_click_selection: Option<TextViewMultiClickSelection>,
    selected_text_override: Option<String>,
    select_all: bool,
    pub(super) auto_scroll: AutoScroll,

    pub(super) parsed_content: ParsedContent,
    /// Visible text-line geometry collected during the latest paint.
    ///
    /// Inline elements append directly to this view-owned registry so passive
    /// scrolling does not perform an entity update for every visible fragment.
    selection_geometry: Arc<Mutex<Vec<Bounds<Pixels>>>>,
    /// Content format (markdown / html), used to parse synchronously on the
    /// main thread for full-replace updates.
    format: TextViewFormat,
    text: String,
    revision: usize,
    epoch: usize,
    rendered_revision: usize,
    source_revision: usize,
    settled_source_revision: usize,
    canonical_queued_source_revision: Option<usize>,
    canonical_parse_revisions: HashMap<usize, usize>,
    streaming_config: Option<StreamingMarkdownConfig>,
    streaming_generation: usize,
    streaming_pending: String,
    parsed_error: Option<SharedString>,
    tx: Sender<UpdateOptions>,
    _parse_task: Task<()>,
    _receive_task: Task<()>,
    _settle_task: Task<()>,
    _streaming_parse_task: Task<()>,
}

impl TextViewState {
    /// Create a Markdown TextViewState.
    pub(crate) fn markdown(text: &str, cx: &mut Context<Self>) -> Self {
        Self::new(TextViewFormat::Markdown, text, cx)
    }

    /// Create a HTML TextViewState.
    pub fn html(text: &str, cx: &mut Context<Self>) -> Self {
        Self::new(TextViewFormat::Html, text, cx)
    }

    /// Create a new TextViewState.
    fn new(format: TextViewFormat, text: &str, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let entity_id = cx.entity_id();

        let (tx, rx) = unbounded::<UpdateOptions>();
        let (tx_result, rx_result) = unbounded::<ParsedUpdate>();
        let _receive_task = cx.spawn({
            async move |weak_self, cx| {
                while let Ok(parsed_update) = rx_result.recv().await {
                    _ = weak_self.update(cx, |state, cx| {
                        if parsed_update.epoch != state.epoch
                            || parsed_update.revision <= state.rendered_revision
                        {
                            return;
                        }

                        match parsed_update.result {
                            Ok(content) => {
                                state.parsed_content = content;
                                state.rendered_revision = parsed_update.revision;
                                state.parsed_error = None;
                                if parsed_update.kind == UpdateKind::Canonical {
                                    let source_revision = state
                                        .canonical_parse_revisions
                                        .remove(&parsed_update.revision)
                                        .unwrap_or(state.source_revision);
                                    state.settled_source_revision = source_revision;
                                    if state.canonical_queued_source_revision
                                        == Some(source_revision)
                                    {
                                        state.canonical_queued_source_revision = None;
                                    }
                                }
                                if parsed_update.kind == UpdateKind::Replace && !state.is_selecting
                                {
                                    state.reset_selection();
                                }
                            }
                            Err(err) => {
                                if parsed_update.kind == UpdateKind::Replace {
                                    state.parsed_error = Some(err);
                                }
                            }
                        }
                        cx.notify();
                    });
                }
            }
        });

        let _parse_task = cx.background_spawn(UpdateFuture::new(format, rx, tx_result));

        let mut this = Self {
            focus_handle,
            entity_id,
            bounds: Bounds::default(),
            multi_click_selection: None,
            selected_text_override: None,
            select_all: false,
            selectable: false,
            scroll_handle: None,
            root_scroll_anchors: Vec::new(),
            pending_scroll_block: None,
            text_view_style: TextViewStyle::default(),
            code_block_actions: None,
            markdown_extensions: Arc::default(),
            markdown_style: Arc::default(),
            markdown_block_renderers: Arc::default(),
            markdown_resource_policy: MarkdownResourcePolicy::default(),
            markdown_options: MarkdownOptions::default(),
            markdown_style_profile: MarkdownStyleProfile::Agent,
            search_matches: Arc::default(),
            active_search_match: None,
            on_link_click: None,
            is_selecting: false,
            auto_scroll: AutoScroll::default(),
            parsed_content: Default::default(),
            selection_geometry: Arc::new(Mutex::new(Vec::new())),
            format,
            parsed_error: None,
            text: text.to_string(),
            revision: 0,
            epoch: 0,
            rendered_revision: 0,
            source_revision: 0,
            settled_source_revision: 0,
            canonical_queued_source_revision: None,
            canonical_parse_revisions: HashMap::new(),
            streaming_config: None,
            streaming_generation: 0,
            streaming_pending: String::new(),
            tx,
            _parse_task,
            _receive_task,
            _settle_task: Task::ready(()),
            _streaming_parse_task: Task::ready(()),
        };
        this.increment_update(&text, UpdateKind::Replace, cx);
        this
    }

    /// Get the text content.
    pub(crate) fn source(&self) -> SharedString {
        self.parsed_content.document.source.clone()
    }

    /// Return the complete received source, including provider deltas waiting
    /// for the next progressive parse.
    pub(crate) fn complete_source(&self) -> SharedString {
        self.text.clone().into()
    }

    /// Return whether a newer Markdown revision is waiting to be rendered.
    pub(crate) fn is_parsing(&self) -> bool {
        self.rendered_revision < self.revision || !self.streaming_pending.is_empty()
    }

    pub(crate) fn heading_offset(&self, slug: &str) -> Option<usize> {
        self.parsed_content
            .document
            .heading_slugs
            .get(slug)
            .copied()
    }

    pub(crate) fn footnote_definition_offset(&self, label: &str) -> Option<usize> {
        self.parsed_content
            .document
            .footnote_definitions
            .get(label)
            .copied()
    }

    pub(crate) fn root_block_starts(&self) -> Arc<[usize]> {
        self.parsed_content.document.root_block_starts.clone()
    }

    /// Scroll to the root Markdown block containing the canonical source offset.
    pub(crate) fn scroll_to_source_index(&mut self, source_index: usize, cx: &mut Context<Self>) {
        let starts = &self.parsed_content.document.root_block_starts;
        self.pending_scroll_block = Some(
            starts
                .partition_point(|start| *start <= source_index)
                .saturating_sub(1),
        );
        cx.notify();
    }

    /// Set whether the text is selectable, default false.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Set whether the text is selectable, default false.
    pub fn set_selectable(&mut self, selectable: bool, cx: &mut Context<Self>) {
        self.selectable = selectable;
        cx.notify();
    }

    /// Connect navigation and selection auto-scroll to the host-owned scroll area.
    pub(crate) fn set_scroll_handle(
        &mut self,
        scroll_handle: Option<ScrollHandle>,
        cx: &mut Context<Self>,
    ) {
        let availability_changed = self.scroll_handle.is_some() != scroll_handle.is_some();
        self.scroll_handle = scroll_handle;
        if availability_changed {
            self.root_scroll_anchors.clear();
            cx.notify();
        }
    }

    pub(super) fn has_scroll_handle(&self) -> bool {
        self.scroll_handle.is_some()
    }

    /// Start an explicit LLM streaming session with the default batching policy.
    pub(crate) fn begin_streaming(&mut self, cx: &mut Context<Self>) {
        self.begin_streaming_with_config(StreamingMarkdownConfig::default(), cx);
    }

    /// Start an explicit LLM streaming session with a custom batching policy.
    pub(crate) fn begin_streaming_with_config(
        &mut self,
        config: StreamingMarkdownConfig,
        cx: &mut Context<Self>,
    ) {
        if self.streaming_config.is_some() {
            self.flush_streaming_pending(cx);
        }
        self.streaming_generation = self.streaming_generation.wrapping_add(1);
        self.streaming_config = Some(config);
        self.streaming_pending.clear();
        self._settle_task = Task::ready(());
        self._streaming_parse_task = Task::ready(());
        cx.notify();
    }

    /// Set the text content.
    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let had_unparsed_streaming_text = !self.streaming_pending.is_empty();
        self.end_streaming_session();
        if self.text.as_str() == text && !had_unparsed_streaming_text {
            return;
        }

        self.text.clear();
        self.text.push_str(text);
        self.source_revision = self.source_revision.wrapping_add(1);
        self.canonical_queued_source_revision = None;
        self.parsed_error = None;
        self.increment_update(text, UpdateKind::Replace, cx);
    }

    /// Append partial text content to the existing text.
    pub fn push_str(&mut self, new_text: &str, cx: &mut Context<Self>) {
        if new_text.is_empty() {
            return;
        }
        self.text.push_str(new_text);
        self.source_revision = self.source_revision.wrapping_add(1);
        self.canonical_queued_source_revision = None;
        if self.streaming_config.is_some() {
            self.streaming_pending.push_str(new_text);
            self.schedule_streaming_parse(cx);
        } else {
            self.increment_update(new_text, UpdateKind::Append, cx);
            self.schedule_streaming_settle(cx);
        }
    }

    /// Request an immediate canonical parse of all accumulated streaming text.
    ///
    /// Streaming also settles automatically after a short idle period. Calling
    /// this method at an LLM stream's completion removes that delay. Appending
    /// more text afterwards starts a new revision and safely supersedes the
    /// pending canonical result.
    pub fn finish_streaming(&mut self, cx: &mut Context<Self>) {
        self.flush_streaming_pending(cx);
        self.end_streaming_session();
        self.queue_canonical_parse();
        cx.notify();
    }

    pub(crate) fn set_markdown_extensions(
        &mut self,
        markdown_extensions: Arc<MarkdownExtensions>,
        cx: &mut Context<Self>,
    ) {
        if self.markdown_extensions.revision() == markdown_extensions.revision() {
            return;
        }

        self.markdown_extensions = markdown_extensions;
        if self.format == TextViewFormat::Markdown {
            let text = self.text.clone();
            self.increment_update(&text, UpdateKind::Replace, cx);
        }
    }

    /// Replace optional Markdown behavior and reparse the canonical source.
    pub(crate) fn set_markdown_options(
        &mut self,
        options: MarkdownOptions,
        cx: &mut Context<Self>,
    ) {
        if self.markdown_options == options {
            return;
        }
        self.markdown_options = options;
        self.parsed_content.node_cx.markdown_options = options;
        if self.format == TextViewFormat::Markdown {
            let text = self.text.clone();
            self.increment_update(&text, UpdateKind::Replace, cx);
        }
    }

    /// Replace the built-in Markdown profile and reparse profile-sensitive breaks.
    pub(crate) fn set_markdown_style_profile(
        &mut self,
        profile: MarkdownStyleProfile,
        cx: &mut Context<Self>,
    ) {
        if self.markdown_style_profile == profile {
            return;
        }
        self.markdown_style_profile = profile;
        self.parsed_content.node_cx.markdown_style_profile = profile;
        if self.format == TextViewFormat::Markdown {
            let text = self.text.clone();
            self.increment_update(&text, UpdateKind::Replace, cx);
        }
    }

    /// Return the selected text.
    pub fn selected_text(&self) -> String {
        if self.select_all {
            return self.parsed_content.document.text();
        }

        if let Some(text) = &self.selected_text_override {
            return text.clone();
        }

        self.parsed_content.document.selected_text()
    }

    pub(crate) fn selected_source(&self) -> Option<&str> {
        self.parsed_content.document.selected_source()
    }

    /// Find literal query occurrences in canonical source and update rendered highlights.
    pub(crate) fn search(
        &mut self,
        query: &str,
        case_sensitive: bool,
        cx: &mut Context<Self>,
    ) -> usize {
        let matches = if query.is_empty() {
            Vec::new()
        } else {
            regex::RegexBuilder::new(&regex::escape(query))
                .case_insensitive(!case_sensitive)
                .build()
                .map(|pattern| {
                    pattern
                        .find_iter(&self.text)
                        .map(|matched| matched.range())
                        .collect()
                })
                .unwrap_or_default()
        };
        let count = matches.len();
        self.search_matches = Arc::new(matches);
        self.active_search_match = (count > 0).then_some(0);
        cx.notify();
        count
    }

    /// Replace source-indexed search ranges and select one active result.
    pub(crate) fn set_search_highlights(
        &mut self,
        highlights: Vec<std::ops::Range<usize>>,
        active: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        self.active_search_match = active.filter(|index| *index < highlights.len());
        self.search_matches = Arc::new(highlights);
        cx.notify();
    }

    /// Remove every source-indexed search highlight.
    pub(crate) fn clear_search_highlights(&mut self, cx: &mut Context<Self>) {
        if self.search_matches.is_empty() && self.active_search_match.is_none() {
            return;
        }
        self.search_matches = Arc::default();
        self.active_search_match = None;
        cx.notify();
    }

    pub(crate) fn search_highlights(&self) -> &[std::ops::Range<usize>] {
        &self.search_matches
    }

    pub(crate) fn active_search_match(&self) -> Option<usize> {
        self.active_search_match
    }

    /// Select the emphasized occurrence among the current search results.
    pub(crate) fn set_active_search_match(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        self.active_search_match = index.filter(|index| *index < self.search_matches.len());
        cx.notify();
    }

    fn increment_update(&mut self, text: &str, kind: UpdateKind, cx: &mut Context<Self>) {
        if kind == UpdateKind::Replace {
            self.epoch = self.epoch.wrapping_add(1);
            self.canonical_queued_source_revision = None;
            self.canonical_parse_revisions.clear();
        }
        self.revision += 1;
        let update_options = UpdateOptions {
            revision: self.revision,
            epoch: self.epoch,
            kind,
            pending_text: text.to_string(),
            markdown_extensions: self.markdown_extensions.clone(),
            seed_content: None,
        };

        // Full-replace updates (initial content / `set_text`) parse
        // synchronously on the main thread so the first layout already has the
        // correct height. Otherwise parsing finishes later on a background task
        // and the first layout sees an empty `parsed_content` (~0 height); when
        // this `TextView` is an item inside an outer `list` with `measure_all`,
        // off-screen items get measured at that empty height and the total
        // content height keeps growing as items scroll into view; the scrollbar
        // thumb jitters. Streaming appends stay async to avoid re-parsing the
        // whole document on every chunk.
        if kind == UpdateKind::Replace {
            match parse_content(self.format, ParsedContent::default(), text, &update_options) {
                Ok(content) => {
                    self.parsed_content = content;
                    self.rendered_revision = self.revision;
                    self.settled_source_revision = self.source_revision;
                    self.parsed_error = None;
                    if !self.is_selecting {
                        self.reset_selection();
                    }
                }
                Err(err) => {
                    self.parsed_error = Some(err);
                }
            }
            cx.notify();
            // Seed the background parser with the same full source. Without
            // this reset, the first append after non-empty initial content or
            // `set_text` would use the worker's stale baseline.
            _ = self.tx.try_send(UpdateOptions {
                kind: UpdateKind::Seed,
                seed_content: Some(self.parsed_content.clone()),
                ..update_options
            });
            return;
        }

        _ = self.tx.try_send(update_options);
    }

    fn schedule_streaming_settle(&mut self, cx: &mut Context<Self>) {
        let revision = self.revision;
        let source_revision = self.source_revision;
        let delay = StreamingMarkdownConfig::default().idle_settle_delay;
        self._settle_task = cx.spawn(async move |weak_self, cx| {
            cx.background_executor().timer(delay).await;
            _ = weak_self.update(cx, |state, cx| {
                if state.revision != revision || state.source_revision != source_revision {
                    return;
                }
                state.queue_canonical_parse();
                cx.notify();
            });
        });
    }

    /// Schedule one progressive parse for all deltas received during the cadence window.
    fn schedule_streaming_parse(&mut self, cx: &mut Context<Self>) {
        if !self._streaming_parse_task.is_ready() {
            return;
        }
        let generation = self.streaming_generation;
        let delay = self.streaming_config.unwrap_or_default().parse_interval;
        self._streaming_parse_task = cx.spawn(async move |weak_self, cx| {
            cx.background_executor().timer(delay).await;
            _ = weak_self.update(cx, |state, cx| {
                if state.streaming_generation != generation || state.streaming_config.is_none() {
                    return;
                }
                state.flush_streaming_pending(cx);
            });
        });
    }

    /// Submit accumulated provider deltas as one append parse.
    fn flush_streaming_pending(&mut self, cx: &mut Context<Self>) {
        self._streaming_parse_task = Task::ready(());
        if self.streaming_pending.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.streaming_pending);
        self.increment_update(&pending, UpdateKind::Append, cx);
    }

    /// Cancel timers and invalidate callbacks owned by the current streaming session.
    fn end_streaming_session(&mut self) {
        self.streaming_generation = self.streaming_generation.wrapping_add(1);
        self.streaming_config = None;
        self.streaming_pending.clear();
        self._settle_task = Task::ready(());
        self._streaming_parse_task = Task::ready(());
    }

    fn queue_canonical_parse(&mut self) {
        if self.canonical_queued_source_revision == Some(self.source_revision)
            || self.settled_source_revision == self.source_revision
        {
            return;
        }
        self.revision += 1;
        self.canonical_queued_source_revision = Some(self.source_revision);
        self.canonical_parse_revisions
            .insert(self.revision, self.source_revision);
        _ = self.tx.try_send(UpdateOptions {
            revision: self.revision,
            epoch: self.epoch,
            kind: UpdateKind::Canonical,
            pending_text: self.text.clone(),
            markdown_extensions: self.markdown_extensions.clone(),
            seed_content: None,
        });
    }

    /// Save bounds and unselect if bounds changed.
    pub(super) fn update_bounds(&mut self, bounds: Bounds<Pixels>) {
        if self.bounds.size != bounds.size {
            self.reset_selection();
        }
        self.bounds = bounds;
    }

    pub(super) fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    /// Clear text geometry before painting the current frame.
    pub(super) fn clear_selection_geometry(&self) {
        if let Ok(mut geometry) = self.selection_geometry.lock() {
            geometry.clear();
        }
    }

    /// Append visible visual-line bounds without updating the owning entity.
    pub(super) fn register_selection_geometry(
        &self,
        bounds: impl IntoIterator<Item = Bounds<Pixels>>,
    ) {
        if let Ok(mut geometry) = self.selection_geometry.lock() {
            geometry.extend(bounds);
        }
    }

    /// Return whether a window position lies on painted text rather than row whitespace.
    pub(super) fn selection_geometry_contains(&self, position: Point<Pixels>) -> bool {
        self.selection_geometry
            .lock()
            .is_ok_and(|geometry| geometry.iter().any(|bounds| bounds.contains(&position)))
    }

    #[cfg(test)]
    pub(super) fn selection_geometry_snapshot(&self) -> Vec<Bounds<Pixels>> {
        self.selection_geometry
            .lock()
            .map(|geometry| geometry.clone())
            .unwrap_or_default()
    }

    /// Whether this view has a view-local selection (select-all, multi-click, or override),
    /// independent of the window-level selection.
    pub(super) fn has_view_selection(&self) -> bool {
        self.select_all
            || self.multi_click_selection.is_some()
            || self.selected_text_override.is_some()
    }

    pub(super) fn stop_auto_scroll(&mut self) {
        self.auto_scroll.stop();
    }

    fn reset_selection(&mut self) {
        self.multi_click_selection = None;
        self.selected_text_override = None;
        self.select_all = false;
        self.is_selecting = false;
        self.auto_scroll.stop();
        // Clear the inline selection state synchronously, so offscreen
        // (virtualized) views that won't repaint don't leak stale selection
        // text into a new cross-view copy.
        self.parsed_content.document.clear_selection();
    }

    /// Clear the current text selection.
    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.reset_selection();
        cx.notify();
    }

    /// Select all rendered text in this view.
    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        self.multi_click_selection = None;
        self.selected_text_override = None;
        self.select_all = true;
        self.is_selecting = false;
        self.auto_scroll.stop();
        cx.notify();
    }

    pub(crate) fn set_multi_click_selection(
        &mut self,
        pos: Point<Pixels>,
        kind: TextViewMultiClickKind,
        selected_text: String,
    ) {
        let pos = pos - self.bounds.origin;
        self.multi_click_selection = Some(TextViewMultiClickSelection { pos, kind });
        self.selected_text_override = Some(selected_text);
        self.select_all = false;
        self.is_selecting = false;
        self.auto_scroll.stop();
    }

    pub(super) fn set_auto_scroll(&mut self, delta: Option<Pixels>, cx: &mut Context<Self>) {
        if self.scroll_handle.is_none() {
            self.auto_scroll.stop();
            return;
        }
        self.auto_scroll.set(delta, cx, |delta, state, cx| {
            let Some(handle) = &state.scroll_handle else {
                return;
            };
            let mut offset = handle.offset();
            offset.y = (offset.y - delta).clamp(-handle.max_offset().y, px(0.));
            handle.set_offset(offset);
            cx.notify();
        });
    }

    /// Return the window selection (anchor, cursor) in window coordinates if
    /// this view participates in it.
    ///
    /// Single-view fast path: when both endpoints are anchored inside one
    /// TextView, only that view participates (identical to the previous
    /// per-view behavior).
    pub(crate) fn selection_points(
        &self,
        window: &Window,
        cx: &App,
    ) -> Option<(Point<Pixels>, Point<Pixels>)> {
        if !self.selectable {
            return None;
        }
        let root = window.root::<crate::Root>().flatten()?;
        let selection = &root.read(cx).text_selection;
        if let Some(view_id) = selection.single_view() {
            if view_id != self.entity_id {
                return None;
            }
        }
        selection.resolved_points(cx)
    }

    pub(crate) fn has_selection(&self, window: &Window, cx: &App) -> bool {
        self.has_view_selection() || self.selection_points(window, cx).is_some()
    }

    pub(super) fn on_action_select_all(
        &mut self,
        _: &SelectAll,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.selectable {
            cx.propagate();
            return;
        }

        self.select_all(cx);
    }

    pub(crate) fn is_selectable(&self) -> bool {
        self.selectable
    }

    pub(crate) fn is_all_selected(&self) -> bool {
        self.select_all
    }

    pub(crate) fn multi_click_selection(&self) -> Option<TextViewMultiClickSelection> {
        self.multi_click_selection.map(|selection| {
            let pos = selection.pos + self.bounds.origin;
            TextViewMultiClickSelection { pos, ..selection }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextViewMultiClickSelection {
    pub(crate) pos: Point<Pixels>,
    pub(crate) kind: TextViewMultiClickKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextViewMultiClickKind {
    Word,
    Paragraph,
}

impl Render for TextViewState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.entity();
        let document = self.parsed_content.document.clone();
        let mut node_cx = self.parsed_content.node_cx.clone();

        node_cx.code_block_actions = self.code_block_actions.clone();
        node_cx.markdown_extensions = self.markdown_extensions.clone();
        node_cx.markdown_style = self.markdown_style.clone();
        node_cx.markdown_block_renderers = self.markdown_block_renderers.clone();
        node_cx.link_handler = MarkdownLinkHandler {
            policy: self.markdown_resource_policy,
            on_click: self.on_link_click.clone(),
        };
        node_cx.source = document.source.clone();
        node_cx.search_matches = self.search_matches.clone();
        node_cx.active_search_match = self.active_search_match;
        node_cx.style = self.text_view_style.clone();

        if self.root_scroll_anchors.len() != document.blocks.len() {
            self.root_scroll_anchors = self
                .scroll_handle
                .as_ref()
                .map(|handle| {
                    (0..document.blocks.len())
                        .map(|_| ScrollAnchor::for_handle(handle.clone()))
                        .collect()
                })
                .unwrap_or_default();
        }
        if let Some(ix) = self.pending_scroll_block
            && let Some(anchor) = self.root_scroll_anchors.get(ix)
        {
            anchor.scroll_to(window, cx);
            self.pending_scroll_block = None;
        }

        v_flex()
            .w_full()
            .map(|this| match &mut self.parsed_error {
                None => this.child(document.render_root(
                    &self.root_scroll_anchors,
                    &node_cx,
                    window,
                    cx,
                )),
                Some(err) => this.child(
                    v_flex()
                        .gap_1()
                        .child("Failed to parse content")
                        .child(err.to_string()),
                ),
            })
            .on_prepaint(move |bounds, window, cx| {
                let size_changed = state.read(cx).bounds().size != bounds.size;
                let id = state.entity_id();
                state.update(cx, |state, _| {
                    state.update_bounds(bounds);
                });
                if size_changed {
                    if let Some(root) = window.root::<crate::Root>().flatten() {
                        root.update(cx, |root, cx| {
                            root.clear_text_selection_for_resized_view(id, cx);
                        });
                    }
                }
            })
    }
}

#[derive(Clone, PartialEq, Default)]
pub(crate) struct ParsedContent {
    pub(crate) document: ParsedDocument,
    pub(crate) node_cx: node::NodeContext,
}

struct UpdateFuture {
    format: TextViewFormat,
    /// Complete received source, including chunks that have not parsed successfully.
    source: String,
    epoch: usize,
    content: ParsedContent,
    rx: Pin<Box<Receiver<UpdateOptions>>>,
    tx_result: Sender<ParsedUpdate>,
}

impl UpdateFuture {
    fn new(
        format: TextViewFormat,
        rx: Receiver<UpdateOptions>,
        tx_result: Sender<ParsedUpdate>,
    ) -> Self {
        Self {
            format,
            source: String::new(),
            epoch: 0,
            content: Default::default(),
            rx: Box::pin(rx),
            tx_result,
        }
    }
}

impl Future for UpdateFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.rx.as_mut().poll_next(cx) {
                Poll::Ready(Some(mut options)) => {
                    if options.kind == UpdateKind::Seed {
                        self.epoch = options.epoch;
                        self.source = options.pending_text;
                        self.content = options.seed_content.take().unwrap_or_default();
                        continue;
                    }
                    let hit_coalesce_budget =
                        merge_pending_options(&mut options, self.rx.as_ref().get_ref());
                    if options.kind == UpdateKind::Seed {
                        self.epoch = options.epoch;
                        self.source = options.pending_text;
                        self.content = options.seed_content.take().unwrap_or_default();
                        continue;
                    }

                    let res = self.parse_update(&options);
                    _ = self.tx_result.try_send(ParsedUpdate {
                        revision: options.revision,
                        epoch: options.epoch,
                        kind: options.kind,
                        result: res,
                    });
                    if hit_coalesce_budget {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    continue;
                }
                Poll::Ready(None) => return Poll::Ready(()),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl UpdateFuture {
    /// Parse one update while keeping received source separate from valid AST state.
    fn parse_update(&mut self, options: &UpdateOptions) -> Result<ParsedContent, SharedString> {
        self.update_source(options);
        let canonical = parse_content(self.format, self.content.clone(), &self.source, options)?;
        self.content = canonical.clone();

        if options.kind == UpdateKind::Append {
            Ok(streaming_display_content(
                self.format,
                canonical,
                &self.source,
                options,
            ))
        } else {
            Ok(canonical)
        }
    }

    /// Advance the source baseline independently from the last valid parsed AST.
    fn update_source(&mut self, options: &UpdateOptions) {
        if self.epoch != options.epoch {
            self.epoch = options.epoch;
            self.source.clear();
            self.content = ParsedContent::default();
        }

        match options.kind {
            UpdateKind::Append => self.source.push_str(&options.pending_text),
            UpdateKind::Replace | UpdateKind::Canonical | UpdateKind::Seed => {
                self.source.clone_from(&options.pending_text)
            }
        }
    }
}

/// Build a display-only replacement for the unstable Markdown tail.
///
/// The canonical AST remains the worker baseline. Synthetic closing markers
/// are parsed only for presentation and never become part of the retained
/// source, selection ranges, or the final settled document.
fn streaming_display_content(
    format: TextViewFormat,
    canonical: ParsedContent,
    complete_source: &str,
    options: &UpdateOptions,
) -> ParsedContent {
    if format != TextViewFormat::Markdown {
        return canonical;
    }
    let Some(last_block) = canonical.document.blocks.last() else {
        return canonical;
    };
    if last_block.is_code_block() {
        return canonical;
    }
    let Some(span) = last_block.span() else {
        return canonical;
    };
    let Some(tail) = complete_source.get(span.start..) else {
        return canonical;
    };
    let Some(repair) = close_hanging_markdown(tail) else {
        return canonical;
    };

    let mut display_node_cx = NodeContext {
        offset: span.start,
        link_refs: canonical.node_cx.link_refs.clone(),
        markdown_extensions: options.markdown_extensions.clone(),
        markdown_options: canonical.node_cx.markdown_options,
        markdown_style_profile: canonical.node_cx.markdown_style_profile,
        ..NodeContext::default()
    };
    let Ok(mut display_tail) = format::markdown::parse(&repair.markdown, &mut display_node_cx)
    else {
        return canonical;
    };
    if let Some(synthetic_suffix) = repair.synthetic_text_suffix
        && !display_tail.blocks.last_mut().is_some_and(|block| {
            Arc::make_mut(block).remove_trailing_synthetic_char(synthetic_suffix)
        })
    {
        return canonical;
    }
    for block in &mut display_tail.blocks {
        Arc::make_mut(block).clamp_spans(complete_source.len());
    }

    let mut display = canonical.clone();
    display.document.blocks.pop();
    display.document.blocks.extend(display_tail.blocks);
    display.document.source = complete_source.to_string().into();
    display
}

#[derive(Clone)]
struct UpdateOptions {
    revision: usize,
    epoch: usize,
    pending_text: String,
    kind: UpdateKind,
    markdown_extensions: Arc<MarkdownExtensions>,
    seed_content: Option<ParsedContent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UpdateKind {
    Replace,
    Append,
    Canonical,
    Seed,
}

impl UpdateOptions {
    fn merge(&mut self, next: UpdateOptions) {
        if next.epoch != self.epoch {
            *self = next;
            return;
        }
        if next.kind == UpdateKind::Append {
            self.pending_text.push_str(&next.pending_text);
            self.revision = next.revision;
            if self.kind == UpdateKind::Canonical {
                self.kind = UpdateKind::Canonical;
            }
        } else {
            *self = next;
        }
    }
}

struct ParsedUpdate {
    revision: usize,
    epoch: usize,
    kind: UpdateKind,
    result: Result<ParsedContent, SharedString>,
}

fn merge_pending_options(options: &mut UpdateOptions, rx: &Receiver<UpdateOptions>) -> bool {
    let mut update_count = 1;

    while update_count < MAX_COALESCED_UPDATES_PER_PARSE {
        match rx.try_recv() {
            Ok(next_options) => {
                let is_seed = next_options.kind == UpdateKind::Seed;
                options.merge(next_options);
                update_count += 1;
                if is_seed {
                    return false;
                }
            }
            Err(_) => return false,
        }
    }

    true
}

fn parse_content(
    format: TextViewFormat,
    mut content: ParsedContent,
    complete_source: &str,
    options: &UpdateOptions,
) -> Result<ParsedContent, SharedString> {
    let mut node_cx = NodeContext {
        markdown_extensions: options.markdown_extensions.clone(),
        markdown_options: content.node_cx.markdown_options,
        markdown_style_profile: content.node_cx.markdown_style_profile,
        ..NodeContext::default()
    };

    // `pulldown-cmark` resolves document-wide references and can reclassify a
    // preceding paragraph when a table row, setext underline, or definition
    // arrives. Parse the canonical source in the background, then recover the
    // unchanged Arc prefix. This keeps source semantics exact while preserving
    // stable rendered identity and cached layout for all unaffected blocks.
    let mut new_document = match format {
        TextViewFormat::Markdown => format::markdown::parse(complete_source, &mut node_cx),
        TextViewFormat::Html => format::html::parse(complete_source, &mut node_cx),
    }?;
    reuse_unchanged_prefix(&content.document, &mut new_document);
    content.document = new_document;
    content.node_cx = node_cx;

    Ok(content)
}

fn reuse_unchanged_prefix(previous: &ParsedDocument, next: &mut ParsedDocument) {
    for (old, new) in previous.blocks.iter().zip(next.blocks.iter_mut()) {
        if old != new {
            Arc::make_mut(new).reuse_runtime_state_from(old);
            break;
        }
        *new = old.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::MarkdownNode;
    use gpui::TestAppContext;

    #[test]
    fn streaming_markdown_default_matches_the_stream_frame_cadence() {
        assert_eq!(
            StreamingMarkdownConfig::default().parse_interval,
            Duration::from_millis(24)
        );
    }

    #[gpui::test]
    fn set_text_then_push_str_appends_to_replaced_content(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("old", cx)));
        cx.run_until_parked();

        state.update(cx, |state, cx| {
            state.set_text("", cx);
            state.push_str("new", cx);
            state.push_str(" text", cx);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            assert_eq!(state.text.as_str(), "new text");
            assert_eq!(state.source().as_str(), "new text");
        });

        state.update(cx, |state, cx| {
            state.set_text("", cx);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            assert_eq!(state.text.as_str(), "");
            assert_eq!(state.source().as_str(), "");
        });
    }

    #[gpui::test]
    fn non_empty_initial_text_is_the_streaming_baseline(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("base", cx)));
        cx.run_until_parked();

        state.update(cx, |state, cx| state.push_str(" tail", cx));
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            assert_eq!(state.text, "base tail");
            assert_eq!(state.source().as_ref(), "base tail");
        });
    }

    #[gpui::test]
    fn search_highlights_use_canonical_source_ranges(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("Alpha alpha", cx)));
        cx.run_until_parked();

        state.update(cx, |state, cx| {
            assert_eq!(state.search("alpha", false, cx), 2);
            assert_eq!(state.search_highlights(), &[0..5, 6..11]);
            assert_eq!(state.active_search_match(), Some(0));
            state.set_active_search_match(Some(1), cx);
            assert_eq!(state.active_search_match(), Some(1));
            state.clear_search_highlights(cx);
            assert!(state.search_highlights().is_empty());
            assert_eq!(state.active_search_match(), None);
        });
    }

    #[test]
    fn appended_reference_definitions_match_a_canonical_parse() {
        let extensions = Arc::new(MarkdownExtensions::default());
        let initial_options = UpdateOptions {
            revision: 1,
            epoch: 1,
            pending_text: "Read [the docs].".to_string(),
            kind: UpdateKind::Replace,
            markdown_extensions: extensions.clone(),
            seed_content: None,
        };
        let initial = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            "Read [the docs].",
            &initial_options,
        )
        .unwrap();
        let streamed = parse_content(
            TextViewFormat::Markdown,
            initial,
            "Read [the docs].\n\n[the docs]: https://example.com",
            &UpdateOptions {
                revision: 2,
                epoch: 1,
                pending_text: "\n\n[the docs]: https://example.com".to_string(),
                kind: UpdateKind::Append,
                markdown_extensions: extensions.clone(),
                seed_content: None,
            },
        )
        .unwrap();
        let canonical = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            "Read [the docs].\n\n[the docs]: https://example.com",
            &UpdateOptions {
                revision: 2,
                epoch: 1,
                pending_text: "Read [the docs].\n\n[the docs]: https://example.com".to_string(),
                kind: UpdateKind::Canonical,
                markdown_extensions: extensions,
                seed_content: None,
            },
        )
        .unwrap();

        assert_eq!(streamed.document, canonical.document);
    }

    #[test]
    fn unchanged_document_prefix_reuses_shared_blocks() {
        let extensions = Arc::new(MarkdownExtensions::default());
        let options = |source: &str| UpdateOptions {
            revision: 1,
            epoch: 1,
            pending_text: source.to_string(),
            kind: UpdateKind::Replace,
            markdown_extensions: extensions.clone(),
            seed_content: None,
        };
        let previous = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            "# Stable\n\nold tail",
            &options("# Stable\n\nold tail"),
        )
        .unwrap();
        let mut next = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            "# Stable\n\nnew tail",
            &options("# Stable\n\nnew tail"),
        )
        .unwrap();

        reuse_unchanged_prefix(&previous.document, &mut next.document);

        assert!(Arc::ptr_eq(
            &previous.document.blocks[0],
            &next.document.blocks[0]
        ));
        assert!(!Arc::ptr_eq(
            &previous.document.blocks[1],
            &next.document.blocks[1]
        ));
    }

    #[test]
    fn appending_a_reference_use_after_a_definition_matches_canonical_parse() {
        let extensions = Arc::new(MarkdownExtensions::default());
        let initial_source = "[docs]: https://example.com\n\nStart";
        let initial = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            initial_source,
            &UpdateOptions {
                revision: 1,
                epoch: 1,
                pending_text: initial_source.to_string(),
                kind: UpdateKind::Replace,
                markdown_extensions: extensions.clone(),
                seed_content: None,
            },
        )
        .unwrap();
        let streamed = parse_content(
            TextViewFormat::Markdown,
            initial,
            &format!("{initial_source} [docs]"),
            &UpdateOptions {
                revision: 2,
                epoch: 1,
                pending_text: " [docs]".to_string(),
                kind: UpdateKind::Append,
                markdown_extensions: extensions.clone(),
                seed_content: None,
            },
        )
        .unwrap();
        let canonical = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            &format!("{initial_source} [docs]"),
            &UpdateOptions {
                revision: 2,
                epoch: 1,
                pending_text: format!("{initial_source} [docs]"),
                kind: UpdateKind::Canonical,
                markdown_extensions: extensions,
                seed_content: None,
            },
        )
        .unwrap();

        assert_eq!(streamed.document, canonical.document);
    }

    #[test]
    fn plain_text_after_reference_definitions_remains_incremental() {
        let extensions = Arc::new(MarkdownExtensions::default());
        let initial_source = "[docs]: https://example.com\n\n# Stable\n\nTail";
        let initial = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            initial_source,
            &UpdateOptions {
                revision: 1,
                epoch: 1,
                pending_text: initial_source.to_string(),
                kind: UpdateKind::Replace,
                markdown_extensions: extensions.clone(),
                seed_content: None,
            },
        )
        .unwrap();
        let stable = initial.document.blocks[0].clone();
        let complete = format!("{initial_source} text");
        let streamed = parse_content(
            TextViewFormat::Markdown,
            initial,
            &complete,
            &UpdateOptions {
                revision: 2,
                epoch: 1,
                pending_text: " text".to_string(),
                kind: UpdateKind::Append,
                markdown_extensions: extensions.clone(),
                seed_content: None,
            },
        )
        .unwrap();
        let canonical = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            &complete,
            &UpdateOptions {
                revision: 2,
                epoch: 1,
                pending_text: complete.clone(),
                kind: UpdateKind::Canonical,
                markdown_extensions: extensions,
                seed_content: None,
            },
        )
        .unwrap();

        assert!(Arc::ptr_eq(&stable, &streamed.document.blocks[0]));
        assert_eq!(streamed.document, canonical.document);
    }

    #[test]
    fn streaming_display_mends_inline_tail_without_changing_canonical_source() {
        let extensions = Arc::new(MarkdownExtensions::default());
        let options = UpdateOptions {
            revision: 1,
            epoch: 1,
            pending_text: "**bold".to_string(),
            kind: UpdateKind::Append,
            markdown_extensions: extensions,
            seed_content: None,
        };
        let canonical = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            "**bold",
            &options,
        )
        .unwrap();
        let display = streaming_display_content(
            TextViewFormat::Markdown,
            canonical.clone(),
            "**bold",
            &options,
        );

        let node::BlockNode::Paragraph(paragraph) = &*display.document.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(paragraph.children[0].marks[0].1.bold);
        assert_eq!(display.document.source.as_ref(), "**bold");
        assert_ne!(display.document, canonical.document);
    }

    #[test]
    fn streaming_display_styles_pending_link_but_keeps_it_noncanonical() {
        let extensions = Arc::new(MarkdownExtensions::default());
        let source = "[docs](https://exa";
        let options = UpdateOptions {
            revision: 1,
            epoch: 1,
            pending_text: source.to_string(),
            kind: UpdateKind::Append,
            markdown_extensions: extensions,
            seed_content: None,
        };
        let canonical = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            source,
            &options,
        )
        .unwrap();
        let display =
            streaming_display_content(TextViewFormat::Markdown, canonical, source, &options);

        let node::BlockNode::Paragraph(paragraph) = &*display.document.blocks[0] else {
            panic!("expected paragraph");
        };
        let pending = paragraph.children[0].marks[0]
            .1
            .link
            .as_ref()
            .expect("pending link should retain link semantics");
        assert_eq!(
            pending.url.as_ref(),
            crate::text::streaming::PENDING_LINK_URL
        );
        assert_eq!(display.document.source.as_ref(), source);
        assert!(
            display.document.blocks[0]
                .span()
                .is_some_and(|span| span.end <= source.len())
        );
    }

    #[test]
    fn streaming_display_keeps_incomplete_images_as_literal_text() {
        let extensions = Arc::new(MarkdownExtensions::default());
        let source = "![alt](https://exa";
        let options = UpdateOptions {
            revision: 1,
            epoch: 1,
            pending_text: source.to_string(),
            kind: UpdateKind::Append,
            markdown_extensions: extensions,
            seed_content: None,
        };
        let canonical = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            source,
            &options,
        )
        .unwrap();
        let display = streaming_display_content(
            TextViewFormat::Markdown,
            canonical.clone(),
            source,
            &options,
        );

        let node::BlockNode::Paragraph(paragraph) = &*display.document.blocks[0] else {
            panic!("expected literal image syntax to remain a paragraph");
        };
        assert!(paragraph.children.iter().all(|child| child.image.is_none()));
        assert_eq!(display.document, canonical.document);
        assert!(display.document.text().contains(source));
    }

    #[test]
    fn streaming_display_removes_only_the_synthetic_setext_guard() {
        let extensions = Arc::new(MarkdownExtensions::default());
        let source = "heading\n---";
        let options = UpdateOptions {
            revision: 1,
            epoch: 1,
            pending_text: source.to_string(),
            kind: UpdateKind::Append,
            markdown_extensions: extensions.clone(),
            seed_content: None,
        };
        let canonical = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            source,
            &options,
        )
        .unwrap();
        let display =
            streaming_display_content(TextViewFormat::Markdown, canonical, source, &options);

        assert!(matches!(
            *display.document.blocks[0],
            node::BlockNode::Paragraph(_)
        ));
        assert_eq!(display.document.text(), "heading\n—\n");
        assert!(!display.document.text().contains('\u{200B}'));
        assert_eq!(display.document.source.as_ref(), source);

        let user_source = "heading\n---\u{200B}";
        let canonical_user_content = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            user_source,
            &UpdateOptions {
                revision: 2,
                epoch: 1,
                pending_text: user_source.to_string(),
                kind: UpdateKind::Append,
                markdown_extensions: extensions,
                seed_content: None,
            },
        )
        .unwrap();
        assert!(canonical_user_content.document.text().contains('\u{200B}'));
    }

    #[test]
    fn streaming_display_does_not_mend_code_blocks() {
        let extensions = Arc::new(MarkdownExtensions::default());
        let source = "```rust\nlet value = **raw";
        let options = UpdateOptions {
            revision: 1,
            epoch: 1,
            pending_text: source.to_string(),
            kind: UpdateKind::Append,
            markdown_extensions: extensions,
            seed_content: None,
        };
        let canonical = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            source,
            &options,
        )
        .unwrap();
        let display = streaming_display_content(
            TextViewFormat::Markdown,
            canonical.clone(),
            source,
            &options,
        );
        assert_eq!(display.document, canonical.document);
    }

    #[test]
    fn arbitrary_utf8_stream_chunks_match_a_canonical_parse() {
        let source = "# 标题\n\nA **strong** [link][docs].\n\n- [x] done\n- pending\n\n```rust\nlet value = 1;\n```\n\n| A | B |\n| - | - |\n| 1 | 2 |\n\n[docs]: https://example.com";
        let extensions = Arc::new(MarkdownExtensions::default());
        let mut streamed = ParsedContent::default();
        let mut streamed_source = String::new();

        for (revision, chunk) in source
            .char_indices()
            .map(|(start, ch)| &source[start..start + ch.len_utf8()])
            .enumerate()
        {
            streamed_source.push_str(chunk);
            streamed = parse_content(
                TextViewFormat::Markdown,
                streamed,
                &streamed_source,
                &UpdateOptions {
                    revision: revision + 1,
                    epoch: 1,
                    pending_text: chunk.to_string(),
                    kind: if revision == 0 {
                        UpdateKind::Replace
                    } else {
                        UpdateKind::Append
                    },
                    markdown_extensions: extensions.clone(),
                    seed_content: None,
                },
            )
            .unwrap();
        }

        let canonical = parse_content(
            TextViewFormat::Markdown,
            ParsedContent::default(),
            source,
            &UpdateOptions {
                revision: source.chars().count(),
                epoch: 1,
                pending_text: source.to_string(),
                kind: UpdateKind::Canonical,
                markdown_extensions: extensions,
                seed_content: None,
            },
        )
        .unwrap();

        assert_eq!(streamed.document, canonical.document);
    }

    #[gpui::test]
    fn streaming_settles_to_a_canonical_revision_after_idle(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("", cx)));

        state.update(cx, |state, cx| state.push_str("# Title", cx));
        cx.run_until_parked();
        let append_revision = state.read_with(cx, |state, _| state.revision);

        cx.background_executor
            .advance_clock(StreamingMarkdownConfig::default().idle_settle_delay);
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            assert!(state.revision > append_revision);
            assert_eq!(state.rendered_revision, state.revision);
            assert_eq!(state.source().as_ref(), "# Title");
        });
    }

    #[gpui::test]
    fn finish_streaming_requests_canonical_parse_without_idle_delay(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("", cx)));

        state.update(cx, |state, cx| {
            state.push_str("# Title", cx);
            let append_revision = state.revision;
            state.finish_streaming(cx);
            assert!(state.revision > append_revision);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            assert_eq!(state.rendered_revision, state.revision);
            assert_eq!(state.source().as_ref(), "# Title");
        });
    }

    #[gpui::test]
    fn explicit_streaming_batches_provider_deltas_and_settles_once(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("", cx)));

        state.update(cx, |state, cx| {
            state.begin_streaming(cx);
            state.push_str("# ", cx);
            state.push_str("Title", cx);
        });
        cx.run_until_parked();
        assert_eq!(state.read_with(cx, |state, _| state.source()), "");

        cx.background_executor
            .advance_clock(StreamingMarkdownConfig::default().parse_interval);
        cx.run_until_parked();
        let append_revision = state.read_with(cx, |state, _| {
            assert_eq!(state.source().as_ref(), "# Title");
            state.revision
        });

        cx.background_executor
            .advance_clock(StreamingMarkdownConfig::default().idle_settle_delay);
        cx.run_until_parked();
        assert_eq!(
            state.read_with(cx, |state, _| state.revision),
            append_revision,
            "an active explicit session must not settle on idle"
        );

        state.update(cx, |state, cx| {
            state.finish_streaming(cx);
            let settled_revision = state.revision;
            state.finish_streaming(cx);
            assert_eq!(state.revision, settled_revision);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            assert_eq!(state.source().as_ref(), "# Title");
            assert_eq!(state.rendered_revision, state.revision);
        });
    }

    #[gpui::test]
    fn set_text_terminates_an_explicit_streaming_session(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("", cx)));

        state.update(cx, |state, cx| {
            state.begin_streaming(cx);
            state.push_str("discarded pending batch", cx);
            state.set_text("replacement", cx);
            state.push_str(" tail", cx);
            assert!(state.streaming_config.is_none());
            assert!(state.streaming_pending.is_empty());
        });
        cx.run_until_parked();

        assert_eq!(
            state.read_with(cx, |state, _| state.source()),
            "replacement tail"
        );
    }

    #[test]
    fn update_options_merge_keeps_latest_full_text() {
        let mut options = UpdateOptions {
            revision: 1,
            epoch: 1,
            pending_text: "old".to_string(),
            kind: UpdateKind::Append,
            markdown_extensions: Arc::default(),
            seed_content: None,
        };

        options.merge(UpdateOptions {
            revision: 2,
            epoch: 2,
            pending_text: "new".to_string(),
            kind: UpdateKind::Replace,
            markdown_extensions: Arc::default(),
            seed_content: None,
        });
        options.merge(UpdateOptions {
            revision: 3,
            epoch: 2,
            pending_text: " text".to_string(),
            kind: UpdateKind::Append,
            markdown_extensions: Arc::default(),
            seed_content: None,
        });

        assert_eq!(options.revision, 3);
        assert_eq!(options.pending_text, "new text");
        assert_eq!(options.kind, UpdateKind::Replace);
    }

    #[test]
    fn update_future_yields_before_coalescing_all_queued_updates() {
        let (tx, rx) = unbounded::<UpdateOptions>();
        let (tx_result, rx_result) = unbounded::<ParsedUpdate>();
        let total_updates = 128;

        for revision in 1..=total_updates {
            tx.try_send(UpdateOptions {
                revision,
                epoch: 1,
                pending_text: format!("{revision}\n"),
                kind: if revision == 1 {
                    UpdateKind::Replace
                } else {
                    UpdateKind::Append
                },
                markdown_extensions: Arc::default(),
                seed_content: None,
            })
            .unwrap();
        }

        let mut future = Box::pin(UpdateFuture::new(TextViewFormat::Markdown, rx, tx_result));
        let waker = futures::task::noop_waker();
        let mut task_cx = std::task::Context::from_waker(&waker);

        assert!(matches!(
            std::future::Future::poll(future.as_mut(), &mut task_cx),
            Poll::Pending
        ));
        let parsed_update = rx_result.try_recv().expect("parse result");

        assert!(
            parsed_update.revision < total_updates,
            "single poll coalesced every queued update through revision {}",
            parsed_update.revision
        );

        assert!(matches!(
            std::future::Future::poll(future.as_mut(), &mut task_cx),
            Poll::Pending
        ));
        let parsed_update = rx_result.try_recv().expect("next parse result");
        assert_eq!(parsed_update.revision, total_updates);
    }

    #[gpui::test]
    fn select_all_returns_rendered_text(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("**quick** value", cx)));
        cx.run_until_parked();

        state.update(cx, |state, cx| {
            state.select_all(cx);
        });

        state.read_with(cx, |state, _| {
            assert!(state.has_view_selection());
            assert_eq!(state.selected_text().trim(), "quick value");
        });

        state.update(cx, |state, cx| {
            state.clear_selection(cx);
        });

        state.read_with(cx, |state, _| {
            assert!(!state.has_view_selection());
            assert_eq!(state.selected_text(), "");
        });
    }

    #[gpui::test]
    fn set_markdown_extensions_reparses_existing_text(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let state = cx.update(|cx| cx.new(|cx| TextViewState::markdown("$TSLA.US", cx)));
        cx.run_until_parked();

        let extensions = MarkdownExtensions::default().block_parser(|event, cx| {
            let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Paragraph) = event.event() else {
                return None;
            };
            let source = cx.event_source(event)?;
            let symbol = source.strip_prefix('$')?.to_string();
            let node_text = format!("${symbol}");

            Some(
                MarkdownNode::new("ticker", symbol)
                    .text(node_text)
                    .markdown(source),
            )
        });

        state.update(cx, |state, cx| {
            state.set_markdown_extensions(Arc::new(extensions), cx);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            let node::BlockNode::Custom(node) = &*state.parsed_content.document.blocks[0] else {
                panic!("expected custom markdown node");
            };
            assert_eq!(node.name(), "ticker");
            assert_eq!(node.data::<String>().map(String::as_str), Some("TSLA.US"));
        });
    }
}
