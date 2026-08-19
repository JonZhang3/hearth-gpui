use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    rc::Rc,
    sync::Arc,
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use gpui::{
    AbsoluteLength, AnyElement, App, AppContext as _, Bounds, ClipboardItem, Context,
    DefiniteLength, DispatchPhase, Edges, Element, ElementId, Entity, FocusHandle, GlobalElementId,
    HighlightStyle, Hitbox, HitboxBehavior, Hsla, ImageSource, InspectorElementId,
    InteractiveElement as _, IntoElement, KeyContext, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement as _, Pixels, Refineable as _, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, StrikethroughStyle, StyleRefinement, Styled, StyledText, Task,
    TextAlign, TextLayout, TextRun, TextStyle, TextStyleRefinement, UnderlineStyle, WhiteSpace,
    Window, actions, div, fill, img, point, prelude::FluentBuilder as _, px, rems,
};
use linkify::{LinkFinder, LinkKind};
use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};

use crate::{
    ActiveTheme as _, Sizable as _,
    button::Button,
    clipboard::Clipboard,
    h_flex,
    highlighter::HighlightTheme,
    scroll::ScrollableElement as _,
    table::{TableGrid, TableGridCell, TableGridRow, TableGridSizing},
};

use super::{
    inline::InlineState,
    inline_flow::{
        InlineBoxStyle, InlineFlow, InlineFlowItem, InlineFlowLayoutCache, InlineFlowLayoutState,
        InlineImageSizing,
    },
};

/// Optional parser and renderer behavior. Expensive extensions remain opt-in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarkdownOptions {
    pub parse_links_only: bool,
    pub parse_html: bool,
    pub render_mermaid_diagrams: bool,
    pub parse_heading_slugs: bool,
    pub render_metadata_blocks: bool,
}

/// Typography family used by the built-in Markdown style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkdownFont {
    #[default]
    Agent,
    Editor,
    Preview,
}

/// Optional per-heading refinements layered over the shared heading style.
#[derive(Clone, Default)]
pub struct HeadingLevelStyles {
    pub h1: Option<TextStyleRefinement>,
    pub h2: Option<TextStyleRefinement>,
    pub h3: Option<TextStyleRefinement>,
    pub h4: Option<TextStyleRefinement>,
    pub h5: Option<TextStyleRefinement>,
    pub h6: Option<TextStyleRefinement>,
}

/// Semantic colors for GitHub-style block quote callouts.
#[derive(Clone, Copy, Default)]
pub struct BlockQuoteKindColors {
    pub note: Hsla,
    pub tip: Hsla,
    pub important: Hsla,
    pub warning: Hsla,
    pub caution: Hsla,
}

impl BlockQuoteKindColors {
    fn color(self, kind: Option<BlockQuoteKind>, fallback: Hsla) -> Hsla {
        match kind {
            Some(BlockQuoteKind::Note) => self.note,
            Some(BlockQuoteKind::Tip) => self.tip,
            Some(BlockQuoteKind::Important) => self.important,
            Some(BlockQuoteKind::Warning) => self.warning,
            Some(BlockQuoteKind::Caution) => self.caution,
            None => fallback,
        }
    }
}

/// Complete visual configuration consumed by [`MarkdownElement`].
#[derive(Clone)]
pub struct MarkdownStyle {
    pub base_text_style: TextStyle,
    pub container_style: StyleRefinement,
    pub code_block: StyleRefinement,
    pub code_block_overflow_x_scroll: bool,
    pub inline_code: TextStyleRefinement,
    pub block_quote: TextStyleRefinement,
    pub link: TextStyleRefinement,
    pub rule_color: Hsla,
    pub block_quote_border_color: Hsla,
    pub block_quote_kind_colors: BlockQuoteKindColors,
    pub syntax: Arc<HighlightTheme>,
    pub selection_background_color: Hsla,
    pub heading: StyleRefinement,
    pub heading_level_styles: Option<HeadingLevelStyles>,
    pub heading_border_color: Option<Hsla>,
    pub prevent_mouse_interaction: bool,
    pub table_columns_min_size: bool,
    pub soft_break_as_hard_break: bool,
}

impl MarkdownStyle {
    /// Resolves a Zed-compatible style from Hearth semantic theme tokens.
    pub fn themed(font: MarkdownFont, window: &Window, cx: &App) -> Self {
        let theme = cx.theme();
        let mut base = window.text_style();
        let body_size = match font {
            MarkdownFont::Preview => theme.font_size * 0.92,
            MarkdownFont::Agent | MarkdownFont::Editor => theme.font_size,
        };
        base.font_family = theme.font_family.clone();
        base.font_size = body_size.into();
        base.line_height = (body_size * 1.75).into();
        base.color = if matches!(font, MarkdownFont::Preview) {
            theme.muted_foreground
        } else {
            theme.foreground
        };

        let heading_sizes = match font {
            MarkdownFont::Agent => Some([1.15, 1.10, 1.05, 1.0, 0.95, 0.875]),
            MarkdownFont::Editor => None,
            MarkdownFont::Preview => Some([1.45, 1.30, 1.10, 1.01, 0.95, 0.85]),
        };
        let heading_style = |scale: f32| TextStyleRefinement {
            font_size: Some(rems(scale).into()),
            ..Default::default()
        };

        Self {
            base_text_style: base,
            container_style: StyleRefinement::default(),
            code_block: StyleRefinement::default().bg(theme.muted.opacity(0.5)),
            code_block_overflow_x_scroll: true,
            inline_code: TextStyleRefinement {
                font_family: Some(theme.mono_font_family.clone()),
                font_size: Some(theme.mono_font_size.into()),
                background_color: Some(theme.foreground.opacity(0.08)),
                ..Default::default()
            },
            block_quote: TextStyleRefinement {
                color: Some(theme.muted_foreground),
                ..Default::default()
            },
            link: TextStyleRefinement {
                color: Some(theme.link),
                background_color: Some(theme.foreground.opacity(0.025)),
                underline: Some(UnderlineStyle {
                    color: Some(theme.link.opacity(0.5)),
                    thickness: px(1.),
                    ..Default::default()
                }),
                ..Default::default()
            },
            rule_color: theme.border,
            block_quote_border_color: theme.border,
            block_quote_kind_colors: BlockQuoteKindColors {
                note: theme.info,
                tip: theme.success,
                important: theme.info,
                warning: theme.warning,
                caution: theme.danger,
            },
            syntax: theme.highlight_theme.clone(),
            selection_background_color: theme.selection,
            heading: StyleRefinement::default(),
            heading_level_styles: heading_sizes.map(|sizes| HeadingLevelStyles {
                h1: Some(heading_style(sizes[0])),
                h2: Some(heading_style(sizes[1])),
                h3: Some(heading_style(sizes[2])),
                h4: Some(heading_style(sizes[3])),
                h5: Some(heading_style(sizes[4])),
                h6: Some(heading_style(sizes[5])),
            }),
            heading_border_color: matches!(font, MarkdownFont::Preview).then_some(theme.border),
            prevent_mouse_interaction: false,
            table_columns_min_size: false,
            soft_break_as_hard_break: matches!(font, MarkdownFont::Agent),
        }
    }
}

impl Default for MarkdownStyle {
    fn default() -> Self {
        Self {
            base_text_style: TextStyle::default(),
            container_style: StyleRefinement::default(),
            code_block: StyleRefinement::default(),
            code_block_overflow_x_scroll: false,
            inline_code: TextStyleRefinement::default(),
            block_quote: TextStyleRefinement::default(),
            link: TextStyleRefinement::default(),
            rule_color: Hsla::default(),
            block_quote_border_color: Hsla::default(),
            block_quote_kind_colors: BlockQuoteKindColors::default(),
            syntax: HighlightTheme::default_light(),
            selection_background_color: Hsla::default(),
            heading: StyleRefinement::default(),
            heading_level_styles: None,
            heading_border_color: None,
            prevent_mouse_interaction: false,
            table_columns_min_size: false,
            soft_break_as_hard_break: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyButtonVisibility {
    Hidden,
    AlwaysVisible,
    VisibleOnHover,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapButtonVisibility {
    Hidden,
    AlwaysVisible,
    VisibleOnHover,
}

/// Code block rendering strategy. The default keeps the optimized built-in path.
#[derive(Clone)]
pub enum CodeBlockRenderer {
    Default {
        copy_button_visibility: CopyButtonVisibility,
        wrap_button_visibility: WrapButtonVisibility,
        border: bool,
    },
    Custom(Arc<dyn Fn(CodeBlockRenderContext, &mut Window, &mut App) -> AnyElement>),
}

impl Default for CodeBlockRenderer {
    fn default() -> Self {
        Self::Default {
            copy_button_visibility: CopyButtonVisibility::VisibleOnHover,
            wrap_button_visibility: WrapButtonVisibility::Hidden,
            border: false,
        }
    }
}

/// Source-mapped data passed to a custom code block renderer.
#[derive(Clone)]
pub struct CodeBlockRenderContext {
    pub code: SharedString,
    pub language: Option<SharedString>,
    pub info: Option<SharedString>,
    pub source_path: Option<SharedString>,
    pub source_range: Range<usize>,
    pub wrapped: bool,
}

actions!(markdown, [CopyAsMarkdown]);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SelectionMode {
    #[default]
    Character,
    Word,
    Line,
    All,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Selection {
    anchor: usize,
    head: usize,
    pending: bool,
    mode: SelectionMode,
}

impl Selection {
    fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct InlineFlags {
    strong: bool,
    emphasis: bool,
    strike: bool,
    underline: bool,
    code: bool,
    link: bool,
    soft_break: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct TextSegment {
    text: SharedString,
    source: Range<usize>,
    flags: InlineFlags,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedLink {
    source: Range<usize>,
    destination: SharedString,
}

#[derive(Clone, Debug)]
struct SegmentMapping {
    rendered: Range<usize>,
    source: Range<usize>,
}

#[derive(Clone)]
struct RenderedLink {
    source: Range<usize>,
    destination: SharedString,
}

#[derive(Clone, Debug, PartialEq)]
struct TextPreparationKey {
    block_style: TextStyle,
    inline_code: TextStyleRefinement,
    link: TextStyleRefinement,
    selection_background_color: Hsla,
    soft_break_as_hard_break: bool,
    search: Vec<Range<usize>>,
    active_search: Option<usize>,
}

#[derive(Clone)]
struct CachedTextPreparation {
    key: TextPreparationKey,
    text: SharedString,
    mappings: Arc<[SegmentMapping]>,
    runs: Vec<TextRun>,
    links: Arc<[RenderedLink]>,
}

#[derive(Clone, Default)]
struct TextPreparationCache(Arc<std::sync::Mutex<Option<CachedTextPreparation>>>);

impl std::fmt::Debug for TextPreparationCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("TextPreparationCache").finish()
    }
}

#[derive(Clone, Default)]
struct IntrinsicTextWidthCache(Arc<std::sync::Mutex<Option<(TextPreparationKey, Pixels)>>>);

impl std::fmt::Debug for IntrinsicTextWidthCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("IntrinsicTextWidthCache").finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum TextBlockKind {
    Paragraph,
    Heading(HeadingLevel),
    BlockQuote(Option<BlockQuoteKind>),
    ListItem { depth: usize },
    HtmlTableCell { header: bool, alignment: TextAlign },
    Footnote,
    Html,
    Metadata,
}

#[derive(Clone, Debug)]
struct ParsedTextBlock {
    kind: TextBlockKind,
    source: Range<usize>,
    segments: Vec<TextSegment>,
    links: Vec<ParsedLink>,
    images: Vec<ParsedInlineImage>,
    render_cache: TextPreparationCache,
    intrinsic_width_cache: IntrinsicTextWidthCache,
    inline_flow_layout_cache: InlineFlowLayoutCache,
}

impl PartialEq for ParsedTextBlock {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.source == other.source
            && self.segments == other.segments
            && self.links == other.links
            && self.images == other.images
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedCodeBlock {
    source: Range<usize>,
    code: SharedString,
    display_code: SharedString,
    language: Option<SharedString>,
    info: Option<SharedString>,
    source_path: Option<SharedString>,
    mermaid_data_uri: Option<SharedString>,
    highlight: super::node::CodeBlock,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedInlineImage {
    segment_index: usize,
    source: Range<usize>,
    destination: SharedString,
    title: SharedString,
    alt: SharedString,
    width: Option<DefiniteLength>,
    height: Option<DefiniteLength>,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedHtmlBlock {
    source: Range<usize>,
    raw_source: SharedString,
    blocks: Arc<[ParsedBlock]>,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedHtmlTableCell {
    content: ParsedTextBlock,
    col_span: usize,
    row_span: usize,
    is_header: bool,
    alignment: TextAlign,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ParsedHtmlTableRow {
    cells: Vec<ParsedHtmlTableCell>,
}

/// Identifies the source syntax so Markdown and HTML tables retain Zed's
/// intentionally different cell densities while sharing one layout path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParsedTableKind {
    Markdown,
    Html,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedHtmlTable {
    source: Range<usize>,
    rows: Arc<[ParsedHtmlTableRow]>,
    kind: ParsedTableKind,
}

#[derive(Clone, Debug, PartialEq)]
enum ParsedBlock {
    Text(ParsedTextBlock),
    Code(ParsedCodeBlock),
    Html(ParsedHtmlBlock),
    HtmlTable(ParsedHtmlTable),
    Rule(Range<usize>),
}

#[derive(Clone, Debug, Default)]
struct ParsedMarkdown {
    source: SharedString,
    blocks: Arc<[ParsedBlock]>,
    root_block_starts: Arc<[usize]>,
    headings: Arc<HashMap<SharedString, usize>>,
    footnotes: Arc<HashMap<SharedString, usize>>,
}

/// Stateful Markdown document. All mutable behavior is local to this entity.
pub struct Markdown {
    source: String,
    options: MarkdownOptions,
    parsed: Arc<ParsedMarkdown>,
    revision: u64,
    pending_parse: Option<Task<()>>,
    should_reparse: bool,
    selection: Selection,
    focus_handle: FocusHandle,
    search_highlights: Vec<Range<usize>>,
    active_search_highlight: Option<usize>,
    autoscroll_request: Option<usize>,
    pressed_link: Option<SharedString>,
    wrapped_code_blocks: HashSet<usize>,
    code_block_scroll_handles: HashMap<usize, ScrollHandle>,
    table_scroll_handles: HashMap<usize, ScrollHandle>,
}

impl Markdown {
    /// Creates a document and schedules its first full-source parse off the UI thread.
    pub fn new(source: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self::new_with_options(source, MarkdownOptions::default(), cx)
    }

    /// Creates a document with explicit parser options.
    pub fn new_with_options(
        source: impl Into<SharedString>,
        options: MarkdownOptions,
        cx: &mut Context<Self>,
    ) -> Self {
        let source: SharedString = source.into();
        let mut this = Self {
            source: source.to_string(),
            options,
            parsed: Arc::new(ParsedMarkdown {
                source,
                ..Default::default()
            }),
            revision: 0,
            pending_parse: None,
            should_reparse: false,
            selection: Selection::default(),
            focus_handle: cx.focus_handle(),
            search_highlights: Vec::new(),
            active_search_highlight: None,
            autoscroll_request: None,
            pressed_link: None,
            wrapped_code_blocks: HashSet::new(),
            code_block_scroll_handles: HashMap::new(),
            table_scroll_handles: HashMap::new(),
        };
        this.schedule_parse(cx);
        this
    }

    /// Returns the canonical source, including deltas not yet published by the parser.
    pub fn source(&self) -> SharedString {
        self.source.clone().into()
    }

    /// Replaces the canonical source and rejects every older parse result.
    pub fn replace(&mut self, source: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.source = source.into().to_string();
        self.revision = self.revision.wrapping_add(1);
        if self.source.is_empty() {
            self.parsed = Arc::new(ParsedMarkdown::default());
            self.selection = Selection::default();
            cx.notify();
        }
        self.schedule_parse(cx);
    }

    /// Appends one provider delta and coalesces it with any parse already in flight.
    pub fn append(&mut self, chunk: &str, cx: &mut Context<Self>) {
        if chunk.is_empty() {
            return;
        }
        self.source.push_str(chunk);
        self.revision = self.revision.wrapping_add(1);
        self.schedule_parse(cx);
    }

    /// Returns whether the published document trails the canonical source.
    pub fn is_parsing(&self) -> bool {
        self.pending_parse.is_some() || self.parsed.source.as_ref() != self.source
    }

    /// Returns the persistent horizontal scroll handle for one unwrapped code block.
    fn code_block_scroll_handle(&mut self, source_start: usize) -> ScrollHandle {
        self.code_block_scroll_handles
            .entry(source_start)
            .or_default()
            .clone()
    }

    /// Drops scroll state for code blocks that are no longer horizontally scrollable.
    fn retain_code_block_scroll_handles(&mut self, active: &HashSet<usize>) {
        self.code_block_scroll_handles
            .retain(|source_start, _| active.contains(source_start));
    }

    /// Returns the persistent horizontal scroll handle for one table.
    fn table_scroll_handle(&mut self, source_start: usize) -> ScrollHandle {
        self.table_scroll_handles
            .entry(source_start)
            .or_default()
            .clone()
    }

    /// Drops scroll state for tables that are no longer present.
    fn retain_table_scroll_handles(&mut self, active: &HashSet<usize>) {
        self.table_scroll_handles
            .retain(|source_start, _| active.contains(source_start));
    }

    /// Returns canonical source offsets for every parsed root block.
    pub fn root_block_starts(&self) -> Arc<[usize]> {
        self.parsed.root_block_starts.clone()
    }

    /// Requests source-index navigation during the next prepaint.
    pub fn scroll_to_source_index(&mut self, source_index: usize, cx: &mut Context<Self>) {
        self.autoscroll_request = Some(source_index.min(self.source.len()));
        cx.notify();
    }

    /// Resolves and requests navigation to a heading slug.
    pub fn scroll_to_heading(&mut self, slug: &str, cx: &mut Context<Self>) -> bool {
        let Some(offset) = self.parsed.headings.get(slug).copied() else {
            return false;
        };
        self.scroll_to_source_index(offset, cx);
        true
    }

    /// Resolves and requests navigation to a footnote definition.
    pub fn scroll_to_footnote(&mut self, label: &str, cx: &mut Context<Self>) -> bool {
        let Some(offset) = self.parsed.footnotes.get(label).copied() else {
            return false;
        };
        self.scroll_to_source_index(offset, cx);
        true
    }

    /// Replaces search ranges. Ranges are canonical source byte offsets.
    pub fn set_search_highlights(
        &mut self,
        ranges: Vec<Range<usize>>,
        active: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        self.search_highlights = ranges;
        self.active_search_highlight = active.filter(|index| *index < self.search_highlights.len());
        cx.notify();
    }

    /// Finds literal query occurrences without changing canonical source.
    pub fn search(&mut self, query: &str, case_sensitive: bool, cx: &mut Context<Self>) -> usize {
        let mut ranges = Vec::new();
        if !query.is_empty() {
            if case_sensitive {
                let mut offset = 0;
                while let Some(found) = self.source[offset..].find(query) {
                    let start = offset + found;
                    ranges.push(start..start + query.len());
                    offset = start + query.len();
                }
            } else {
                let (folded, source_ranges) = lowercase_with_source_ranges(&self.source);
                let needle = query.to_lowercase();
                let mut offset = 0;
                while let Some(found) = folded[offset..].find(&needle) {
                    let folded_start = offset + found;
                    let folded_end = folded_start + needle.len();
                    if let (Some(start), Some(end)) = (
                        source_ranges.get(folded_start),
                        source_ranges.get(folded_end.saturating_sub(1)),
                    ) {
                        ranges.push(start.start..end.end);
                    }
                    offset = folded_end.max(offset + 1);
                }
            }
        }
        let count = ranges.len();
        self.set_search_highlights(ranges, (count > 0).then_some(0), cx);
        count
    }

    /// Returns the selected canonical Markdown range.
    pub fn selected_source(&self) -> Option<SharedString> {
        let range = self.selection.range();
        (!range.is_empty())
            .then(|| self.source.get(range).map(SharedString::from))
            .flatten()
    }

    fn schedule_parse(&mut self, cx: &mut Context<Self>) {
        if self.pending_parse.is_some() {
            self.should_reparse = true;
            return;
        }

        let source: SharedString = self.source.clone().into();
        let options = self.options;
        let revision = self.revision;
        let previous = self.parsed.clone();
        self.pending_parse = Some(cx.spawn(async move |entity, cx| {
            let mut parsed = cx
                .background_spawn(async move {
                    parse_markdown_with_previous(source, options, Some(&previous))
                })
                .await;
            let _ = entity.update(cx, |this, cx| {
                this.pending_parse = None;
                if revision == this.revision {
                    reuse_unchanged_block_prefix(&this.parsed, &mut parsed);
                    this.parsed = Arc::new(parsed);
                    this.selection.anchor = this.selection.anchor.min(this.source.len());
                    this.selection.head = this.selection.head.min(this.source.len());
                    cx.notify();
                }
                if this.should_reparse || this.parsed.source.as_ref() != this.source {
                    this.should_reparse = false;
                    this.schedule_parse(cx);
                }
            });
        }));
    }
}

/// Reuse parsed blocks whose canonical structure is unchanged so their syntax and
/// rendering state remains stable across tail-only streaming updates.
fn reuse_unchanged_block_prefix(previous: &ParsedMarkdown, next: &mut ParsedMarkdown) {
    let blocks = Arc::make_mut(&mut next.blocks);
    for (old, new) in previous.blocks.iter().zip(blocks.iter_mut()) {
        if old != new {
            break;
        }
        *new = old.clone();
    }
}

fn lowercase_with_source_ranges(source: &str) -> (String, Vec<Range<usize>>) {
    let mut folded = String::new();
    let mut ranges = Vec::new();
    for (start, character) in source.char_indices() {
        let source_range = start..start + character.len_utf8();
        for lowercase in character.to_lowercase() {
            folded.push(lowercase);
            ranges.extend(std::iter::repeat_n(
                source_range.clone(),
                lowercase.len_utf8(),
            ));
        }
    }
    (folded, ranges)
}

#[derive(Clone)]
struct ListContext {
    next: Option<u64>,
}

struct MarkdownTableBuilder {
    source_start: usize,
    alignments: Vec<Alignment>,
    rows: Vec<ParsedHtmlTableRow>,
    current_row: Vec<ParsedHtmlTableCell>,
    in_header: bool,
}

impl MarkdownTableBuilder {
    fn alignment(&self) -> TextAlign {
        match self.alignments.get(self.current_row.len()) {
            Some(Alignment::Center) => TextAlign::Center,
            Some(Alignment::Right) => TextAlign::Right,
            _ => TextAlign::Left,
        }
    }
}

#[derive(Clone)]
struct ActiveLink {
    source_start: usize,
    destination: SharedString,
}

struct TextBlockBuilder {
    kind: TextBlockKind,
    source_start: usize,
    source_end: usize,
    segments: Vec<TextSegment>,
    links: Vec<ParsedLink>,
    images: Vec<ParsedInlineImage>,
    flags: InlineFlags,
    active_links: Vec<ActiveLink>,
}

impl TextBlockBuilder {
    fn new(kind: TextBlockKind, source_start: usize) -> Self {
        Self {
            kind,
            source_start,
            source_end: source_start,
            segments: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            flags: InlineFlags::default(),
            active_links: Vec::new(),
        }
    }

    fn push(&mut self, text: impl Into<SharedString>, source: Range<usize>) {
        let text = text.into();
        if text.is_empty() {
            return;
        }
        self.source_end = self.source_end.max(source.end);
        self.segments.push(TextSegment {
            text,
            source,
            flags: self.flags,
        });
    }

    /// Inserts an atomic image placeholder while retaining canonical source mapping.
    fn push_image(
        &mut self,
        source: Range<usize>,
        destination: SharedString,
        title: SharedString,
        alt: SharedString,
        width: Option<DefiniteLength>,
        height: Option<DefiniteLength>,
    ) {
        let segment_index = self.segments.len();
        self.push("\u{fffc}", source.clone());
        self.images.push(ParsedInlineImage {
            segment_index,
            source,
            destination,
            title,
            alt,
            width,
            height,
        });
    }

    fn finish(mut self, source_end: usize) -> ParsedTextBlock {
        self.source_end = self.source_end.max(source_end);
        ParsedTextBlock {
            kind: self.kind,
            source: self.source_start..self.source_end,
            segments: self.segments,
            links: self.links,
            images: self.images,
            render_cache: TextPreparationCache::default(),
            intrinsic_width_cache: IntrinsicTextWidthCache::default(),
            inline_flow_layout_cache: InlineFlowLayoutCache::default(),
        }
    }
}

/// Applies one standalone inline HTML tag to the active Markdown text builder.
fn apply_inline_html_tag(source: &str, range: Range<usize>, block: &mut TextBlockBuilder) -> bool {
    let tag = source.trim();
    let Some(inner) = tag.strip_prefix('<').and_then(|tag| tag.strip_suffix('>')) else {
        return false;
    };
    if inner.starts_with('!') || inner.starts_with('?') {
        return true;
    }
    let closing = inner.starts_with('/');
    let name = inner
        .trim_start_matches('/')
        .split(|character: char| character.is_ascii_whitespace() || character == '/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();

    match (closing, name.as_str()) {
        (false, "strong" | "b") => block.flags.strong = true,
        (true, "strong" | "b") => block.flags.strong = false,
        (false, "em" | "i") => block.flags.emphasis = true,
        (true, "em" | "i") => block.flags.emphasis = false,
        (false, "del" | "s") => block.flags.strike = true,
        (true, "del" | "s") => block.flags.strike = false,
        (false, "u" | "ins") => block.flags.underline = true,
        (true, "u" | "ins") => block.flags.underline = false,
        (false, "code") => block.flags.code = true,
        (true, "code") => block.flags.code = false,
        (false, "a") => {
            if let Some(destination) = super::format::html::parse_inline_link(source) {
                block.flags.link = true;
                block.active_links.push(ActiveLink {
                    source_start: range.start,
                    destination,
                });
            }
        }
        (true, "a") => {
            block.flags.link = false;
            if let Some(link) = block.active_links.pop() {
                block.links.push(ParsedLink {
                    source: link.source_start..range.end,
                    destination: link.destination,
                });
            }
        }
        (_, "span" | "small" | "sub" | "sup" | "mark") => {}
        _ => return false,
    }
    true
}

/// Assigns deterministic canonical ranges to visible HTML content.
///
/// html5ever does not retain byte locations, so this follows Zed's monotonic
/// allocator contract while keeping every boundary valid for the UTF-8 source.
struct HtmlSourceAllocator<'a> {
    source: &'a str,
    range: Range<usize>,
    next: usize,
}

impl<'a> HtmlSourceAllocator<'a> {
    fn new(source: &'a str, range: Range<usize>) -> Self {
        Self {
            source,
            next: range.start,
            range,
        }
    }

    fn allocate(&mut self, requested_len: usize) -> Range<usize> {
        let start = self.next;
        let mut end = start
            .saturating_add(requested_len)
            .min(self.range.end)
            .min(self.source.len());
        while end > start && !self.source.is_char_boundary(end) {
            end -= 1;
        }
        self.next = end;
        start..end
    }
}

fn html_inline_flags(mark: &super::node::TextMark) -> InlineFlags {
    InlineFlags {
        strong: mark.bold,
        emphasis: mark.italic,
        strike: mark.strikethrough,
        underline: mark.underline,
        code: mark.code,
        link: mark.link.is_some(),
        soft_break: false,
    }
}

/// Converts one temporary HTML paragraph into the renderer's cacheable text model.
fn convert_html_paragraph(
    paragraph: &super::node::Paragraph,
    kind: TextBlockKind,
    prefix: Option<SharedString>,
    allocator: &mut HtmlSourceAllocator<'_>,
) -> Option<ParsedTextBlock> {
    let source_start = allocator.next;
    let mut builder = TextBlockBuilder::new(kind, source_start);
    if let Some(prefix) = prefix {
        builder.push(prefix, source_start..source_start);
    }

    for inline in &paragraph.children {
        if let Some(image) = &inline.image {
            let image_source = allocator.allocate(1);
            builder.push_image(
                image_source,
                image.url.to_string().into(),
                image.title.clone().unwrap_or_default(),
                image.alt.clone().unwrap_or_default(),
                image.width,
                image.height,
            );
            continue;
        }

        let text = inline.text.as_ref();
        if text.is_empty() {
            continue;
        }
        let allocated = allocator.allocate(text.len());
        let allocated_len = allocated.len();
        let mut boundaries = vec![0, text.len()];
        for (range, _) in &inline.marks {
            boundaries.push(range.start.min(text.len()));
            boundaries.push(range.end.min(text.len()));
        }
        boundaries.sort_unstable();
        boundaries.dedup();

        for boundary in boundaries.windows(2) {
            let start = boundary[0];
            let end = boundary[1];
            if start >= end || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
                continue;
            }
            let mark = inline
                .marks
                .iter()
                .filter(|(range, _)| range.start < end && range.end > start)
                .fold(super::node::TextMark::default(), |mut merged, (_, mark)| {
                    merged.merge(mark.clone());
                    merged
                });
            builder.flags = html_inline_flags(&mark);
            let source = (allocated.start + start.min(allocated_len))
                ..(allocated.start + end.min(allocated_len));
            builder.push(&text[start..end], source.clone());
            if let Some(link) = mark.link {
                builder.links.push(ParsedLink {
                    source,
                    destination: link.url,
                });
            }
        }
    }

    (!builder.segments.is_empty()).then(|| builder.finish(allocator.next))
}

fn convert_html_node(
    node: &super::node::BlockNode,
    inherited_kind: Option<TextBlockKind>,
    list_depth: usize,
    allocator: &mut HtmlSourceAllocator<'_>,
    blocks: &mut Vec<ParsedBlock>,
) {
    use super::node::BlockNode;

    match node {
        BlockNode::Root { children, .. } => {
            for child in children {
                convert_html_node(child, inherited_kind.clone(), list_depth, allocator, blocks);
            }
        }
        BlockNode::Paragraph(paragraph) => {
            if let Some(block) = convert_html_paragraph(
                paragraph,
                inherited_kind.unwrap_or(TextBlockKind::Paragraph),
                None,
                allocator,
            ) {
                blocks.push(ParsedBlock::Text(block));
            }
        }
        BlockNode::Heading {
            level, children, ..
        } => {
            let level = match level {
                1 => HeadingLevel::H1,
                2 => HeadingLevel::H2,
                3 => HeadingLevel::H3,
                4 => HeadingLevel::H4,
                5 => HeadingLevel::H5,
                _ => HeadingLevel::H6,
            };
            if let Some(block) =
                convert_html_paragraph(children, TextBlockKind::Heading(level), None, allocator)
            {
                blocks.push(ParsedBlock::Text(block));
            }
        }
        BlockNode::Blockquote { children, kind, .. } => {
            for child in children {
                convert_html_node(
                    child,
                    Some(TextBlockKind::BlockQuote(*kind)),
                    list_depth,
                    allocator,
                    blocks,
                );
            }
        }
        BlockNode::List {
            children,
            ordered,
            start,
            ..
        } => {
            let mut next = start.unwrap_or(1);
            for child in children {
                let prefix: SharedString = if *ordered {
                    let prefix = format!("{next}. ");
                    next += 1;
                    prefix.into()
                } else {
                    "• ".into()
                };
                if let BlockNode::ListItem { children, .. } = child {
                    let mut prefix = Some(prefix);
                    for item_child in children {
                        if matches!(item_child, BlockNode::List { .. }) {
                            convert_html_node(item_child, None, list_depth + 1, allocator, blocks);
                            continue;
                        }
                        if let BlockNode::Paragraph(paragraph) = item_child {
                            if let Some(block) = convert_html_paragraph(
                                paragraph,
                                TextBlockKind::ListItem {
                                    depth: list_depth.max(1),
                                },
                                prefix.take(),
                                allocator,
                            ) {
                                blocks.push(ParsedBlock::Text(block));
                            }
                        } else {
                            convert_html_node(
                                item_child,
                                Some(TextBlockKind::ListItem {
                                    depth: list_depth.max(1),
                                }),
                                list_depth,
                                allocator,
                                blocks,
                            );
                        }
                    }
                }
            }
        }
        BlockNode::ListItem { children, .. } => {
            for child in children {
                convert_html_node(
                    child,
                    Some(TextBlockKind::ListItem {
                        depth: list_depth.max(1),
                    }),
                    list_depth,
                    allocator,
                    blocks,
                );
            }
        }
        BlockNode::Table(table) => {
            let source_start = allocator.next;
            let rows = table
                .children
                .iter()
                .map(|row| ParsedHtmlTableRow {
                    cells: row
                        .children
                        .iter()
                        .filter_map(|cell| {
                            let alignment = match cell.alignment {
                                super::node::ColumnumnAlign::Left => TextAlign::Left,
                                super::node::ColumnumnAlign::Center => TextAlign::Center,
                                super::node::ColumnumnAlign::Right => TextAlign::Right,
                            };
                            convert_html_paragraph(
                                &cell.children,
                                TextBlockKind::HtmlTableCell {
                                    header: cell.is_header,
                                    alignment,
                                },
                                None,
                                allocator,
                            )
                            .map(|content| ParsedHtmlTableCell {
                                content,
                                col_span: cell.col_span,
                                row_span: cell.row_span,
                                is_header: cell.is_header,
                                alignment,
                            })
                        })
                        .collect(),
                })
                .filter(|row| !row.cells.is_empty())
                .collect::<Vec<_>>();
            if !rows.is_empty() {
                blocks.push(ParsedBlock::HtmlTable(ParsedHtmlTable {
                    source: source_start..allocator.next,
                    rows: rows.into(),
                    kind: ParsedTableKind::Html,
                }));
            }
        }
        BlockNode::Break { .. } => {
            let source = allocator.allocate(1);
            let mut builder = TextBlockBuilder::new(
                inherited_kind.unwrap_or(TextBlockKind::Paragraph),
                source.start,
            );
            builder.push("\n", source.clone());
            blocks.push(ParsedBlock::Text(builder.finish(source.end)));
        }
        BlockNode::HorizontalRule { .. } => blocks.push(ParsedBlock::Rule(allocator.allocate(1))),
        BlockNode::CodeBlock(_)
        | BlockNode::Custom(_)
        | BlockNode::Definition { .. }
        | BlockNode::Unknown => {}
    }
}

fn parse_html_block(source: &str, range: Range<usize>) -> Option<ParsedHtmlBlock> {
    let raw_source: SharedString = source.get(range.clone())?.to_string().into();
    let mut context = super::node::NodeContext::default();
    let document = super::format::html::parse(&raw_source, &mut context).ok()?;
    let mut allocator = HtmlSourceAllocator::new(source, range.clone());
    let mut blocks = Vec::new();
    for node in &document.blocks {
        convert_html_node(node, None, 1, &mut allocator, &mut blocks);
    }
    Some(ParsedHtmlBlock {
        source: range,
        raw_source,
        blocks: blocks.into(),
    })
}

fn reusable_html_block(
    previous: Option<&ParsedMarkdown>,
    source_start: usize,
    raw_source: &str,
) -> Option<ParsedHtmlBlock> {
    previous?.blocks.iter().find_map(|block| match block {
        ParsedBlock::Html(html)
            if html.source.start == source_start && html.raw_source.as_ref() == raw_source =>
        {
            Some(html.clone())
        }
        _ => None,
    })
}

/// Converts source into an immutable render model. This function never touches GPUI state.
#[cfg(test)]
fn parse_markdown(source: SharedString, options: MarkdownOptions) -> ParsedMarkdown {
    parse_markdown_with_previous(source, options, None)
}

/// Parses one revision while reusing completed HTML blocks before invoking html5ever.
fn parse_markdown_with_previous(
    source: SharedString,
    options: MarkdownOptions,
    previous: Option<&ParsedMarkdown>,
) -> ParsedMarkdown {
    if options.parse_links_only {
        return parse_links_only(source);
    }
    let mut blocks = Vec::new();
    let mut root_starts = Vec::new();
    let mut headings = HashMap::new();
    let mut heading_counts: HashMap<String, usize> = HashMap::new();
    let mut footnotes = HashMap::new();
    let mut current: Option<TextBlockBuilder> = None;
    let mut code: Option<(
        usize,
        String,
        Option<SharedString>,
        Option<SharedString>,
        Option<SharedString>,
    )> = None;
    let mut image: Option<(
        usize,
        SharedString,
        SharedString,
        String,
        Option<DefiniteLength>,
        Option<DefiniteLength>,
    )> = None;
    let mut lists: Vec<ListContext> = Vec::new();
    let mut pending_item_prefix: Option<(SharedString, usize)> = None;
    let mut quote_kind: Option<Option<BlockQuoteKind>> = None;
    let mut heading_text = String::new();
    let mut heading_start: Option<usize> = None;
    let mut table: Option<MarkdownTableBuilder> = None;
    let mut in_parsed_html_block = false;

    let parser = Parser::new_ext(&source, parser_options(options));
    for (event, range) in parser.into_offset_iter() {
        if in_parsed_html_block {
            if matches!(event, Event::End(TagEnd::HtmlBlock)) {
                in_parsed_html_block = false;
            }
            continue;
        }
        match event {
            Event::Start(Tag::Paragraph) => {
                if table.is_some() && current.is_some() {
                    continue;
                }
                if matches!(
                    current.as_ref().map(|block| &block.kind),
                    Some(TextBlockKind::ListItem { .. })
                ) {
                    continue;
                }
                let kind = if let Some(kind) = quote_kind {
                    TextBlockKind::BlockQuote(kind)
                } else if !lists.is_empty() {
                    TextBlockKind::ListItem { depth: lists.len() }
                } else {
                    TextBlockKind::Paragraph
                };
                current = Some(TextBlockBuilder::new(kind, range.start));
                if let Some((prefix, source_start)) = pending_item_prefix.take() {
                    current
                        .as_mut()
                        .expect("paragraph builder")
                        .push(prefix, source_start..source_start);
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if let Some(block) = current.take() {
                    root_starts.push(block.source_start);
                    blocks.push(ParsedBlock::Text(block.finish(range.end)));
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some(TextBlockBuilder::new(
                    TextBlockKind::Heading(level),
                    range.start,
                ));
                heading_text.clear();
                heading_start = Some(range.start);
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(block) = current.take() {
                    root_starts.push(block.source_start);
                    blocks.push(ParsedBlock::Text(block.finish(range.end)));
                }
                if options.parse_heading_slugs {
                    let base = heading_slug(&heading_text);
                    let count = heading_counts.entry(base.clone()).or_default();
                    let slug = if *count == 0 {
                        base
                    } else {
                        format!("{base}-{count}")
                    };
                    *count += 1;
                    headings.insert(slug.into(), heading_start.take().unwrap_or(range.start));
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let info = match kind {
                    CodeBlockKind::Fenced(info) if !info.is_empty() => {
                        Some(info.to_string().into())
                    }
                    _ => None,
                };
                let language = info
                    .as_ref()
                    .and_then(|info: &SharedString| info.split_whitespace().next())
                    .filter(|language| !language.is_empty())
                    .map(|language| language.to_string().into());
                let source_path = info.as_ref().and_then(|info| {
                    info.split_whitespace()
                        .find(|token| token.contains('/') || token.contains('\\'))
                        .map(|token| token.to_string().into())
                });
                code = Some((range.start, String::new(), language, info, source_path));
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((start, code_text, language, info, source_path)) = code.take() {
                    let mermaid_data_uri = (options.render_mermaid_diagrams
                        && language.as_deref() == Some("mermaid"))
                    .then(|| render_mermaid_data_uri(&code_text))
                    .flatten();
                    let code_text: SharedString = code_text.into();
                    let display_code = code_block_display_text(&code_text);
                    let highlight = if let Some(info) = info.clone() {
                        super::node::CodeBlock::new_fenced(
                            code_text.clone(),
                            info,
                            None::<super::node::Span>,
                        )
                    } else {
                        super::node::CodeBlock::new(
                            code_text.clone(),
                            language.clone(),
                            None::<super::node::Span>,
                        )
                    };
                    root_starts.push(start);
                    blocks.push(ParsedBlock::Code(ParsedCodeBlock {
                        source: start..range.end,
                        code: code_text,
                        display_code,
                        language,
                        info,
                        source_path,
                        mermaid_data_uri,
                        highlight,
                    }));
                }
            }
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                current.get_or_insert_with(|| {
                    TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                });
                image = Some((
                    range.start,
                    dest_url.to_string().into(),
                    title.to_string().into(),
                    String::new(),
                    None,
                    None,
                ));
            }
            Event::End(TagEnd::Image) => {
                if let Some((start, destination, title, alt, width, height)) = image.take() {
                    current
                        .get_or_insert_with(|| {
                            TextBlockBuilder::new(TextBlockKind::Paragraph, start)
                        })
                        .push_image(
                            start..range.end,
                            destination,
                            title,
                            alt.into(),
                            width,
                            height,
                        );
                }
            }
            Event::Start(Tag::BlockQuote(kind)) => quote_kind = Some(kind),
            Event::End(TagEnd::BlockQuote(_)) => quote_kind = None,
            Event::Start(Tag::List(start)) => lists.push(ListContext { next: start }),
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::Start(Tag::Item) => {
                let prefix = if let Some(list) = lists.last_mut() {
                    match list.next.as_mut() {
                        Some(index) => {
                            let prefix = format!("{index}. ");
                            *index += 1;
                            prefix
                        }
                        None => "• ".to_string(),
                    }
                } else {
                    "• ".to_string()
                };
                let mut block = TextBlockBuilder::new(
                    TextBlockKind::ListItem { depth: lists.len() },
                    range.start,
                );
                block.push(prefix, range.start..range.start);
                current = Some(block);
                pending_item_prefix = None;
            }
            Event::End(TagEnd::Item) => {
                if let Some(block) = current.take() {
                    root_starts.push(block.source_start);
                    blocks.push(ParsedBlock::Text(block.finish(range.end)));
                }
            }
            Event::Start(Tag::Table(alignments)) => {
                table = Some(MarkdownTableBuilder {
                    source_start: range.start,
                    alignments,
                    rows: Vec::new(),
                    current_row: Vec::new(),
                    in_header: false,
                });
            }
            Event::End(TagEnd::Table) => {
                if let Some(mut table) = table.take() {
                    if !table.current_row.is_empty() {
                        table.rows.push(ParsedHtmlTableRow {
                            cells: std::mem::take(&mut table.current_row),
                        });
                    }
                    root_starts.push(table.source_start);
                    blocks.push(ParsedBlock::HtmlTable(ParsedHtmlTable {
                        source: table.source_start..range.end,
                        rows: table.rows.into(),
                        kind: ParsedTableKind::Markdown,
                    }));
                }
            }
            Event::Start(Tag::TableRow) => {
                if let Some(table) = table.as_mut() {
                    if !table.current_row.is_empty() {
                        table.rows.push(ParsedHtmlTableRow {
                            cells: std::mem::take(&mut table.current_row),
                        });
                    }
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(table) = table.as_mut()
                    && !table.current_row.is_empty()
                {
                    table.rows.push(ParsedHtmlTableRow {
                        cells: std::mem::take(&mut table.current_row),
                    });
                }
            }
            Event::Start(Tag::TableCell) => {
                if let Some(table) = table.as_ref() {
                    current = Some(TextBlockBuilder::new(
                        TextBlockKind::HtmlTableCell {
                            header: table.in_header,
                            alignment: table.alignment(),
                        },
                        range.start,
                    ));
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let (Some(table), Some(block)) = (table.as_mut(), current.take()) {
                    table.current_row.push(ParsedHtmlTableCell {
                        content: block.finish(range.end),
                        col_span: 1,
                        row_span: 1,
                        is_header: table.in_header,
                        alignment: table.alignment(),
                    });
                }
            }
            Event::Start(Tag::TableHead) => {
                if let Some(table) = table.as_mut() {
                    table.in_header = true;
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(table) = table.as_mut() {
                    if !table.current_row.is_empty() {
                        table.rows.push(ParsedHtmlTableRow {
                            cells: std::mem::take(&mut table.current_row),
                        });
                    }
                    table.in_header = false;
                }
            }
            Event::Start(Tag::FootnoteDefinition(label)) => {
                footnotes.insert(SharedString::from(label.to_string()), range.start);
                current = Some(TextBlockBuilder::new(TextBlockKind::Footnote, range.start));
                current
                    .as_mut()
                    .expect("footnote builder")
                    .push(format!("[{label}] "), range.start..range.start);
            }
            Event::End(TagEnd::FootnoteDefinition) => {
                if let Some(block) = current.take() {
                    root_starts.push(block.source_start);
                    blocks.push(ParsedBlock::Text(block.finish(range.end)));
                }
            }
            Event::Start(Tag::HtmlBlock) if options.parse_html => {
                if let Some(raw_source) = source.get(range.clone()) {
                    let parsed = reusable_html_block(previous, range.start, raw_source)
                        .or_else(|| parse_html_block(&source, range.clone()));
                    root_starts.push(range.start);
                    if let Some(parsed) = parsed {
                        blocks.push(ParsedBlock::Html(parsed));
                    } else {
                        let mut fallback = TextBlockBuilder::new(TextBlockKind::Html, range.start);
                        fallback.push(raw_source.to_string(), range.clone());
                        blocks.push(ParsedBlock::Text(fallback.finish(range.end)));
                    }
                }
                in_parsed_html_block = true;
            }
            Event::Start(Tag::MetadataBlock(_)) if options.render_metadata_blocks => {
                current = Some(TextBlockBuilder::new(TextBlockKind::Metadata, range.start));
            }
            Event::End(TagEnd::MetadataBlock(_)) if options.render_metadata_blocks => {
                if let Some(block) = current.take() {
                    root_starts.push(block.source_start);
                    blocks.push(ParsedBlock::Text(block.finish(range.end)));
                }
            }
            Event::Start(Tag::Emphasis) => {
                current
                    .get_or_insert_with(|| {
                        TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                    })
                    .flags
                    .emphasis = true;
            }
            Event::End(TagEnd::Emphasis) => {
                if let Some(block) = current.as_mut() {
                    block.flags.emphasis = false;
                }
            }
            Event::Start(Tag::Strong) => {
                current
                    .get_or_insert_with(|| {
                        TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                    })
                    .flags
                    .strong = true;
            }
            Event::End(TagEnd::Strong) => {
                if let Some(block) = current.as_mut() {
                    block.flags.strong = false;
                }
            }
            Event::Start(Tag::Strikethrough) => {
                current
                    .get_or_insert_with(|| {
                        TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                    })
                    .flags
                    .strike = true;
            }
            Event::End(TagEnd::Strikethrough) => {
                if let Some(block) = current.as_mut() {
                    block.flags.strike = false;
                }
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                let block = current.get_or_insert_with(|| {
                    TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                });
                block.flags.link = true;
                block.active_links.push(ActiveLink {
                    source_start: range.start,
                    destination: dest_url.to_string().into(),
                });
            }
            Event::End(TagEnd::Link) => {
                if let Some(block) = current.as_mut() {
                    block.flags.link = false;
                    if let Some(link) = block.active_links.pop() {
                        block.links.push(ParsedLink {
                            source: link.source_start..range.end,
                            destination: link.destination,
                        });
                    }
                }
            }
            Event::Text(text) => {
                if let Some((_, code_text, _, _, _)) = code.as_mut() {
                    code_text.push_str(&text);
                } else if let Some((_, _, _, alt, _, _)) = image.as_mut() {
                    alt.push_str(&text);
                } else {
                    let block = current.get_or_insert_with(|| {
                        TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                    });
                    if heading_start.is_some() {
                        heading_text.push_str(&text);
                    }
                    block.push(text.to_string(), range);
                }
            }
            Event::Code(text) => {
                let block = current.get_or_insert_with(|| {
                    TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                });
                let previous = block.flags.code;
                block.flags.code = true;
                block.push(text.to_string(), range);
                block.flags.code = previous;
            }
            Event::Html(text) | Event::InlineHtml(text) if options.parse_html => {
                let block = current.get_or_insert_with(|| {
                    TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                });
                if let Some(image) = super::format::html::parse_inline_image(&text) {
                    block.push_image(
                        range,
                        image.url.to_string().into(),
                        image.title.unwrap_or_default(),
                        image.alt.unwrap_or_default(),
                        image.width,
                        image.height,
                    );
                } else if text.trim_start().to_ascii_lowercase().starts_with("<br") {
                    block.push("\n", range);
                } else if !apply_inline_html_tag(&text, range.clone(), block)
                    && !(text.trim().starts_with('<') && text.trim().ends_with('>'))
                {
                    block.push(text.to_string(), range);
                }
            }
            Event::Html(text) | Event::InlineHtml(text) => {
                current
                    .get_or_insert_with(|| {
                        TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                    })
                    .push(text.to_string(), range);
            }
            Event::FootnoteReference(label) => {
                current
                    .get_or_insert_with(|| {
                        TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                    })
                    .push(format!("[{label}]"), range);
            }
            Event::SoftBreak => {
                let block = current.get_or_insert_with(|| {
                    TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                });
                block.flags.soft_break = true;
                block.push("\n", range);
                block.flags.soft_break = false;
            }
            Event::HardBreak => {
                current
                    .get_or_insert_with(|| {
                        TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                    })
                    .push("\n", range);
            }
            Event::Rule => {
                root_starts.push(range.start);
                blocks.push(ParsedBlock::Rule(range));
            }
            Event::TaskListMarker(checked) => {
                let marker = if checked { "☑ " } else { "☐ " };
                current
                    .get_or_insert_with(|| {
                        TextBlockBuilder::new(
                            TextBlockKind::ListItem { depth: lists.len() },
                            range.start,
                        )
                    })
                    .push(marker, range);
            }
            Event::InlineMath(text) | Event::DisplayMath(text) => {
                current
                    .get_or_insert_with(|| {
                        TextBlockBuilder::new(TextBlockKind::Paragraph, range.start)
                    })
                    .push(text.to_string(), range);
            }
            _ => {}
        }
    }

    if let Some(block) = current.take() {
        root_starts.push(block.source_start);
        blocks.push(ParsedBlock::Text(block.finish(source.len())));
    }

    ParsedMarkdown {
        source,
        blocks: blocks.into(),
        root_block_starts: root_starts.into(),
        headings: Arc::new(headings),
        footnotes: Arc::new(footnotes),
    }
}

/// Removes only the terminal newline introduced before a Markdown closing fence.
fn code_block_display_text(code: &SharedString) -> SharedString {
    code.strip_suffix('\n').map_or_else(
        || code.clone(),
        |display| SharedString::from(display.to_string()),
    )
}

fn render_mermaid_data_uri(source: &str) -> Option<SharedString> {
    let svg = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let renderer = merman::render::HeadlessRenderer::new().with_vendored_text_measurer();
        let pipeline = merman::render::SvgPipeline::resvg_safe().with_postprocessor(
            merman::render::CssOverridePostprocessor::strip_existing_important(),
        );
        renderer
            .render_svg_with_pipeline_sync(source, &pipeline)
            .ok()
            .flatten()
    }))
    .ok()??;
    Some(format!("data:image/svg+xml;base64,{}", BASE64.encode(svg)).into())
}

fn parse_links_only(source: SharedString) -> ParsedMarkdown {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    let mut segments = Vec::new();
    let mut links = Vec::new();
    let mut cursor = 0;
    for link in finder.links(&source) {
        if cursor < link.start() {
            segments.push(TextSegment {
                text: source[cursor..link.start()].to_string().into(),
                source: cursor..link.start(),
                flags: InlineFlags::default(),
            });
        }
        let range = link.start()..link.end();
        segments.push(TextSegment {
            text: link.as_str().to_string().into(),
            source: range.clone(),
            flags: InlineFlags {
                link: true,
                ..Default::default()
            },
        });
        links.push(ParsedLink {
            source: range.clone(),
            destination: link.as_str().to_string().into(),
        });
        cursor = link.end();
    }
    if cursor < source.len() {
        segments.push(TextSegment {
            text: source[cursor..].to_string().into(),
            source: cursor..source.len(),
            flags: InlineFlags::default(),
        });
    }
    let block = ParsedTextBlock {
        kind: TextBlockKind::Paragraph,
        source: 0..source.len(),
        segments,
        links,
        images: Vec::new(),
        render_cache: TextPreparationCache::default(),
        intrinsic_width_cache: IntrinsicTextWidthCache::default(),
        inline_flow_layout_cache: InlineFlowLayoutCache::default(),
    };
    ParsedMarkdown {
        source,
        blocks: Arc::from([ParsedBlock::Text(block)]),
        root_block_starts: Arc::from([0]),
        headings: Arc::default(),
        footnotes: Arc::default(),
    }
}

fn heading_slug(text: &str) -> String {
    text.trim()
        .chars()
        .filter_map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_') {
                Some(character.to_lowercase().next().unwrap_or(character))
            } else if character.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

fn parser_options(options: MarkdownOptions) -> Options {
    let mut parser_options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM;
    if options.render_metadata_blocks {
        parser_options |= Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
        parser_options |= Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS;
    }
    parser_options
}

type UrlClickCallback = Arc<dyn Fn(SharedString, &mut Window, &mut App)>;
type UrlHoverCallback = Arc<dyn Fn(Option<SharedString>, &mut Window, &mut App)>;
type CodeSpanLinkCallback = Arc<dyn Fn(&str, &App) -> Option<SharedString>>;
type SourceClickCallback = Arc<dyn Fn(usize, usize, &mut Window, &mut App) -> bool>;
type CheckboxToggleCallback = Arc<dyn Fn(Range<usize>, bool, &mut Window, &mut App)>;
type ImageResolver = Arc<dyn Fn(&str, &App) -> Option<ImageSource>>;

enum AutoscrollBehavior {
    Propagate,
    Controlled(ScrollHandle),
}

/// Native GPUI Markdown element backed by one [`Markdown`] entity.
pub struct MarkdownElement {
    markdown: Entity<Markdown>,
    style: MarkdownStyle,
    code_block_renderer: CodeBlockRenderer,
    on_url_click: Option<UrlClickCallback>,
    on_url_hover: Option<UrlHoverCallback>,
    code_span_link: Option<CodeSpanLinkCallback>,
    on_source_click: Option<SourceClickCallback>,
    on_checkbox_toggle: Option<CheckboxToggleCallback>,
    image_resolver: Option<ImageResolver>,
    autoscroll: AutoscrollBehavior,
}

impl MarkdownElement {
    /// Creates a renderer for a persistent Markdown entity.
    pub fn new(markdown: Entity<Markdown>, style: MarkdownStyle) -> Self {
        Self {
            markdown,
            style,
            code_block_renderer: CodeBlockRenderer::default(),
            on_url_click: None,
            on_url_hover: None,
            code_span_link: None,
            on_source_click: None,
            on_checkbox_toggle: None,
            image_resolver: None,
            autoscroll: AutoscrollBehavior::Propagate,
        }
    }

    /// Uses a host-owned scroll handle for source navigation and selection autoscroll.
    pub fn scroll_handle(mut self, scroll_handle: ScrollHandle) -> Self {
        self.autoscroll = AutoscrollBehavior::Controlled(scroll_handle);
        self
    }

    /// Handles activation of a rendered Markdown or resolved inline-code link.
    pub fn on_url_click(
        mut self,
        callback: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_url_click = Some(Arc::new(callback));
        self
    }

    /// Reports the link under the pointer, or `None` after the pointer leaves a link.
    pub fn on_url_hover(
        mut self,
        callback: impl Fn(Option<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_url_hover = Some(Arc::new(callback));
        self
    }

    /// Resolves inline-code content to an optional clickable destination.
    pub fn on_code_span_link(
        mut self,
        callback: impl Fn(&str, &App) -> Option<SharedString> + 'static,
    ) -> Self {
        self.code_span_link = Some(Arc::new(callback));
        self
    }

    /// Handles a source position before the default selection behavior.
    /// Returning `true` consumes the click.
    pub fn on_source_click(
        mut self,
        callback: impl Fn(usize, usize, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_source_click = Some(Arc::new(callback));
        self
    }

    /// Handles a task marker click with its canonical range and next checked state.
    pub fn on_checkbox_toggle(
        mut self,
        callback: impl Fn(Range<usize>, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_checkbox_toggle = Some(Arc::new(callback));
        self
    }

    /// Resolves an image destination without granting the renderer direct network access.
    pub fn image_resolver(
        mut self,
        resolver: impl Fn(&str, &App) -> Option<ImageSource> + 'static,
    ) -> Self {
        self.image_resolver = Some(Arc::new(resolver));
        self
    }

    /// Replaces the built-in code block composition strategy.
    pub fn code_block_renderer(mut self, renderer: CodeBlockRenderer) -> Self {
        self.code_block_renderer = renderer;
        self
    }
}

impl Styled for MarkdownElement {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style.container_style
    }
}

struct RenderedLine {
    layout: RenderedLineLayout,
    text: SharedString,
    source: Range<usize>,
    mappings: Arc<[SegmentMapping]>,
    links: Vec<RenderedLink>,
}

enum RenderedLineLayout {
    Text(TextLayout),
    Inline(InlineFlowLayoutState),
}

impl From<TextLayout> for RenderedLineLayout {
    fn from(layout: TextLayout) -> Self {
        Self::Text(layout)
    }
}

impl RenderedLineLayout {
    fn bounds(&self) -> Option<Bounds<Pixels>> {
        match self {
            Self::Text(layout) => Some(layout.bounds()),
            Self::Inline(layout) => layout.bounds(),
        }
    }

    fn index_for_position(&self, position: gpui::Point<Pixels>) -> usize {
        match self {
            Self::Text(layout) => match layout.index_for_position(position) {
                Ok(index) | Err(index) => index,
            },
            Self::Inline(layout) => layout.index_for_position(position),
        }
    }

    fn position_for_index(&self, index: usize) -> Option<(gpui::Point<Pixels>, Pixels)> {
        match self {
            Self::Text(layout) => layout
                .position_for_index(index)
                .map(|position| (position, layout.line_height())),
            Self::Inline(layout) => layout.position_for_index(index),
        }
    }

    fn selection_bounds(&self, range: Range<usize>) -> Vec<Bounds<Pixels>> {
        match self {
            Self::Inline(layout) => layout.selection_bounds(range),
            Self::Text(layout) => {
                let Some(start_point) = layout.position_for_index(range.start) else {
                    return Vec::new();
                };
                let Some(end_point) = layout.position_for_index(range.end) else {
                    return Vec::new();
                };
                let line_height = layout.line_height();
                let layout_bounds = layout.bounds();
                if (start_point.y - end_point.y).abs() < px(0.5) {
                    vec![Bounds::from_corners(
                        start_point,
                        point(end_point.x.max(start_point.x), start_point.y + line_height),
                    )]
                } else {
                    let mut bounds = vec![Bounds::from_corners(
                        start_point,
                        point(layout_bounds.right(), start_point.y + line_height),
                    )];
                    if end_point.y > start_point.y + line_height {
                        bounds.push(Bounds::from_corners(
                            point(layout_bounds.left(), start_point.y + line_height),
                            point(layout_bounds.right(), end_point.y),
                        ));
                    }
                    bounds.push(Bounds::from_corners(
                        point(layout_bounds.left(), end_point.y),
                        point(end_point.x, end_point.y + line_height),
                    ));
                    bounds
                }
            }
        }
    }
}

impl RenderedLine {
    fn source_for_rendered(&self, rendered_index: usize, exclusive_end: bool) -> usize {
        let rendered_index = rendered_index.min(self.text.len());
        let mapping = self
            .mappings
            .iter()
            .find(|mapping| {
                mapping.rendered.start <= rendered_index && rendered_index <= mapping.rendered.end
            })
            .or_else(|| {
                self.mappings
                    .iter()
                    .rev()
                    .find(|mapping| mapping.rendered.start <= rendered_index)
            });
        let Some(mapping) = mapping else {
            return self.source.start;
        };
        if mapping.rendered.len() == mapping.source.len() {
            return (mapping.source.start + rendered_index.saturating_sub(mapping.rendered.start))
                .min(mapping.source.end);
        }
        if exclusive_end && rendered_index >= mapping.rendered.end {
            mapping.source.end
        } else {
            mapping.source.start
        }
    }

    fn rendered_for_source(&self, source_index: usize) -> usize {
        let source_index = source_index.clamp(self.source.start, self.source.end);
        let mapping = self
            .mappings
            .iter()
            .find(|mapping| {
                mapping.source.start <= source_index && source_index <= mapping.source.end
            })
            .or_else(|| {
                self.mappings
                    .iter()
                    .rev()
                    .find(|mapping| mapping.source.start <= source_index)
            });
        let Some(mapping) = mapping else {
            return 0;
        };
        if mapping.rendered.len() == mapping.source.len() {
            (mapping.rendered.start + source_index.saturating_sub(mapping.source.start))
                .min(mapping.rendered.end)
        } else if source_index >= mapping.source.end {
            mapping.rendered.end
        } else {
            mapping.rendered.start
        }
    }
}

#[derive(Default)]
struct RenderedText {
    lines: Vec<RenderedLine>,
}

impl RenderedText {
    fn source_index_for_position(&self, position: gpui::Point<Pixels>) -> usize {
        let mut previous_end = 0;
        for line in &self.lines {
            let Some(bounds) = line.layout.bounds() else {
                continue;
            };
            if bounds.contains(&position) {
                let index = line.layout.index_for_position(position);
                return line.source_for_rendered(index, false);
            }
            if position.y < bounds.top() {
                return previous_end;
            }
            previous_end = line.source.end;
        }
        previous_end
    }

    fn link_at(&self, source_index: usize) -> Option<&RenderedLink> {
        self.lines
            .iter()
            .flat_map(|line| &line.links)
            .find(|link| link.source.contains(&source_index))
    }

    fn plain_text_for_source_range(&self, range: Range<usize>) -> SharedString {
        let mut text = String::new();
        for line in &self.lines {
            let start = range.start.max(line.source.start);
            let end = range.end.min(line.source.end);
            if start >= end {
                continue;
            }
            let rendered_start = line.rendered_for_source(start);
            let rendered_end = line.rendered_for_source(end);
            if let Some(selected) = line.text.get(rendered_start..rendered_end) {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(selected);
            }
        }
        text.into()
    }

    fn position_for_source_index(
        &self,
        source_index: usize,
    ) -> Option<(gpui::Point<Pixels>, Pixels)> {
        let line = self
            .lines
            .iter()
            .find(|line| line.source.contains(&source_index) || source_index == line.source.end)?;
        let rendered = line.rendered_for_source(source_index);
        line.layout.position_for_index(rendered)
    }

    fn selection_bounds(&self, range: Range<usize>) -> Vec<Bounds<Pixels>> {
        let mut bounds = Vec::new();
        for line in &self.lines {
            let start = range.start.max(line.source.start);
            let end = range.end.min(line.source.end);
            if start >= end {
                continue;
            }
            let rendered_start = line.rendered_for_source(start);
            let rendered_end = line.rendered_for_source(end);
            bounds.extend(line.layout.selection_bounds(rendered_start..rendered_end));
        }
        bounds
    }
}

pub struct RenderedMarkdown {
    element: AnyElement,
    text: Rc<RenderedText>,
}

fn text_refinement(style: &TextStyle) -> TextStyleRefinement {
    TextStyleRefinement {
        color: Some(style.color),
        font_family: Some(style.font_family.clone()),
        font_fallbacks: style.font_fallbacks.clone(),
        font_features: Some(style.font_features.clone()),
        font_size: Some(style.font_size),
        font_weight: Some(style.font_weight),
        font_style: Some(style.font_style),
        line_height: Some(style.line_height),
        white_space: Some(style.white_space),
        ..Default::default()
    }
}

fn heading_refinement(style: &MarkdownStyle, level: HeadingLevel) -> Option<&TextStyleRefinement> {
    let headings = style.heading_level_styles.as_ref()?;
    match level {
        HeadingLevel::H1 => headings.h1.as_ref(),
        HeadingLevel::H2 => headings.h2.as_ref(),
        HeadingLevel::H3 => headings.h3.as_ref(),
        HeadingLevel::H4 => headings.h4.as_ref(),
        HeadingLevel::H5 => headings.h5.as_ref(),
        HeadingLevel::H6 => headings.h6.as_ref(),
    }
}

/// Resolves the typography applied to visible and measured table-cell text.
fn table_cell_text_style(style: &MarkdownStyle, header: bool, alignment: TextAlign) -> TextStyle {
    let mut text_style = style.base_text_style.clone();
    text_style.text_align = alignment;
    if header {
        text_style.font_weight = gpui::FontWeight::SEMIBOLD;
    }
    if !style.table_columns_min_size {
        text_style.white_space = WhiteSpace::Nowrap;
    }
    text_style
}

fn render_text_block(
    block: &ParsedTextBlock,
    style: &MarkdownStyle,
    search_state: (&[Range<usize>], Option<usize>),
    code_span_link: Option<&CodeSpanLinkCallback>,
    image_resolver: Option<&ImageResolver>,
    window: &mut Window,
    cx: &App,
) -> (AnyElement, RenderedLine) {
    let (search, active_search) = search_state;
    let mut block_text_style = style.base_text_style.clone();
    if let TextBlockKind::Heading(level) = block.kind {
        if let Some(refinement) = heading_refinement(style, level) {
            block_text_style.refine(refinement);
        }
    }
    if matches!(block.kind, TextBlockKind::BlockQuote(_)) {
        block_text_style.refine(&style.block_quote);
    }
    if matches!(block.kind, TextBlockKind::Metadata) {
        block_text_style.refine(&style.inline_code);
    }
    if let TextBlockKind::HtmlTableCell { header, alignment } = block.kind {
        block_text_style = table_cell_text_style(style, header, alignment);
    }

    let key = TextPreparationKey {
        block_style: block_text_style.clone(),
        inline_code: style.inline_code.clone(),
        link: style.link.clone(),
        selection_background_color: style.selection_background_color,
        soft_break_as_hard_break: style.soft_break_as_hard_break,
        search: search.to_vec(),
        active_search,
    };
    let prepared = prepare_text_block(block, key.clone());
    let text = prepared.text.clone();
    let styled_text = StyledText::new(text.clone()).with_runs(prepared.runs.clone());
    let layout = styled_text.layout().clone();
    let mut links = prepared.links.to_vec();
    if let Some(resolve) = code_span_link {
        links.extend(block.segments.iter().filter_map(|segment| {
            if segment.flags.code {
                resolve(segment.text.as_ref(), cx).map(|destination| RenderedLink {
                    source: segment.source.clone(),
                    destination,
                })
            } else {
                None
            }
        }));
    }

    let table_cell = matches!(block.kind, TextBlockKind::HtmlTableCell { .. });
    let mut container = div().when(!table_cell, |container| container.w_full().min_w_0().mb_3());
    container.style().text = text_refinement(&block_text_style);
    container = match block.kind {
        TextBlockKind::Heading(level) => {
            let mut heading = container.mt_2().font_weight(gpui::FontWeight::SEMIBOLD);
            heading.style().refine(&style.heading);
            if style.heading_border_color.is_some()
                && matches!(
                    level,
                    HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3
                )
            {
                heading = heading
                    .pb_1()
                    .border_b_1()
                    .border_color(style.heading_border_color.unwrap_or(style.rule_color));
            }
            heading
        }
        TextBlockKind::BlockQuote(kind) => container.pl_3().border_l_3().border_color(
            style
                .block_quote_kind_colors
                .color(kind, style.block_quote_border_color),
        ),
        TextBlockKind::ListItem { depth } => container.pl(px(12. * depth as f32)),
        TextBlockKind::HtmlTableCell { .. } => container,
        TextBlockKind::Metadata => container.p_2().bg(style.rule_color.opacity(0.08)),
        _ => container,
    };
    if block.images.is_empty() {
        return (
            container.child(styled_text).into_any_element(),
            RenderedLine {
                layout: layout.into(),
                text,
                source: block.source.clone(),
                mappings: prepared.mappings.clone(),
                links,
            },
        );
    }

    let mut items = Vec::with_capacity(block.segments.len());
    let mut flow_text = String::new();
    let mut mappings = Vec::with_capacity(block.segments.len());
    let image_sizing = image_sizing_for_block(block);
    for (segment_index, segment) in block.segments.iter().enumerate() {
        let start = flow_text.len();
        if let Some(image) = block
            .images
            .iter()
            .find(|image| image.segment_index == segment_index)
        {
            if let Some(source) =
                image_resolver.and_then(|resolver| resolver(&image.destination, cx))
            {
                flow_text.push('\u{1a}');
                items.push(InlineFlowItem::Image {
                    url: image.destination.to_string().into(),
                    source: Some(source),
                    sizing: image_sizing,
                    link: None,
                    title: image.title.to_string(),
                    width: image.width,
                    height: image.height,
                    style: Box::default(),
                });
            } else {
                flow_text.push_str(&image.alt);
                items.push(inline_text_item(
                    image.alt.clone(),
                    start,
                    segment,
                    &block_text_style,
                    &key,
                    window,
                ));
            }
        } else {
            let display: SharedString = if segment.flags.soft_break && !key.soft_break_as_hard_break
            {
                " ".into()
            } else {
                segment.text.clone()
            };
            flow_text.push_str(&display);
            items.push(inline_text_item(
                display,
                start,
                segment,
                &block_text_style,
                &key,
                window,
            ));
        }
        mappings.push(SegmentMapping {
            rendered: start..flow_text.len(),
            source: segment.source.clone(),
        });
    }

    let layout_state = InlineFlowLayoutState::default();
    let flow = InlineFlow::new(("markdown-inline-flow", block.source.start), items)
        .layout_cache(block.inline_flow_layout_cache.clone())
        .layout_state(layout_state.clone());
    (
        container.child(flow).into_any_element(),
        RenderedLine {
            layout: RenderedLineLayout::Inline(layout_state),
            text: flow_text.into(),
            source: block.source.clone(),
            mappings: mappings.into(),
            links,
        },
    )
}

/// Selects block sizing only when a paragraph contains no visible text beside its images.
fn image_sizing_for_block(block: &ParsedTextBlock) -> InlineImageSizing {
    let image_only = block.segments.iter().enumerate().all(|(index, segment)| {
        block
            .images
            .iter()
            .any(|image| image.segment_index == index)
            || segment.text.trim().is_empty()
    });
    if image_only {
        InlineImageSizing::Intrinsic
    } else {
        InlineImageSizing::Compact
    }
}

/// Builds one cacheable inline text item with the same resolved style as `StyledText`.
fn inline_text_item(
    text: SharedString,
    paragraph_start: usize,
    segment: &TextSegment,
    block_style: &TextStyle,
    key: &TextPreparationKey,
    window: &Window,
) -> InlineFlowItem {
    let mut resolved = block_style.clone();
    if segment.flags.strong {
        resolved.font_weight = gpui::FontWeight::BOLD;
    }
    if segment.flags.emphasis {
        resolved.font_style = gpui::FontStyle::Italic;
    }
    if segment.flags.strike {
        resolved.strikethrough = Some(StrikethroughStyle::default());
    }
    if segment.flags.underline {
        resolved.underline = Some(UnderlineStyle::default());
    }
    if segment.flags.code {
        resolved.refine(&key.inline_code);
    }
    if segment.flags.link {
        resolved.refine(&key.link);
    }

    let mut boundaries = vec![0, text.len()];
    for range in &key.search {
        if range.start < segment.source.end && range.end > segment.source.start {
            if segment.source.len() == text.len() {
                boundaries.push(range.start.max(segment.source.start) - segment.source.start);
                boundaries.push(range.end.min(segment.source.end) - segment.source.start);
            }
        }
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    let highlights = boundaries
        .windows(2)
        .filter_map(|bounds| {
            let range = bounds[0]..bounds[1];
            if range.is_empty() {
                return None;
            }
            let mut highlight = HighlightStyle {
                color: Some(resolved.color),
                font_weight: Some(resolved.font_weight),
                font_style: Some(resolved.font_style),
                background_color: resolved.background_color,
                underline: resolved.underline,
                strikethrough: resolved.strikethrough,
                fade_out: None,
            };
            if segment.source.len() == text.len()
                && let Some((index, _)) = key.search.iter().enumerate().find(|(_, search)| {
                    let source_start = segment.source.start + range.start;
                    let source_end = segment.source.start + range.end;
                    search.start < source_end && search.end > source_start
                })
            {
                highlight.background_color = Some(key.selection_background_color.opacity(
                    if key.active_search == Some(index) {
                        0.8
                    } else {
                        0.45
                    },
                ));
            }
            Some((range, highlight))
        })
        .collect();
    let rem_size = window.rem_size();
    let box_style = (resolved.font_family != block_style.font_family
        || resolved.font_size != block_style.font_size
        || resolved.line_height != block_style.line_height)
        .then(|| InlineBoxStyle {
            font_family: Some(resolved.font_family.clone()),
            font_size: Some(resolved.font_size.to_pixels(rem_size)),
            line_height: Some(resolved.line_height.to_pixels(resolved.font_size, rem_size)),
            ..Default::default()
        });
    InlineFlowItem::Text {
        state: Arc::new(std::sync::Mutex::new(InlineState::default())),
        paragraph_range: paragraph_start..(paragraph_start + text.len()),
        text,
        links: Vec::new(),
        highlights,
        link_hover_style: None,
        box_style,
    }
}

/// Flatten and style one parsed text block, reusing the result while its semantic
/// content and visual inputs remain unchanged.
fn prepare_text_block(block: &ParsedTextBlock, key: TextPreparationKey) -> CachedTextPreparation {
    if let Ok(cache) = block.render_cache.0.lock()
        && let Some(cached) = cache.as_ref()
        && cached.key == key
    {
        return cached.clone();
    }

    let mut text = String::new();
    let mut mappings = Vec::new();
    let mut runs = Vec::new();
    for segment in &block.segments {
        let start = text.len();
        if segment.flags.soft_break && !key.soft_break_as_hard_break {
            text.push(' ');
        } else {
            text.push_str(&segment.text);
        }
        let end = text.len();
        mappings.push(SegmentMapping {
            rendered: start..end,
            source: segment.source.clone(),
        });

        let mut segment_style = key.block_style.clone();
        if segment.flags.strong {
            segment_style.font_weight = gpui::FontWeight::BOLD;
        }
        if segment.flags.emphasis {
            segment_style.font_style = gpui::FontStyle::Italic;
        }
        if segment.flags.strike {
            segment_style.strikethrough = Some(StrikethroughStyle::default());
        }
        if segment.flags.underline {
            segment_style.underline = Some(UnderlineStyle::default());
        }
        if segment.flags.code {
            segment_style.refine(&key.inline_code);
        }
        if segment.flags.link {
            segment_style.refine(&key.link);
        }
        let segment_len = end - start;
        let mut highlights = key
            .search
            .iter()
            .enumerate()
            .filter(|(_, range)| {
                range.start < segment.source.end && range.end > segment.source.start
            })
            .map(|(index, range)| {
                let rendered_range = if segment.source.len() == segment_len {
                    range.start.max(segment.source.start) - segment.source.start
                        ..range.end.min(segment.source.end) - segment.source.start
                } else {
                    0..segment_len
                };
                (index, rendered_range)
            })
            .collect::<Vec<_>>();
        highlights.sort_by_key(|(_, range)| range.start);

        let mut run_cursor = 0;
        for (index, range) in highlights {
            let range = range.start.max(run_cursor)..range.end.min(segment_len);
            if range.start >= range.end {
                continue;
            }
            if run_cursor < range.start {
                runs.push(segment_style.clone().to_run(range.start - run_cursor));
            }
            let mut run = segment_style.clone().to_run(range.len());
            run.background_color = Some(key.selection_background_color.opacity(
                if key.active_search == Some(index) {
                    0.8
                } else {
                    0.45
                },
            ));
            runs.push(run);
            run_cursor = range.end;
        }
        if run_cursor < segment_len {
            runs.push(segment_style.to_run(segment_len - run_cursor));
        }
    }

    let cached = CachedTextPreparation {
        key,
        text: text.into(),
        mappings: mappings.into(),
        runs,
        links: block
            .links
            .iter()
            .map(|link| RenderedLink {
                source: link.source.clone(),
                destination: link.destination.clone(),
            })
            .collect::<Vec<_>>()
            .into(),
    };
    if let Ok(mut cache) = block.render_cache.0.lock() {
        *cache = Some(cached.clone());
    }
    cached
}

fn render_code_block(
    block: &ParsedCodeBlock,
    style: &MarkdownStyle,
    renderer: &CodeBlockRenderer,
    wrapped: bool,
    scroll_handle: Option<&ScrollHandle>,
    markdown: &Entity<Markdown>,
) -> (AnyElement, RenderedLine) {
    let mut code_style = style.base_text_style.clone();
    code_style.refine(&TextStyleRefinement {
        font_family: style.inline_code.font_family.clone(),
        font_size: style.inline_code.font_size,
        ..Default::default()
    });
    let text = block.display_code.clone();
    let display_len = text.len();
    let mut runs = Vec::new();
    let mut cursor = 0;
    for (range, highlight) in block.highlight.styles(&style.syntax) {
        let range = range.start.min(text.len())..range.end.min(text.len());
        if cursor < range.start {
            runs.push(code_style.clone().to_run(range.start - cursor));
        }
        if range.start < range.end {
            runs.push(code_style.clone().highlight(highlight).to_run(range.len()));
        }
        cursor = cursor.max(range.end);
    }
    if cursor < text.len() {
        runs.push(code_style.clone().to_run(text.len() - cursor));
    }
    if runs.is_empty() {
        runs.push(code_style.to_run(text.len()));
    }
    let styled_text = StyledText::new(text.clone()).with_runs(runs);
    let layout = styled_text.layout().clone();
    let context = CodeBlockRenderContext {
        code: block.code.clone(),
        language: block.language.clone(),
        info: block.info.clone(),
        source_path: block.source_path.clone(),
        source_range: block.source.clone(),
        wrapped,
    };
    let element = match renderer {
        CodeBlockRenderer::Custom(callback) => {
            // Custom callbacks need Window/App and are resolved by the caller below.
            let _ = callback;
            div().child(styled_text).into_any_element()
        }
        CodeBlockRenderer::Default {
            copy_button_visibility,
            wrap_button_visibility,
            border,
        } => {
            if let Some(source) = block.mermaid_data_uri.clone() {
                return (
                    div()
                        .id(("markdown-mermaid", block.source.start))
                        .w_full()
                        .min_w_0()
                        .mb_3()
                        .child(img(source).max_w_full())
                        .into_any_element(),
                    RenderedLine {
                        layout: layout.into(),
                        text,
                        source: block.source.clone(),
                        mappings: vec![SegmentMapping {
                            rendered: 0..display_len,
                            source: block.source.clone(),
                        }]
                        .into(),
                        links: Vec::new(),
                    },
                );
            }
            let group_id: SharedString = format!("markdown-code-{}", block.source.start).into();
            let mut container = div()
                .id(("markdown-code", block.source.start))
                .group(group_id.clone())
                .relative()
                .w_full()
                .min_w_0()
                .mb_3()
                .rounded_lg()
                .bg(style.rule_color.opacity(0.08));
            container.style().text = text_refinement(&code_style);
            container.style().refine(&style.code_block);
            if *border {
                container = container
                    .rounded_md()
                    .border_1()
                    .border_color(style.rule_color);
            }
            let content = if let Some(scroll_handle) = scroll_handle {
                div()
                    .id(("markdown-code-content", block.source.start))
                    .flex()
                    .w_full()
                    .min_w_0()
                    .p_2()
                    .whitespace_nowrap()
                    .overflow_x_scroll()
                    .restrict_scroll_to_axis()
                    .track_scroll(scroll_handle)
                    .child(styled_text)
            } else {
                div()
                    .id(("markdown-code-content", block.source.start))
                    .w_full()
                    .min_w_0()
                    .p_2()
                    .when(wrapped, |this| this.whitespace_normal())
                    .when(!wrapped, |this| this.whitespace_nowrap())
                    .child(styled_text)
            };

            let actions = h_flex()
                .absolute()
                .top_1()
                .right_1()
                .gap_1()
                .when(
                    !matches!(copy_button_visibility, CopyButtonVisibility::Hidden),
                    |this| {
                        this.child(
                            div()
                                .when(
                                    matches!(
                                        copy_button_visibility,
                                        CopyButtonVisibility::VisibleOnHover
                                    ),
                                    |this| {
                                        this.invisible()
                                            .group_hover(group_id.clone(), |this| this.visible())
                                    },
                                )
                                .child(
                                    Clipboard::new(("markdown-code-copy", block.source.start))
                                        .value(block.code.clone())
                                        .tooltip("Copy code"),
                                ),
                        )
                    },
                )
                .when(
                    !matches!(wrap_button_visibility, WrapButtonVisibility::Hidden),
                    |this| {
                        let markdown = markdown.clone();
                        let source_start = block.source.start;
                        this.child(
                            div()
                                .when(
                                    matches!(
                                        wrap_button_visibility,
                                        WrapButtonVisibility::VisibleOnHover
                                    ),
                                    |this| {
                                        this.invisible()
                                            .group_hover(group_id.clone(), |this| this.visible())
                                    },
                                )
                                .child(
                                    Button::new(("markdown-code-wrap", source_start))
                                        .xsmall()
                                        .ghost()
                                        .label(if wrapped { "Unwrap" } else { "Wrap" })
                                        .on_click(move |_, _, cx| {
                                            markdown.update(cx, |markdown, cx| {
                                                if !markdown
                                                    .wrapped_code_blocks
                                                    .remove(&source_start)
                                                {
                                                    markdown
                                                        .wrapped_code_blocks
                                                        .insert(source_start);
                                                }
                                                cx.notify();
                                            });
                                        }),
                                ),
                        )
                    },
                );
            let _ = context;
            container = container.child(content).child(actions);
            if let Some(scroll_handle) = scroll_handle {
                container = container.horizontal_scrollbar(scroll_handle);
            }
            container.into_any_element()
        }
    };
    (
        element,
        RenderedLine {
            layout: layout.into(),
            text,
            source: block.source.clone(),
            mappings: vec![SegmentMapping {
                rendered: 0..display_len,
                source: block.source.clone(),
            }]
            .into(),
            links: Vec::new(),
        },
    )
}

fn html_table_column_count(rows: &[ParsedHtmlTableRow]) -> usize {
    const MAX_TABLE_COLUMNS: usize = 1_000;

    let mut occupied = vec![Vec::<bool>::new(); rows.len()];
    let mut column_count = 0;
    for (row_index, row) in rows.iter().enumerate() {
        let mut column_index = 0;
        for cell in &row.cells {
            while occupied[row_index]
                .get(column_index)
                .copied()
                .unwrap_or(false)
            {
                column_index += 1;
            }
            let col_span = cell
                .col_span
                .max(1)
                .min(MAX_TABLE_COLUMNS.saturating_sub(column_index));
            if col_span == 0 {
                break;
            }
            let row_span = cell.row_span.min(rows.len() - row_index).max(1);
            let required_columns = column_index.saturating_add(col_span);
            column_count = column_count.max(required_columns).min(MAX_TABLE_COLUMNS);
            for occupied_row in occupied.iter_mut().skip(row_index).take(row_span) {
                occupied_row.resize(column_count, false);
                occupied_row[column_index..required_columns].fill(true);
            }
            column_index = required_columns;
        }
    }
    column_count
}

/// Returns the text runs intersecting one UTF-8 byte range while preserving
/// the original run styles used by the visible table cell.
fn text_runs_for_range(runs: &[TextRun], range: Range<usize>) -> Vec<TextRun> {
    let mut result = Vec::new();
    let mut cursor = 0;
    for run in runs {
        let run_start = cursor;
        let run_end = cursor + run.len;
        cursor = run_end;
        if run_end <= range.start {
            continue;
        }
        if run_start >= range.end {
            break;
        }
        let start = range.start.max(run_start);
        let end = range.end.min(run_end);
        if start < end {
            result.push(TextRun {
                len: end - start,
                ..run.clone()
            });
        }
    }
    result
}

/// Measures the widest explicit line in one cell. The value is cached on the
/// parsed block and reused when streaming keeps the block and typography stable.
fn table_cell_intrinsic_text_width(
    cell: &ParsedHtmlTableCell,
    style: &MarkdownStyle,
    window: &mut Window,
) -> Pixels {
    let text_style = table_cell_text_style(style, cell.is_header, cell.alignment);
    let key = TextPreparationKey {
        block_style: text_style.clone(),
        inline_code: style.inline_code.clone(),
        link: style.link.clone(),
        selection_background_color: style.selection_background_color,
        soft_break_as_hard_break: style.soft_break_as_hard_break,
        search: Vec::new(),
        active_search: None,
    };
    if let Ok(cache) = cell.content.intrinsic_width_cache.0.lock()
        && let Some((cached_key, width)) = cache.as_ref()
        && cached_key == &key
    {
        return *width;
    }

    let prepared = prepare_text_block(&cell.content, key.clone());
    let font_size = text_style.font_size.to_pixels(window.rem_size());
    let mut widest = Pixels::ZERO;
    let mut line_start = 0;
    for line in prepared.text.split_inclusive('\n') {
        let visible_len = line.strip_suffix('\n').map_or(line.len(), str::len);
        let line_end = line_start + visible_len;
        if line_end > line_start {
            let line_runs = text_runs_for_range(&prepared.runs, line_start..line_end);
            widest = widest.max(
                window
                    .text_system()
                    .layout_line(
                        &prepared.text[line_start..line_end],
                        font_size,
                        &line_runs,
                        None,
                    )
                    .width,
            );
        }
        line_start += line.len();
    }
    let image_width = cell
        .content
        .images
        .iter()
        .map(|image| {
            image
                .width
                .map(|width| width.to_pixels(AbsoluteLength::Pixels(font_size), window.rem_size()))
                .unwrap_or(font_size)
        })
        .sum::<Pixels>();
    widest += image_width;

    if let Ok(mut cache) = cell.content.intrinsic_width_cache.0.lock() {
        *cache = Some((key, widest));
    }
    widest
}

struct HtmlTableRenderContext<'a> {
    style: &'a MarkdownStyle,
    search_state: (&'a [Range<usize>], Option<usize>),
    code_span_link: Option<&'a CodeSpanLinkCallback>,
    image_resolver: Option<&'a ImageResolver>,
    scroll_handle: &'a ScrollHandle,
}

/// Adapts parsed Markdown and HTML table cells to the shared table module.
fn render_html_table(
    table: &ParsedHtmlTable,
    context: HtmlTableRenderContext<'_>,
    window: &mut Window,
    cx: &App,
) -> (AnyElement, Vec<RenderedLine>) {
    let HtmlTableRenderContext {
        style,
        search_state,
        code_span_link,
        image_resolver,
        scroll_handle,
    } = context;
    let column_count = html_table_column_count(&table.rows);
    if column_count == 0 {
        return (div().into_any_element(), Vec::new());
    }

    let cell_padding = match table.kind {
        ParsedTableKind::Markdown => Edges {
            top: px(2.),
            right: px(6.),
            bottom: px(2.),
            left: px(6.),
        },
        ParsedTableKind::Html => Edges {
            top: px(4.),
            right: px(8.),
            bottom: px(4.),
            left: px(8.),
        },
    };
    let mut rendered_lines = Vec::new();
    let mut grid = TableGrid::new(
        ("markdown-table", table.source.start),
        column_count,
        scroll_handle,
    )
    .sizing(if style.table_columns_min_size {
        TableGridSizing::MinContent
    } else {
        TableGridSizing::MaxContent
    })
    .cell_padding(cell_padding)
    .min_column_width(Pixels::ZERO)
    .border_color(style.rule_color)
    .header_background(cx.theme().table_head)
    .stripe_background(cx.theme().table_even)
    .debug_selector("markdown-table-grid")
    .mb_2()
    .min_w_0();

    for (row_index, row) in table.rows.iter().enumerate() {
        let mut table_row = TableGridRow::new();
        for (cell_index, cell) in row.cells.iter().enumerate() {
            let (content, line) = render_text_block(
                &cell.content,
                style,
                search_state,
                code_span_link,
                image_resolver,
                window,
                cx,
            );
            rendered_lines.push(line);

            let cell_selector = format!(
                "markdown-table-cell-{}-{row_index}-{cell_index}",
                table.source.start
            );
            let content_selector = format!(
                "markdown-table-cell-content-{}-{row_index}-{cell_index}",
                table.source.start
            );
            let accessible_label = cell
                .content
                .segments
                .iter()
                .map(|segment| segment.text.as_ref())
                .collect::<String>();
            let content_element = div()
                .debug_selector(move || content_selector.clone())
                .flex()
                .flex_col()
                .justify_center()
                .text_align(cell.alignment)
                .child(content);
            let mut table_cell = TableGridCell::new()
                .id(SharedString::from(cell_selector.clone()))
                .debug_selector(cell_selector)
                .intrinsic_width(table_cell_intrinsic_text_width(cell, style, window))
                .header(cell.is_header)
                .text_align(cell.alignment)
                .col_span(cell.col_span)
                .row_span(cell.row_span)
                .child(content_element);
            if !accessible_label.trim().is_empty() {
                table_cell = table_cell.aria_label(SharedString::from(accessible_label));
            }
            table_row = table_row.child(table_cell);
        }
        grid = grid.child(table_row);
    }

    (grid.into_any_element(), rendered_lines)
}

impl Element for MarkdownElement {
    type RequestLayoutState = RenderedMarkdown;
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let (parsed, search_highlights, active_search, wrapped_code_blocks) = {
            let markdown = self.markdown.read(cx);
            (
                markdown.parsed.clone(),
                markdown.search_highlights.clone(),
                markdown.active_search_highlight,
                markdown.wrapped_code_blocks.clone(),
            )
        };
        let mut root = div().w_full().min_w_0();
        root.style().refine(&self.style.container_style);
        root.style().text = text_refinement(&self.style.base_text_style);
        let mut rendered = RenderedText::default();
        let mut active_code_block_scroll_handles = HashSet::new();
        let mut active_table_scroll_handles = HashSet::new();

        for block in parsed.blocks.iter() {
            match block {
                ParsedBlock::Text(block) => {
                    let (element, line) = render_text_block(
                        block,
                        &self.style,
                        (&search_highlights, active_search),
                        self.code_span_link.as_ref(),
                        self.image_resolver.as_ref(),
                        window,
                        cx,
                    );
                    root = root.child(element);
                    rendered.lines.push(line);
                }
                ParsedBlock::Code(block) => {
                    if let CodeBlockRenderer::Custom(callback) = &self.code_block_renderer {
                        let context = CodeBlockRenderContext {
                            code: block.code.clone(),
                            language: block.language.clone(),
                            info: block.info.clone(),
                            source_path: block.source_path.clone(),
                            source_range: block.source.clone(),
                            wrapped: wrapped_code_blocks.contains(&block.source.start),
                        };
                        root = root.child(callback(context, window, cx));
                        let (_, line) = render_code_block(
                            block,
                            &self.style,
                            &self.code_block_renderer,
                            wrapped_code_blocks.contains(&block.source.start),
                            None,
                            &self.markdown,
                        );
                        rendered.lines.push(line);
                        continue;
                    }
                    let wrapped = wrapped_code_blocks.contains(&block.source.start);
                    let scroll_handle = (self.style.code_block_overflow_x_scroll
                        && !wrapped
                        && block.mermaid_data_uri.is_none())
                    .then(|| {
                        active_code_block_scroll_handles.insert(block.source.start);
                        self.markdown.update(cx, |markdown, _| {
                            markdown.code_block_scroll_handle(block.source.start)
                        })
                    });
                    let (element, line) = render_code_block(
                        block,
                        &self.style,
                        &self.code_block_renderer,
                        wrapped,
                        scroll_handle.as_ref(),
                        &self.markdown,
                    );
                    root = root.child(element);
                    rendered.lines.push(line);
                }
                ParsedBlock::Html(html) => {
                    for child in html.blocks.iter() {
                        match child {
                            ParsedBlock::Text(block) => {
                                let (element, line) = render_text_block(
                                    block,
                                    &self.style,
                                    (&search_highlights, active_search),
                                    self.code_span_link.as_ref(),
                                    self.image_resolver.as_ref(),
                                    window,
                                    cx,
                                );
                                root = root.child(element);
                                rendered.lines.push(line);
                            }
                            ParsedBlock::Rule(_) => {
                                root = root.child(
                                    div()
                                        .w_full()
                                        .my_2()
                                        .border_b_1()
                                        .border_color(self.style.rule_color),
                                );
                            }
                            ParsedBlock::HtmlTable(table) => {
                                active_table_scroll_handles.insert(table.source.start);
                                let scroll_handle = self.markdown.update(cx, |markdown, _| {
                                    markdown.table_scroll_handle(table.source.start)
                                });
                                let (element, lines) = render_html_table(
                                    table,
                                    HtmlTableRenderContext {
                                        style: &self.style,
                                        search_state: (&search_highlights, active_search),
                                        code_span_link: self.code_span_link.as_ref(),
                                        image_resolver: self.image_resolver.as_ref(),
                                        scroll_handle: &scroll_handle,
                                    },
                                    window,
                                    cx,
                                );
                                root = root.child(element);
                                rendered.lines.extend(lines);
                            }
                            ParsedBlock::Code(_) | ParsedBlock::Html(_) => {}
                        }
                    }
                }
                ParsedBlock::HtmlTable(table) => {
                    active_table_scroll_handles.insert(table.source.start);
                    let scroll_handle = self.markdown.update(cx, |markdown, _| {
                        markdown.table_scroll_handle(table.source.start)
                    });
                    let (element, lines) = render_html_table(
                        table,
                        HtmlTableRenderContext {
                            style: &self.style,
                            search_state: (&search_highlights, active_search),
                            code_span_link: self.code_span_link.as_ref(),
                            image_resolver: self.image_resolver.as_ref(),
                            scroll_handle: &scroll_handle,
                        },
                        window,
                        cx,
                    );
                    root = root.child(element);
                    rendered.lines.extend(lines);
                }
                ParsedBlock::Rule(_source_range) => {
                    root = root.child(
                        div()
                            .w_full()
                            .my_2()
                            .border_b_1()
                            .border_color(self.style.rule_color),
                    );
                }
            }
        }

        self.markdown.update(cx, |markdown, _| {
            markdown.retain_code_block_scroll_handles(&active_code_block_scroll_handles);
            markdown.retain_table_scroll_handles(&active_table_scroll_handles);
        });

        let mut element = root.into_any_element();
        let layout = element.request_layout(window, cx);
        (
            layout,
            RenderedMarkdown {
                element,
                text: Rc::new(rendered),
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        window.set_focus_handle(&self.markdown.read(cx).focus_handle, cx);
        window.set_view_id(self.markdown.entity_id());
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        state.element.prepaint(window, cx);

        if let Some(source_index) = self
            .markdown
            .update(cx, |markdown, _| markdown.autoscroll_request.take())
            && let Some((position, line_height)) =
                state.text.position_for_source_index(source_index)
        {
            match &self.autoscroll {
                AutoscrollBehavior::Controlled(handle) => {
                    let viewport = handle.bounds();
                    let offset = handle.offset();
                    let margin = line_height * 2.;
                    let next = if position.y < viewport.top() + margin {
                        offset.y + viewport.top() + margin - position.y
                    } else if position.y + line_height > viewport.bottom() - margin {
                        offset.y + viewport.bottom() - margin - position.y - line_height
                    } else {
                        offset.y
                    };
                    handle.set_offset(point(offset.x, next.clamp(-handle.max_offset().y, px(0.))));
                }
                AutoscrollBehavior::Propagate => {
                    window.request_autoscroll(Bounds::new(
                        point(position.x, position.y - line_height * 2.),
                        gpui::size(px(1.), line_height * 5.),
                    ));
                }
            }
        }
        hitbox
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let mut context = KeyContext::default();
        context.add("Markdown");
        window.set_key_context(context);

        let rendered = state.text.clone();
        window.on_action(std::any::TypeId::of::<crate::input::Copy>(), {
            let markdown = self.markdown.clone();
            let rendered = rendered.clone();
            move |_, phase, _, cx| {
                if phase == DispatchPhase::Bubble {
                    let range = markdown.read(cx).selection.range();
                    if !range.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            rendered.plain_text_for_source_range(range).to_string(),
                        ));
                    }
                }
            }
        });
        window.on_action(std::any::TypeId::of::<CopyAsMarkdown>(), {
            let markdown = self.markdown.clone();
            move |_, phase, _, cx| {
                if phase == DispatchPhase::Bubble
                    && let Some(source) = markdown.read(cx).selected_source()
                {
                    cx.write_to_clipboard(ClipboardItem::new_string(source.to_string()));
                }
            }
        });

        if !self.style.prevent_mouse_interaction {
            self.install_mouse_handlers(hitbox, rendered.clone(), window, cx);
        }
        state.element.paint(window, cx);

        let selection = self.markdown.read(cx).selection.range();
        if !selection.is_empty() {
            for bounds in rendered.selection_bounds(selection) {
                window.paint_quad(fill(
                    bounds,
                    self.style.selection_background_color.opacity(0.35),
                ));
            }
        }
    }
}

impl MarkdownElement {
    fn install_mouse_handlers(
        &self,
        hitbox: &Hitbox,
        rendered: Rc<RenderedText>,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let entity = self.markdown.downgrade();
        let hitbox = hitbox.clone();
        let move_hitbox = hitbox.clone();
        let rendered_down = rendered.clone();
        let on_source_click = self.on_source_click.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble
                || event.button != MouseButton::Left
                || !hitbox.is_hovered(window)
            {
                return;
            }
            let index = rendered_down.source_index_for_position(event.position);
            if let Some(callback) = on_source_click.as_ref() {
                let source = entity
                    .upgrade()
                    .map(|markdown| markdown.read(cx).source.clone())
                    .unwrap_or_default();
                let (line, column) = source_line_column(&source, index);
                if callback(line, column, window, cx) {
                    window.prevent_default();
                    return;
                }
            }
            let _ = entity.update(cx, |markdown, cx| {
                let range = match event.click_count {
                    2 => super::selection::word_range_at(&markdown.source, index)
                        .unwrap_or(index..index),
                    3 => source_line_range(&markdown.source, index),
                    count if count >= 4 => 0..markdown.source.len(),
                    _ => index..index,
                };
                markdown.selection = Selection {
                    anchor: range.start,
                    head: range.end,
                    pending: true,
                    mode: match event.click_count {
                        2 => SelectionMode::Word,
                        3 => SelectionMode::Line,
                        count if count >= 4 => SelectionMode::All,
                        _ => SelectionMode::Character,
                    },
                };
                markdown.pressed_link = rendered_down
                    .link_at(index)
                    .map(|link| link.destination.clone());
                window.focus(&markdown.focus_handle, cx);
                cx.notify();
            });
            window.prevent_default();
        });

        let entity = self.markdown.downgrade();
        let rendered_move = rendered.clone();
        let hover = self.on_url_hover.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase == DispatchPhase::Capture {
                return;
            }
            let index = rendered_move.source_index_for_position(event.position);
            let hovered = move_hitbox
                .is_hovered(window)
                .then(|| rendered_move.link_at(index))
                .flatten()
                .map(|link| link.destination.clone());
            if let Some(callback) = hover.as_ref() {
                callback(hovered, window, cx);
            }
            let _ = entity.update(cx, |markdown, cx| {
                if markdown.selection.pending {
                    markdown.selection.head = match markdown.selection.mode {
                        SelectionMode::Character => index,
                        SelectionMode::Word => {
                            super::selection::word_range_at(&markdown.source, index)
                                .map_or(index, |range| range.end)
                        }
                        SelectionMode::Line => source_line_range(&markdown.source, index).end,
                        SelectionMode::All => markdown.source.len(),
                    };
                    markdown.autoscroll_request = Some(index);
                    cx.notify();
                }
            });
        });

        let entity = self.markdown.downgrade();
        let rendered_up = rendered;
        let on_click = self.on_url_click.clone();
        let on_checkbox_toggle = self.on_checkbox_toggle.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if event.button != MouseButton::Left || phase != DispatchPhase::Bubble {
                return;
            }
            let index = rendered_up.source_index_for_position(event.position);
            let released = rendered_up
                .link_at(index)
                .map(|link| link.destination.clone());
            let _ = entity.update(cx, |markdown, cx| {
                let pressed = markdown.pressed_link.take();
                let was_click = markdown.selection.range().is_empty();
                markdown.selection.pending = false;
                if was_click
                    && let Some(callback) = on_checkbox_toggle.as_ref()
                    && let Some((range, checked)) = task_marker_at(&markdown.source, index)
                {
                    callback(range, !checked, window, cx);
                    cx.notify();
                    return;
                }
                if was_click
                    && let Some(destination) = pressed
                    && Some(destination.clone()) == released
                {
                    if let Some(callback) = on_click.as_ref() {
                        callback(destination, window, cx);
                    } else {
                        cx.open_url(&destination);
                    }
                }
                cx.notify();
            });
        });
    }
}

fn source_line_column(source: &str, index: usize) -> (usize, usize) {
    let range = source_line_range(source, index);
    let line = source[..range.start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    let column = source[range.start..index.min(range.end)].chars().count();
    (line, column)
}

fn task_marker_at(source: &str, index: usize) -> Option<(Range<usize>, bool)> {
    let line = source_line_range(source, index);
    let text = source.get(line.clone())?;
    for (marker, checked) in [("[ ]", false), ("[x]", true), ("[X]", true)] {
        if let Some(offset) = text.find(marker) {
            let range = line.start + offset..line.start + offset + marker.len();
            if range.contains(&index) || index == range.end {
                return Some((range, checked));
            }
        }
    }
    None
}

fn source_line_range(source: &str, index: usize) -> Range<usize> {
    let mut index = index.min(source.len());
    while !source.is_char_boundary(index) {
        index -= 1;
    }
    let start = source[..index].rfind('\n').map_or(0, |offset| offset + 1);
    let end = source[index..]
        .find('\n')
        .map_or(source.len(), |offset| index + offset);
    start..end
}

impl IntoElement for MarkdownElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    use gpui::{Context, Render, ScrollDelta, ScrollWheelEvent, VisualTestContext};

    struct CodeBlockScrollTestRoot {
        markdown: Entity<Markdown>,
    }

    struct TableScrollTestRoot {
        markdown: Entity<Markdown>,
        width: Pixels,
    }

    struct TableTextLayoutTestRoot {
        table: ParsedHtmlTable,
        layouts: Rc<RefCell<Vec<(TextLayout, SharedString)>>>,
        scroll_handle: ScrollHandle,
    }

    impl Render for CodeBlockScrollTestRoot {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().w(px(220.)).child(MarkdownElement::new(
                self.markdown.clone(),
                MarkdownStyle::themed(MarkdownFont::Agent, window, cx),
            ))
        }
    }

    impl Render for TableScrollTestRoot {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .debug_selector(|| "table-test-root".to_string())
                .w(self.width)
                .child(MarkdownElement::new(
                    self.markdown.clone(),
                    MarkdownStyle::themed(MarkdownFont::Agent, window, cx),
                ))
        }
    }

    impl Render for TableTextLayoutTestRoot {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let style = MarkdownStyle::themed(MarkdownFont::Agent, window, cx);
            let (element, lines) = render_html_table(
                &self.table,
                HtmlTableRenderContext {
                    style: &style,
                    search_state: (&[], None),
                    code_span_link: None,
                    image_resolver: None,
                    scroll_handle: &self.scroll_handle,
                },
                window,
                cx,
            );
            *self.layouts.borrow_mut() = lines
                .into_iter()
                .filter_map(|line| match line.layout {
                    RenderedLineLayout::Text(layout) => Some((layout, line.text)),
                    RenderedLineLayout::Inline(_) => None,
                })
                .collect();
            element
        }
    }

    #[test]
    fn parser_preserves_ordered_start_callout_and_code_info() {
        let parsed = parse_markdown(
            "3. third\n4. fourth\n\n> [!WARNING]\n> careful\n\n```rust src/main.rs\nfn main() {}\n```"
                .into(),
            MarkdownOptions::default(),
        );

        let list_text = parsed
            .blocks
            .iter()
            .filter_map(|block| match block {
                ParsedBlock::Text(block) => Some(
                    block
                        .segments
                        .iter()
                        .map(|segment| segment.text.as_ref())
                        .collect::<String>(),
                ),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(list_text.contains("3. third"));
        assert!(list_text.contains("4. fourth"));
        assert!(parsed.blocks.iter().any(|block| matches!(
            block,
            ParsedBlock::Text(ParsedTextBlock {
                kind: TextBlockKind::BlockQuote(Some(BlockQuoteKind::Warning)),
                ..
            })
        )));
        assert!(parsed.blocks.iter().any(|block| matches!(
            block,
            ParsedBlock::Code(ParsedCodeBlock { language: Some(language), info: Some(info), .. })
                if language.as_ref() == "rust" && info.as_ref() == "rust src/main.rs"
        )));
    }

    #[test]
    fn code_block_display_trims_one_terminal_newline_without_changing_copy_source() {
        let parsed = parse_markdown(
            "```rust\nlet value = 1;\n\n```".into(),
            MarkdownOptions::default(),
        );
        let ParsedBlock::Code(block) = &parsed.blocks[0] else {
            panic!("expected code block");
        };

        assert_eq!(block.code.as_ref(), "let value = 1;\n\n");
        assert_eq!(block.display_code.as_ref(), "let value = 1;\n");
        assert_eq!(code_block_display_text(&"plain".into()).as_ref(), "plain");
        assert_eq!(code_block_display_text(&"two\n\n".into()).as_ref(), "two\n");
    }

    #[test]
    fn parser_keeps_source_mapping_on_unicode_and_hidden_delimiters() {
        let source = "A **粗体** and `code`";
        let parsed = parse_markdown(source.into(), MarkdownOptions::default());
        let ParsedBlock::Text(block) = &parsed.blocks[0] else {
            panic!("expected text block");
        };
        assert!(block.segments.iter().any(|segment| {
            segment.text.as_ref() == "粗体"
                && segment.flags.strong
                && &source[segment.source.clone()] == "粗体"
        }));
        assert!(block.segments.iter().any(|segment| {
            segment.text.as_ref() == "code"
                && segment.flags.code
                && source[segment.source.clone()].contains("`code`")
        }));
    }

    #[test]
    fn parser_indexes_duplicate_headings_and_footnotes() {
        let parsed = parse_markdown(
            "# Same\n\n# Same\n\nref[^a]\n\n[^a]: definition".into(),
            MarkdownOptions {
                parse_heading_slugs: true,
                ..Default::default()
            },
        );
        assert!(parsed.headings.contains_key("same"));
        assert!(parsed.headings.contains_key("same-1"));
        assert!(parsed.footnotes.contains_key("a"));
    }

    #[test]
    fn source_line_range_handles_unicode_boundaries() {
        let source = "alpha\n中文 line\nomega";
        assert_eq!(source_line_range(source, 8), 6..17);
    }

    #[test]
    fn every_streaming_prefix_is_parseable_and_preserves_canonical_source() {
        let source = "# 标题\n\nIncomplete **strong**, `code`, [link](https://example.com), and ![图](asset.svg)";
        for end in (0..=source.len()).filter(|end| source.is_char_boundary(*end)) {
            let prefix = &source[..end];
            let parsed = parse_markdown(prefix.to_string().into(), MarkdownOptions::default());
            assert_eq!(parsed.source.as_ref(), prefix);
        }
    }

    #[test]
    fn tail_append_reuses_unchanged_block_runtime_cache() {
        let previous = parse_markdown(
            "First paragraph.\n\n```rust\nfn stable() {}\n```\n\nTail".into(),
            MarkdownOptions::default(),
        );
        let mut next = parse_markdown(
            "First paragraph.\n\n```rust\nfn stable() {}\n```\n\nTail grows".into(),
            MarkdownOptions::default(),
        );

        let ParsedBlock::Text(first_before) = &previous.blocks[0] else {
            panic!("expected a text prefix");
        };
        let prefix_cache = first_before.render_cache.0.clone();
        reuse_unchanged_block_prefix(&previous, &mut next);

        let ParsedBlock::Text(first_after) = &next.blocks[0] else {
            panic!("expected a text prefix");
        };
        assert!(Arc::ptr_eq(&prefix_cache, &first_after.render_cache.0));
        assert_eq!(previous.blocks[1], next.blocks[1]);
        assert_ne!(previous.blocks.last(), next.blocks.last());
    }

    #[gpui::test]
    fn coalesced_streaming_appends_publish_the_latest_canonical_source(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::init);
        let markdown = cx.update(|cx| cx.new(|cx| Markdown::new("base", cx)));
        markdown.update(cx, |markdown, cx| {
            markdown.append(" one", cx);
            markdown.append(" two", cx);
            markdown.append(" 三", cx);
        });
        cx.run_until_parked();

        markdown.read_with(cx, |markdown, _| {
            assert_eq!(markdown.source(), "base one two 三");
            assert_eq!(markdown.parsed.source.as_ref(), "base one two 三");
            assert!(!markdown.is_parsing());
        });
    }

    #[gpui::test]
    fn code_block_scroll_handles_reuse_state_and_clear_inactive_blocks(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::init);
        let markdown = cx.update(|cx| cx.new(|cx| Markdown::new("", cx)));
        let first = markdown.update(cx, |markdown, _| markdown.code_block_scroll_handle(7));
        first.set_offset(point(px(-24.), px(0.)));

        let reused = markdown.update(cx, |markdown, _| markdown.code_block_scroll_handle(7));
        assert_eq!(reused.offset().x, px(-24.));

        markdown.update(cx, |markdown, _| {
            markdown.retain_code_block_scroll_handles(&HashSet::new());
        });
        let replaced = markdown.update(cx, |markdown, _| markdown.code_block_scroll_handle(7));
        assert_eq!(replaced.offset().x, px(0.));
    }

    #[gpui::test]
    fn long_code_block_tracks_horizontal_scroll_without_vertical_remapping(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::init);
        let markdown_slot = Rc::new(RefCell::new(None));
        let slot = markdown_slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let markdown = cx.new(|cx| {
                Markdown::new(
                    "```text\nabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz\n```",
                    cx,
                )
            });
            *slot.borrow_mut() = Some(markdown.clone());
            let content = cx.new(|_| CodeBlockScrollTestRoot { markdown });
            crate::Root::new(content, window, cx)
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let markdown = markdown_slot
            .borrow()
            .clone()
            .expect("markdown entity should be captured");
        let scroll_handle = markdown.read_with(cx, |markdown, _| {
            markdown
                .code_block_scroll_handles
                .values()
                .next()
                .cloned()
                .expect("code block scroll handle")
        });
        assert!(scroll_handle.max_offset().x > px(0.));

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(20.), px(20.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(0.))),
            ..Default::default()
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        assert!(scroll_handle.offset().x < px(0.));

        let horizontal_offset = scroll_handle.offset().x;
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(20.), px(20.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        assert_eq!(scroll_handle.offset().x, horizontal_offset);
    }

    #[gpui::test]
    fn table_keeps_content_width_and_scrolls_only_when_needed(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let markdown_slot = Rc::new(RefCell::new(None));
        let slot = markdown_slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let markdown = cx.new(|cx| {
                Markdown::new(
                    "| Header 1 | Centered | Header 3 | Align Right |\n| --- | :---: | --- | ---: |\n| Cell 0 | Cell 1 | This is a long cell with line break. | Cell 3 |",
                    cx,
                )
            });
            *slot.borrow_mut() = Some(markdown.clone());
            let content = cx.new(|_| TableScrollTestRoot {
                markdown,
                width: px(1400.),
            });
            crate::Root::new(content, window, cx)
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let first_cell = cx
            .debug_bounds("markdown-table-cell-0-0-0")
            .expect("first table cell bounds");
        assert!(
            first_cell.size.width > px(70.),
            "table cell collapsed to {:?}",
            first_cell.size.width
        );
        let last_cell = cx
            .debug_bounds("markdown-table-cell-0-0-3")
            .expect("last table cell bounds");
        let second_cell = cx
            .debug_bounds("markdown-table-cell-0-0-1")
            .expect("second table cell bounds");
        let third_cell = cx
            .debug_bounds("markdown-table-cell-0-0-2")
            .expect("third table cell bounds");
        let first_body_cell = cx
            .debug_bounds("markdown-table-cell-0-1-0")
            .expect("first body table cell bounds");
        let last_body_cell = cx
            .debug_bounds("markdown-table-cell-0-1-3")
            .expect("last body table cell bounds");
        let grid = cx
            .debug_bounds("markdown-table-grid")
            .expect("table grid bounds");
        assert_eq!(first_cell.left(), first_body_cell.left());
        assert_eq!(first_cell.size.width, first_body_cell.size.width);
        assert_eq!(last_cell.left(), last_body_cell.left());
        assert_eq!(last_cell.size.width, last_body_cell.size.width);
        assert!(
            (first_cell.right() - second_cell.left()).abs() <= px(0.5),
            "first={first_cell:?}, second={second_cell:?}, third={third_cell:?}, last={last_cell:?}, grid={grid:?}"
        );
        assert!(
            (second_cell.right() - third_cell.left()).abs() <= px(0.5),
            "second={second_cell:?}, third={third_cell:?}"
        );
        assert!(
            (third_cell.right() - last_cell.left()).abs() <= px(0.5),
            "third={third_cell:?}, last={last_cell:?}"
        );
        assert!((grid.right() - last_cell.right() - px(1.5)).abs() <= px(0.5));
        assert!(
            last_cell.right() <= grid.right(),
            "last cell {:?} escaped table frame {:?}",
            last_cell,
            grid
        );

        let markdown = markdown_slot
            .borrow()
            .clone()
            .expect("markdown entity should be captured");
        let handle = markdown.read_with(cx, |markdown, _| {
            markdown
                .table_scroll_handles
                .values()
                .next()
                .cloned()
                .expect("table scroll handle")
        });
        assert_eq!(handle.max_offset().x, px(0.));
    }

    #[gpui::test]
    fn table_keeps_unbreakable_headers_on_one_line(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let markdown = cx.new(|cx| {
                Markdown::new(
                    "| Syntax | Description | Test Text |\n| --- | --- | --- |\n| Header | Title | Here's this |",
                    cx,
                )
            });
            let content = cx.new(|_| TableScrollTestRoot {
                markdown,
                width: px(700.),
            });
            crate::Root::new(content, window, cx)
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let syntax = cx
            .debug_bounds("markdown-table-cell-content-0-0-0")
            .expect("syntax content bounds");
        let description = cx
            .debug_bounds("markdown-table-cell-content-0-0-1")
            .expect("description content bounds");
        assert_eq!(
            description.size.height, syntax.size.height,
            "Description wrapped inside the word: syntax={syntax:?}, description={description:?}"
        );
        assert!(
            description.size.width > syntax.size.width,
            "content-sized Description column collapsed: syntax={syntax:?}, description={description:?}"
        );
    }

    #[gpui::test]
    fn table_text_layout_keeps_soft_content_and_preserves_hard_breaks(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::init);
        let parsed = parse_markdown(
            "<table><tr><th>Description</th><th>Line<br>Break</th><th>This is a long cell with line break.</th></tr></table>"
                .into(),
            MarkdownOptions {
                parse_html: true,
                ..Default::default()
            },
        );
        let ParsedBlock::Html(html) = &parsed.blocks[0] else {
            panic!("expected HTML block");
        };
        let table = html
            .blocks
            .iter()
            .find_map(|block| match block {
                ParsedBlock::HtmlTable(table) => Some(table.clone()),
                _ => None,
            })
            .expect("expected structured HTML table");
        let layouts = Rc::new(RefCell::new(Vec::new()));
        let captured_layouts = layouts.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let content = cx.new(|_| TableTextLayoutTestRoot {
                table,
                layouts: captured_layouts,
                scroll_handle: ScrollHandle::new(),
            });
            crate::Root::new(content, window, cx)
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let layouts = layouts.borrow();
        let (description_layout, description) = &layouts[0];
        let description_start = description_layout
            .position_for_index(0)
            .expect("description start position");
        let description_end = description_layout
            .position_for_index(description.len())
            .expect("description end position");
        assert_eq!(description_start.y, description_end.y);

        let (hard_break_layout, hard_break_text) = &layouts[1];
        let break_index = hard_break_text.find('\n').expect("explicit hard break");
        let before_break = hard_break_layout
            .position_for_index(break_index)
            .expect("position before hard break");
        let after_break = hard_break_layout
            .position_for_index(break_index + 1)
            .expect("position after hard break");
        assert!(after_break.y > before_break.y);

        let (long_text_layout, long_text) = &layouts[2];
        let long_text_end = long_text_layout
            .position_for_index(long_text.len())
            .expect("long text end position");
        let long_text_cell = cx
            .debug_bounds("markdown-table-cell-0-0-2")
            .expect("long text cell bounds");
        assert!(long_text_end.x <= long_text_cell.right() - px(8.) + px(0.5));
    }

    #[gpui::test]
    fn wide_table_scrolls_horizontally_without_collapsing_columns(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let markdown_slot = Rc::new(RefCell::new(None));
        let slot = markdown_slot.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let markdown = cx.new(|cx| {
                Markdown::new(
                    "| Left | Right |\n| --- | --- |\n| value | far_right_cell_content_that_is_much_wider_than_the_viewport |",
                    cx,
                )
            });
            *slot.borrow_mut() = Some(markdown.clone());
            let content = cx.new(|_| TableScrollTestRoot {
                markdown,
                width: px(220.),
            });
            crate::Root::new(content, window, cx)
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let markdown = markdown_slot
            .borrow()
            .clone()
            .expect("markdown entity should be captured");
        let handle = markdown.read_with(cx, |markdown, _| {
            markdown
                .table_scroll_handles
                .values()
                .next()
                .cloned()
                .expect("table scroll handle")
        });
        let root_bounds = cx
            .debug_bounds("table-test-root")
            .expect("table root bounds");
        let grid_bounds = cx
            .debug_bounds("markdown-table-grid")
            .expect("table grid bounds");
        let last_cell = cx
            .debug_bounds("markdown-table-cell-0-1-1")
            .expect("last table cell bounds");
        assert!(
            last_cell.right() <= grid_bounds.right(),
            "last cell {:?} escaped table frame {:?}",
            last_cell,
            grid_bounds
        );
        assert!(
            handle.max_offset().x > px(0.),
            "root={:?}, grid={:?}, max_offset={:?}",
            root_bounds.size.width,
            grid_bounds.size.width,
            handle.max_offset().x
        );

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(20.), px(20.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(0.))),
            ..Default::default()
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        assert!(handle.offset().x < px(0.));
    }

    #[gpui::test]
    fn html_table_rowspan_keeps_explicit_shared_column_tracks(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let markdown = cx.new(|cx| {
                Markdown::new_with_options(
                    r#"<table><tr><td rowspan="2">A</td><td>B</td></tr><tr><td>C</td><td>D</td></tr></table>"#,
                    MarkdownOptions {
                        parse_html: true,
                        ..Default::default()
                    },
                    cx,
                )
            });
            let content = cx.new(|_| TableScrollTestRoot {
                markdown,
                width: px(700.),
            });
            crate::Root::new(content, window, cx)
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let spanning = cx
            .debug_bounds("markdown-table-cell-0-0-0")
            .expect("row-spanning cell bounds");
        let second_column = cx
            .debug_bounds("markdown-table-cell-0-0-1")
            .expect("second-column cell bounds");
        let second_row_second_column = cx
            .debug_bounds("markdown-table-cell-0-1-0")
            .expect("second-row second-column cell bounds");
        let third_column = cx
            .debug_bounds("markdown-table-cell-0-1-1")
            .expect("third-column cell bounds");
        let grid = cx
            .debug_bounds("markdown-table-grid")
            .expect("HTML table grid bounds");

        assert_eq!(second_column.left(), second_row_second_column.left());
        assert!(spanning.bottom() >= second_row_second_column.bottom());
        assert!(third_column.right() <= grid.right());
    }

    #[test]
    fn links_only_uses_plain_source_ranges() {
        let source = "before https://example.com/path after";
        let parsed = parse_markdown(
            source.into(),
            MarkdownOptions {
                parse_links_only: true,
                ..Default::default()
            },
        );
        let ParsedBlock::Text(block) = &parsed.blocks[0] else {
            panic!("expected one links-only text block");
        };
        assert_eq!(block.links.len(), 1);
        assert_eq!(
            &source[block.links[0].source.clone()],
            "https://example.com/path"
        );
    }

    #[test]
    fn parser_retains_image_and_task_source_ranges() {
        let source = "- [x] done\n\n![alt text](asset.svg \"Title\")";
        let parsed = parse_markdown(source.into(), MarkdownOptions::default());
        assert!(parsed.blocks.iter().any(|block| matches!(
            block,
            ParsedBlock::Text(ParsedTextBlock { images, .. })
                if images.iter().any(|image| image.destination.as_ref() == "asset.svg"
                    && image.alt.as_ref() == "alt text")
        )));
        let marker = source.find("[x]").expect("task marker");
        assert_eq!(
            task_marker_at(source, marker + 1),
            Some((marker..marker + 3, true))
        );
    }

    #[test]
    fn parser_keeps_markdown_and_html_images_inside_their_paragraph() {
        let source = "Build [![status](badge.svg)](https://example.com) with <img src=\"avatar.png\" alt=\"avatar\" width=\"32\" height=\"24\" /> inline.";
        let parsed = parse_markdown(
            source.into(),
            MarkdownOptions {
                parse_html: true,
                ..Default::default()
            },
        );
        assert_eq!(parsed.blocks.len(), 1);
        let ParsedBlock::Text(block) = &parsed.blocks[0] else {
            panic!("expected one inline paragraph");
        };
        assert_eq!(block.images.len(), 2);
        assert_eq!(block.images[0].destination.as_ref(), "badge.svg");
        assert_eq!(block.images[1].destination.as_ref(), "avatar.png");
        assert_eq!(block.images[1].alt.as_ref(), "avatar");
        assert!(block.images[1].width.is_some());
        assert!(block.images[1].height.is_some());
        assert!(block.links.iter().any(|link| {
            link.destination.as_ref() == "https://example.com"
                && link.source.start <= block.images[0].source.start
                && link.source.end >= block.images[0].source.end
        }));
    }

    #[test]
    fn image_sizing_distinguishes_standalone_and_mixed_paragraphs() {
        let standalone = parse_markdown("![large](image.png)".into(), MarkdownOptions::default());
        let mixed = parse_markdown(
            "before ![badge](badge.svg) after".into(),
            MarkdownOptions::default(),
        );
        let ParsedBlock::Text(standalone) = &standalone.blocks[0] else {
            panic!("expected standalone image paragraph");
        };
        let ParsedBlock::Text(mixed) = &mixed.blocks[0] else {
            panic!("expected mixed image paragraph");
        };

        assert_eq!(
            image_sizing_for_block(standalone),
            InlineImageSizing::Intrinsic
        );
        assert_eq!(image_sizing_for_block(mixed), InlineImageSizing::Compact);
    }

    #[test]
    fn soft_break_keeps_render_policy_out_of_the_parser() {
        let parsed = parse_markdown("first\nsecond".into(), MarkdownOptions::default());
        let ParsedBlock::Text(block) = &parsed.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            block
                .segments
                .iter()
                .any(|segment| segment.flags.soft_break)
        );
    }

    #[test]
    fn lowercase_search_mapping_preserves_expanding_unicode_ranges() {
        let source = "İstanbul 中文";
        let (folded, ranges) = lowercase_with_source_ranges(source);
        let found = folded.find("i\u{307}").expect("expanded lowercase");
        assert_eq!(ranges[found].start..ranges[found + 2].end, 0..2);
    }

    #[test]
    fn opted_in_mermaid_uses_the_vendored_renderer() {
        let parsed = parse_markdown(
            "```mermaid\ngraph TD\n  A-->B\n```".into(),
            MarkdownOptions {
                render_mermaid_diagrams: true,
                ..Default::default()
            },
        );
        assert!(parsed.blocks.iter().any(|block| matches!(
            block,
            ParsedBlock::Code(ParsedCodeBlock { mermaid_data_uri: Some(uri), .. })
                if uri.starts_with("data:image/svg+xml;base64,")
        )));
    }

    #[test]
    fn html_block_uses_semantic_text_instead_of_literal_tags() {
        let source = r#"<div>
            Here is a test in div.
            <p>A <a href="https://example.com">link</a>, <strong>bold</strong>, <em>italic</em>, and <code>code</code>.</p>
        </div>"#;
        let parsed = parse_markdown(
            source.into(),
            MarkdownOptions {
                parse_html: true,
                ..Default::default()
            },
        );
        let ParsedBlock::Html(html) = &parsed.blocks[0] else {
            panic!("expected semantic HTML block");
        };
        let text_blocks = html
            .blocks
            .iter()
            .filter_map(|block| match block {
                ParsedBlock::Text(block) => Some(block),
                _ => None,
            })
            .collect::<Vec<_>>();
        let visible = text_blocks
            .iter()
            .flat_map(|block| &block.segments)
            .map(|segment| segment.text.as_ref())
            .collect::<String>();
        assert!(visible.contains("Here is a test in div."));
        assert!(visible.contains("A link, bold, italic, and code."));
        assert!(!visible.contains("<div>"));
        assert!(
            text_blocks
                .iter()
                .flat_map(|block| &block.segments)
                .any(|segment| segment.text.as_ref() == "bold" && segment.flags.strong)
        );
        assert!(
            text_blocks
                .iter()
                .flat_map(|block| &block.segments)
                .any(|segment| segment.text.as_ref() == "italic" && segment.flags.emphasis)
        );
        assert!(
            text_blocks
                .iter()
                .flat_map(|block| &block.segments)
                .any(|segment| segment.text.as_ref() == "code" && segment.flags.code)
        );
        assert!(
            text_blocks
                .iter()
                .flat_map(|block| &block.links)
                .any(|link| link.destination.as_ref() == "https://example.com")
        );
    }

    #[test]
    fn inline_html_styles_merge_into_the_markdown_text_block() {
        let parsed = parse_markdown(
            "before <strong>bold</strong> and <a href=\"https://example.com\">link</a> after"
                .into(),
            MarkdownOptions {
                parse_html: true,
                ..Default::default()
            },
        );
        let ParsedBlock::Text(block) = &parsed.blocks[0] else {
            panic!("expected text block");
        };
        assert!(
            block
                .segments
                .iter()
                .any(|segment| segment.text.as_ref() == "bold" && segment.flags.strong)
        );
        assert!(
            block
                .links
                .iter()
                .any(|link| link.destination.as_ref() == "https://example.com")
        );
        assert!(
            block
                .segments
                .iter()
                .all(|segment| !segment.text.contains('<'))
        );
    }

    #[test]
    fn completed_html_block_is_reused_before_streaming_tail_parse() {
        let options = MarkdownOptions {
            parse_html: true,
            ..Default::default()
        };
        let previous = parse_markdown("<p>stable</p>\n\nTail".into(), options);
        let next = parse_markdown_with_previous(
            "<p>stable</p>\n\nTail grows".into(),
            options,
            Some(&previous),
        );
        let ParsedBlock::Html(before) = &previous.blocks[0] else {
            panic!("expected previous HTML block");
        };
        let ParsedBlock::Html(after) = &next.blocks[0] else {
            panic!("expected next HTML block");
        };
        assert!(Arc::ptr_eq(&before.blocks, &after.blocks));
    }

    #[test]
    fn html_omits_non_renderable_content() {
        let parsed = parse_markdown(
            "<div>safe<script>alert('x')</script><iframe>hidden</iframe></div>".into(),
            MarkdownOptions {
                parse_html: true,
                ..Default::default()
            },
        );
        let ParsedBlock::Html(html) = &parsed.blocks[0] else {
            panic!("expected HTML block");
        };
        let visible = html
            .blocks
            .iter()
            .filter_map(|block| match block {
                ParsedBlock::Text(block) => Some(block),
                _ => None,
            })
            .flat_map(|block| &block.segments)
            .map(|segment| segment.text.as_ref())
            .collect::<String>();
        assert_eq!(visible, "safe");
    }

    #[test]
    fn html_lists_and_images_use_existing_markdown_semantics() {
        let parsed = parse_markdown(
            r#"<ol start="3"><li>third</li><li>fourth<ul><li>nested</li></ul></li></ol><img src="avatar.png" alt="Avatar" width="32" height="32">"#
                .into(),
            MarkdownOptions {
                parse_html: true,
                ..Default::default()
            },
        );
        let ParsedBlock::Html(html) = &parsed.blocks[0] else {
            panic!("expected HTML block");
        };
        let text_blocks = html
            .blocks
            .iter()
            .filter_map(|block| match block {
                ParsedBlock::Text(block) => Some(block),
                _ => None,
            })
            .collect::<Vec<_>>();
        let visible = text_blocks
            .iter()
            .flat_map(|block| &block.segments)
            .map(|segment| segment.text.as_ref())
            .collect::<String>();
        assert!(visible.contains("3. third"));
        assert!(visible.contains("4. fourth"));
        assert!(visible.contains("• nested"));
        assert!(
            text_blocks
                .iter()
                .any(|block| matches!(block.kind, TextBlockKind::ListItem { depth: 2 }))
        );
        assert!(
            text_blocks
                .iter()
                .flat_map(|block| &block.images)
                .any(|image| {
                    image.destination.as_ref() == "avatar.png"
                        && image.width == Some(gpui::px(32.).into())
                        && image.height == Some(gpui::px(32.).into())
                })
        );
    }

    #[test]
    fn every_streaming_html_prefix_is_parseable() {
        let source = "<div><p>streaming <strong>HTML</strong></p><img src=\"avatar.png\"></div>";
        for end in (0..=source.len()).filter(|end| source.is_char_boundary(*end)) {
            let prefix = &source[..end];
            let parsed = parse_markdown(
                prefix.to_string().into(),
                MarkdownOptions {
                    parse_html: true,
                    ..Default::default()
                },
            );
            assert_eq!(parsed.source.as_ref(), prefix);
        }
    }

    #[test]
    fn disabled_html_parsing_keeps_literal_source() {
        let parsed = parse_markdown("<p>literal</p>".into(), MarkdownOptions::default());
        let visible = parsed
            .blocks
            .iter()
            .filter_map(|block| match block {
                ParsedBlock::Text(block) => Some(block),
                _ => None,
            })
            .flat_map(|block| &block.segments)
            .map(|segment| segment.text.as_ref())
            .collect::<String>();
        assert_eq!(visible, "<p>literal</p>");
    }

    #[test]
    fn markdown_table_preserves_grid_headers_and_alignment() {
        let parsed = parse_markdown(
            "| Header 1 | Centered | Align Right |\n| --- | :---: | ---: |\n| Cell 1 | Cell 2 | Cell 3 |"
                .into(),
            MarkdownOptions::default(),
        );
        let ParsedBlock::HtmlTable(table) = &parsed.blocks[0] else {
            panic!("expected structured table");
        };
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells.len(), 3);
        assert!(table.rows[0].cells.iter().all(|cell| cell.is_header));
        assert_eq!(table.rows[0].cells[1].alignment, TextAlign::Center);
        assert_eq!(table.rows[0].cells[2].alignment, TextAlign::Right);
        assert_eq!(table.rows[1].cells[1].alignment, TextAlign::Center);
    }

    #[test]
    fn table_column_count_accounts_for_rowspan_occupancy() {
        let parsed = parse_markdown(
            r#"<table><tr><td rowspan="2">A</td><td>B</td></tr><tr><td>C</td><td>D</td></tr></table>"#
                .into(),
            MarkdownOptions {
                parse_html: true,
                ..Default::default()
            },
        );
        let ParsedBlock::Html(html) = &parsed.blocks[0] else {
            panic!("expected HTML block");
        };
        let table = html
            .blocks
            .iter()
            .find_map(|block| match block {
                ParsedBlock::HtmlTable(table) => Some(table),
                _ => None,
            })
            .expect("expected structured HTML table");
        assert_eq!(html_table_column_count(&table.rows), 3);
    }

    #[test]
    fn max_content_table_style_disables_soft_wrapping_on_visible_text() {
        let mut style = MarkdownStyle::default();
        style.table_columns_min_size = false;
        let resolved = table_cell_text_style(&style, true, TextAlign::Center);
        assert_eq!(resolved.white_space, WhiteSpace::Nowrap);
        assert_eq!(resolved.text_align, TextAlign::Center);
        assert_eq!(resolved.font_weight, gpui::FontWeight::SEMIBOLD);

        style.table_columns_min_size = true;
        let shrinkable = table_cell_text_style(&style, false, TextAlign::Left);
        assert_eq!(shrinkable.white_space, style.base_text_style.white_space);
    }

    #[test]
    fn html_table_preserves_spans_headers_and_alignment() {
        let parsed = parse_markdown(
            r#"<table><thead><tr><th colspan="2">Header</th></tr></thead><tbody><tr><td rowspan="2">A</td><td align="right">B</td></tr><tr><td>C</td></tr></tbody></table>"#
                .into(),
            MarkdownOptions {
                parse_html: true,
                ..Default::default()
            },
        );
        let ParsedBlock::Html(html) = &parsed.blocks[0] else {
            panic!("expected HTML block");
        };
        let table = html.blocks.iter().find_map(|block| match block {
            ParsedBlock::HtmlTable(table) => Some(table),
            _ => None,
        });
        let table = table.expect("expected structured HTML table");
        assert_eq!(table.rows.len(), 3);
        assert!(table.rows[0].cells[0].is_header);
        assert_eq!(table.rows[0].cells[0].col_span, 2);
        assert_eq!(table.rows[1].cells[0].row_span, 2);
        assert_eq!(table.rows[1].cells[1].alignment, TextAlign::Right);
    }

    #[test]
    fn streaming_tail_reuses_completed_table_cell_caches() {
        let table = "| Header | Value |\n| --- | ---: |\n| stable | 1 |";
        let previous = parse_markdown(
            format!("{table}\n\nTail").into(),
            MarkdownOptions::default(),
        );
        let mut next = parse_markdown(
            format!("{table}\n\nTail grows").into(),
            MarkdownOptions::default(),
        );
        let ParsedBlock::HtmlTable(before) = &previous.blocks[0] else {
            panic!("expected previous table");
        };
        let render_cache = before.rows[1].cells[0].content.render_cache.0.clone();
        let width_cache = before.rows[1].cells[0]
            .content
            .intrinsic_width_cache
            .0
            .clone();
        reuse_unchanged_block_prefix(&previous, &mut next);
        let ParsedBlock::HtmlTable(after) = &next.blocks[0] else {
            panic!("expected next table");
        };
        assert!(Arc::ptr_eq(
            &render_cache,
            &after.rows[1].cells[0].content.render_cache.0
        ));
        assert!(Arc::ptr_eq(
            &width_cache,
            &after.rows[1].cells[0].content.intrinsic_width_cache.0
        ));
    }
}
