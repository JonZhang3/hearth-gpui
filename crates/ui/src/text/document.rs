use std::{collections::HashMap, ops::Range, sync::Arc};

use gpui::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, ScrollAnchor, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _,
};

use crate::{
    StyledExt as _,
    text::{
        MarkdownElementKind,
        node::{BlockNode, NodeContext},
    },
};

/// The parsed document AST.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ParsedDocument {
    pub(crate) source: SharedString,
    pub(crate) blocks: Vec<Arc<BlockNode>>,
    pub(crate) events: Arc<[MarkdownSourceEvent]>,
    pub(crate) root_block_starts: Arc<[usize]>,
    pub(crate) heading_slugs: Arc<HashMap<SharedString, usize>>,
    pub(crate) footnote_definitions: Arc<HashMap<SharedString, usize>>,
}

/// One parser event tied to its canonical Markdown byte range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MarkdownSourceEvent {
    pub(crate) range: Range<usize>,
    pub(crate) kind: MarkdownSourceEventKind,
}

/// Stable event categories retained after parser lifetimes end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarkdownSourceEventKind {
    Start,
    End,
    Text,
    Code,
    Html,
    SoftBreak,
    HardBreak,
    Rule,
    TaskListMarker,
    FootnoteReference,
    Math,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct NodeRenderOptions {
    pub(crate) ix: usize,
    pub(crate) in_list: bool,
    pub(crate) todo: bool,
    pub(crate) ordered: bool,
    pub(crate) ordered_start: u64,
    pub(crate) depth: usize,
    pub(crate) is_last: bool,
}

impl NodeRenderOptions {
    pub(crate) fn is_last(mut self, is_last: bool) -> Self {
        self.is_last = is_last;
        self
    }
}

impl ParsedDocument {
    pub(super) fn text(&self) -> String {
        let mut text = String::new();
        for block in self.blocks.iter() {
            text.push_str(&block.text());
        }
        text
    }

    pub(super) fn selected_text(&self) -> String {
        let mut text = String::new();
        for block in self.blocks.iter() {
            text.push_str(&block.selected_text());
        }
        text
    }

    pub(super) fn selected_source(&self) -> Option<&str> {
        let mut ranges = Vec::new();
        for block in &self.blocks {
            block.selected_source_ranges(&mut ranges);
        }
        let start = ranges.iter().map(|range| range.start).min()?;
        let end = ranges.iter().map(|range| range.end).max()?;
        self.source.get(start..end)
    }

    /// Synchronously clear the selection stored in every inline state.
    ///
    /// This mirrors the [`selected_text`](Self::selected_text) traversal so the
    /// stored selection can be cleared without relying on a repaint. Offscreen
    /// (virtualized) views do not repaint, so their `InlineState.selection`
    /// would otherwise retain stale values from the last painted frame.
    pub(super) fn clear_selection(&self) {
        for block in self.blocks.iter() {
            block.clear_selection();
        }
    }

    /// Converts the node to markdown format.
    ///
    /// This is used to generate markdown for test.
    #[allow(dead_code)]
    pub(crate) fn to_markdown(&self) -> String {
        self.blocks
            .iter()
            .map(|child| child.to_markdown())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub(super) fn render_root(
        &self,
        scroll_anchors: &[ScrollAnchor],
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let blocks_len = self.blocks.len();
        div()
            .id("document")
            .refine_style(node_cx.markdown_style.base_text_style())
            .when_some(
                node_cx
                    .markdown_style
                    .element_style(MarkdownElementKind::Document),
                |this, style| this.refine_style(style),
            )
            .children(self.blocks.iter().enumerate().map(move |(ix, node)| {
                let is_last = ix + 1 == blocks_len;
                div()
                    .id(("root-block", ix))
                    .w_full()
                    .when_some(scroll_anchors.get(ix).cloned(), |this, anchor| {
                        this.anchor_scroll(Some(anchor))
                    })
                    .child(node.render_block(
                        NodeRenderOptions {
                            ix,
                            is_last,
                            ..Default::default()
                        },
                        node_cx,
                        window,
                        cx,
                    ))
            }))
    }
}
