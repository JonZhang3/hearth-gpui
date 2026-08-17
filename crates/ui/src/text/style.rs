use std::{collections::HashMap, sync::Arc};

use gpui::{
    FontStyle, FontWeight, HighlightStyle, Hsla, Pixels, Rems, StrikethroughStyle, StyleRefinement,
    UnderlineStyle, px, rems,
};

use crate::highlighter::HighlightTheme;

/// TextViewStyle used to customize the style for [`TextView`].
#[derive(Clone)]
pub struct TextViewStyle {
    /// Gap of each paragraphs, default is 1 rem.
    pub paragraph_gap: Rems,
    /// Base font size for headings, default is 14px.
    pub heading_base_font_size: Pixels,
    /// Function to calculate heading font size based on heading level (1-6).
    ///
    /// The first parameter is the heading level (1-6), the second parameter is the base font size.
    /// The second parameter is the base font size.
    pub heading_font_size: Option<Arc<dyn Fn(u8, Pixels) -> Pixels + Send + Sync + 'static>>,
    /// Highlight theme for code blocks. Default: [`HighlightTheme::default_light()`]
    pub highlight_theme: Arc<HighlightTheme>,
    /// The style refinement for code blocks.
    pub code_block: StyleRefinement,
    /// Style refinement applied to the table container (the bordered wrapper
    /// in wrap mode, the scroll viewport in horizontal-scroll mode).
    ///
    /// Set `overflow_x: scroll` here for adaptive table layout: columns fit
    /// their content when space allows, shrink (wrapping cell text) down to a
    /// per-column floor when the frame is narrower, and below that the table
    /// scrolls horizontally instead of squeezing further, e.g.
    /// `TextViewStyle::default().table({ let mut s = StyleRefinement::default(); s.overflow.x = Some(Overflow::Scroll); s })`.
    pub table: StyleRefinement,
    /// Style refinement applied to each table cell.
    ///
    /// With the scroll layout, set `white_space: nowrap` here to keep cells
    /// on a single line — columns then never shrink and the table scrolls as
    /// soon as the content is wider than the frame.
    pub table_cell: StyleRefinement,
    pub is_dark: bool,
}

impl PartialEq for TextViewStyle {
    fn eq(&self, other: &Self) -> bool {
        self.paragraph_gap == other.paragraph_gap
            && self.heading_base_font_size == other.heading_base_font_size
            && self.highlight_theme == other.highlight_theme
    }
}

impl Default for TextViewStyle {
    fn default() -> Self {
        Self {
            paragraph_gap: rems(1.),
            heading_base_font_size: px(14.),
            heading_font_size: None,
            highlight_theme: HighlightTheme::default_light().clone(),
            code_block: StyleRefinement::default(),
            table: StyleRefinement::default(),
            table_cell: StyleRefinement::default(),
            is_dark: false,
        }
    }
}

impl TextViewStyle {
    /// Set paragraph gap, default is 1 rem.
    pub fn paragraph_gap(mut self, gap: Rems) -> Self {
        self.paragraph_gap = gap;
        self
    }

    pub fn heading_font_size<F>(mut self, f: F) -> Self
    where
        F: Fn(u8, Pixels) -> Pixels + Send + Sync + 'static,
    {
        self.heading_font_size = Some(Arc::new(f));
        self
    }

    /// Set style for code blocks.
    pub fn code_block(mut self, style: StyleRefinement) -> Self {
        self.code_block = style;
        self
    }

    /// Set extra style for the table container.
    ///
    /// Set `overflow_x: scroll` on the refinement for adaptive layout: cells
    /// wrap as the frame narrows, and once columns reach their minimum width
    /// the table scrolls horizontally instead of shrinking further.
    pub fn table(mut self, style: StyleRefinement) -> Self {
        self.table = style;
        self
    }

    /// Set extra style for each table cell.
    ///
    /// With the scroll table layout, `white_space: nowrap` here keeps cells
    /// on a single line and the table scrolls whenever the content is wider
    /// than the frame.
    pub fn table_cell(mut self, style: StyleRefinement) -> Self {
        self.table_cell = style;
        self
    }
}

/// A Markdown heading level used by semantic styling and renderers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkdownHeadingLevel {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl MarkdownHeadingLevel {
    pub(crate) fn from_depth(depth: u8) -> Self {
        match depth {
            1 => Self::H1,
            2 => Self::H2,
            3 => Self::H3,
            4 => Self::H4,
            5 => Self::H5,
            _ => Self::H6,
        }
    }
}

/// A visible Markdown element that can receive a GPUI style refinement.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkdownElementKind {
    Document,
    Paragraph,
    Heading(MarkdownHeadingLevel),
    Blockquote,
    OrderedList,
    UnorderedList,
    ListItem,
    ListMarker,
    TaskCheckbox,
    CodeBlock,
    CodeBlockActions,
    Table,
    TableHeaderRow,
    TableBodyRow,
    TableHeaderCell,
    TableCell,
    Image,
    HorizontalRule,
}

/// An inline Markdown semantic that can receive text styling.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkdownInlineKind {
    Plain,
    Strong,
    Emphasis,
    Strikethrough,
    Underline,
    InlineCode,
    Link,
    LinkHover,
    Mark,
    FootnoteReference,
    CodeBlockText,
}

/// A partial inline text style.
///
/// Builder methods only change the fields they name. The `no_*` methods are
/// explicit removals and therefore differ from leaving a field inherited.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarkdownTextStyle {
    color: Option<Hsla>,
    font_weight: Option<FontWeight>,
    font_style: Option<FontStyle>,
    background_color: Option<Option<Hsla>>,
    underline: Option<Option<UnderlineStyle>>,
    strikethrough: Option<Option<StrikethroughStyle>>,
    fade_out: Option<f32>,
    padding_x: Option<Pixels>,
    corner_radius: Option<Pixels>,
}

impl MarkdownTextStyle {
    /// Set the foreground color.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the font weight.
    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.font_weight = Some(weight);
        self
    }

    /// Set the font style.
    pub fn font_style(mut self, style: FontStyle) -> Self {
        self.font_style = Some(style);
        self
    }

    /// Set the background color.
    pub fn background(mut self, color: Hsla) -> Self {
        self.background_color = Some(Some(color));
        self
    }

    /// Remove an inherited background color.
    pub fn no_background(mut self) -> Self {
        self.background_color = Some(None);
        self
    }

    /// Set the underline style.
    pub fn underline(mut self, style: UnderlineStyle) -> Self {
        self.underline = Some(Some(style));
        self
    }

    /// Remove an inherited underline.
    pub fn no_underline(mut self) -> Self {
        self.underline = Some(None);
        self
    }

    /// Set the strikethrough style.
    pub fn strikethrough(mut self, style: StrikethroughStyle) -> Self {
        self.strikethrough = Some(Some(style));
        self
    }

    /// Remove an inherited strikethrough.
    pub fn no_strikethrough(mut self) -> Self {
        self.strikethrough = Some(None);
        self
    }

    /// Fade the text by the provided factor.
    pub fn fade_out(mut self, factor: f32) -> Self {
        self.fade_out = Some(factor);
        self
    }

    /// Set horizontal padding for an inline semantic box.
    pub fn padding_x(mut self, padding: Pixels) -> Self {
        self.padding_x = Some(padding);
        self
    }

    /// Set the corner radius for an inline semantic box.
    pub fn corner_radius(mut self, radius: Pixels) -> Self {
        self.corner_radius = Some(radius);
        self
    }

    pub(crate) fn inline_box_metrics(&self) -> Option<(Pixels, Pixels)> {
        (self.padding_x.is_some() || self.corner_radius.is_some()).then_some((
            self.padding_x.unwrap_or_default(),
            self.corner_radius.unwrap_or_default(),
        ))
    }

    pub(crate) fn refine(&self, style: &mut HighlightStyle) {
        if let Some(color) = self.color {
            style.color = Some(color);
        }
        if let Some(weight) = self.font_weight {
            style.font_weight = Some(weight);
        }
        if let Some(font_style) = self.font_style {
            style.font_style = Some(font_style);
        }
        if let Some(background) = self.background_color {
            style.background_color = background;
        }
        if let Some(underline) = self.underline {
            style.underline = underline;
        }
        if let Some(strikethrough) = self.strikethrough {
            style.strikethrough = strikethrough;
        }
        if let Some(fade_out) = self.fade_out {
            style.fade_out = Some(fade_out);
        }
    }
}

/// Complete semantic styling for Markdown rendered by [`crate::text::TextView`].
///
/// This type complements [`TextViewStyle`]. Its private storage keeps existing
/// `TextViewStyle` struct literals source-compatible and allows new semantic
/// selectors to be added without changing callers.
#[derive(Clone, Default)]
pub struct MarkdownStyle {
    elements: HashMap<MarkdownElementKind, StyleRefinement>,
    inline: HashMap<MarkdownInlineKind, MarkdownTextStyle>,
    syntax_theme: Option<Arc<HighlightTheme>>,
}

impl MarkdownStyle {
    /// Refine the style of a visible Markdown element.
    pub fn element(mut self, kind: MarkdownElementKind, style: StyleRefinement) -> Self {
        self.elements.insert(kind, style);
        self
    }

    /// Refine an inline Markdown semantic.
    pub fn inline(mut self, kind: MarkdownInlineKind, style: MarkdownTextStyle) -> Self {
        self.inline.insert(kind, style);
        self
    }

    /// Override syntax highlighting for code blocks in this view.
    pub fn syntax_theme(mut self, theme: Arc<HighlightTheme>) -> Self {
        self.syntax_theme = Some(theme);
        self
    }

    pub(crate) fn element_style(&self, kind: MarkdownElementKind) -> Option<&StyleRefinement> {
        self.elements.get(&kind)
    }

    pub(crate) fn inline_style(&self, kind: MarkdownInlineKind) -> Option<&MarkdownTextStyle> {
        self.inline.get(&kind)
    }

    pub(crate) fn syntax_theme_ref(&self) -> Option<&Arc<HighlightTheme>> {
        self.syntax_theme.as_ref()
    }

    /// Capture the inline-only portion used by paragraph render caches.
    pub(crate) fn inline_snapshot(&self) -> Vec<(MarkdownInlineKind, MarkdownTextStyle)> {
        const KINDS: &[MarkdownInlineKind] = &[
            MarkdownInlineKind::Plain,
            MarkdownInlineKind::Strong,
            MarkdownInlineKind::Emphasis,
            MarkdownInlineKind::Strikethrough,
            MarkdownInlineKind::Underline,
            MarkdownInlineKind::InlineCode,
            MarkdownInlineKind::Link,
            MarkdownInlineKind::Mark,
            MarkdownInlineKind::FootnoteReference,
        ];
        KINDS
            .iter()
            .filter_map(|kind| {
                self.inline_style(*kind)
                    .cloned()
                    .map(|style| (*kind, style))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{hsla, px};

    #[test]
    fn markdown_text_style_can_replace_and_clear_inherited_values() {
        let mut resolved = HighlightStyle {
            background_color: Some(hsla(0., 0., 0., 1.)),
            underline: Some(UnderlineStyle {
                thickness: px(1.),
                ..Default::default()
            }),
            ..Default::default()
        };

        MarkdownTextStyle::default()
            .color(hsla(0.6, 0.8, 0.5, 1.))
            .no_background()
            .no_underline()
            .refine(&mut resolved);

        assert_eq!(resolved.color, Some(hsla(0.6, 0.8, 0.5, 1.)));
        assert_eq!(resolved.background_color, None);
        assert_eq!(resolved.underline, None);
    }

    #[test]
    fn markdown_text_style_exposes_inline_box_metrics() {
        let style = MarkdownTextStyle::default()
            .padding_x(px(4.))
            .corner_radius(px(6.));

        assert_eq!(style.inline_box_metrics(), Some((px(4.), px(6.))));
    }
}
