use std::{collections::HashMap, ops::Range, sync::Arc};

use gpui::SharedString;
use linkify::{LinkFinder, LinkKind};
use pulldown_cmark::{
    BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, MetadataBlockKind, Options, Parser, Tag,
    TagEnd,
};

use crate::text::{
    MarkdownOptions,
    document::{MarkdownSourceEvent, MarkdownSourceEventKind, ParsedDocument},
    markdown_ext::{MarkdownParseContext, MarkdownParseEvent},
    node::{
        BlockNode, CodeBlock, ImageNode, InlineNode, LinkMark, NodeContext, Paragraph, Span, Table,
        TableCell, TableRow, TextMark,
    },
};

const BASE_PARSE_OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_FOOTNOTES)
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_SMART_PUNCTUATION)
    .union(Options::ENABLE_HEADING_ATTRIBUTES)
    .union(Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS)
    .union(Options::ENABLE_OLD_FOOTNOTES)
    .union(Options::ENABLE_GFM)
    .union(Options::ENABLE_SUPERSCRIPT)
    .union(Options::ENABLE_SUBSCRIPT);

/// Build the parser extension set for one Markdown document.
pub(crate) fn parse_options(options: MarkdownOptions) -> Options {
    if options.render_metadata_blocks {
        BASE_PARSE_OPTIONS.union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS)
    } else {
        BASE_PARSE_OPTIONS
    }
}

/// Parse Markdown into Hearth's source-mapped document model.
///
/// `pulldown-cmark` supplies a streaming event interface. The conversion keeps
/// parser details private so rendering, selection, and incremental updates all
/// continue to use the same deep document module.
#[stacksafe::stacksafe]
pub(crate) fn parse(source: &str, cx: &mut NodeContext) -> Result<ParsedDocument, SharedString> {
    cx.source = source.to_string().into();
    if cx.markdown_options.parse_links_only {
        return Ok(parse_links_only(source, cx));
    }
    let options = cx.markdown_options;
    let mut builder = DocumentBuilder::new(source, cx);
    for (event, range) in Parser::new_ext(source, parse_options(options)).into_offset_iter() {
        if let Err(error) = builder.consume(event, range) {
            tracing::warn!(
                ?error,
                "recovering from unsupported Markdown event sequence"
            );
        }
    }
    Ok(builder.finish_or_literal())
}

fn parse_links_only(source: &str, cx: &NodeContext) -> ParsedDocument {
    let mut paragraph = Paragraph::default();
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    let mut cursor = 0;
    for link in finder.links(source) {
        if cursor < link.start() {
            paragraph.push(InlineNode::new(&source[cursor..link.start()]));
        }
        paragraph.push(InlineNode::new(link.as_str()).marks(vec![(
            0..link.as_str().len(),
            TextMark::default().link(LinkMark {
                url: link.as_str().to_string().into(),
                ..Default::default()
            }),
        )]));
        cursor = link.end();
    }
    if cursor < source.len() {
        paragraph.push(InlineNode::new(&source[cursor..]));
    }
    paragraph.set_span(Span {
        start: cx.offset,
        end: cx.offset + source.len(),
    });
    ParsedDocument {
        source: source.to_string().into(),
        blocks: vec![Arc::new(BlockNode::Paragraph(paragraph))],
        ..Default::default()
    }
}

#[derive(Debug)]
enum Frame {
    Root(Vec<BlockNode>),
    Paragraph {
        paragraph: Paragraph,
        span: Span,
    },
    Heading {
        level: u8,
        paragraph: Paragraph,
        span: Span,
    },
    Blockquote {
        children: Vec<BlockNode>,
        kind: Option<BlockQuoteKind>,
        span: Span,
    },
    List {
        children: Vec<BlockNode>,
        ordered: bool,
        start: Option<u64>,
        span: Span,
    },
    Item {
        children: Vec<BlockNode>,
        inline: Paragraph,
        checked: Option<bool>,
        span: Span,
    },
    CodeBlock {
        code: String,
        language: Option<SharedString>,
        info: Option<SharedString>,
        span: Span,
    },
    HtmlBlock {
        html: String,
        span: Span,
    },
    Metadata {
        text: String,
        kind: MetadataBlockKind,
        span: Span,
    },
    Footnote {
        label: SharedString,
        children: Vec<BlockNode>,
        span: Span,
    },
    Table {
        table: Table,
        span: Span,
    },
    TableRow(TableRow),
    TableCell(Paragraph),
}

struct ImageCapture {
    url: SharedString,
    title: Option<SharedString>,
    alt: String,
}

struct DocumentBuilder<'a, 'cx> {
    source: &'a str,
    cx: &'cx mut NodeContext,
    frames: Vec<Frame>,
    marks: Vec<TextMark>,
    image: Option<ImageCapture>,
    skipped_custom_depth: usize,
    events: Vec<MarkdownSourceEvent>,
    footnote_definitions: HashMap<SharedString, usize>,
    heading_slugs: HashMap<SharedString, usize>,
    heading_slug_counts: HashMap<String, usize>,
}

impl<'a, 'cx> DocumentBuilder<'a, 'cx> {
    fn new(source: &'a str, cx: &'cx mut NodeContext) -> Self {
        Self {
            source,
            cx,
            frames: vec![Frame::Root(Vec::new())],
            marks: Vec::new(),
            image: None,
            skipped_custom_depth: 0,
            events: Vec::new(),
            footnote_definitions: HashMap::new(),
            heading_slugs: HashMap::new(),
            heading_slug_counts: HashMap::new(),
        }
    }

    fn consume(&mut self, event: Event<'a>, range: Range<usize>) -> Result<(), SharedString> {
        self.events.push(MarkdownSourceEvent {
            range: (self.cx.offset + range.start)..(self.cx.offset + range.end),
            kind: source_event_kind(&event),
        });
        if self.skipped_custom_depth > 0 {
            match event {
                Event::Start(_) => self.skipped_custom_depth += 1,
                Event::End(_) => self.skipped_custom_depth -= 1,
                _ => {}
            }
            return Ok(());
        }

        let extension_event = MarkdownParseEvent::new(event.clone(), range.clone());
        let parse_cx = MarkdownParseContext::new(self.source, self.cx.offset);

        if matches!(event, Event::Start(_))
            && let Some(mut custom) = self
                .cx
                .markdown_extensions
                .parse_block(&extension_event, &parse_cx)
        {
            custom.set_span(Some(self.span(range)));
            self.push_block(BlockNode::Custom(custom));
            self.skipped_custom_depth = 1;
            return Ok(());
        }

        if !matches!(event, Event::Start(_) | Event::End(_))
            && let Some(mut custom) = self
                .cx
                .markdown_extensions
                .parse_inline(&extension_event, &parse_cx)
        {
            custom.set_span(Some(self.span(range)));
            self.paragraph_mut()?.push(InlineNode::custom(custom));
            return Ok(());
        }

        match event {
            Event::Start(tag) => self.start(tag, range),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.push_text_at(&text, Some(range)),
            Event::Code(code) => {
                let mut mark = self.combined_mark();
                mark.code = true;
                self.push_inline_text_at(&code, mark, Some(range))
            }
            Event::InlineMath(math) => {
                let mut mark = self.combined_mark();
                mark.code = true;
                self.push_inline_text_at(&math, mark, Some(range))
            }
            Event::DisplayMath(math) => {
                let span = self.span(range);
                self.push_block(BlockNode::CodeBlock(CodeBlock::new(
                    math.into_string().into(),
                    Some("math".into()),
                    Some(span),
                )));
                Ok(())
            }
            Event::Html(html) => {
                if self.cx.markdown_options.parse_html {
                    self.push_html(&html, range)
                } else {
                    self.push_text(&html)
                }
            }
            Event::InlineHtml(html) => {
                if self.cx.markdown_options.parse_html {
                    self.push_inline_html(&html, range)
                } else {
                    self.push_text(&html)
                }
            }
            Event::SoftBreak => self.push_text_at(
                if self.cx.markdown_style_profile == crate::text::MarkdownStyleProfile::Agent {
                    "\n"
                } else {
                    " "
                },
                Some(range),
            ),
            Event::HardBreak => self.push_text_at("\n", Some(range)),
            Event::Rule => {
                let span = self.span(range);
                self.push_block(BlockNode::HorizontalRule { span: Some(span) });
                Ok(())
            }
            Event::TaskListMarker(checked) => {
                for frame in self.frames.iter_mut().rev() {
                    if let Frame::Item {
                        checked: item_checked,
                        ..
                    } = frame
                    {
                        *item_checked = Some(checked);
                        break;
                    }
                }
                Ok(())
            }
            Event::FootnoteReference(label) => {
                let text = format!("[{label}]");
                let mut mark = self.combined_mark();
                mark.italic = true;
                mark.footnote_reference = true;
                mark.link = Some(LinkMark {
                    url: format!("#fn-{label}").into(),
                    identifier: Some(label.into_string().into()),
                    ..Default::default()
                });
                self.push_inline_text_at(&text, mark, Some(range))
            }
        }
    }

    fn start(&mut self, tag: Tag<'a>, range: Range<usize>) -> Result<(), SharedString> {
        let span = self.span(range);
        match tag {
            Tag::Paragraph => self.frames.push(Frame::Paragraph {
                paragraph: Paragraph::default(),
                span,
            }),
            Tag::Heading { level, .. } => self.frames.push(Frame::Heading {
                level: heading_level(level),
                paragraph: Paragraph::default(),
                span,
            }),
            Tag::BlockQuote(kind) => self.frames.push(Frame::Blockquote {
                children: Vec::new(),
                kind,
                span,
            }),
            Tag::List(start) => self.frames.push(Frame::List {
                children: Vec::new(),
                ordered: start.is_some(),
                start,
                span,
            }),
            Tag::Item => self.frames.push(Frame::Item {
                children: Vec::new(),
                inline: Paragraph::default(),
                checked: None,
                span,
            }),
            Tag::CodeBlock(kind) => {
                let (language, info) = match kind {
                    CodeBlockKind::Indented => (None, None),
                    CodeBlockKind::Fenced(info) => {
                        let info: SharedString = info.trim().to_string().into();
                        let language = info
                            .split_ascii_whitespace()
                            .next()
                            .filter(|language| !language.is_empty() && !language.contains('/'))
                            .map(|language| SharedString::from(language.to_string()));
                        (language, Some(info))
                    }
                };
                self.frames.push(Frame::CodeBlock {
                    code: String::new(),
                    language,
                    info,
                    span,
                });
            }
            Tag::HtmlBlock => {
                self.frames.push(Frame::HtmlBlock {
                    html: String::new(),
                    span,
                });
            }
            Tag::MetadataBlock(kind) => self.frames.push(Frame::Metadata {
                text: String::new(),
                kind,
                span,
            }),
            Tag::FootnoteDefinition(label) => self.frames.push(Frame::Footnote {
                label: label.into_string().into(),
                children: Vec::new(),
                span,
            }),
            Tag::Table(alignments) => {
                let table = Table {
                    column_aligns: alignments.into_iter().map(Into::into).collect(),
                    ..Default::default()
                };
                self.frames.push(Frame::Table { table, span });
            }
            Tag::TableHead => self.frames.push(Frame::TableRow(TableRow::default())),
            Tag::TableRow => self.frames.push(Frame::TableRow(TableRow::default())),
            Tag::TableCell => self.frames.push(Frame::TableCell(Paragraph::default())),
            Tag::Emphasis => self.marks.push(TextMark::default().italic()),
            Tag::Strong => self.marks.push(TextMark::default().bold()),
            Tag::Strikethrough => self.marks.push(TextMark::default().strikethrough()),
            Tag::Link {
                dest_url, title, ..
            } => self.marks.push(TextMark {
                link: Some(LinkMark {
                    url: dest_url.into_string().into(),
                    title: (!title.is_empty()).then(|| title.into_string().into()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            Tag::Image {
                dest_url, title, ..
            } => {
                self.image = Some(ImageCapture {
                    url: dest_url.into_string().into(),
                    title: (!title.is_empty()).then(|| title.into_string().into()),
                    alt: String::new(),
                });
            }
            Tag::Superscript
            | Tag::Subscript
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition => {}
        }
        Ok(())
    }

    fn end(&mut self, tag: TagEnd) -> Result<(), SharedString> {
        match tag {
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                self.marks.pop();
                Ok(())
            }
            TagEnd::Image => {
                if let Some(image) = self.image.take() {
                    let link = self.combined_mark().link;
                    self.paragraph_mut()?.push_image(ImageNode {
                        url: image.url.to_string().into(),
                        link,
                        title: image.title,
                        alt: Some(image.alt.into()),
                        ..Default::default()
                    });
                }
                Ok(())
            }
            TagEnd::Superscript
            | TagEnd::Subscript
            | TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition => Ok(()),
            _ => self.close_frame(),
        }
    }

    fn close_frame(&mut self) -> Result<(), SharedString> {
        let frame = self
            .frames
            .pop()
            .ok_or_else(|| SharedString::from("unbalanced Markdown parser event"))?;
        match frame {
            Frame::Root(_) => Err("attempted to close the Markdown root".into()),
            Frame::Paragraph {
                mut paragraph,
                span,
            } => {
                paragraph.set_span(span);
                self.push_block(BlockNode::Paragraph(paragraph));
                Ok(())
            }
            Frame::Heading {
                level,
                mut paragraph,
                span,
            } => {
                if self.cx.markdown_options.parse_heading_slugs {
                    let text = paragraph.text();
                    let base = heading_slug(&text);
                    let count = self.heading_slug_counts.entry(base.clone()).or_insert(0);
                    let slug = if *count == 0 {
                        base.clone()
                    } else {
                        format!("{base}-{count}")
                    };
                    *count += 1;
                    if !slug.is_empty() && *count <= 128 {
                        let relative_start = span.start.saturating_sub(self.cx.offset);
                        let relative_end = span.end.saturating_sub(self.cx.offset);
                        let offset = self
                            .source
                            .get(relative_start..relative_end)
                            .and_then(|heading_source| heading_source.find(&text))
                            .map_or(span.start, |offset| span.start + offset);
                        self.heading_slugs.insert(slug.into(), offset);
                    }
                }
                paragraph.set_span(span);
                self.push_block(BlockNode::Heading {
                    level,
                    children: paragraph,
                    span: Some(span),
                });
                Ok(())
            }
            Frame::Blockquote {
                children,
                kind,
                span,
            } => {
                self.push_block(BlockNode::Blockquote {
                    children,
                    kind,
                    span: Some(span),
                });
                Ok(())
            }
            Frame::List {
                children,
                ordered,
                start,
                span,
            } => {
                self.push_block(BlockNode::List {
                    children,
                    ordered,
                    start,
                    span: Some(span),
                });
                Ok(())
            }
            Frame::Item {
                mut children,
                mut inline,
                checked,
                span,
            } => {
                if !inline.is_empty() {
                    inline.set_span(span);
                    children.insert(0, BlockNode::Paragraph(inline));
                }
                self.push_block(BlockNode::ListItem {
                    children,
                    spread: false,
                    checked,
                    span: Some(span),
                });
                Ok(())
            }
            Frame::CodeBlock {
                code,
                language,
                info,
                span,
            } => {
                let block = if let Some(info) = info {
                    CodeBlock::new_fenced(code.into(), info, Some(span))
                } else {
                    CodeBlock::new(code.into(), language, Some(span))
                };
                self.push_block(BlockNode::CodeBlock(block));
                Ok(())
            }
            Frame::HtmlBlock { html, span } => {
                if self.cx.markdown_options.parse_html {
                    self.push_html_block(&html, span);
                } else {
                    let mut paragraph = Paragraph::new(html);
                    paragraph.set_span(span);
                    self.push_block(BlockNode::Paragraph(paragraph));
                }
                Ok(())
            }
            Frame::Metadata { text, kind, span } => {
                let language = match kind {
                    MetadataBlockKind::YamlStyle => "yaml",
                    MetadataBlockKind::PlusesStyle => "toml",
                };
                self.push_block(BlockNode::CodeBlock(CodeBlock::new(
                    text.into(),
                    Some(language.into()),
                    Some(span),
                )));
                Ok(())
            }
            Frame::Footnote {
                label,
                children,
                span,
            } => {
                self.footnote_definitions.insert(label.clone(), span.start);
                let mut paragraph = Paragraph::default();
                let prefix = format!("[{label}]: ");
                paragraph.push(
                    InlineNode::new(prefix.clone())
                        .marks(vec![(0..prefix.len(), TextMark::default().italic())]),
                );
                for child in children {
                    let text = child.text();
                    if !text.is_empty() {
                        paragraph.push_str(text.trim_end());
                    }
                }
                paragraph.set_span(span);
                self.push_block(BlockNode::Paragraph(paragraph));
                Ok(())
            }
            Frame::Table { mut table, span } => {
                table.span = Some(span);
                self.push_block(BlockNode::Table(table));
                Ok(())
            }
            Frame::TableRow(row) => {
                match self.frames.last_mut() {
                    Some(Frame::Table { table, .. }) => table.children.push(row),
                    _ => return Err("table row outside table".into()),
                }
                Ok(())
            }
            Frame::TableCell(paragraph) => {
                let Some(Frame::TableRow(row)) = self.frames.last_mut() else {
                    return Err("table cell outside row".into());
                };
                row.children.push(TableCell {
                    children: paragraph,
                    ..Default::default()
                });
                Ok(())
            }
        }
    }

    fn push_text(&mut self, text: &str) -> Result<(), SharedString> {
        self.push_text_at(text, None)
    }

    fn push_text_at(
        &mut self,
        text: &str,
        source_range: Option<Range<usize>>,
    ) -> Result<(), SharedString> {
        if let Some(image) = &mut self.image {
            image.alt.push_str(text);
            return Ok(());
        }
        if let Some(frame) = self.frames.last_mut() {
            match frame {
                Frame::CodeBlock { code, .. }
                | Frame::HtmlBlock { html: code, .. }
                | Frame::Metadata { text: code, .. } => {
                    code.push_str(text);
                    return Ok(());
                }
                _ => {}
            }
        }

        let mark = self.combined_mark();
        if mark.link.is_some() || mark.code || text.trim().is_empty() {
            return self.push_inline_text_at(text, mark, source_range);
        }

        let mut finder = LinkFinder::new();
        finder.kinds(&[LinkKind::Url]);
        let mut cursor = 0;
        for link in finder.links(text) {
            if cursor < link.start() {
                self.push_inline_text_at(
                    &text[cursor..link.start()],
                    mark.clone(),
                    source_range
                        .as_ref()
                        .map(|range| (range.start + cursor)..(range.start + link.start())),
                )?;
            }
            let mut link_mark = mark.clone();
            link_mark.link = Some(LinkMark {
                url: link.as_str().to_string().into(),
                ..Default::default()
            });
            self.push_inline_text_at(
                link.as_str(),
                link_mark,
                source_range
                    .as_ref()
                    .map(|range| (range.start + link.start())..(range.start + link.end())),
            )?;
            cursor = link.end();
        }
        if cursor < text.len() {
            self.push_inline_text_at(
                &text[cursor..],
                mark,
                source_range.map(|range| (range.start + cursor)..range.end),
            )?;
        }
        Ok(())
    }

    fn push_inline_text_at(
        &mut self,
        text: &str,
        mark: TextMark,
        source_range: Option<Range<usize>>,
    ) -> Result<(), SharedString> {
        if text.is_empty() {
            return Ok(());
        }
        let mut node = InlineNode::new(text.to_string()).marks(vec![(0..text.len(), mark)]);
        if let Some(range) = source_range {
            node = node.source_range((self.cx.offset + range.start)..(self.cx.offset + range.end));
        }
        self.paragraph_mut()?.push(node);
        Ok(())
    }

    /// Accumulate block HTML until its frame closes; only standalone HTML is
    /// eligible for inline conversion.
    fn push_html(&mut self, html: &str, range: Range<usize>) -> Result<(), SharedString> {
        if let Some(Frame::HtmlBlock {
            html: block_html, ..
        }) = self.frames.last_mut()
        {
            block_html.push_str(html);
            return Ok(());
        }
        self.push_inline_html(html, range)
    }

    fn push_inline_html(&mut self, html: &str, range: Range<usize>) -> Result<(), SharedString> {
        if html.trim_start().to_ascii_lowercase().starts_with("<br") {
            return self.push_text("\n");
        }

        let mut html_cx = self.cx.clone();
        html_cx.offset = self.cx.offset + range.start;
        if let Ok(document) = super::html::parse(html, &mut html_cx) {
            for block in document.blocks {
                match Arc::unwrap_or_clone(block) {
                    BlockNode::Paragraph(paragraph) => {
                        for child in paragraph.children {
                            self.paragraph_mut()?.push(child);
                        }
                    }
                    BlockNode::Break { .. } => self.push_text("\n")?,
                    _ => return self.push_text(html),
                }
            }
            return Ok(());
        }
        self.push_text(html)
    }

    fn push_html_block(&mut self, html: &str, span: Span) {
        let mut html_cx = self.cx.clone();
        html_cx.offset = span.start;
        match super::html::parse(html, &mut html_cx) {
            Ok(document) => self.push_block(BlockNode::Root {
                children: document
                    .blocks
                    .into_iter()
                    .map(Arc::unwrap_or_clone)
                    .collect(),
                span: Some(span),
            }),
            Err(error) => {
                tracing::warn!(?error, "failed to parse Markdown HTML block");
                let mut paragraph = Paragraph::new(html.to_string());
                paragraph.set_span(span);
                self.push_block(BlockNode::Paragraph(paragraph));
            }
        }
    }

    fn paragraph_mut(&mut self) -> Result<&mut Paragraph, SharedString> {
        for frame in self.frames.iter_mut().rev() {
            match frame {
                Frame::Paragraph { paragraph, .. }
                | Frame::Heading { paragraph, .. }
                | Frame::TableCell(paragraph)
                | Frame::Item {
                    inline: paragraph, ..
                } => return Ok(paragraph),
                _ => {}
            }
        }
        Err("inline Markdown event outside a text container".into())
    }

    fn combined_mark(&self) -> TextMark {
        let mut combined = TextMark::default();
        for mark in &self.marks {
            combined.merge(mark.clone());
        }
        combined
    }

    fn push_block(&mut self, block: BlockNode) {
        for frame in self.frames.iter_mut().rev() {
            match frame {
                Frame::Root(children)
                | Frame::Blockquote { children, .. }
                | Frame::List { children, .. }
                | Frame::Item { children, .. }
                | Frame::Footnote { children, .. } => {
                    children.push(block);
                    return;
                }
                _ => {}
            }
        }
    }

    fn span(&self, range: Range<usize>) -> Span {
        Span {
            start: self.cx.offset + range.start,
            end: self.cx.offset + range.end,
        }
    }

    fn finish(&mut self) -> Result<ParsedDocument, SharedString> {
        while self.frames.len() > 1 {
            self.close_frame()?;
        }
        let Some(Frame::Root(children)) = self.frames.pop() else {
            return Err("missing Markdown root".into());
        };
        Ok(ParsedDocument {
            source: self.source.to_string().into(),
            root_block_starts: children
                .iter()
                .filter_map(BlockNode::span)
                .map(|span| span.start)
                .collect::<Vec<_>>()
                .into(),
            blocks: children.into_iter().map(Arc::new).collect(),
            events: std::mem::take(&mut self.events).into(),
            heading_slugs: Arc::new(std::mem::take(&mut self.heading_slugs)),
            footnote_definitions: Arc::new(std::mem::take(&mut self.footnote_definitions)),
        })
    }

    /// Finish the parser without exposing builder-state failures to the UI.
    fn finish_or_literal(mut self) -> ParsedDocument {
        match self.finish() {
            Ok(document) => document,
            Err(error) => {
                tracing::warn!(?error, "falling back to literal Markdown text");
                let mut paragraph = Paragraph::new(self.source.to_string());
                paragraph.set_span(Span {
                    start: self.cx.offset,
                    end: self.cx.offset + self.source.len(),
                });
                ParsedDocument {
                    source: self.source.to_string().into(),
                    blocks: vec![Arc::new(BlockNode::Paragraph(paragraph))],
                    events: std::mem::take(&mut self.events).into(),
                    ..Default::default()
                }
            }
        }
    }
}

fn heading_slug(text: &str) -> String {
    text.trim()
        .chars()
        .filter_map(|character| {
            if character.is_alphanumeric() || character == '-' || character == '_' {
                Some(character.to_lowercase().next().unwrap_or(character))
            } else if character == ' ' {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

fn source_event_kind(event: &Event<'_>) -> MarkdownSourceEventKind {
    match event {
        Event::Start(_) => MarkdownSourceEventKind::Start,
        Event::End(_) => MarkdownSourceEventKind::End,
        Event::Text(_) => MarkdownSourceEventKind::Text,
        Event::Code(_) => MarkdownSourceEventKind::Code,
        Event::Html(_) | Event::InlineHtml(_) => MarkdownSourceEventKind::Html,
        Event::SoftBreak => MarkdownSourceEventKind::SoftBreak,
        Event::HardBreak => MarkdownSourceEventKind::HardBreak,
        Event::Rule => MarkdownSourceEventKind::Rule,
        Event::TaskListMarker(_) => MarkdownSourceEventKind::TaskListMarker,
        Event::FootnoteReference(_) => MarkdownSourceEventKind::FootnoteReference,
        Event::InlineMath(_) | Event::DisplayMath(_) => MarkdownSourceEventKind::Math,
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestInlineBadgePlugin;

    impl crate::text::MarkdownPlugin for TestInlineBadgePlugin {
        fn name(&self) -> &str {
            "inline-badge"
        }

        fn parse(
            &self,
            event: &crate::text::MarkdownParseEvent<'_>,
            cx: &crate::text::MarkdownParseContext<'_>,
        ) -> Option<crate::text::MarkdownNode> {
            let Event::Code(code) = event.event() else {
                return None;
            };
            let label = code.strip_prefix("badge:")?.trim().to_string();
            Some(
                crate::text::MarkdownNode::new("inline-badge", label.clone())
                    .text(label)
                    .markdown(cx.event_source(event).unwrap_or_default()),
            )
        }

        fn render(
            &self,
            _: &crate::text::MarkdownNode,
            _: &mut gpui::Window,
            _: &mut gpui::App,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }

    #[test]
    fn parses_nested_emphasis_and_plain_urls() {
        let mut cx = NodeContext::default();
        let document = parse(
            "This has **_nested_** text and https://example.com.",
            &mut cx,
        )
        .unwrap();
        let BlockNode::Paragraph(paragraph) = &*document.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(paragraph.children.iter().any(|node| {
            node.text.as_ref() == "nested"
                && node.marks.iter().any(|(_, mark)| mark.bold && mark.italic)
        }));
        assert!(paragraph.children.iter().any(|node| {
            node.text.as_ref() == "https://example.com"
                && node.marks.iter().any(|(_, mark)| mark.link.is_some())
        }));
    }

    #[test]
    fn parses_tables_tasks_and_source_ranges() {
        let source = "- [x] done\n\n| A | B |\n|:-|:-:|\n| 1 | 2 |";
        let mut cx = NodeContext::default();
        let document = parse(source, &mut cx).unwrap();
        assert!(matches!(&*document.blocks[0], BlockNode::List { .. }));
        assert!(matches!(&*document.blocks[1], BlockNode::Table(_)));
        assert_eq!(document.source.as_ref(), source);
        assert!(!document.events.is_empty());
        assert_eq!(document.root_block_starts.as_ref(), &[0, 12]);
    }

    #[test]
    fn preserves_ordered_start_callout_kind_and_footnote_targets() {
        let source = "7. seven\n8. eight\n\n> [!WARNING]\n> Careful\n\nref[^a]\n\n[^a]: note";
        let mut cx = NodeContext::default();
        let document = parse(source, &mut cx).unwrap();

        let BlockNode::List { start, .. } = &*document.blocks[0] else {
            panic!("expected ordered list");
        };
        assert_eq!(*start, Some(7));
        assert!(matches!(
            &*document.blocks[1],
            BlockNode::Blockquote {
                kind: Some(BlockQuoteKind::Warning),
                ..
            }
        ));
        assert!(document.footnote_definitions.contains_key("a"));
        assert!(document.events.iter().any(|event| {
            event.kind == MarkdownSourceEventKind::FootnoteReference
                && &source[event.range.clone()] == "[^a]"
        }));
    }

    #[test]
    fn parser_options_gate_html_metadata_and_mermaid() {
        let mut disabled = NodeContext::default();
        let html = parse("<b>bold</b>", &mut disabled).unwrap();
        assert!(html.text().contains("<b>bold</b>"));

        let mut enabled = NodeContext {
            markdown_options: MarkdownOptions {
                parse_html: true,
                render_metadata_blocks: true,
                render_mermaid_diagrams: true,
                ..Default::default()
            },
            ..NodeContext::default()
        };
        let parsed = parse("---\ntitle: Test\n---\n\n<b>bold</b>", &mut enabled).unwrap();
        assert!(matches!(&*parsed.blocks[0], BlockNode::CodeBlock(_)));
        assert!(parsed.text().contains("bold"));
    }

    #[test]
    fn parses_unclosed_fence_for_streaming_display() {
        let mut cx = NodeContext::default();
        let document = parse("```rust\nfn main() {}", &mut cx).unwrap();
        let BlockNode::CodeBlock(code) = &*document.blocks[0] else {
            panic!("expected code block");
        };
        assert_eq!(code.lang().as_deref(), Some("rust"));
        assert_eq!(code.code().as_ref(), "fn main() {}");
    }

    #[test]
    fn keeps_definition_list_syntax_literal_like_zed() {
        let mut cx = NodeContext::default();
        let document = parse("Term\n: Definition", &mut cx).unwrap();

        assert_eq!(document.text(), "Term\n: Definition\n");
    }

    #[test]
    fn parses_every_story_stream_prefix_with_inline_extensions() {
        let fixture = include_str!("../../../../story/examples/fixtures/test.md");
        let source = format!(
            "Streaming repairs **strong text**, `inline code`, and [pending links](https://example.com/path) before their closers arrive.\n\n{fixture}"
        );
        let extensions = crate::text::MarkdownExtensions::default().plugin(TestInlineBadgePlugin);

        let mut end = 0;
        while end < source.len() {
            end = (end + 24).min(source.len());
            while !source.is_char_boundary(end) {
                end -= 1;
            }
            let mut cx = NodeContext {
                markdown_extensions: Arc::new(extensions.clone()),
                ..NodeContext::default()
            };
            parse(&source[..end], &mut cx).unwrap_or_else(|error| {
                let tail: String = source[..end]
                    .chars()
                    .rev()
                    .take(200)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                panic!("prefix ending at byte {end} failed: {error}\nTAIL:\n{tail}")
            });
        }
    }
}
