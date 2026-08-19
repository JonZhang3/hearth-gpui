use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    rc::Rc,
    sync::Arc,
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use gpui::{
    AnyElement, App, AppContext as _, Bounds, ClipboardItem, Context, DefiniteLength,
    DispatchPhase, Element, ElementId, Entity, FocusHandle, GlobalElementId, HighlightStyle,
    Hitbox, HitboxBehavior, Hsla, ImageSource, InspectorElementId, InteractiveElement as _,
    IntoElement, KeyContext, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement as _, Pixels, Refineable as _, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, StrikethroughStyle, StyleRefinement, Styled, StyledText, Task,
    TextLayout, TextRun, TextStyle, TextStyleRefinement, UnderlineStyle, Window, actions, div,
    fill, img, point, prelude::FluentBuilder as _, px, rems,
};
use linkify::{LinkFinder, LinkKind};
use pulldown_cmark::{
    BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};

use crate::{
    ActiveTheme as _, Sizable as _, button::Button, clipboard::Clipboard, h_flex,
    highlighter::HighlightTheme,
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

#[derive(Clone, Debug, PartialEq)]
enum TextBlockKind {
    Paragraph,
    Heading(HeadingLevel),
    BlockQuote(Option<BlockQuoteKind>),
    ListItem { depth: usize },
    Table,
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
enum ParsedBlock {
    Text(ParsedTextBlock),
    Code(ParsedCodeBlock),
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
        self.pending_parse = Some(cx.spawn(async move |entity, cx| {
            let mut parsed = cx
                .background_spawn(async move { parse_markdown(source, options) })
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
            inline_flow_layout_cache: InlineFlowLayoutCache::default(),
        }
    }
}

/// Converts source into an immutable render model. This function never touches GPUI state.
fn parse_markdown(source: SharedString, options: MarkdownOptions) -> ParsedMarkdown {
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
    let mut table_depth = 0usize;
    let mut in_table_cell = false;

    let parser = Parser::new_ext(&source, parser_options(options));
    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Paragraph) => {
                if matches!(
                    current.as_ref().map(|block| &block.kind),
                    Some(TextBlockKind::ListItem { .. })
                ) {
                    continue;
                }
                let kind = if table_depth > 0 {
                    TextBlockKind::Table
                } else if let Some(kind) = quote_kind {
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
            Event::Start(Tag::Table(_)) => table_depth += 1,
            Event::End(TagEnd::Table) => {
                table_depth = table_depth.saturating_sub(1);
                if let Some(block) = current.take() {
                    root_starts.push(block.source_start);
                    blocks.push(ParsedBlock::Text(block.finish(range.end)));
                }
            }
            Event::Start(Tag::TableRow) => {
                current.get_or_insert_with(|| {
                    TextBlockBuilder::new(TextBlockKind::Table, range.start)
                });
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(block) = current.as_mut() {
                    block.push("\n", range.end..range.end);
                }
            }
            Event::Start(Tag::TableCell) => {
                if in_table_cell && let Some(block) = current.as_mut() {
                    block.push("   ", range.start..range.start);
                }
                in_table_cell = true;
            }
            Event::End(TagEnd::TableCell) => in_table_cell = false,
            Event::End(TagEnd::TableHead) => {}
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
                current = Some(TextBlockBuilder::new(TextBlockKind::Html, range.start));
            }
            Event::End(TagEnd::HtmlBlock) if options.parse_html => {
                if let Some(block) = current.take() {
                    root_starts.push(block.source_start);
                    blocks.push(ParsedBlock::Text(block.finish(range.end)));
                }
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
                } else {
                    block.push(text.to_string(), range);
                }
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

    let mut container = div().w_full().min_w_0().mb_3();
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
        TextBlockKind::Table => {
            let mut table = container.p_2().border_1().border_color(style.rule_color);
            if style.table_columns_min_size {
                table.style().overflow.x = Some(gpui::Overflow::Scroll);
                table.whitespace_nowrap()
            } else {
                table.whitespace_normal()
            }
        }
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
    markdown: &Entity<Markdown>,
) -> (AnyElement, RenderedLine) {
    let mut code_style = style.base_text_style.clone();
    code_style.refine(&TextStyleRefinement {
        font_family: style.inline_code.font_family.clone(),
        font_size: style.inline_code.font_size,
        ..Default::default()
    });
    let text = block.code.clone();
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
                            rendered: 0..block.code.len(),
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
                .p_2()
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
            if style.code_block_overflow_x_scroll && !wrapped {
                container = container.overflow_x_scroll();
            }
            container = if wrapped {
                container.whitespace_normal()
            } else {
                container.whitespace_nowrap()
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
            container
                .child(actions)
                .child(styled_text)
                .into_any_element()
        }
    };
    (
        element,
        RenderedLine {
            layout: layout.into(),
            text,
            source: block.source.clone(),
            mappings: vec![SegmentMapping {
                rendered: 0..block.code.len(),
                source: block.source.clone(),
            }]
            .into(),
            links: Vec::new(),
        },
    )
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
                            &self.markdown,
                        );
                        rendered.lines.push(line);
                        continue;
                    }
                    let (element, line) = render_code_block(
                        block,
                        &self.style,
                        &self.code_block_renderer,
                        wrapped_code_blocks.contains(&block.source.start),
                        &self.markdown,
                    );
                    root = root.child(element);
                    rendered.lines.push(line);
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
}
