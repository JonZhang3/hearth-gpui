use futures::Stream as _;
use std::{pin::Pin, sync::Arc, task::Poll, time::Duration};

use gpui::{
    App, AppContext as _, Bounds, Context, FocusHandle, IntoElement, KeyBinding, ListState,
    ParentElement as _, Pixels, Point, Render, SharedString, Styled as _, Task, Window,
    prelude::FluentBuilder as _, px,
};

use crate::{
    ElementExt,
    async_util::{Receiver, Sender, unbounded},
    input::{self, SelectAll},
    scroll::AutoScroll,
    text::{
        CodeBlockActionsFn, MarkdownBlockRenderers, MarkdownExtensions, MarkdownStyle,
        TextViewStyle,
        document::ParsedDocument,
        format,
        node::{self, NodeContext},
    },
    v_flex,
};

const CONTEXT: &'static str = "TextView";
// Keep coalescing bounded so sustained streams still render intermediate updates.
const MAX_COALESCED_UPDATES_PER_PARSE: usize = 64;
const STREAMING_SETTLE_DELAY: Duration = Duration::from_millis(100);

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
    pub(super) list_state: ListState,

    /// The bounds of the text view
    bounds: Bounds<Pixels>,

    pub(super) selectable: bool,
    pub(super) scrollable: bool,
    pub(super) text_view_style: TextViewStyle,
    pub(super) code_block_actions: Option<std::sync::Arc<CodeBlockActionsFn>>,
    pub(super) markdown_extensions: Arc<MarkdownExtensions>,
    pub(super) markdown_style: Arc<MarkdownStyle>,
    pub(super) markdown_block_renderers: Arc<MarkdownBlockRenderers>,

    pub(super) is_selecting: bool,
    multi_click_selection: Option<TextViewMultiClickSelection>,
    selected_text_override: Option<String>,
    select_all: bool,
    pub(super) auto_scroll: AutoScroll,

    pub(super) parsed_content: ParsedContent,
    /// Content format (markdown / html), used to parse synchronously on the
    /// main thread for full-replace updates.
    format: TextViewFormat,
    text: String,
    revision: usize,
    epoch: usize,
    rendered_revision: usize,
    parsed_error: Option<SharedString>,
    tx: Sender<UpdateOptions>,
    _parse_task: Task<()>,
    _receive_task: Task<()>,
    _settle_task: Task<()>,
}

impl TextViewState {
    /// Create a Markdown TextViewState.
    pub fn markdown(text: &str, cx: &mut Context<Self>) -> Self {
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
            scrollable: false,
            // Measure all blocks (not just visible ones) so the scrollbar
            // thumb size stays stable. Without this, off-screen blocks count
            // as zero height until scrolled into view, which makes the
            // scrollbar jitter as more blocks get measured during scrolling.
            list_state: ListState::new(0, gpui::ListAlignment::Top, px(1000.)).measure_all(),
            text_view_style: TextViewStyle::default(),
            code_block_actions: None,
            markdown_extensions: Arc::default(),
            markdown_style: Arc::default(),
            markdown_block_renderers: Arc::default(),
            is_selecting: false,
            auto_scroll: AutoScroll::default(),
            parsed_content: Default::default(),
            format,
            parsed_error: None,
            text: text.to_string(),
            revision: 0,
            epoch: 0,
            rendered_revision: 0,
            tx,
            _parse_task,
            _receive_task,
            _settle_task: Task::ready(()),
        };
        this.increment_update(&text, UpdateKind::Replace, cx);
        this
    }

    /// Get the text content.
    pub(crate) fn source(&self) -> SharedString {
        self.parsed_content.document.source.clone()
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

    /// Set whether the text is selectable, default false.
    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    /// Set whether the text is selectable, default false.
    pub fn set_scrollable(&mut self, scrollable: bool, cx: &mut Context<Self>) {
        if !scrollable {
            self.reset_selection();
        }
        self.scrollable = scrollable;
        cx.notify();
    }

    /// Set the text content.
    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.text.as_str() == text {
            return;
        }

        self.text.clear();
        self.text.push_str(text);
        self.parsed_error = None;
        self.increment_update(text, UpdateKind::Replace, cx);
    }

    /// Append partial text content to the existing text.
    pub fn push_str(&mut self, new_text: &str, cx: &mut Context<Self>) {
        if new_text.is_empty() {
            return;
        }
        self.text.push_str(new_text);
        self.increment_update(new_text, UpdateKind::Append, cx);
        self.schedule_streaming_settle(cx);
    }

    /// Request an immediate canonical parse of all accumulated streaming text.
    ///
    /// Streaming also settles automatically after a short idle period. Calling
    /// this method at an LLM stream's completion removes that delay. Appending
    /// more text afterwards starts a new revision and safely supersedes the
    /// pending canonical result.
    pub fn finish_streaming(&mut self, cx: &mut Context<Self>) {
        self._settle_task = Task::ready(());
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

    fn increment_update(&mut self, text: &str, kind: UpdateKind, cx: &mut Context<Self>) {
        if kind == UpdateKind::Replace {
            self.epoch = self.epoch.wrapping_add(1);
        }
        self.revision += 1;
        let update_options = UpdateOptions {
            revision: self.revision,
            epoch: self.epoch,
            kind,
            pending_text: text.to_string(),
            markdown_extensions: self.markdown_extensions.clone(),
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
            _ = self.tx.try_send(update_options);
            return;
        }

        _ = self.tx.try_send(update_options);
    }

    fn schedule_streaming_settle(&mut self, cx: &mut Context<Self>) {
        let revision = self.revision;
        self._settle_task = cx.spawn(async move |weak_self, cx| {
            cx.background_executor().timer(STREAMING_SETTLE_DELAY).await;
            _ = weak_self.update(cx, |state, cx| {
                if state.revision != revision {
                    return;
                }
                state.queue_canonical_parse();
                cx.notify();
            });
        });
    }

    fn queue_canonical_parse(&mut self) {
        self.revision += 1;
        _ = self.tx.try_send(UpdateOptions {
            revision: self.revision,
            epoch: self.epoch,
            kind: UpdateKind::Canonical,
            pending_text: self.text.clone(),
            markdown_extensions: self.markdown_extensions.clone(),
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

    pub(super) fn scroll_offset(&self) -> Point<Pixels> {
        if self.scrollable {
            self.list_state.scroll_px_offset_for_scrollbar()
        } else {
            Point::default()
        }
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
        let scroll_offset = self.scroll_offset();
        let pos = pos - self.bounds.origin - scroll_offset;
        self.multi_click_selection = Some(TextViewMultiClickSelection { pos, kind });
        self.selected_text_override = Some(selected_text);
        self.select_all = false;
        self.is_selecting = false;
        self.auto_scroll.stop();
    }

    pub(super) fn set_auto_scroll(&mut self, delta: Option<Pixels>, cx: &mut Context<Self>) {
        self.auto_scroll.set(delta, cx, |delta, state, cx| {
            state.list_state.scroll_by(delta);
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
        let scroll_offset = self.scroll_offset();
        self.multi_click_selection.map(|selection| {
            let pos = selection.pos + scroll_offset + self.bounds.origin;
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
        node_cx.source = document.source.clone();
        node_cx.style = self.text_view_style.clone();

        v_flex()
            .size_full()
            .map(|this| match &mut self.parsed_error {
                None => this.child(document.render_root(
                    if self.scrollable {
                        Some(self.list_state.clone())
                    } else {
                        None
                    },
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
                    let hit_coalesce_budget =
                        merge_pending_options(&mut options, self.rx.as_ref().get_ref());

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
        let result = parse_content(self.format, self.content.clone(), &self.source, options);
        if let Ok(content) = &result {
            self.content = content.clone();
        }
        result
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
            UpdateKind::Replace | UpdateKind::Canonical => {
                self.source.clone_from(&options.pending_text)
            }
        }
    }
}

#[derive(Clone)]
struct UpdateOptions {
    revision: usize,
    epoch: usize,
    pending_text: String,
    kind: UpdateKind,
    markdown_extensions: Arc<MarkdownExtensions>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UpdateKind {
    Replace,
    Append,
    Canonical,
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
                options.merge(next_options);
                update_count += 1;
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
        ..NodeContext::default()
    };

    let append = options.kind == UpdateKind::Append;
    // Reference definitions affect parsing outside the unstable tail. Once a
    // document contains one, suffix-only parsing is not semantically safe.
    let requires_full_parse = !content.node_cx.link_refs.is_empty();
    let mut source = if append && requires_full_parse {
        complete_source.to_string()
    } else {
        String::new()
    };

    if append
        && !requires_full_parse
        && let Some(last_block) = content.document.blocks.pop()
        && let Some(span) = last_block.span()
    {
        node_cx.offset = span.start;
        source.push_str(&complete_source[span.start..]);
    } else if source.is_empty() {
        source = complete_source.to_string();
    }

    let mut new_document = match format {
        TextViewFormat::Markdown => format::markdown::parse(&source, &mut node_cx),
        TextViewFormat::Html => format::html::parse(&source, &mut node_cx),
    }?;

    // A newly appended definition can retroactively turn earlier plain text
    // into reference links, so reparse the canonical source before publishing.
    if append && !requires_full_parse && !node_cx.link_refs.is_empty() {
        node_cx = NodeContext {
            markdown_extensions: options.markdown_extensions.clone(),
            ..NodeContext::default()
        };
        new_document = match format {
            TextViewFormat::Markdown => format::markdown::parse(complete_source, &mut node_cx),
            TextViewFormat::Html => format::html::parse(complete_source, &mut node_cx),
        }?;
        reuse_unchanged_prefix(&content.document, &mut new_document);
        content.document = new_document;
    } else if append && !requires_full_parse {
        content.document.source = complete_source.to_string().into();
        content.document.blocks.extend(new_document.blocks);
    } else {
        reuse_unchanged_prefix(&content.document, &mut new_document);
        content.document = new_document;
    }
    content.node_cx = node_cx;

    Ok(content)
}

fn reuse_unchanged_prefix(previous: &ParsedDocument, next: &mut ParsedDocument) {
    for (old, new) in previous.blocks.iter().zip(next.blocks.iter_mut()) {
        if old != new {
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

    #[test]
    fn appended_reference_definitions_match_a_canonical_parse() {
        let extensions = Arc::new(MarkdownExtensions::default());
        let initial_options = UpdateOptions {
            revision: 1,
            epoch: 1,
            pending_text: "Read [the docs].".to_string(),
            kind: UpdateKind::Replace,
            markdown_extensions: extensions.clone(),
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
            },
        )
        .unwrap();

        assert_eq!(streamed.document, canonical.document);
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
            },
        )
        .unwrap();

        assert_eq!(streamed.document, canonical.document);
    }

    #[test]
    fn failed_mdx_chunk_is_retained_for_the_next_append() {
        let (_tx, rx) = unbounded::<UpdateOptions>();
        let (tx_result, _rx_result) = unbounded::<ParsedUpdate>();
        let extensions = Arc::new(MarkdownExtensions::default().mdx());
        let mut future = UpdateFuture::new(TextViewFormat::Markdown, rx, tx_result);

        let incomplete = UpdateOptions {
            revision: 1,
            epoch: 1,
            pending_text: "{value".to_string(),
            kind: UpdateKind::Append,
            markdown_extensions: extensions.clone(),
        };
        assert!(future.parse_update(&incomplete).is_err());
        assert_eq!(future.source, "{value");
        assert!(future.content.document.source.is_empty());

        let completed = future
            .parse_update(&UpdateOptions {
                revision: 2,
                epoch: 1,
                pending_text: "}".to_string(),
                kind: UpdateKind::Append,
                markdown_extensions: extensions,
            })
            .expect("completed MDX expression should parse without waiting for settle");

        assert_eq!(future.source, "{value}");
        assert_eq!(completed.document.source.as_ref(), "{value}");
    }

    #[test]
    fn failed_replace_resets_the_previous_epoch_parser_baseline() {
        let (_tx, rx) = unbounded::<UpdateOptions>();
        let (tx_result, _rx_result) = unbounded::<ParsedUpdate>();
        let extensions = Arc::new(MarkdownExtensions::default().mdx());
        let mut future = UpdateFuture::new(TextViewFormat::Markdown, rx, tx_result);

        future
            .parse_update(&UpdateOptions {
                revision: 1,
                epoch: 1,
                pending_text: "previous document".to_string(),
                kind: UpdateKind::Replace,
                markdown_extensions: extensions.clone(),
            })
            .expect("initial document");
        assert_eq!(future.content.document.source.as_ref(), "previous document");

        assert!(
            future
                .parse_update(&UpdateOptions {
                    revision: 2,
                    epoch: 2,
                    pending_text: "{next".to_string(),
                    kind: UpdateKind::Replace,
                    markdown_extensions: extensions.clone(),
                })
                .is_err()
        );
        assert!(future.content.document.source.is_empty());

        let completed = future
            .parse_update(&UpdateOptions {
                revision: 3,
                epoch: 2,
                pending_text: "}".to_string(),
                kind: UpdateKind::Append,
                markdown_extensions: extensions,
            })
            .expect("new epoch should recover without the previous source");
        assert_eq!(completed.document.source.as_ref(), "{next}");
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

        cx.background_executor.advance_clock(STREAMING_SETTLE_DELAY);
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

    #[test]
    fn update_options_merge_keeps_latest_full_text() {
        let mut options = UpdateOptions {
            revision: 1,
            epoch: 1,
            pending_text: "old".to_string(),
            kind: UpdateKind::Append,
            markdown_extensions: Arc::default(),
        };

        options.merge(UpdateOptions {
            revision: 2,
            epoch: 2,
            pending_text: "new".to_string(),
            kind: UpdateKind::Replace,
            markdown_extensions: Arc::default(),
        });
        options.merge(UpdateOptions {
            revision: 3,
            epoch: 2,
            pending_text: " text".to_string(),
            kind: UpdateKind::Append,
            markdown_extensions: Arc::default(),
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

        let extensions = MarkdownExtensions::default().block_parser(|node, cx| {
            let markdown::mdast::Node::Paragraph(paragraph) = node else {
                return None;
            };
            let [markdown::mdast::Node::Text(text)] = paragraph.children.as_slice() else {
                return None;
            };
            let symbol = text.value.strip_prefix('$')?.to_string();
            let node_text = format!("${symbol}");

            Some(
                MarkdownNode::new("ticker", symbol)
                    .text(node_text)
                    .markdown(cx.node_source(node).unwrap_or_default()),
            )
        });

        state.update(cx, |state, cx| {
            state.set_markdown_extensions(Arc::new(extensions), cx);
        });
        cx.run_until_parked();

        state.read_with(cx, |state, _| {
            let node::BlockNode::Custom(node) = &state.parsed_content.document.blocks[0] else {
                panic!("expected custom markdown node");
            };
            assert_eq!(node.name(), "ticker");
            assert_eq!(node.data::<String>().map(String::as_str), Some("TSLA.US"));
        });
    }
}
