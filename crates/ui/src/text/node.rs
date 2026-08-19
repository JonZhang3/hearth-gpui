// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Migrated text-node rendering to semantic typography metrics.
use std::{
    collections::HashMap,
    ops::Range,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use gpui::{
    AnyElement, App, AppContext as _, DefiniteLength, Div, ElementId, FontStyle, FontWeight, Half,
    HighlightStyle, Hsla, InteractiveElement as _, IntoElement, Length, ObjectFit, Overflow,
    ParentElement, RenderImage, Role, ScrollHandle, SharedString, SharedUri,
    StatefulInteractiveElement, Styled, StyledImage as _, WhiteSpace, Window, div, img,
    prelude::FluentBuilder as _, px, relative, rems,
};
use pulldown_cmark::{Alignment, BlockQuoteKind};
use ropey::Rope;

use crate::{
    ActiveTheme as _, Icon, IconName, StyledExt, h_flex,
    highlighter::{HighlightTheme, LanguageRegistry, SyntaxHighlighter},
    input::{InputEdit, RopeExt as _},
    scroll::horizontal_scroll_area,
    text::{
        CodeBlockActionsFn, LegacyMarkdownStyle, MarkdownBlockKind, MarkdownBlockRenderContext,
        MarkdownBlockRenderers, MarkdownElementKind, MarkdownExtensions, MarkdownHeadingLevel,
        MarkdownInlineKind, MarkdownLinkHandler, MarkdownNode, MarkdownOptions,
        MarkdownStyleProfile, MarkdownTextStyle,
        document::NodeRenderOptions,
        inline::{Inline, InlineLink, InlineState},
        inline_flow::InlineBoxStyle,
        inline_flow::{InlineFlow, InlineFlowItem, InlineFlowLayoutCache, InlineImageSizing},
    },
    tooltip::Tooltip,
    v_flex,
};

use super::{TextViewStyle, utils::list_item_prefix};

/// The block-level nodes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BlockNode {
    /// Something like a Div container in HTML.
    Root {
        children: Vec<BlockNode>,
        span: Option<Span>,
    },
    Paragraph(Paragraph),
    Heading {
        level: u8,
        children: Paragraph,
        span: Option<Span>,
    },
    Blockquote {
        children: Vec<BlockNode>,
        kind: Option<BlockQuoteKind>,
        span: Option<Span>,
    },
    List {
        /// Only contains ListItem, others will be ignored
        children: Vec<BlockNode>,
        ordered: bool,
        start: Option<u64>,
        span: Option<Span>,
    },
    ListItem {
        children: Vec<BlockNode>,
        spread: bool,
        /// Whether the list item is checked, if None, it's not a checkbox
        checked: Option<bool>,
        span: Option<Span>,
    },
    CodeBlock(CodeBlock),
    /// A custom Markdown node produced by [`MarkdownExtensions`].
    Custom(MarkdownNode),
    Table(Table),
    Break {
        html: bool,
        span: Option<Span>,
    },
    HorizontalRule {
        span: Option<Span>,
    },
    /// Use for to_markdown get raw definition
    #[allow(dead_code)]
    Definition {
        identifier: SharedString,
        url: SharedString,
        title: Option<SharedString>,
        span: Option<Span>,
    },
    Unknown,
}

#[derive(Clone, Copy)]
enum BlockTextKind {
    All,
    Selected,
}

impl BlockNode {
    pub(crate) fn is_code_block(&self) -> bool {
        matches!(self, Self::CodeBlock(_))
    }

    /// Clamp display-only synthetic spans to the canonical source boundary.
    pub(crate) fn clamp_spans(&mut self, source_len: usize) {
        fn clamp(span: &mut Option<Span>, source_len: usize) {
            if let Some(span) = span {
                span.start = span.start.min(source_len);
                span.end = span.end.min(source_len).max(span.start);
            }
        }

        match self {
            Self::Root { children, span }
            | Self::Blockquote { children, span, .. }
            | Self::List { children, span, .. }
            | Self::ListItem { children, span, .. } => {
                clamp(span, source_len);
                for child in children {
                    child.clamp_spans(source_len);
                }
            }
            Self::Paragraph(paragraph) => clamp(&mut paragraph.span, source_len),
            Self::Heading { children, span, .. } => {
                clamp(span, source_len);
                clamp(&mut children.span, source_len);
            }
            Self::CodeBlock(code) => clamp(&mut code.span, source_len),
            Self::Custom(node) => {
                let mut span = node.span;
                clamp(&mut span, source_len);
                node.set_span(span);
            }
            Self::Table(table) => {
                clamp(&mut table.span, source_len);
                for row in &mut table.children {
                    for cell in &mut row.children {
                        clamp(&mut cell.children.span, source_len);
                    }
                }
            }
            Self::Break { span, .. }
            | Self::HorizontalRule { span }
            | Self::Definition { span, .. } => clamp(span, source_len),
            Self::Unknown => {}
        }
    }

    /// Remove one synthetic trailing character from the final textual leaf.
    ///
    /// Streaming display repairs use this after parsing so a character that
    /// affected Markdown classification never enters rendering or selection.
    pub(crate) fn remove_trailing_synthetic_char(&mut self, expected: char) -> bool {
        match self {
            Self::Root { children, .. }
            | Self::Blockquote { children, .. }
            | Self::List { children, .. }
            | Self::ListItem { children, .. } => children
                .last_mut()
                .is_some_and(|child| child.remove_trailing_synthetic_char(expected)),
            Self::Paragraph(paragraph)
            | Self::Heading {
                children: paragraph,
                ..
            } => paragraph.remove_trailing_synthetic_char(expected),
            Self::Table(table) => table.children.last_mut().is_some_and(|row| {
                row.children
                    .last_mut()
                    .is_some_and(|cell| cell.children.remove_trailing_synthetic_char(expected))
            }),
            Self::CodeBlock(_)
            | Self::Custom(_)
            | Self::Break { .. }
            | Self::HorizontalRule { .. }
            | Self::Definition { .. }
            | Self::Unknown => false,
        }
    }

    /// Reuse append-only runtime state that is not represented in the AST.
    pub(crate) fn reuse_runtime_state_from(&mut self, previous: &Self) {
        match (self, previous) {
            (Self::Paragraph(current), Self::Paragraph(previous)) => {
                current.reuse_runtime_state_from(previous);
            }
            (
                Self::Heading {
                    level: current_level,
                    children: current,
                    ..
                },
                Self::Heading {
                    level: previous_level,
                    children: previous,
                    ..
                },
            ) if current_level == previous_level => {
                current.reuse_runtime_state_from(previous);
            }
            (
                Self::Root {
                    children: current, ..
                }
                | Self::Blockquote {
                    children: current, ..
                },
                Self::Root {
                    children: previous, ..
                }
                | Self::Blockquote {
                    children: previous, ..
                },
            ) => reuse_child_runtime_state(current, previous),
            (
                Self::List {
                    children: current,
                    ordered: current_ordered,
                    ..
                },
                Self::List {
                    children: previous,
                    ordered: previous_ordered,
                    ..
                },
            ) if current_ordered == previous_ordered => {
                reuse_child_runtime_state(current, previous);
            }
            (
                Self::ListItem {
                    children: current, ..
                },
                Self::ListItem {
                    children: previous, ..
                },
            ) => reuse_child_runtime_state(current, previous),
            (Self::Table(current), Self::Table(previous)) => {
                for (current_row, previous_row) in
                    current.children.iter_mut().zip(&previous.children)
                {
                    for (current_cell, previous_cell) in
                        current_row.children.iter_mut().zip(&previous_row.children)
                    {
                        current_cell
                            .children
                            .reuse_runtime_state_from(&previous_cell.children);
                    }
                }
            }
            (Self::CodeBlock(current), Self::CodeBlock(previous))
                if current.lang == previous.lang
                    && current.code().starts_with(previous.code().as_ref()) =>
            {
                current.styles = previous.styles.clone();
            }
            _ => {}
        }
    }

    pub(super) fn is_list_item(&self) -> bool {
        matches!(self, Self::ListItem { .. })
    }

    /// Combine all children, omitting the empt parent nodes.
    pub(super) fn compact(self) -> BlockNode {
        match self {
            Self::Root { mut children, .. } if children.len() == 1 => children.remove(0).compact(),
            _ => self,
        }
    }

    /// Get the span of the node.
    pub(crate) fn span(&self) -> Option<Span> {
        match self {
            BlockNode::Root { span, .. } => *span,
            BlockNode::Paragraph(paragraph) => paragraph.span,
            BlockNode::Heading { span, .. } => *span,
            BlockNode::Blockquote { span, .. } => *span,
            BlockNode::List { span, .. } => *span,
            BlockNode::ListItem { span, .. } => *span,
            BlockNode::CodeBlock(code_block) => code_block.span,
            BlockNode::Custom(el) => el.span,
            BlockNode::Table(table) => table.span,
            BlockNode::Break { span, .. } => *span,
            BlockNode::HorizontalRule { span, .. } => *span,
            BlockNode::Definition { span, .. } => *span,
            BlockNode::Unknown { .. } => None,
        }
    }

    pub(super) fn text(&self) -> String {
        self.text_by_kind(BlockTextKind::All)
    }

    pub(super) fn selected_text(&self) -> String {
        self.text_by_kind(BlockTextKind::Selected)
    }

    pub(super) fn selected_source_ranges(&self, ranges: &mut Vec<Range<usize>>) {
        match self {
            Self::Root { children, .. }
            | Self::Blockquote { children, .. }
            | Self::List { children, .. }
            | Self::ListItem { children, .. } => {
                for child in children {
                    child.selected_source_ranges(ranges);
                }
            }
            Self::Paragraph(paragraph)
            | Self::Heading {
                children: paragraph,
                ..
            } => {
                paragraph.selected_source_ranges(ranges);
            }
            Self::Table(table) => {
                for row in &table.children {
                    for cell in &row.children {
                        cell.children.selected_source_ranges(ranges);
                    }
                }
            }
            Self::CodeBlock(code) => {
                if code.selected_text().is_empty() {
                    return;
                }
                if let Some(span) = code.span {
                    ranges.push(span.start..span.end);
                }
            }
            Self::Custom(_)
            | Self::Break { .. }
            | Self::HorizontalRule { .. }
            | Self::Definition { .. }
            | Self::Unknown => {}
        }
    }

    fn text_by_kind(&self, kind: BlockTextKind) -> String {
        let mut text = String::new();
        match self {
            BlockNode::Root { children, .. } => {
                let block_text = Self::children_text(children, kind);
                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::Paragraph(paragraph) => {
                let block_text = match kind {
                    BlockTextKind::All => paragraph.text(),
                    BlockTextKind::Selected => paragraph.selected_text(),
                };
                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::Heading { children, .. } => {
                let block_text = match kind {
                    BlockTextKind::All => children.text(),
                    BlockTextKind::Selected => children.selected_text(),
                };
                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::List { children, .. } | BlockNode::ListItem { children, .. } => {
                text.push_str(&Self::children_text(children, kind));
            }
            BlockNode::Blockquote { children, .. } => {
                let block_text = Self::children_text(children, kind);

                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::Table(table) => {
                let mut block_text = String::new();
                for row in table.children.iter() {
                    let mut row_texts = vec![];
                    for cell in row.children.iter() {
                        row_texts.push(match kind {
                            BlockTextKind::All => cell.children.text(),
                            BlockTextKind::Selected => cell.children.selected_text(),
                        });
                    }
                    if !row_texts.is_empty() {
                        block_text.push_str(&row_texts.join(" "));
                        block_text.push('\n');
                    }
                }

                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::CodeBlock(code_block) => {
                let block_text = match kind {
                    BlockTextKind::All => code_block.text(),
                    BlockTextKind::Selected => code_block.selected_text(),
                };
                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::Custom(node) => {
                if let BlockTextKind::All = kind {
                    let content = node.as_text();
                    if !content.is_empty() {
                        text.push_str(content);
                        text.push('\n');
                    }
                }
            }
            BlockNode::Definition { .. }
            | BlockNode::Break { .. }
            | BlockNode::HorizontalRule { .. }
            | BlockNode::Unknown { .. } => {}
        }

        text
    }

    fn children_text(children: &[BlockNode], kind: BlockTextKind) -> String {
        let mut text = String::new();
        for child in children.iter() {
            text.push_str(&child.text_by_kind(kind));
        }

        text
    }

    /// Synchronously clear the selection stored in every inline state.
    ///
    /// Mirrors the [`selected_text`](Self::selected_text) traversal so the
    /// selection can be cleared without relying on a repaint.
    pub(super) fn clear_selection(&self) {
        match self {
            BlockNode::Root { children, .. }
            | BlockNode::Blockquote { children, .. }
            | BlockNode::List { children, .. }
            | BlockNode::ListItem { children, .. } => {
                for child in children.iter() {
                    child.clear_selection();
                }
            }
            BlockNode::Paragraph(paragraph) => paragraph.clear_selection(),
            BlockNode::Heading { children, .. } => children.clear_selection(),
            BlockNode::Table(table) => {
                for row in table.children.iter() {
                    for cell in row.children.iter() {
                        cell.children.clear_selection();
                    }
                }
            }
            BlockNode::CodeBlock(code_block) => code_block.clear_selection(),
            BlockNode::Custom { .. }
            | BlockNode::Definition { .. }
            | BlockNode::Break { .. }
            | BlockNode::HorizontalRule { .. }
            | BlockNode::Unknown { .. } => {}
        }
    }
}

#[allow(unused)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LinkMark {
    pub url: SharedString,
    /// Optional identifier for footnotes.
    pub identifier: Option<SharedString>,
    pub title: Option<SharedString>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TextMark {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub underline: bool,
    pub code: bool,
    pub footnote_reference: bool,
    /// Highlight (`<mark>`) the text with this background color.
    ///
    /// `None` means the text is not highlighted.
    pub highlight: Option<Hsla>,
    pub link: Option<LinkMark>,
}

impl TextMark {
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    pub fn code(mut self) -> Self {
        self.code = true;
        self
    }

    /// Mark the text as highlighted (`<mark>`) with the given background color.
    pub fn highlight(mut self, color: Hsla) -> Self {
        self.highlight = Some(color);
        self
    }

    pub fn link(mut self, link: impl Into<LinkMark>) -> Self {
        self.link = Some(link.into());
        self
    }

    pub fn merge(&mut self, other: TextMark) {
        self.bold |= other.bold;
        self.italic |= other.italic;
        self.strikethrough |= other.strikethrough;
        self.underline |= other.underline;
        self.code |= other.code;
        self.footnote_reference |= other.footnote_reference;
        if other.highlight.is_some() {
            self.highlight = other.highlight;
        }
        if let Some(link) = other.link {
            self.link = Some(link);
        }
    }
}

/// The bytes
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl From<Span> for ElementId {
    fn from(value: Span) -> Self {
        ElementId::Name(format!("md-{}:{}", value.start, value.end).into())
    }
}

#[allow(unused)]
#[derive(Debug, Default, Clone)]
pub struct ImageNode {
    pub url: SharedUri,
    pub link: Option<LinkMark>,
    pub title: Option<SharedString>,
    pub alt: Option<SharedString>,
    pub width: Option<DefiniteLength>,
    pub height: Option<DefiniteLength>,
}

impl ImageNode {
    pub fn title(&self) -> String {
        self.title
            .clone()
            .unwrap_or_else(|| self.alt.clone().unwrap_or_default())
            .to_string()
    }
}

impl PartialEq for ImageNode {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
            && self.link == other.link
            && self.title == other.title
            && self.alt == other.alt
            && self.width == other.width
            && self.height == other.height
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) struct InlineNode {
    /// The text content.
    pub(crate) text: SharedString,
    pub(crate) image: Option<ImageNode>,
    pub(crate) custom: Option<MarkdownNode>,
    /// The text styles, each tuple contains the range of the text and the style.
    pub(crate) marks: Vec<(Range<usize>, TextMark)>,
    pub(crate) source_range: Option<Range<usize>>,

    state: Arc<Mutex<InlineState>>,
}

impl PartialEq for InlineNode {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.image == other.image
            && self.custom == other.custom
            && self.marks == other.marks
            && self.source_range == other.source_range
    }
}

impl InlineNode {
    pub(crate) fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            image: None,
            custom: None,
            marks: vec![],
            source_range: None,
            state: Arc::new(Mutex::new(InlineState::default())),
        }
    }

    pub(crate) fn image(image: ImageNode) -> Self {
        let mut this = Self::new("");
        this.image = Some(image);
        this
    }

    pub(crate) fn custom(node: MarkdownNode) -> Self {
        let text = node.as_text().to_string();
        let mut this = Self::new(text);
        this.custom = Some(node);
        this
    }

    pub(crate) fn marks(mut self, marks: Vec<(Range<usize>, TextMark)>) -> Self {
        self.marks = marks;
        self
    }

    /// Attach the canonical source range represented by this inline node.
    pub(crate) fn source_range(mut self, range: Range<usize>) -> Self {
        self.source_range = Some(range);
        self
    }
}

/// The paragraph element, contains multiple text nodes.
///
/// Unlike other Element, this is cloneable, because it is used in the Node AST.
/// We are keep the selection state inside this AST Nodes.
#[derive(Debug, Clone, Default)]
pub(crate) struct Paragraph {
    pub(super) span: Option<Span>,
    pub(super) children: Vec<InlineNode>,
    /// The link references in this paragraph, used for reference links.
    ///
    /// The key is the identifier, the value is the url.
    pub(super) link_refs: HashMap<SharedString, SharedString>,

    pub(crate) state: Arc<Mutex<InlineState>>,
    pub(crate) render_cache: Arc<Mutex<Option<CachedParagraphRender>>>,
    pub(crate) inline_flow_layout_cache: InlineFlowLayoutCache,
}

impl PartialEq for Paragraph {
    fn eq(&self, other: &Self) -> bool {
        self.span == other.span
            && self.children == other.children
            && self.link_refs == other.link_refs
    }
}

impl Paragraph {
    /// Preserve paragraph selection, link hover, and compatible inline states.
    fn reuse_runtime_state_from(&mut self, previous: &Self) {
        self.state = previous.state.clone();
        for (current, previous) in self.children.iter_mut().zip(&previous.children) {
            let compatible = current.image == previous.image
                && (current.text.starts_with(previous.text.as_ref())
                    || previous.text.starts_with(current.text.as_ref()));
            if compatible {
                current.state = previous.state.clone();
            }
        }
        if self == previous {
            self.render_cache = previous.render_cache.clone();
            self.inline_flow_layout_cache = previous.inline_flow_layout_cache.clone();
        }
    }

    pub(crate) fn new(text: String) -> Self {
        Self {
            span: None,
            children: vec![InlineNode::new(&text)],
            link_refs: HashMap::new(),
            state: Arc::new(Mutex::new(InlineState::default())),
            render_cache: Arc::new(Mutex::new(None)),
            inline_flow_layout_cache: InlineFlowLayoutCache::default(),
        }
    }

    pub(super) fn selected_text(&self) -> String {
        if let Ok(state) = self.state.lock()
            && let Some(selection) = &state.selection
        {
            return state.text[selection.start..selection.end].to_string();
        }

        let mut text = String::new();

        for c in self.children.iter() {
            let Ok(state) = c.state.lock() else {
                continue;
            };
            if let Some(selection) = &state.selection {
                text.push_str(&state.text[selection.start..selection.end]);
            }
        }

        text
    }

    fn selected_source_ranges(&self, ranges: &mut Vec<Range<usize>>) {
        if let Ok(state) = self.state.lock()
            && let Some(selection) = &state.selection
        {
            let mut rendered_offset = 0;
            for child in &self.children {
                let child_end = rendered_offset + child.text.len();
                let start = selection.start.max(rendered_offset).min(child_end);
                let end = selection.end.max(rendered_offset).min(child_end);
                if start < end {
                    push_mapped_source_range(
                        ranges,
                        child,
                        (start - rendered_offset)..(end - rendered_offset),
                    );
                }
                rendered_offset = child_end;
            }
            return;
        }

        for child in &self.children {
            if let Ok(state) = child.state.lock()
                && let Some(selection) = &state.selection
            {
                push_mapped_source_range(ranges, child, (*selection).into());
            }
        }
    }

    pub(super) fn text(&self) -> String {
        let mut text = String::new();
        for node in self.children.iter() {
            text.push_str(&node.text);
        }
        text
    }

    /// Synchronously clear the selection stored in every inline state.
    ///
    /// Mirrors the [`selected_text`](Self::selected_text) traversal.
    pub(super) fn clear_selection(&self) {
        for c in self.children.iter() {
            if let Ok(mut state) = c.state.lock() {
                state.selection = None;
            }
        }

        if let Ok(mut state) = self.state.lock() {
            state.selection = None;
        }
    }
}

fn push_mapped_source_range(
    ranges: &mut Vec<Range<usize>>,
    node: &InlineNode,
    rendered: Range<usize>,
) {
    let Some(source) = &node.source_range else {
        return;
    };
    if source.end.saturating_sub(source.start) == node.text.len() {
        ranges.push((source.start + rendered.start)..(source.start + rendered.end));
    } else if !rendered.is_empty() {
        ranges.push(source.clone());
    }
}

/// Reuse runtime state for structurally corresponding nested blocks.
fn reuse_child_runtime_state(current: &mut [BlockNode], previous: &[BlockNode]) {
    for (current, previous) in current.iter_mut().zip(previous) {
        current.reuse_runtime_state_from(previous);
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Table {
    pub(crate) children: Vec<TableRow>,
    pub(crate) column_aligns: Vec<ColumnumnAlign>,
    pub(crate) span: Option<Span>,
}

impl Table {
    pub(crate) fn column_align(&self, index: usize) -> ColumnumnAlign {
        self.column_aligns.get(index).copied().unwrap_or_default()
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub(crate) enum ColumnumnAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl From<Alignment> for ColumnumnAlign {
    fn from(value: Alignment) -> Self {
        match value {
            Alignment::None => ColumnumnAlign::Left,
            Alignment::Left => ColumnumnAlign::Left,
            Alignment::Center => ColumnumnAlign::Center,
            Alignment::Right => ColumnumnAlign::Right,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TableRow {
    pub children: Vec<TableCell>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TableCell {
    pub children: Paragraph,
    pub width: Option<DefiniteLength>,
}

impl Paragraph {
    /// Remove one expected suffix from the last textual inline node.
    fn remove_trailing_synthetic_char(&mut self, expected: char) -> bool {
        let Some(index) = self.children.iter().rposition(|node| !node.text.is_empty()) else {
            return false;
        };
        let node = &mut self.children[index];
        let Some(text) = node.text.strip_suffix(expected) else {
            return false;
        };

        let text_len = text.len();
        node.text = text.to_string().into();
        for (range, _) in &mut node.marks {
            range.start = range.start.min(text_len);
            range.end = range.end.min(text_len).max(range.start);
        }
        node.marks.retain(|(range, _)| !range.is_empty());
        if node.text.is_empty() && node.image.is_none() {
            self.children.remove(index);
        }
        if let Ok(mut cache) = self.render_cache.lock() {
            *cache = None;
        }
        true
    }

    pub(crate) fn take(&mut self) -> Paragraph {
        std::mem::replace(
            self,
            Paragraph {
                span: None,
                children: vec![],
                link_refs: Default::default(),
                state: Arc::new(Mutex::new(InlineState::default())),
                render_cache: Arc::new(Mutex::new(None)),
                inline_flow_layout_cache: InlineFlowLayoutCache::default(),
            },
        )
    }

    pub(crate) fn is_image(&self) -> bool {
        false
    }

    pub(crate) fn set_span(&mut self, span: Span) {
        self.span = Some(span);
    }

    pub(crate) fn push_str(&mut self, text: &str) {
        self.children.push(
            InlineNode::new(text.to_string()).marks(vec![(0..text.len(), TextMark::default())]),
        );
    }

    pub(crate) fn push(&mut self, text: InlineNode) {
        self.children.push(text);
    }

    pub(crate) fn push_image(&mut self, image: ImageNode) {
        self.children.push(InlineNode::image(image));
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.children.is_empty()
            || self
                .children
                .iter()
                .all(|node| node.text.is_empty() && node.image.is_none())
    }

    /// Return length of children text.
    pub(crate) fn text_len(&self) -> usize {
        self.children
            .iter()
            .map(|node| node.text.len())
            .sum::<usize>()
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.children.extend(other.children);
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ParagraphRenderKey {
    inline_styles: Vec<(MarkdownInlineKind, MarkdownTextStyle)>,
    link_refs: HashMap<SharedString, LinkMark>,
    accent: Hsla,
    link: Hsla,
}

#[derive(Clone, Debug)]
pub(crate) struct CachedParagraphRender {
    key: ParagraphRenderKey,
    text: SharedString,
    links: Vec<InlineLink>,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
}

#[derive(Clone)]
struct CachedCodeBlockStyles {
    /// The active theme used to compute `styles`.
    highlight_theme: Arc<HighlightTheme>,
    source: SharedString,
    highlighter: Arc<Mutex<SyntaxHighlighter>>,
    styles: Vec<(Range<usize>, HighlightStyle)>,
}

impl std::fmt::Debug for CachedCodeBlockStyles {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CachedCodeBlockStyles")
            .field("highlight_theme", &self.highlight_theme.name)
            .field("source_len", &self.source.len())
            .field("styles", &self.styles)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct CodeBlock {
    lang: Option<SharedString>,
    info: Option<SharedString>,
    source_path: Option<SharedString>,
    styles: Arc<Mutex<Option<CachedCodeBlockStyles>>>,
    state: Arc<Mutex<InlineState>>,
    mermaid: Arc<MermaidRenderCache>,
    pub span: Option<Span>,
}

impl PartialEq for CodeBlock {
    fn eq(&self, other: &Self) -> bool {
        self.lang == other.lang
            && self.code() == other.code()
            && self.span == other.span
            && self.info == other.info
            && self.source_path == other.source_path
    }
}

impl CodeBlock {
    /// Get the language of the code block.
    pub fn lang(&self) -> Option<SharedString> {
        self.lang.clone()
    }

    /// Return the complete trimmed fenced info string.
    pub fn info(&self) -> Option<SharedString> {
        self.info.clone()
    }

    /// Return a fenced source path when the info string identifies a path.
    pub fn source_path(&self) -> Option<SharedString> {
        self.source_path.clone()
    }

    /// Get the code content of the code block.
    pub fn code(&self) -> SharedString {
        self.state
            .lock()
            .map(|state| state.text.clone())
            .unwrap_or_default()
    }

    pub(crate) fn new(
        code: SharedString,
        lang: Option<SharedString>,
        span: Option<impl Into<Span>>,
    ) -> Self {
        let state = Arc::new(Mutex::new(InlineState::default()));
        if let Ok(mut state) = state.lock() {
            state.set_text(code);
        }

        Self {
            info: lang.clone(),
            lang,
            source_path: None,
            styles: Arc::new(Mutex::new(None)),
            state,
            mermaid: Arc::default(),
            span: span.map(|s| s.into()),
        }
    }

    /// Create a fenced code block while retaining its complete info string.
    pub(crate) fn new_fenced(
        code: SharedString,
        info: SharedString,
        span: Option<impl Into<Span>>,
    ) -> Self {
        let trimmed = info.trim();
        let source_path = trimmed.contains('/').then(|| trimmed.to_string().into());
        let lang = source_path.is_none().then(|| {
            trimmed
                .split_ascii_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
                .into()
        });
        let mut block = Self::new(code, lang, span);
        block.info = (!trimmed.is_empty()).then(|| trimmed.to_string().into());
        block.source_path = source_path;
        block
    }

    pub(crate) fn styles(
        &self,
        highlight_theme: &Arc<HighlightTheme>,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let Some(lang) = &self.lang else {
            return Vec::new();
        };

        let Ok(mut styles) = self.styles.lock() else {
            return Vec::new();
        };

        let code = self.code();
        let desired_language = LanguageRegistry::singleton()
            .language(lang)
            .map(|config| config.name)
            .unwrap_or_else(|| SharedString::from("text"));
        // Pointer identity is the common render-path fast check. Equivalent
        // reallocated themes are adopted without reparsing source text.
        if let Some(cached) = styles.as_mut() {
            let language_matches = cached
                .highlighter
                .lock()
                .is_ok_and(|highlighter| highlighter.language() == &desired_language);
            if language_matches
                && cached.source == code
                && Arc::ptr_eq(&cached.highlight_theme, highlight_theme)
            {
                return cached.styles.clone();
            }

            if language_matches
                && cached.source == code
                && cached.highlight_theme.as_ref() == highlight_theme.as_ref()
            {
                cached.highlight_theme = highlight_theme.clone();
                return cached.styles.clone();
            }
        }

        let highlighter = styles
            .as_ref()
            .map(|cached| cached.highlighter.clone())
            .unwrap_or_else(|| Arc::new(Mutex::new(SyntaxHighlighter::new(lang))));
        let code_rope = Rope::from_str(code.as_str());
        let computed_styles = if let Ok(mut highlighter) = highlighter.lock() {
            if let Some(config) = LanguageRegistry::singleton().language(lang)
                && highlighter.language() != &config.name
            {
                *highlighter = SyntaxHighlighter::new(lang);
            }

            let old_text = highlighter.text();
            let old_end_byte = old_text.len();
            let prefix_len = common_prefix_len(old_text, code.as_ref());
            let edit = InputEdit {
                start_byte: prefix_len,
                old_end_byte,
                new_end_byte: code.len(),
                start_position: old_text.offset_to_point(prefix_len),
                old_end_position: old_text.offset_to_point(old_end_byte),
                new_end_position: code_rope.offset_to_point(code.len()),
            };
            highlighter.update(Some(edit), &code_rope, None);
            highlighter.styles(&(0..code.len()), highlight_theme)
        } else {
            Vec::new()
        };
        *styles = Some(CachedCodeBlockStyles {
            highlight_theme: highlight_theme.clone(),
            source: code,
            highlighter,
            styles: computed_styles.clone(),
        });
        computed_styles
    }

    pub(super) fn selected_text(&self) -> String {
        let mut text = String::new();
        if let Ok(state) = self.state.lock()
            && let Some(selection) = &state.selection
        {
            text.push_str(&state.text[selection.start..selection.end]);
        }
        text
    }

    pub(super) fn text(&self) -> String {
        self.state
            .lock()
            .map(|state| state.text.to_string())
            .unwrap_or_default()
    }

    /// Synchronously clear the selection stored in the inline state.
    ///
    /// Mirrors the [`selected_text`](Self::selected_text) traversal.
    pub(super) fn clear_selection(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.selection = None;
        }
    }

    fn render(
        &self,
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        if self.lang.as_deref() == Some("mermaid")
            && node_cx.markdown_options.render_mermaid_diagrams
            && self.is_closed_fenced_block(node_cx)
            && is_supported_mermaid_diagram(&self.code())
        {
            return self.render_mermaid(options, node_cx, window, cx);
        }
        self.render_source(options, node_cx, window, cx)
    }

    /// Return whether the canonical source contains a closing fence.
    fn is_closed_fenced_block(&self, node_cx: &NodeContext) -> bool {
        let Some(span) = self.span else {
            return false;
        };
        let Some(source) = node_cx.source.get(span.start..span.end) else {
            return false;
        };
        source.lines().next_back().is_some_and(|line| {
            line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~")
        })
    }

    fn render_source(
        &self,
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let style = &node_cx.style;

        div()
            .w_full()
            .min_w_0()
            .when(!options.is_last, |this| this.pb(style.paragraph_gap))
            .child(
                div()
                    .id(("codeblock", options.ix))
                    .w_full()
                    .min_w_0()
                    .p_3()
                    .rounded(cx.theme().style.radii.md)
                    .bg(cx.theme().tokens.muted)
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(cx.theme().mono_font_size)
                    .relative()
                    .refine_style(&style.code_block)
                    .when_some(
                        node_cx
                            .markdown_style
                            .element_style(MarkdownElementKind::CodeBlock),
                        |this, style| this.refine_style(style),
                    )
                    .child({
                        let mut styles = self.styles(node_cx.syntax_theme(cx));
                        let mut semantic = HighlightStyle::default();
                        node_cx.refine_inline(MarkdownInlineKind::CodeBlockText, &mut semantic);
                        if semantic != HighlightStyle::default() {
                            styles.push((0..self.code().len(), semantic));
                        }
                        Inline::new("code", self.state.clone(), vec![], styles)
                    })
                    .when_some(node_cx.code_block_actions.clone(), |this, actions| {
                        this.child(
                            div()
                                .id("actions")
                                .absolute()
                                .top_2()
                                .right_2()
                                .bg(cx.theme().tokens.muted)
                                .rounded(cx.theme().style.radii.md)
                                .when_some(
                                    node_cx
                                        .markdown_style
                                        .element_style(MarkdownElementKind::CodeBlockActions),
                                    |this, style| this.refine_style(style),
                                )
                                .child(actions(&self, window, cx)),
                        )
                    }),
            )
            .into_any_element()
    }

    /// Render Mermaid off the UI thread and cache the rasterized SVG for this
    /// source node. The original code remains visible until rendering succeeds.
    fn render_mermaid(
        &self,
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let dark = cx.theme().is_dark();
        let theme_cache = self.mermaid.for_theme(dark);
        if let Some(result) = theme_cache.result.get() {
            return match result {
                Ok(image) => div()
                    .w_full()
                    .min_w_0()
                    .when(!options.is_last, |this| {
                        this.pb(node_cx.style.paragraph_gap)
                    })
                    .child(
                        div().w_full().min_w_0().overflow_hidden().child(
                            img(image.clone())
                                .object_fit(ObjectFit::Contain)
                                .max_w_full(),
                        ),
                    )
                    .into_any_element(),
                Err(error) => div()
                    .w_full()
                    .min_w_0()
                    .child(self.render_source(options, node_cx, window, cx))
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(cx.theme().danger)
                            .child(error.to_string()),
                    )
                    .into_any_element(),
            };
        }

        if !theme_cache.started.swap(true, Ordering::AcqRel) {
            let source = self.code().to_string();
            let cache = theme_cache;
            let svg_renderer = cx.svg_renderer();
            cx.spawn(async move |cx| {
                let result = cx
                    .background_spawn(async move {
                        let svg = render_mermaid_svg(&source, dark)?;
                        svg_renderer
                            .render_single_frame(svg.as_bytes(), 1.0)
                            .map_err(|error| error.to_string())
                    })
                    .await
                    .map_err(SharedString::from);
                _ = cache.result.set(result);
                _ = cx.update(|cx| cx.refresh_windows());
            })
            .detach();
        }

        self.render_source(options, node_cx, window, cx)
    }
}

/// Convert Mermaid source into an SVG accepted by GPUI's `resvg` renderer.
fn render_mermaid_svg(source: &str, dark: bool) -> Result<String, String> {
    const MAX_CACHED_DIAGRAMS: usize = 64;
    static CACHE: OnceLock<Mutex<HashMap<(String, bool), Result<String, String>>>> =
        OnceLock::new();

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (source.to_string(), dark);
    if let Ok(cache) = cache.lock()
        && let Some(result) = cache.get(&key)
    {
        return result.clone();
    }

    let profile = merman::render::HostThemeProfile::from_preset(if dark {
        merman::render::HostThemePreset::EditorDark
    } else {
        merman::render::HostThemePreset::EditorLight
    });
    let result = merman::render::HeadlessRenderer::new()
        .with_vendored_text_measurer()
        .with_host_theme(&profile)
        .render_svg_sync(source)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Mermaid source did not contain a diagram".to_string());

    if let Ok(mut cache) = cache.lock() {
        if cache.len() >= MAX_CACHED_DIAGRAMS {
            cache.clear();
        }
        cache.insert(key, result.clone());
    }
    result
}

/// Restrict Mermaid rendering to the diagram types verified by Zed's viewer.
fn is_supported_mermaid_diagram(source: &str) -> bool {
    const SUPPORTED_PREFIXES: &[&str] = &[
        "flowchart",
        "graph",
        "sequenceDiagram",
        "classDiagram",
        "stateDiagram",
        "stateDiagram-v2",
        "erDiagram",
        "gantt",
        "pie",
        "gitGraph",
        "mindmap",
        "timeline",
        "quadrantChart",
        "xychart-beta",
        "journey",
    ];
    let first_token = source
        .trim_start()
        .split(char::is_whitespace)
        .next()
        .unwrap_or_default();
    SUPPORTED_PREFIXES
        .iter()
        .any(|prefix| first_token.eq_ignore_ascii_case(prefix))
}

#[derive(Default)]
struct MermaidRenderCache {
    themes: Mutex<HashMap<bool, Arc<MermaidThemeRenderCache>>>,
}

impl MermaidRenderCache {
    /// Return independent light and dark raster state so theme changes never reuse stale colors.
    fn for_theme(&self, dark: bool) -> Arc<MermaidThemeRenderCache> {
        self.themes
            .lock()
            .map(|mut themes| themes.entry(dark).or_default().clone())
            .unwrap_or_default()
    }
}

#[derive(Default)]
struct MermaidThemeRenderCache {
    started: AtomicBool,
    result: OnceLock<Result<Arc<RenderImage>, SharedString>>,
}

impl std::fmt::Debug for MermaidRenderCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MermaidRenderCache")
            .field(
                "theme_count",
                &self.themes.lock().map(|themes| themes.len()),
            )
            .finish()
    }
}

/// Return the longest shared UTF-8 boundary between cached and current code.
fn common_prefix_len(previous: &Rope, current: &str) -> usize {
    let previous = previous.to_string();
    let mut prefix = previous
        .as_bytes()
        .iter()
        .zip(current.as_bytes())
        .take_while(|(left, right)| left == right)
        .count();
    while prefix > 0 && (!previous.is_char_boundary(prefix) || !current.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    prefix
}

/// A context for rendering nodes, contains link references.
#[derive(Default, Clone)]
pub(crate) struct NodeContext {
    /// The byte offset of the node in the original markdown text.
    /// Used for incremental updates.
    pub(crate) offset: usize,
    pub(crate) link_refs: HashMap<SharedString, LinkMark>,
    pub(crate) style: TextViewStyle,
    pub(crate) code_block_actions: Option<Arc<CodeBlockActionsFn>>,
    pub(crate) markdown_extensions: Arc<MarkdownExtensions>,
    pub(crate) markdown_options: MarkdownOptions,
    pub(crate) markdown_style_profile: MarkdownStyleProfile,
    pub(crate) markdown_style: Arc<LegacyMarkdownStyle>,
    pub(crate) markdown_block_renderers: Arc<MarkdownBlockRenderers>,
    pub(crate) source: SharedString,
    pub(crate) search_matches: Arc<Vec<Range<usize>>>,
    pub(crate) active_search_match: Option<usize>,
    pub(crate) link_handler: MarkdownLinkHandler,
}

impl NodeContext {
    #[allow(dead_code)]
    pub(super) fn add_ref(&mut self, identifier: SharedString, link: LinkMark) {
        self.link_refs.insert(identifier, link);
    }

    fn refine_inline(&self, kind: MarkdownInlineKind, style: &mut HighlightStyle) {
        if let Some(refinement) = self.markdown_style.inline_style(kind) {
            refinement.refine(style);
        }
    }

    fn link_hover_style(&self) -> Option<Arc<MarkdownTextStyle>> {
        Some(Arc::new(
            self.markdown_style
                .inline_style(MarkdownInlineKind::LinkHover)?
                .clone(),
        ))
    }

    /// Resolve layout and paint properties for one atomic inline semantic box.
    fn inline_box_style(&self, kind: MarkdownInlineKind, accent: Hsla) -> Option<InlineBoxStyle> {
        let refinement = self.markdown_style.inline_style(kind)?;
        let metrics = refinement.inline_box_metrics()?;
        let mut style = HighlightStyle {
            background_color: (kind == MarkdownInlineKind::InlineCode).then_some(accent),
            ..Default::default()
        };
        refinement.refine(&mut style);
        Some(InlineBoxStyle {
            background: style.background_color,
            padding_x: metrics.padding_x,
            padding_y: metrics.padding_y,
            margin_x: metrics.margin_x,
            margin_y: metrics.margin_y,
            corner_radius: metrics.corner_radius,
            border_width: metrics.border_width,
            border_color: metrics.border_color,
            font_family: metrics.font_family,
            font_size: metrics.font_size,
            line_height: metrics.line_height,
        })
    }

    fn syntax_theme<'a>(&'a self, cx: &'a App) -> &'a Arc<HighlightTheme> {
        if let Some(theme) = self.markdown_style.syntax_theme_ref() {
            return theme;
        }
        // `TextViewStyle::default` clones this exact static Arc. Treat that
        // identity as "inherit ActiveTheme" so existing dark-mode behavior is
        // unchanged, while a caller-provided Arc becomes a local override.
        if !Arc::ptr_eq(
            &self.style.highlight_theme,
            &HighlightTheme::default_light(),
        ) {
            return &self.style.highlight_theme;
        }
        &cx.theme().highlight_theme
    }
}

/// Append deterministic, non-overlapping highlights and links for one inline text node.
///
/// Semantic refinements are resolved before converting the result into GPUI highlight ranges.
/// This preserves selector precedence and explicit removals such as `no_underline()` without
/// relying on the unordered overlap behavior of `gpui::combine_highlights`.
fn append_resolved_inline_styles(
    inline_node: &InlineNode,
    offset: usize,
    node_cx: &NodeContext,
    accent: Hsla,
    link_color: Hsla,
    links: &mut Vec<InlineLink>,
    highlights: &mut Vec<(Range<usize>, HighlightStyle)>,
) {
    let text_len = inline_node.text.len();
    if text_len == 0 {
        return;
    }

    for (range, mark) in &inline_node.marks {
        let Some(mut link_mark) = mark.link.clone() else {
            continue;
        };
        if let Some(identifier) = link_mark.identifier.as_ref()
            && let Some(resolved) = node_cx.link_refs.get(identifier)
        {
            link_mark = resolved.clone();
        }
        links.push(InlineLink {
            id: links.len(),
            range: (offset + range.start)..(offset + range.end),
            mark: link_mark,
        });
    }

    // Mark boundaries define the smallest ranges on which the active semantic set is stable.
    let mut boundaries = vec![0, text_len];
    for (range, _) in &inline_node.marks {
        boundaries.push(range.start.min(text_len));
        boundaries.push(range.end.min(text_len));
    }
    let search_ranges = mapped_search_ranges(inline_node, node_cx);
    for (range, _) in &search_ranges {
        boundaries.push(range.start);
        boundaries.push(range.end);
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    for segment in boundaries.windows(2) {
        let local_range = segment[0]..segment[1];
        if local_range.is_empty() {
            continue;
        }

        let mut mark = TextMark::default();
        for (range, active_mark) in &inline_node.marks {
            if range.start <= local_range.start && range.end >= local_range.end {
                mark.merge(active_mark.clone());
            }
        }

        let mut style = HighlightStyle::default();
        node_cx.refine_inline(MarkdownInlineKind::Plain, &mut style);
        if mark.bold {
            style.font_weight = Some(FontWeight::BOLD);
            node_cx.refine_inline(MarkdownInlineKind::Strong, &mut style);
        }
        if mark.italic {
            style.font_style = Some(FontStyle::Italic);
            node_cx.refine_inline(MarkdownInlineKind::Emphasis, &mut style);
        }
        if mark.strikethrough {
            style.strikethrough = Some(gpui::StrikethroughStyle {
                thickness: gpui::px(1.),
                ..Default::default()
            });
            node_cx.refine_inline(MarkdownInlineKind::Strikethrough, &mut style);
        }
        if mark.underline {
            style.underline = Some(gpui::UnderlineStyle {
                thickness: gpui::px(1.),
                ..Default::default()
            });
            node_cx.refine_inline(MarkdownInlineKind::Underline, &mut style);
        }
        if mark.code {
            style.background_color = Some(accent);
            node_cx.refine_inline(MarkdownInlineKind::InlineCode, &mut style);
        }
        if mark.footnote_reference {
            node_cx.refine_inline(MarkdownInlineKind::FootnoteReference, &mut style);
        }
        if let Some(color) = mark.highlight {
            style.background_color = Some(color);
            node_cx.refine_inline(MarkdownInlineKind::Mark, &mut style);
        }
        if mark.link.is_some() {
            style.color = Some(link_color);
            style.underline = Some(gpui::UnderlineStyle {
                thickness: gpui::px(1.),
                ..Default::default()
            });
            node_cx.refine_inline(MarkdownInlineKind::Link, &mut style);
        }
        if let Some((_, active)) = search_ranges
            .iter()
            .find(|(range, _)| range.start <= local_range.start && range.end >= local_range.end)
        {
            style.background_color = Some(accent.opacity(if *active { 0.55 } else { 0.3 }));
        }

        if style == HighlightStyle::default() {
            continue;
        }

        let resolved_range = (offset + local_range.start)..(offset + local_range.end);
        if let Some((last_range, last_style)) = highlights.last_mut()
            && last_range.end == resolved_range.start
            && *last_style == style
        {
            last_range.end = resolved_range.end;
        } else {
            highlights.push((resolved_range, style));
        }
    }
}

/// Map canonical-source search ranges into one rendered inline text node.
fn mapped_search_ranges(
    inline_node: &InlineNode,
    node_cx: &NodeContext,
) -> Vec<(Range<usize>, bool)> {
    let Some(source) = &inline_node.source_range else {
        return Vec::new();
    };
    let source_len = source.end.saturating_sub(source.start);
    node_cx
        .search_matches
        .iter()
        .enumerate()
        .filter_map(|(index, matched)| {
            let start = source.start.max(matched.start);
            let end = source.end.min(matched.end);
            if start >= end {
                return None;
            }
            let range = if source_len == inline_node.text.len() {
                (start - source.start)..(end - source.start)
            } else {
                0..inline_node.text.len()
            };
            Some((range, node_cx.active_search_match == Some(index)))
        })
        .collect()
}

impl PartialEq for NodeContext {
    fn eq(&self, other: &Self) -> bool {
        self.link_refs == other.link_refs
            && self.style == other.style
            && self.search_matches == other.search_matches
            && self.active_search_match == other.active_search_match
        // Note: code_block_actions and markdown_extensions are intentionally
        // not compared (closures can't be compared)
    }
}

impl Paragraph {
    fn render(&self, node_cx: &NodeContext, window: &mut Window, cx: &mut App) -> AnyElement {
        let span = self.span;
        let children = &self.children;

        if self.should_render_inline_flow(node_cx, cx) {
            if let Ok(mut state) = self.state.lock() {
                state.set_text(self.display_text(node_cx).into());
            }
            return InlineFlow::new(
                span.unwrap_or_default(),
                self.inline_flow_items(node_cx, window, cx),
            )
            .selection_state(self.state.clone())
            .link_handler(node_cx.link_handler.clone())
            .layout_cache(self.inline_flow_layout_cache.clone())
            .semantic_styles(node_cx.markdown_style.inline_snapshot())
            .into_any_element();
        }

        if !children.iter().any(|child| child.image.is_some()) {
            let key = ParagraphRenderKey {
                inline_styles: node_cx.markdown_style.inline_snapshot(),
                link_refs: node_cx.link_refs.clone(),
                accent: cx.theme().accent,
                link: cx.theme().link,
            };
            let cached = self.cached_render(key, node_cx);
            if let Ok(mut state) = self.state.lock() {
                state.set_text(cached.text.clone());
            }
            let paragraph_id: ElementId = span.unwrap_or_default().into();
            let inline = Inline::new(
                children.len(),
                self.state.clone(),
                cached.links.clone(),
                cached.highlights.clone(),
            )
            .link_hover_style(node_cx.link_hover_style())
            .link_handler(node_cx.link_handler.clone());
            let paragraph = div().id(paragraph_id.clone()).child(inline);
            let keyboard_link = cached
                .links
                .first()
                .filter(|link| node_cx.link_handler.policy.allows_link(&link.mark.url))
                .cloned();
            return paragraph
                .when_some(keyboard_link, |paragraph, link| {
                    let focus_handle = window
                        .use_keyed_state(paragraph_id, cx, |_, cx| cx.focus_handle())
                        .read(cx)
                        .clone();
                    let handler = node_cx.link_handler.clone();
                    let url = link.mark.url.clone();
                    paragraph
                        .role(Role::Link)
                        .aria_label(cached.text.clone())
                        .track_focus(&focus_handle.tab_stop(true))
                        .on_key_down(move |event, window, cx| {
                            if !event.keystroke.modifiers.modified()
                                && event.keystroke.key == "enter"
                            {
                                window.prevent_default();
                                handler.activate(&url, window, cx);
                            }
                        })
                })
                .into_any_element();
        }

        let mut child_nodes: Vec<AnyElement> = vec![];

        let mut text = String::new();
        let mut highlights: Vec<(Range<usize>, HighlightStyle)> = vec![];
        let mut links: Vec<InlineLink> = vec![];
        let mut offset = 0;

        let mut ix = 0;
        for inline_node in children {
            let text_len = inline_node.text.len();
            text.push_str(&inline_node.text);

            if let Some(image) = &inline_node.image {
                if text.len() > 0 {
                    if let Ok(mut state) = inline_node.state.lock() {
                        state.set_text(text.clone().into());
                    }
                    child_nodes.push(
                        Inline::new(
                            ix,
                            inline_node.state.clone(),
                            links.clone(),
                            highlights.clone(),
                        )
                        .link_hover_style(node_cx.link_hover_style())
                        .link_handler(node_cx.link_handler.clone())
                        .into_any_element(),
                    );
                }
                if !node_cx.link_handler.policy.allows_image(&image.url) {
                    child_nodes.push(
                        div()
                            .child(image.alt.clone().unwrap_or_default())
                            .into_any_element(),
                    );
                } else {
                    let link_handler = node_cx.link_handler.clone();
                    child_nodes.push(
                        img(image.url.clone())
                            .id(ix)
                            .object_fit(ObjectFit::Contain)
                            .max_w(relative(1.))
                            .when_some(
                                node_cx
                                    .markdown_style
                                    .element_style(MarkdownElementKind::Image),
                                |this, style| this.refine_style(style),
                            )
                            .when_some(image.width, |this, width| this.w(width))
                            .when_some(
                                image
                                    .link
                                    .clone()
                                    .filter(|link| link_handler.policy.allows_link(&link.url)),
                                |this, link| {
                                    let title = image.title();
                                    let link_handler = link_handler.clone();
                                    this.cursor_pointer()
                                        .tooltip(move |window, cx| {
                                            Tooltip::new(title.clone()).build(window, cx)
                                        })
                                        .on_click(move |_, window, cx| {
                                            link_handler.activate(&link.url, window, cx);
                                        })
                                },
                            )
                            .into_any_element(),
                    );
                }

                text.clear();
                links.clear();
                highlights.clear();
                offset = 0;
            } else {
                append_resolved_inline_styles(
                    inline_node,
                    offset,
                    node_cx,
                    cx.theme().accent,
                    cx.theme().link,
                    &mut links,
                    &mut highlights,
                );
                offset += text_len;
            }
            ix += 1;
        }

        // Add the last text node
        if text.len() > 0 {
            if let Ok(mut state) = self.state.lock() {
                state.set_text(text.into());
            }
            child_nodes.push(
                Inline::new(ix, self.state.clone(), links, highlights)
                    .link_hover_style(node_cx.link_hover_style())
                    .link_handler(node_cx.link_handler.clone())
                    .into_any_element(),
            );
        }

        div()
            .id(span.unwrap_or_default())
            .children(child_nodes)
            .into_any_element()
    }

    /// Reuse flattened text and semantic ranges while the paragraph and inline style inputs match.
    fn cached_render(
        &self,
        key: ParagraphRenderKey,
        node_cx: &NodeContext,
    ) -> CachedParagraphRender {
        if let Ok(cache) = self.render_cache.lock()
            && let Some(cached) = cache.as_ref()
            && cached.key == key
        {
            return cached.clone();
        }

        let mut text = String::new();
        let mut links = Vec::new();
        let mut highlights = Vec::new();
        let mut offset = 0;
        for inline_node in &self.children {
            text.push_str(&inline_node.text);
            append_resolved_inline_styles(
                inline_node,
                offset,
                node_cx,
                key.accent,
                key.link,
                &mut links,
                &mut highlights,
            );
            offset += inline_node.text.len();
        }
        let cached = CachedParagraphRender {
            key,
            text: text.into(),
            links,
            highlights,
        };
        if let Ok(mut cache) = self.render_cache.lock() {
            *cache = Some(cached.clone());
        }
        cached
    }

    fn should_render_inline_flow(&self, node_cx: &NodeContext, cx: &App) -> bool {
        let has_image = self.children.iter().any(|child| child.image.is_some());
        let has_text = self.children.iter().any(|child| !child.text.is_empty());
        let has_semantic_box = self.children.iter().any(|child| {
            inline_node_kind(child)
                .and_then(|kind| node_cx.inline_box_style(kind, cx.theme().accent))
                .is_some()
        });
        let has_custom = self.children.iter().any(|child| child.custom.is_some());
        (has_image && has_text) || has_semantic_box || has_custom
    }

    fn inline_flow_items(
        &self,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<InlineFlowItem> {
        let mut items = Vec::new();
        let link_hover_style = node_cx.link_hover_style();
        let mut paragraph_offset = 0;
        let mut next_link_id = 0;

        for inline_node in &self.children {
            if let Some(custom) = &inline_node.custom {
                let element = node_cx
                    .markdown_extensions
                    .render_inline(custom, window, cx)
                    .unwrap_or_else(|| {
                        div().child(custom.as_text().to_string()).into_any_element()
                    });
                let text: SharedString = custom.as_text().to_string().into();
                paragraph_offset += text.len();
                items.push(InlineFlowItem::Custom {
                    element: Some(element),
                    text,
                });
                continue;
            }
            if let Some(image) = &inline_node.image {
                if !node_cx.link_handler.policy.allows_image(&image.url) {
                    let alt = image.alt.clone().unwrap_or_default();
                    if let Ok(mut state) = inline_node.state.lock() {
                        state.set_text(alt.clone());
                        state.selection = None;
                    }
                    let end = paragraph_offset + alt.len();
                    items.push(InlineFlowItem::Text {
                        state: inline_node.state.clone(),
                        paragraph_range: paragraph_offset..end,
                        text: alt,
                        links: Vec::new(),
                        highlights: Vec::new(),
                        link_hover_style: link_hover_style.clone(),
                        box_style: None,
                    });
                    paragraph_offset = end;
                    continue;
                }
                items.push(InlineFlowItem::Image {
                    url: image.url.clone(),
                    source: None,
                    sizing: InlineImageSizing::Compact,
                    link: image.link.clone(),
                    title: image.title(),
                    width: image.width,
                    height: image.height,
                    style: Box::new(
                        node_cx
                            .markdown_style
                            .element_style(MarkdownElementKind::Image)
                            .cloned()
                            .unwrap_or_default(),
                    ),
                });

                paragraph_offset += inline_node.text.len();
                continue;
            }

            if inline_node.text.is_empty() {
                continue;
            }

            let mut highlights = Vec::new();
            let mut links = Vec::new();
            append_resolved_inline_styles(
                inline_node,
                0,
                node_cx,
                cx.theme().accent,
                cx.theme().link,
                &mut links,
                &mut highlights,
            );
            for link in &mut links {
                link.id += next_link_id;
            }
            next_link_id += links.len();
            let box_style = inline_node_kind(inline_node)
                .and_then(|kind| node_cx.inline_box_style(kind, cx.theme().accent));
            if box_style.is_some() {
                // The containing rounded box owns the code background.
                for (_, highlight) in &mut highlights {
                    highlight.background_color = None;
                }
            }
            if let Ok(mut state) = inline_node.state.lock() {
                state.set_text(inline_node.text.clone());
                state.selection = None;
            }
            items.push(InlineFlowItem::Text {
                state: inline_node.state.clone(),
                paragraph_range: paragraph_offset..(paragraph_offset + inline_node.text.len()),
                text: inline_node.text.clone(),
                links,
                highlights,
                link_hover_style: link_hover_style.clone(),
                box_style,
            });
            paragraph_offset += inline_node.text.len();
        }

        items
    }

    /// Return visible paragraph text after applying image resource policy fallbacks.
    fn display_text(&self, node_cx: &NodeContext) -> String {
        let mut text = String::new();
        for child in &self.children {
            if let Some(image) = &child.image
                && !node_cx.link_handler.policy.allows_image(&image.url)
            {
                text.push_str(image.alt.as_deref().unwrap_or_default());
            } else {
                text.push_str(&child.text);
            }
        }
        text
    }
}

/// Return the highest-priority semantic covering an entire inline parser node.
fn inline_node_kind(node: &InlineNode) -> Option<MarkdownInlineKind> {
    let full_mark = node
        .marks
        .iter()
        .filter(|(range, _)| range.start == 0 && range.end == node.text.len())
        .map(|(_, mark)| mark)
        .next_back();
    let Some(mark) = full_mark else {
        return (!node.text.is_empty()).then_some(MarkdownInlineKind::Plain);
    };
    if mark.code {
        Some(MarkdownInlineKind::InlineCode)
    } else if mark.link.is_some() {
        Some(MarkdownInlineKind::Link)
    } else if mark.bold {
        Some(MarkdownInlineKind::Strong)
    } else if mark.italic {
        Some(MarkdownInlineKind::Emphasis)
    } else if mark.strikethrough {
        Some(MarkdownInlineKind::Strikethrough)
    } else if mark.underline {
        Some(MarkdownInlineKind::Underline)
    } else if mark.highlight.is_some() {
        Some(MarkdownInlineKind::Mark)
    } else if mark.footnote_reference {
        Some(MarkdownInlineKind::FootnoteReference)
    } else {
        Some(MarkdownInlineKind::Plain)
    }
}

impl Paragraph {
    fn to_markdown(&self) -> String {
        let mut text = self
            .children
            .iter()
            .map(|text_node| {
                if let Some(custom) = &text_node.custom {
                    return custom.to_markdown();
                }
                let mut text = text_node.text.to_string();
                for (range, style) in &text_node.marks {
                    if style.bold {
                        text = format!("**{}**", &text_node.text[range.clone()]);
                    }
                    if style.italic {
                        text = format!("*{}*", &text_node.text[range.clone()]);
                    }
                    if style.strikethrough {
                        text = format!("~~{}~~", &text_node.text[range.clone()]);
                    }
                    if style.code {
                        text = format!("`{}`", &text_node.text[range.clone()]);
                    }
                    if style.highlight.is_some() {
                        text = format!("=={}==", &text_node.text[range.clone()]);
                    }
                    if let Some(link) = &style.link {
                        text = format!("[{}]({})", &text_node.text[range.clone()], link.url);
                    }
                }

                if let Some(image) = &text_node.image {
                    let alt = image.alt.clone().unwrap_or_default();
                    let title = image
                        .title
                        .clone()
                        .map_or(String::new(), |t| format!(" \"{}\"", t));
                    text.push_str(&format!("![{}]({}{})", alt, image.url, title))
                }

                text
            })
            .collect::<Vec<_>>()
            .join("");

        text.push_str("\n\n");
        text
    }
}

impl BlockNode {
    /// Converts the node to markdown format.
    ///
    /// This is used to generate markdown for test.
    #[allow(dead_code)]
    pub(crate) fn to_markdown(&self) -> String {
        match self {
            BlockNode::Root { children, .. } => children
                .iter()
                .map(|child| child.to_markdown())
                .collect::<Vec<_>>()
                .join("\n\n"),
            BlockNode::Paragraph(paragraph) => paragraph.to_markdown(),
            BlockNode::Heading {
                level, children, ..
            } => {
                let hashes = "#".repeat(*level as usize);
                format!("{} {}", hashes, children.to_markdown())
            }
            BlockNode::Blockquote { children, .. } => {
                let content = children
                    .iter()
                    .map(|child| child.to_markdown())
                    .collect::<Vec<_>>()
                    .join("\n\n");

                content
                    .lines()
                    .map(|line| format!("> {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            BlockNode::List {
                children, ordered, ..
            } => children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    let prefix = if *ordered {
                        format!("{}. ", i + 1)
                    } else {
                        "- ".to_string()
                    };
                    format!("{}{}", prefix, child.to_markdown())
                })
                .collect::<Vec<_>>()
                .join("\n"),
            BlockNode::ListItem {
                children, checked, ..
            } => {
                let checkbox = if let Some(checked) = checked {
                    if *checked { "[x] " } else { "[ ] " }
                } else {
                    ""
                };
                format!(
                    "{}{}",
                    checkbox,
                    children
                        .iter()
                        .map(|child| child.to_markdown())
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
            BlockNode::CodeBlock(code_block) => {
                format!(
                    "```{}\n{}\n```",
                    code_block.lang.clone().unwrap_or_default(),
                    code_block.code()
                )
            }
            BlockNode::Table(table) => {
                let header = table
                    .children
                    .first()
                    .map(|row| {
                        row.children
                            .iter()
                            .map(|cell| cell.children.to_markdown())
                            .collect::<Vec<_>>()
                            .join(" | ")
                    })
                    .unwrap_or_default();
                let alignments = table
                    .column_aligns
                    .iter()
                    .map(|align| {
                        match align {
                            ColumnumnAlign::Left => ":--",
                            ColumnumnAlign::Center => ":-:",
                            ColumnumnAlign::Right => "--:",
                        }
                        .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                let rows = table
                    .children
                    .iter()
                    .skip(1)
                    .map(|row| {
                        row.children
                            .iter()
                            .map(|cell| cell.children.to_markdown())
                            .collect::<Vec<_>>()
                            .join(" | ")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{}\n{}\n{}", header, alignments, rows)
            }
            BlockNode::Break { html, .. } => {
                if *html {
                    "<br>".to_string()
                } else {
                    "\n".to_string()
                }
            }
            BlockNode::HorizontalRule { .. } => "---".to_string(),
            BlockNode::Custom(node) => node.to_markdown(),
            BlockNode::Definition {
                identifier,
                url,
                title,
                ..
            } => {
                if let Some(title) = title {
                    format!("[{}]: {} \"{}\"", identifier, url, title)
                } else {
                    format!("[{}]: {}", identifier, url)
                }
            }
            BlockNode::Unknown { .. } => "".to_string(),
        }
        .trim()
        .to_string()
    }
}

impl BlockNode {
    fn render_list_item_row(
        content: AnyElement,
        ix: usize,
        options: NodeRenderOptions,
        checked: Option<bool>,
        node_cx: &NodeContext,
        cx: &mut App,
    ) -> Div {
        h_flex()
            .w_full()
            .flex_1()
            .min_w_0()
            .relative()
            .items_start()
            .content_start()
            .when(!options.todo && checked.is_none(), |this| {
                this.child(
                    div()
                        .when_some(
                            node_cx
                                .markdown_style
                                .element_style(MarkdownElementKind::ListMarker),
                            |this, style| this.refine_style(style),
                        )
                        .child(list_item_prefix(
                            ix.saturating_add(options.ordered_start.saturating_sub(1) as usize),
                            options.ordered,
                            options.depth,
                        )),
                )
            })
            .when_some(checked, |this, checked| {
                // Todo list checkbox
                this.child(
                    div()
                        .flex()
                        .mt(rems(0.4))
                        .mr_1p5()
                        .size(rems(0.875))
                        .items_center()
                        .justify_center()
                        .rounded(cx.theme().style.radii.md.half())
                        .border_1()
                        .border_color(cx.theme().primary)
                        .text_color(cx.theme().primary_foreground)
                        .when_some(
                            node_cx
                                .markdown_style
                                .element_style(MarkdownElementKind::TaskCheckbox),
                            |this, style| this.refine_style(style),
                        )
                        .when(checked, |this| {
                            this.bg(cx.theme().tokens.primary)
                                .child(Icon::new(IconName::Check).size_2().text_xs())
                        }),
                )
            })
            .child(div().flex_1().min_w_0().overflow_hidden().child(content))
    }

    fn render_list_item(
        item: &BlockNode,
        ix: usize,
        options: NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let default = match item {
            BlockNode::ListItem {
                children,
                spread,
                checked,
                ..
            } => v_flex()
                .id(("li", options.ix))
                .w_full()
                .min_w_0()
                .when_some(
                    node_cx
                        .markdown_style
                        .element_style(MarkdownElementKind::ListItem),
                    |this, style| this.refine_style(style),
                )
                .when(*spread, |this| this.child(div()))
                .children({
                    let mut items: Vec<Div> = Vec::with_capacity(children.len());

                    for (child_ix, child) in children.iter().enumerate() {
                        match child {
                            BlockNode::Paragraph { .. } => {
                                let last_not_list = child_ix > 0
                                    && !matches!(children[child_ix - 1], BlockNode::List { .. });

                                let text = child.render_block(
                                    NodeRenderOptions {
                                        depth: options.depth + 1,
                                        todo: checked.is_some(),
                                        is_last: true,
                                        ..options
                                    },
                                    node_cx,
                                    window,
                                    cx,
                                );

                                // Continuation paragraph — stack vertically below
                                // the previous row, indented to align with the text
                                // column (past bullet/number prefix).
                                if last_not_list {
                                    if let Some(preceding_row) = items.pop() {
                                        items.push(
                                            v_flex().child(preceding_row).child(
                                                div()
                                                    .w_full()
                                                    .pl(rems(1.))
                                                    .overflow_hidden()
                                                    .child(text),
                                            ),
                                        );
                                        continue;
                                    }
                                }

                                items.push(Self::render_list_item_row(
                                    text, ix, options, *checked, node_cx, cx,
                                ));
                            }
                            BlockNode::List { .. } => {
                                items.push(div().ml(rems(1.)).child(child.render_block(
                                    NodeRenderOptions {
                                        depth: options.depth + 1,
                                        todo: checked.is_some(),
                                        is_last: true,
                                        ..options
                                    },
                                    node_cx,
                                    window,
                                    cx,
                                )));
                            }
                            BlockNode::Root { .. }
                            | BlockNode::Heading { .. }
                            | BlockNode::Blockquote { .. }
                            | BlockNode::CodeBlock(_)
                            | BlockNode::Custom(_)
                            | BlockNode::Table(_)
                            | BlockNode::HorizontalRule { .. } => {
                                let block = child.render_block(
                                    NodeRenderOptions {
                                        depth: options.depth + 1,
                                        todo: checked.is_some(),
                                        is_last: true,
                                        ..options
                                    },
                                    node_cx,
                                    window,
                                    cx,
                                );

                                if child_ix == 0 {
                                    items.push(Self::render_list_item_row(
                                        block, ix, options, *checked, node_cx, cx,
                                    ));
                                } else {
                                    // Indent continuation blocks to align with a
                                    // nested sub-list (`ml(rems(1.))`) and with
                                    // continuation paragraphs.
                                    items.push(
                                        div()
                                            .w_full()
                                            .min_w_0()
                                            .pl(rems(1.))
                                            .overflow_hidden()
                                            .child(block),
                                    );
                                }
                            }
                            BlockNode::ListItem { .. }
                            | BlockNode::Break { .. }
                            | BlockNode::Definition { .. }
                            | BlockNode::Unknown => {}
                        }
                    }
                    items
                })
                .into_any_element(),
            _ => div().into_any_element(),
        };
        item.apply_renderer(default, options, node_cx, window, cx)
    }

    /// Render a Markdown table. Dispatches to a horizontally scrollable layout
    /// when `style.table` opts in with overflow-x: scroll, otherwise to the
    /// default layout that fits the container width and wraps cell content.
    fn render_table(
        item: &BlockNode,
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        const DEFAULT_LENGTH: usize = 5;

        let table = match item {
            BlockNode::Table(table) => table,
            _ => return div().into_any_element(),
        };

        // Per-column max text length (in chars), used to proportion the columns
        // in the default (wrap) layout.
        let mut col_lens: Vec<usize> = vec![];
        for row in table.children.iter() {
            for (ix, cell) in row.children.iter().enumerate() {
                if col_lens.len() <= ix {
                    col_lens.push(DEFAULT_LENGTH);
                }
                col_lens[ix] = col_lens[ix].max(cell.children.text_len());
            }
        }

        // Scroll mode is opted in via `style.table` overflow-x: scroll.
        if matches!(node_cx.style.table.overflow.x, Some(Overflow::Scroll)) {
            Self::render_scroll_table(table, col_lens.len(), options, node_cx, window, cx)
        } else {
            Self::render_wrap_table(table, &col_lens, options, node_cx, window, cx)
        }
    }

    /// Horizontally scrollable table layout (opt-in via `style.table`
    /// overflow-x: scroll).
    ///
    /// Column widths come from the **measured** shaped text of each cell (the
    /// widest per column across all rows), so columns line up and fit their
    /// content exactly — char-count heuristics are inaccurate on proportional
    /// fonts. The layout adapts to the frame like CSS auto table layout:
    ///
    /// - Wider than the content: cells `flex_grow` proportionally to fill.
    /// - Narrower: columns shrink and their text wraps, but not below a
    ///   per-column floor.
    /// - Narrower than the floors: the table keeps the floor widths and
    ///   scrolls horizontally, so no content ever becomes unreachable.
    ///
    /// `white_space: nowrap` on `style.table_cell` composes like in CSS: the
    /// refinement keeps cell text on a single line, and the floors are raised
    /// to the full content widths so the single-line columns never shrink —
    /// the table scrolls as soon as the content is wider than the frame.
    fn render_scroll_table(
        table: &Table,
        col_count: usize,
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        const CELL_PAD_PX: f32 = 16.0; // px_2 horizontal padding
        const CELL_MIN_PX: f32 = 48.0;
        // Shrinking columns stop (and the table starts to scroll) at a floor
        // scaled to their content: roughly the width at which the text wraps
        // to `CELL_WRAP_MAX_LINES` lines, clamped between the two bounds so
        // moderate columns can still wrap meaningfully while one huge column
        // cannot push the scroll threshold arbitrarily high.
        const CELL_WRAP_MAX_LINES: f32 = 2.0;
        const CELL_WRAP_MIN_PX: f32 = 160.0;
        const CELL_WRAP_MAX_PX: f32 = 480.0;
        const CELL_BORDER_PX: f32 = 1.0; // border_r_1 drawn by every column but the last
        const TABLE_BORDER_PX: f32 = 2.0; // the track's border_1, left + right

        // Measure the widest text per column (max-content width). Never
        // capped: a cap would clip overflowing text *and* leave it outside
        // the scrollable width, making it unreachable.
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let mut col_w = vec![CELL_MIN_PX; col_count];
        for row in table.children.iter() {
            for (ix, cell) in row.children.iter().enumerate() {
                let Some(slot) = col_w.get_mut(ix) else {
                    continue;
                };
                let mut w = 0.0_f32;
                for line in cell.children.text().split('\n') {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let run = text_style.to_run(line.len());
                    let line_w = window
                        .text_system()
                        .layout_line(line, font_size, &[run], None)
                        .width;
                    w = w.max(f32::from(line_w));
                }
                // Border-box widths, so the padding and border the cell draws
                // must leave the measured text its full width.
                let border = if ix + 1 < col_count {
                    CELL_BORDER_PX
                } else {
                    0.
                };
                *slot = slot.max(w + CELL_PAD_PX + border);
            }
        }
        let style = &node_cx.style;
        // Nowrap cells (via the `table_cell` refinement, which cascades to
        // the cell text) must never shrink below their single-line content,
        // so their floor is the content width itself.
        let nowrap = style.table_cell.text.white_space == Some(WhiteSpace::Nowrap);
        let col_min_w: Vec<f32> = if nowrap {
            col_w.clone()
        } else {
            col_w
                .iter()
                .map(|w| {
                    (w / CELL_WRAP_MAX_LINES)
                        .clamp(CELL_WRAP_MIN_PX, CELL_WRAP_MAX_PX)
                        .min(*w)
                })
                .collect()
        };
        let min_total_w: f32 = col_min_w.iter().sum::<f32>() + TABLE_BORDER_PX;

        let table_scroll_key = if let Some(span) = table.span {
            SharedString::from(format!(
                "{}-table-scroll-{}:{}",
                window.current_view(),
                span.start,
                span.end
            ))
        } else {
            SharedString::from(format!(
                "{}-table-scroll-{}",
                window.current_view(),
                options.ix
            ))
        };
        let scroll_handle = window
            .use_keyed_state(table_scroll_key, cx, |_, _| ScrollHandle::default())
            .read(cx)
            .clone();
        let row_count = table.children.len();
        let mut rows = Vec::with_capacity(row_count);
        for (row_ix, row) in table.children.iter().enumerate() {
            let mut cells = Vec::with_capacity(row.children.len());
            for (ix, cell) in row.children.iter().enumerate() {
                let align = table.column_align(ix);
                let is_last_col = ix == row.children.len() - 1;
                let width = col_w.get(ix).copied().unwrap_or(CELL_MIN_PX);
                let min_width = col_min_w.get(ix).copied().unwrap_or(CELL_MIN_PX);
                cells.push(
                    div()
                        .id(("cell", ix))
                        // Measured max-content width is the flex-basis;
                        // `flex_grow` (proportional to it) distributes extra
                        // space so a narrow table still fills the frame, while
                        // shrinking is clamped at `min_w` — the flex engine
                        // squeezes columns (their text wraps) down to the
                        // floors before the track starts to scroll.
                        .flex_basis(px(width))
                        .flex_grow(width)
                        .flex_shrink(1.)
                        .min_w(px(min_width))
                        .overflow_hidden()
                        .when(align == ColumnumnAlign::Center, |this| this.text_center())
                        .when(align == ColumnumnAlign::Right, |this| this.text_right())
                        .px_2()
                        .py_1()
                        .when(!is_last_col, |this| {
                            this.border_r_1().border_color(cx.theme().border)
                        })
                        .refine_style(&style.table_cell)
                        .when_some(
                            node_cx.markdown_style.element_style(if row_ix == 0 {
                                MarkdownElementKind::TableHeaderCell
                            } else {
                                MarkdownElementKind::TableCell
                            }),
                            |this, style| this.refine_style(style),
                        )
                        .child(cell.children.render(node_cx, window, cx)),
                );
            }
            rows.push(
                div()
                    .id("row")
                    .w_full()
                    .when(row_ix < row_count - 1, |this| this.border_b_1())
                    .border_color(cx.theme().border)
                    .flex()
                    .flex_row()
                    .when_some(
                        node_cx.markdown_style.element_style(if row_ix == 0 {
                            MarkdownElementKind::TableHeaderRow
                        } else {
                            MarkdownElementKind::TableBodyRow
                        }),
                        |this, style| this.refine_style(style),
                    )
                    .children(cells),
            );
        }

        div()
            .pb(rems(1.))
            .w_full()
            .child(
                // Scroll viewport: clips and scrolls horizontally (overflow-x
                // is handled by `ScrollableMask`, so vertical wheel events keep
                // bubbling to the parent TextView). No border — the frame is on
                // the inner track so it wraps the table tightly.
                horizontal_scroll_area(
                    ("table", options.ix),
                    &scroll_handle,
                    &style.table,
                    // Bordered track sized to `max(viewport, column floors)`:
                    // `min_w_full` fills the frame while the columns can still
                    // shrink-to-fit (their text wrapping), the definite
                    // `w(min_total_w)` keeps the floors once they are reached,
                    // letting the track exceed the viewport and scroll.
                    div()
                        .min_w_full()
                        .w(px(min_total_w))
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded(cx.theme().style.radii.md)
                        .when_some(
                            node_cx
                                .markdown_style
                                .element_style(MarkdownElementKind::Table),
                            |this, style| this.refine_style(style),
                        )
                        .children(rows),
                ),
            )
            .into_any_element()
    }

    /// Default table layout: a flex grid whose columns are proportioned by
    /// content length and shrink to fit the container width (cell text wraps).
    fn render_wrap_table(
        table: &Table,
        col_lens: &[usize],
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        const MAX_LENGTH: usize = 150;

        let style = &node_cx.style;
        let row_count = table.children.len();
        let mut rows = Vec::with_capacity(row_count);
        for (row_ix, row) in table.children.iter().enumerate() {
            let mut cells = Vec::with_capacity(row.children.len());
            for (ix, cell) in row.children.iter().enumerate() {
                let align = table.column_align(ix);
                let is_last_col = ix == row.children.len() - 1;
                let len = col_lens
                    .get(ix)
                    .copied()
                    .unwrap_or(MAX_LENGTH)
                    .min(MAX_LENGTH);

                cells.push(
                    div()
                        .id(("cell", ix))
                        .overflow_hidden()
                        .when(align == ColumnumnAlign::Center, |this| this.text_center())
                        .when(align == ColumnumnAlign::Right, |this| this.text_right())
                        .min_w_16()
                        .w(Length::Definite(relative(len as f32)))
                        .px_2()
                        .py_1()
                        .when(!is_last_col, |this| {
                            this.border_r_1().border_color(cx.theme().border)
                        })
                        .refine_style(&style.table_cell)
                        .when_some(
                            node_cx.markdown_style.element_style(if row_ix == 0 {
                                MarkdownElementKind::TableHeaderCell
                            } else {
                                MarkdownElementKind::TableCell
                            }),
                            |this, style| this.refine_style(style),
                        )
                        .child(cell.children.render(node_cx, window, cx)),
                );
            }

            rows.push(
                div()
                    .id("row")
                    .w_full()
                    .when(row_ix < row_count - 1, |this| this.border_b_1())
                    .border_color(cx.theme().border)
                    .flex()
                    .flex_row()
                    .when_some(
                        node_cx.markdown_style.element_style(if row_ix == 0 {
                            MarkdownElementKind::TableHeaderRow
                        } else {
                            MarkdownElementKind::TableBodyRow
                        }),
                        |this, style| this.refine_style(style),
                    )
                    .children(cells),
            );
        }

        div()
            .pb(rems(1.))
            .w_full()
            .child(
                div()
                    .id(("table", options.ix))
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().style.radii.md)
                    .overflow_hidden()
                    .children(rows)
                    .refine_style(&style.table)
                    .when_some(
                        node_cx
                            .markdown_style
                            .element_style(MarkdownElementKind::Table),
                        |this, style| this.refine_style(style),
                    ),
            )
            .into_any_element()
    }

    pub(crate) fn render_block(
        &self,
        options: NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let default = self.render_block_default(options, node_cx, window, cx);
        self.apply_renderer(default, options, node_cx, window, cx)
    }

    fn apply_renderer(
        &self,
        default: AnyElement,
        _options: NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let Some(kind) = self.renderer_kind() else {
            return default;
        };
        let Some(renderer) = node_cx.markdown_block_renderers.get(&kind) else {
            return default;
        };

        let span = self.span();
        let source_range = span.map(|span| span.start..span.end);
        let source = source_range
            .as_ref()
            .and_then(|range| node_cx.source.get(range.clone()))
            .unwrap_or_default()
            .to_string()
            .into();
        let (heading_level, code_language, code_info, code_source_path, task_checked) = match self {
            Self::Heading { level, .. } => (
                Some(MarkdownHeadingLevel::from_depth(*level)),
                None,
                None,
                None,
                None,
            ),
            Self::CodeBlock(code) => (None, code.lang(), code.info(), code.source_path(), None),
            Self::ListItem { checked, .. } => (None, None, None, None, *checked),
            _ => (None, None, None, None, None),
        };
        let context = MarkdownBlockRenderContext {
            kind,
            source_range,
            source,
            text: self.text().trim_end().to_string().into(),
            heading_level,
            code_language,
            code_info,
            code_source_path,
            task_checked,
            default,
        };
        renderer(context, window, cx)
    }

    fn renderer_kind(&self) -> Option<MarkdownBlockKind> {
        match self {
            Self::Paragraph(_) => Some(MarkdownBlockKind::Paragraph),
            Self::Heading { .. } => Some(MarkdownBlockKind::Heading),
            Self::Blockquote { .. } => Some(MarkdownBlockKind::Blockquote),
            Self::List { ordered: true, .. } => Some(MarkdownBlockKind::OrderedList),
            Self::List { ordered: false, .. } => Some(MarkdownBlockKind::UnorderedList),
            Self::ListItem {
                checked: Some(_), ..
            } => Some(MarkdownBlockKind::TaskListItem),
            Self::ListItem { .. } => Some(MarkdownBlockKind::ListItem),
            Self::CodeBlock(_) => Some(MarkdownBlockKind::CodeBlock),
            Self::Table(_) => Some(MarkdownBlockKind::Table),
            Self::HorizontalRule { .. } => Some(MarkdownBlockKind::HorizontalRule),
            _ => None,
        }
    }

    fn render_block_default(
        &self,
        options: NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let ix = options.ix;
        let mb = if options.in_list || options.is_last {
            rems(0.)
        } else {
            node_cx.style.paragraph_gap
        };

        match self {
            BlockNode::Root { children, .. } => div()
                .id(("div", ix))
                .children(children.into_iter().enumerate().map(move |(ix, node)| {
                    node.render_block(NodeRenderOptions { ix, ..options }, node_cx, window, cx)
                }))
                .into_any_element(),
            BlockNode::Paragraph(paragraph) => div()
                .id(("p", ix))
                .pb(mb)
                .when_some(
                    node_cx
                        .markdown_style
                        .element_style(MarkdownElementKind::Paragraph),
                    |this, style| this.refine_style(style),
                )
                .child(paragraph.render(node_cx, window, cx))
                .into_any_element(),
            BlockNode::Heading {
                level, children, ..
            } => {
                let (text_size, font_weight) = match level {
                    1 => (rems(2.), FontWeight::BOLD),
                    2 => (rems(1.5), FontWeight::SEMIBOLD),
                    3 => (rems(1.25), FontWeight::SEMIBOLD),
                    4 => (rems(1.125), FontWeight::SEMIBOLD),
                    5 => (rems(1.), FontWeight::SEMIBOLD),
                    6 => (rems(1.), FontWeight::MEDIUM),
                    _ => (rems(1.), FontWeight::NORMAL),
                };

                let mut text_size = text_size.to_pixels(node_cx.style.heading_base_font_size);
                if let Some(f) = node_cx.style.heading_font_size.as_ref() {
                    text_size = (f)(*level, node_cx.style.heading_base_font_size);
                }

                div()
                    .id(SharedString::from(format!("h{}-{}", level, ix)))
                    .pb(rems(0.3))
                    .whitespace_normal()
                    .text_size(text_size)
                    .font_weight(font_weight)
                    .when_some(
                        node_cx
                            .markdown_style
                            .element_style(MarkdownElementKind::Heading(
                                MarkdownHeadingLevel::from_depth(*level),
                            )),
                        |this, style| this.refine_style(style),
                    )
                    .child(children.render(node_cx, window, cx))
                    .into_any_element()
            }
            BlockNode::Blockquote { children, kind, .. } => div()
                .w_full()
                .pb(mb)
                .child(
                    div()
                        .id(("blockquote", ix))
                        .w_full()
                        .text_color(cx.theme().muted_foreground)
                        .border_l_3()
                        .border_color(match kind {
                            Some(BlockQuoteKind::Note | BlockQuoteKind::Important) => {
                                cx.theme().info
                            }
                            Some(BlockQuoteKind::Tip) => cx.theme().success,
                            Some(BlockQuoteKind::Warning) => cx.theme().warning,
                            Some(BlockQuoteKind::Caution) => cx.theme().danger,
                            None => cx.theme().secondary_active,
                        })
                        .px_4()
                        .when_some(
                            node_cx
                                .markdown_style
                                .element_style(MarkdownElementKind::Blockquote),
                            |this, style| this.refine_style(style),
                        )
                        .children({
                            let children_len = children.len();
                            children.into_iter().enumerate().map(move |(index, c)| {
                                let is_last = index == children_len - 1;
                                c.render_block(options.is_last(is_last), node_cx, window, cx)
                            })
                        }),
                )
                .into_any_element(),
            BlockNode::List {
                children,
                ordered,
                start,
                ..
            } => v_flex()
                .id((if *ordered { "ol" } else { "ul" }, ix))
                .w_full()
                .min_w_0()
                .pb(mb)
                .when_some(
                    node_cx.markdown_style.element_style(if *ordered {
                        MarkdownElementKind::OrderedList
                    } else {
                        MarkdownElementKind::UnorderedList
                    }),
                    |this, style| this.refine_style(style),
                )
                .children({
                    let mut items = Vec::with_capacity(children.len());
                    let mut item_index = 0;
                    for (ix, item) in children.into_iter().enumerate() {
                        let is_item = item.is_list_item();

                        items.push(Self::render_list_item(
                            item,
                            item_index,
                            NodeRenderOptions {
                                ix,
                                ordered: *ordered,
                                ordered_start: start.unwrap_or(1),
                                ..options
                            },
                            node_cx,
                            window,
                            cx,
                        ));

                        if is_item {
                            item_index += 1;
                        }
                    }
                    items
                })
                .into_any_element(),
            BlockNode::CodeBlock(code_block) => code_block.render(&options, node_cx, window, cx),
            BlockNode::Custom(node) => {
                let inner = match node_cx.markdown_extensions.render_block(node, window, cx) {
                    Some(rendered) => rendered,
                    None => div().child(node.as_text().to_string()).into_any_element(),
                };

                div().pb(mb).child(inner).into_any_element()
            }
            BlockNode::Table { .. } => {
                Self::render_table(self, &options, node_cx, window, cx).into_any_element()
            }
            BlockNode::HorizontalRule { .. } => div()
                .pb(mb)
                .child(
                    div()
                        .id("horizontal-rule")
                        .bg(cx.theme().border)
                        .h(px(2.))
                        .when_some(
                            node_cx
                                .markdown_style
                                .element_style(MarkdownElementKind::HorizontalRule),
                            |this, style| this.refine_style(style),
                        ),
                )
                .into_any_element(),
            BlockNode::Break { .. } => div().id("break").into_any_element(),
            BlockNode::Unknown { .. } | BlockNode::Definition { .. } => div().into_any_element(),
            _ => {
                if cfg!(debug_assertions) {
                    tracing::warn!("unknown implementation: {:?}", self);
                }

                div().into_any_element()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, hsla};

    #[test]
    fn mermaid_source_renders_to_resvg_safe_svg() {
        let svg = render_mermaid_svg("flowchart LR\n  A --> B", false).unwrap();

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn mermaid_theme_changes_with_application_appearance() {
        let source = "flowchart LR\n  A --> B";
        let light = render_mermaid_svg(source, false).unwrap();
        let dark = render_mermaid_svg(source, true).unwrap();

        assert_ne!(light, dark);
    }

    #[cfg(feature = "tree-sitter")]
    use crate::{
        Theme, ThemeMode,
        text::{TextView, TextViewState},
    };
    #[cfg(feature = "tree-sitter")]
    use gpui::{Context, Entity, Render, VisualTestContext};

    #[cfg(feature = "tree-sitter")]
    fn cached_highlight_theme(block: &CodeBlock) -> Option<Arc<HighlightTheme>> {
        block
            .styles
            .lock()
            .ok()
            .and_then(|styles| styles.as_ref().map(|styles| styles.highlight_theme.clone()))
    }

    #[cfg(feature = "tree-sitter")]
    fn cached_highlighter_language(block: &CodeBlock) -> Option<SharedString> {
        let cache = block.styles.lock().ok()?;
        let highlighter = cache.as_ref()?.highlighter.lock().ok()?;
        Some(highlighter.language().clone())
    }

    #[test]
    fn code_block_equality_includes_code_content() {
        let first = CodeBlock::new("let value = 1;".into(), Some("rust".into()), None::<Span>);
        let second = CodeBlock::new("let value = 2;".into(), Some("rust".into()), None::<Span>);

        assert_ne!(first, second);
    }

    #[test]
    fn semantic_inline_styles_resolve_precedence_and_explicit_removals() {
        let plain_color = hsla(0.1, 0.7, 0.4, 1.);
        let link_color = hsla(0.6, 0.8, 0.5, 1.);
        let background = hsla(0.2, 0.3, 0.8, 1.);
        let markdown_style = LegacyMarkdownStyle::default()
            .inline(
                MarkdownInlineKind::Plain,
                MarkdownTextStyle::default()
                    .color(plain_color)
                    .background(background)
                    .underline(gpui::UnderlineStyle {
                        thickness: px(1.),
                        ..Default::default()
                    }),
            )
            .inline(
                MarkdownInlineKind::Link,
                MarkdownTextStyle::default()
                    .color(link_color)
                    .no_background()
                    .no_underline(),
            );
        let node_cx = NodeContext {
            markdown_style: Arc::new(markdown_style),
            ..Default::default()
        };
        let inline_node = InlineNode::new("plain link").marks(vec![
            (0..10, TextMark::default()),
            (
                6..10,
                TextMark::default().link(LinkMark {
                    url: "https://example.com".into(),
                    ..Default::default()
                }),
            ),
        ]);
        let mut links = Vec::new();
        let mut highlights = Vec::new();

        append_resolved_inline_styles(
            &inline_node,
            0,
            &node_cx,
            background,
            plain_color,
            &mut links,
            &mut highlights,
        );

        assert_eq!(highlights.len(), 2);
        assert_eq!(highlights[0].0, 0..6);
        assert_eq!(highlights[0].1.color, Some(plain_color));
        assert_eq!(highlights[0].1.background_color, Some(background));
        assert!(highlights[0].1.underline.is_some());
        assert_eq!(highlights[1].0, 6..10);
        assert_eq!(highlights[1].1.color, Some(link_color));
        assert_eq!(highlights[1].1.background_color, None);
        assert_eq!(highlights[1].1.underline, None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].range, 6..10);
    }

    #[test]
    fn source_search_ranges_map_to_rendered_inline_offsets() {
        let inline_node = InlineNode::new("value").source_range(10..15);
        let node_cx = NodeContext {
            search_matches: Arc::new(vec![12..14, 20..24]),
            active_search_match: Some(0),
            ..Default::default()
        };

        assert_eq!(
            mapped_search_ranges(&inline_node, &node_cx),
            vec![(2..4, true)]
        );
    }

    #[gpui::test]
    fn built_in_profile_keeps_plain_text_on_the_lightweight_inline_path(cx: &mut TestAppContext) {
        cx.update(crate::init);
        cx.update(|cx| {
            for profile in [
                MarkdownStyleProfile::Agent,
                MarkdownStyleProfile::Editor,
                MarkdownStyleProfile::Preview,
            ] {
                let style = LegacyMarkdownStyle::for_profile(profile, cx);
                assert!(style.base_text_style().text.line_height.is_some());
                assert!(
                    style
                        .element_style(MarkdownElementKind::Paragraph)
                        .is_some_and(|style| style.text.line_height.is_some()),
                    "built-in profiles must retain Zed's paragraph line-height override"
                );
                assert!(
                    style.inline_style(MarkdownInlineKind::Plain).is_none(),
                    "built-in body typography must not create an atomic Plain box"
                );
            }

            let node_cx = NodeContext {
                markdown_style: Arc::new(LegacyMarkdownStyle::for_profile(
                    MarkdownStyleProfile::Agent,
                    cx,
                )),
                ..Default::default()
            };
            let plain = Paragraph::new("plain **semantic** text".to_string());
            assert!(!plain.should_render_inline_flow(&node_cx, cx));

            let mut code = Paragraph::new(String::new());
            code.children =
                vec![InlineNode::new("code").marks(vec![(0..4, TextMark::default().code())])];
            assert!(code.should_render_inline_flow(&node_cx, cx));
        });
    }

    #[test]
    fn paragraph_render_cache_reuses_matching_inputs_and_invalidates_style_changes() {
        let paragraph = Paragraph::new("cached paragraph".to_string());
        let node_cx = NodeContext::default();
        let key = ParagraphRenderKey {
            inline_styles: Vec::new(),
            link_refs: HashMap::new(),
            accent: hsla(0.1, 0.2, 0.3, 1.),
            link: hsla(0.4, 0.5, 0.6, 1.),
        };
        let first = paragraph.cached_render(key.clone(), &node_cx);
        assert_eq!(first.text.as_ref(), "cached paragraph");

        paragraph
            .render_cache
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .text = "cache hit".into();
        assert_eq!(
            paragraph.cached_render(key.clone(), &node_cx).text.as_ref(),
            "cache hit"
        );

        let changed = ParagraphRenderKey {
            accent: hsla(0.7, 0.2, 0.3, 1.),
            ..key
        };
        assert_eq!(
            paragraph.cached_render(changed, &node_cx).text.as_ref(),
            "cached paragraph"
        );
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn code_block_highlighter_cache_refreshes_after_language_registration() {
        let lang = SharedString::from("json-cache-test");
        let theme = HighlightTheme::default_light();

        let block = CodeBlock::new("{\"value\": 1}".into(), Some(lang.clone()), None::<Span>);
        _ = block.styles(&theme);
        assert_eq!(cached_highlighter_language(&block).as_deref(), Some("text"));

        LanguageRegistry::singleton().register(
            lang.as_ref(),
            &crate::highlighter::LanguageConfig::new(
                lang.clone(),
                tree_sitter_json::LANGUAGE.into(),
                vec![],
                r#"
                    (string) @string
                    (number) @number
                    (pair key: (string) @property)
                "#,
                "",
                "",
            ),
        );

        _ = block.styles(&theme);
        assert_eq!(
            cached_highlighter_language(&block).as_deref(),
            Some(lang.as_ref())
        );
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn code_block_styles_follow_the_current_highlight_theme() {
        let lang = SharedString::from("json-theme-cache-test");
        let light_theme = HighlightTheme::default_light();
        let dark_theme = HighlightTheme::default_dark();
        let code = SharedString::from(r#"{"value": 42}"#);
        let number_range = code.find("42").unwrap()..code.find("42").unwrap() + 2;

        let light_number = light_theme.style("number").and_then(|style| style.color);
        let dark_number = dark_theme.style("number").and_then(|style| style.color);
        assert_ne!(
            light_number, dark_number,
            "the test themes must use different number colors"
        );

        LanguageRegistry::singleton().register(
            lang.as_ref(),
            &crate::highlighter::LanguageConfig::new(
                lang.clone(),
                tree_sitter_json::LANGUAGE.into(),
                vec![],
                "(number) @number",
                "",
                "",
            ),
        );

        let block = CodeBlock::new(code.clone(), Some(lang), None::<Span>);
        let light_styles = block.styles(&light_theme);
        let cached_light_theme = cached_highlight_theme(&block).unwrap();
        assert!(Arc::ptr_eq(&cached_light_theme, &light_theme));

        let equivalent_light_theme = Arc::new(light_theme.as_ref().clone());
        let repeated_light_styles = block.styles(&equivalent_light_theme);
        assert_eq!(repeated_light_styles, light_styles);
        assert!(
            Arc::ptr_eq(
                &cached_highlight_theme(&block).unwrap(),
                &equivalent_light_theme
            ),
            "an equivalent replacement should become the cache identity"
        );
        assert_eq!(block.styles(&equivalent_light_theme), light_styles);

        let dark_styles = block.styles(&dark_theme);
        assert_eq!(
            cached_highlight_theme(&block).as_deref(),
            Some(dark_theme.as_ref())
        );

        let color_for_number = |styles: &[(Range<usize>, HighlightStyle)]| -> Option<Hsla> {
            styles
                .iter()
                .find(|(range, _)| {
                    range.start <= number_range.start && range.end >= number_range.end
                })
                .and_then(|(_, style)| style.color)
        };

        assert_eq!(color_for_number(&light_styles), light_number);
        assert_eq!(
            color_for_number(&dark_styles),
            dark_number,
            "a theme change must not reuse syntax styles from the previous theme"
        );
    }

    #[cfg(feature = "tree-sitter")]
    #[gpui::test]
    fn rendered_markdown_code_block_follows_theme_without_reparsing(cx: &mut TestAppContext) {
        struct CodeBlockThemeRoot {
            text_view: Entity<TextViewState>,
        }

        impl Render for CodeBlockThemeRoot {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                div().w(px(480.)).child(TextView::new(&self.text_view))
            }
        }

        let lang = SharedString::from("json-theme-render-test");
        LanguageRegistry::singleton().register(
            lang.as_ref(),
            &crate::highlighter::LanguageConfig::new(
                lang.clone(),
                tree_sitter_json::LANGUAGE.into(),
                vec![],
                "(number) @number",
                "",
                "",
            ),
        );

        cx.update(crate::init);
        let markdown = format!("```{lang}\n{{\"value\": 42}}\n```");
        let (view, cx) = cx.add_window_view(|_, cx| CodeBlockThemeRoot {
            text_view: cx.new(|cx| TextViewState::markdown(&markdown, cx)),
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        let light_theme = cx.update(|_, cx| cx.theme().highlight_theme.clone());
        let light_block = view.read_with(cx, |root, cx| {
            let state = root.text_view.read(cx);
            let BlockNode::CodeBlock(block) = &*state.parsed_content.document.blocks[0] else {
                panic!("expected a code block");
            };

            block.clone()
        });
        let cached_light_theme = cached_highlight_theme(&light_block)
            .expect("initial render should populate the highlight cache");
        assert_eq!(cached_light_theme.as_ref(), light_theme.as_ref());

        cx.update(|window, cx| {
            Theme::change(ThemeMode::Dark, Some(&mut *window), cx);
            let _ = window.draw(cx);
        });

        let dark_theme = cx.update(|_, cx| cx.theme().highlight_theme.clone());
        let dark_block = view.read_with(cx, |root, cx| {
            let state = root.text_view.read(cx);
            let BlockNode::CodeBlock(block) = &*state.parsed_content.document.blocks[0] else {
                panic!("expected a code block");
            };

            block.clone()
        });
        let cached_dark_theme = cached_highlight_theme(&dark_block)
            .expect("theme-change render should refresh the highlight cache");

        assert_ne!(
            light_theme.as_ref(),
            dark_theme.as_ref(),
            "the test themes must have distinct highlight palettes"
        );
        assert!(
            Arc::ptr_eq(&dark_block.styles, &light_block.styles),
            "changing the theme must not require reparsing the Markdown document"
        );
        assert_eq!(cached_dark_theme.as_ref(), dark_theme.as_ref());
    }
}
