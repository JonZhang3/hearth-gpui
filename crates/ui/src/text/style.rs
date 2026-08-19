use std::{collections::HashMap, sync::Arc};

use gpui::{
    FontStyle, FontWeight, HighlightStyle, Hsla, Pixels, Rems, SharedString, StrikethroughStyle,
    StyleRefinement, Styled as _, UnderlineStyle, px, rems,
};

use crate::{ActiveTheme as _, highlighter::HighlightTheme};

/// Legacy TextView profile retained only for the HTML renderer migration seam.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MarkdownStyleProfile {
    #[default]
    Agent,
    Editor,
    Preview,
}

/// Horizontal padding applied around inline code text in a Markdown chip.
///
/// Mirrors the shadcn `Kbd` primitive (`px-1`, four pixels at the default
/// density) and keeps both Markdown render paths visually identical.
pub(crate) const INLINE_CODE_PADDING_X: f32 = 4.;

/// Inline code renders at 87.5% of the surrounding body text size, matching
/// the shadcn prose `code` rule (`font-size: 0.875em`).
pub(crate) const INLINE_CODE_SIZE_SCALE: f32 = 0.875;

/// Resolve the inline code background for the active Hearth theme.
///
/// Keeps the GitHub/Zed foreground-chip convention instead of the shadcn
/// `bg-muted` pill: a muted background without a border has too little
/// contrast in dark themes. The single source of truth lives here so both
/// render paths stay in sync.
pub(crate) fn inline_code_background(cx: &gpui::App) -> Hsla {
    cx.theme().foreground.opacity(0.08)
}

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
pub(crate) enum MarkdownHeadingLevel {
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
pub(crate) enum MarkdownElementKind {
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
pub(crate) enum MarkdownInlineKind {
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
pub(crate) struct MarkdownTextStyle {
    color: Option<Hsla>,
    font_weight: Option<FontWeight>,
    font_style: Option<FontStyle>,
    background_color: Option<Option<Hsla>>,
    underline: Option<Option<UnderlineStyle>>,
    strikethrough: Option<Option<StrikethroughStyle>>,
    fade_out: Option<f32>,
    padding_x: Option<Pixels>,
    padding_y: Option<Pixels>,
    margin_x: Option<Pixels>,
    margin_y: Option<Pixels>,
    corner_radius: Option<Pixels>,
    border_width: Option<Pixels>,
    border_color: Option<Hsla>,
    font_family: Option<SharedString>,
    font_size: Option<Pixels>,
    line_height: Option<Pixels>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct InlineBoxMetrics {
    pub(crate) padding_x: Pixels,
    pub(crate) padding_y: Pixels,
    pub(crate) margin_x: Pixels,
    pub(crate) margin_y: Pixels,
    pub(crate) corner_radius: Pixels,
    pub(crate) border_width: Pixels,
    pub(crate) border_color: Option<Hsla>,
    pub(crate) font_family: Option<SharedString>,
    pub(crate) font_size: Option<Pixels>,
    pub(crate) line_height: Option<Pixels>,
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

    /// Set the font family for an atomic inline semantic box.
    pub fn font_family(mut self, family: impl Into<SharedString>) -> Self {
        self.font_family = Some(family.into());
        self
    }

    /// Set the font size for an atomic inline semantic box.
    pub fn font_size(mut self, size: Pixels) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Set the line height for an atomic inline semantic box.
    pub fn line_height(mut self, height: Pixels) -> Self {
        self.line_height = Some(height);
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

    /// Set vertical padding for an inline semantic box.
    pub fn padding_y(mut self, padding: Pixels) -> Self {
        self.padding_y = Some(padding);
        self
    }

    /// Set horizontal margin for an inline semantic box.
    pub fn margin_x(mut self, margin: Pixels) -> Self {
        self.margin_x = Some(margin);
        self
    }

    /// Set vertical margin for an inline semantic box.
    pub fn margin_y(mut self, margin: Pixels) -> Self {
        self.margin_y = Some(margin);
        self
    }

    /// Set the corner radius for an inline semantic box.
    pub fn corner_radius(mut self, radius: Pixels) -> Self {
        self.corner_radius = Some(radius);
        self
    }

    /// Set border width and color for an inline semantic box.
    pub fn border(mut self, width: Pixels, color: Hsla) -> Self {
        self.border_width = Some(width);
        self.border_color = Some(color);
        self
    }

    pub(crate) fn inline_box_metrics(&self) -> Option<InlineBoxMetrics> {
        (self.padding_x.is_some()
            || self.padding_y.is_some()
            || self.margin_x.is_some()
            || self.margin_y.is_some()
            || self.corner_radius.is_some()
            || self.border_width.is_some()
            || self.font_family.is_some()
            || self.font_size.is_some()
            || self.line_height.is_some())
        .then_some(InlineBoxMetrics {
            padding_x: self.padding_x.unwrap_or_default(),
            padding_y: self.padding_y.unwrap_or_default(),
            margin_x: self.margin_x.unwrap_or_default(),
            margin_y: self.margin_y.unwrap_or_default(),
            corner_radius: self.corner_radius.unwrap_or_default(),
            border_width: self.border_width.unwrap_or_default(),
            border_color: self.border_color,
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            line_height: self.line_height,
        })
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

    /// Apply properties that cannot change glyph geometry or line wrapping.
    ///
    /// Hover must remain paint-only so moving the pointer never changes a
    /// paragraph's measured height or pushes adjacent text onto another line.
    pub(crate) fn refine_paint(&self, style: &mut HighlightStyle) {
        if let Some(color) = self.color {
            style.color = Some(color);
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
pub(crate) struct LegacyMarkdownStyle {
    /// Body typography inherited by ordinary Markdown text.
    ///
    /// Keeping the line height on the document container prevents ordinary text
    /// from being treated as an atomic inline box.
    base_text_style: StyleRefinement,
    elements: HashMap<MarkdownElementKind, StyleRefinement>,
    inline: HashMap<MarkdownInlineKind, MarkdownTextStyle>,
    syntax_theme: Option<Arc<HighlightTheme>>,
}

impl LegacyMarkdownStyle {
    /// Build a semantic style profile using the active Hearth theme.
    pub(crate) fn for_profile(profile: MarkdownStyleProfile, cx: &gpui::App) -> Self {
        let body_size = cx.theme().font_size;
        let heading_scales = match profile {
            MarkdownStyleProfile::Agent => [1.15, 1.10, 1.05, 1.00, 0.95, 0.875],
            MarkdownStyleProfile::Editor => [1.30, 1.20, 1.10, 1.00, 0.95, 0.90],
            MarkdownStyleProfile::Preview => [1.45, 1.30, 1.10, 1.01, 0.95, 0.85],
        };
        let headings = [
            MarkdownHeadingLevel::H1,
            MarkdownHeadingLevel::H2,
            MarkdownHeadingLevel::H3,
            MarkdownHeadingLevel::H4,
            MarkdownHeadingLevel::H5,
            MarkdownHeadingLevel::H6,
        ];

        let inline_code = MarkdownTextStyle::default()
            .font_family(cx.theme().mono_font_family.clone())
            .font_size(body_size * INLINE_CODE_SIZE_SCALE)
            .background(inline_code_background(cx))
            .padding_x(px(INLINE_CODE_PADDING_X))
            .corner_radius(cx.theme().style.radii.sm);
        // Preview prose is muted; inline code keeps the full foreground color
        // so code stays legible inside weakened body text.
        let inline_code = if profile == MarkdownStyleProfile::Preview {
            inline_code.color(cx.theme().foreground)
        } else {
            inline_code
        };

        let mut style = Self {
            base_text_style: StyleRefinement::default().line_height(body_size * 1.75),
            ..Self::default()
        }
        .inline(
            MarkdownInlineKind::Link,
            MarkdownTextStyle::default()
                .color(cx.theme().link)
                .background(cx.theme().foreground.opacity(0.025))
                .underline(UnderlineStyle {
                    color: Some(cx.theme().link.opacity(0.5)),
                    thickness: px(1.),
                    ..Default::default()
                }),
        )
        .inline(
            MarkdownInlineKind::LinkHover,
            MarkdownTextStyle::default().color(cx.theme().link.opacity(0.8)),
        )
        .inline(MarkdownInlineKind::InlineCode, inline_code)
        .element(
            MarkdownElementKind::CodeBlock,
            StyleRefinement::default()
                .p_2()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().tokens.background)
                .rounded(cx.theme().style.radii.sm),
        )
        .element(
            MarkdownElementKind::HorizontalRule,
            StyleRefinement::default().bg(cx.theme().border),
        )
        .element(
            MarkdownElementKind::Paragraph,
            StyleRefinement::default().line_height(body_size * 1.3),
        );

        if profile == MarkdownStyleProfile::Preview {
            style = style.element(
                MarkdownElementKind::Document,
                StyleRefinement::default().text_color(cx.theme().muted_foreground),
            );
        }
        for (heading, scale) in headings.into_iter().zip(heading_scales) {
            style = style.element(
                MarkdownElementKind::Heading(heading),
                StyleRefinement::default()
                    .text_size(body_size * scale)
                    .text_color(cx.theme().foreground),
            );
        }
        style
    }

    /// Overlay caller refinements on top of a built-in profile.
    pub(crate) fn overlay(mut self, overrides: &Self) -> Self {
        if overrides.base_text_style != StyleRefinement::default() {
            self.base_text_style = overrides.base_text_style.clone();
        }
        self.elements.extend(overrides.elements.clone());
        self.inline.extend(overrides.inline.clone());
        if let Some(theme) = &overrides.syntax_theme {
            self.syntax_theme = Some(theme.clone());
        }
        self
    }

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

    pub(crate) fn base_text_style(&self) -> &StyleRefinement {
        &self.base_text_style
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
    use gpui::{TestAppContext, hsla, px};

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
            .padding_y(px(2.))
            .margin_x(px(1.))
            .font_family("Mono")
            .font_size(px(13.))
            .line_height(px(18.))
            .corner_radius(px(6.));

        assert_eq!(
            style.inline_box_metrics(),
            Some(InlineBoxMetrics {
                padding_x: px(4.),
                padding_y: px(2.),
                margin_x: px(1.),
                corner_radius: px(6.),
                font_family: Some("Mono".into()),
                font_size: Some(px(13.)),
                line_height: Some(px(18.)),
                ..Default::default()
            })
        );
    }

    #[gpui::test]
    fn inline_code_profiles_resolve_semantic_metrics(cx: &mut TestAppContext) {
        cx.update(crate::init);
        cx.update(|cx| {
            let theme = cx.theme();
            let editor = LegacyMarkdownStyle::for_profile(MarkdownStyleProfile::Editor, cx);
            let inline = editor
                .inline_style(MarkdownInlineKind::InlineCode)
                .expect("editor profile defines inline code");
            assert_eq!(inline.font_family, Some(theme.mono_font_family.clone()));
            assert_eq!(
                inline.font_size,
                Some(theme.font_size * INLINE_CODE_SIZE_SCALE)
            );
            assert_eq!(
                inline.background_color,
                Some(Some(inline_code_background(cx)))
            );
            assert_eq!(inline.padding_x, Some(px(INLINE_CODE_PADDING_X)));
            assert_eq!(inline.corner_radius, Some(theme.style.radii.sm));
            // Editor and Agent inherit the body color instead of overriding it.
            assert_eq!(inline.color, None);

            let preview = LegacyMarkdownStyle::for_profile(MarkdownStyleProfile::Preview, cx);
            let preview_inline = preview
                .inline_style(MarkdownInlineKind::InlineCode)
                .expect("preview profile defines inline code");
            assert_eq!(preview_inline.color, Some(theme.foreground));
        });
    }
}
