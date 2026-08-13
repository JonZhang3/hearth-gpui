use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative,
};

use crate::{ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _, theme::Density};

/// Component-local geometry matching shadcn's Empty styles without expanding
/// the shared StylePreset contract for a single consumer.
#[derive(Debug, Clone, Copy, PartialEq)]
struct EmptyMetrics {
    root_padding: gpui::Pixels,
    root_gap: gpui::Pixels,
    root_radius: gpui::Pixels,
    header_gap: gpui::Pixels,
    media_margin_bottom: gpui::Pixels,
    icon_media_size: gpui::Pixels,
    icon_size: gpui::Pixels,
    icon_radius: gpui::Pixels,
    content_gap: gpui::Pixels,
    compact_title: bool,
}

impl EmptyMetrics {
    /// Resolves Empty geometry from semantic density and radius values.
    fn resolve(style: &crate::theme::StylePreset) -> Self {
        let compact = style.density == Density::Compact;

        Self {
            root_padding: if compact { px(24.) } else { px(48.) },
            root_gap: px(16.),
            root_radius: if compact {
                style.radii.xl
            } else {
                style.radii.lg
            },
            header_gap: px(8.),
            media_margin_bottom: px(8.),
            icon_media_size: if compact { px(32.) } else { px(40.) },
            icon_size: if compact { px(16.) } else { px(24.) },
            icon_radius: style.radii.lg,
            content_gap: if compact { px(10.) } else { px(16.) },
            compact_title: compact,
        }
    }
}

/// A centered, compositional surface for empty and no-result states.
#[derive(IntoElement)]
pub struct Empty {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Empty {
    /// Creates an empty-state container.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for Empty {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Empty {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Empty {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Empty {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = EmptyMetrics::resolve(&cx.theme().style);

        div()
            .flex()
            .w_full()
            .min_w_0()
            .flex_1()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(metrics.root_gap)
            .p(metrics.root_padding)
            .rounded(metrics.root_radius)
            .border_dashed()
            .text_center()
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// Header region containing media, title, and supporting description.
#[derive(IntoElement)]
pub struct EmptyHeader {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl EmptyHeader {
    /// Creates an Empty header.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for EmptyHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for EmptyHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for EmptyHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for EmptyHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = EmptyMetrics::resolve(&cx.theme().style);

        div()
            .flex()
            .max_w(px(384.))
            .flex_col()
            .items_center()
            .gap(metrics.header_gap)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// Visual treatment used by [`EmptyMedia`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum EmptyMediaVariant {
    /// Leaves arbitrary media unframed.
    #[default]
    Default,
    /// Places media on the semantic muted icon surface.
    Icon,
}

enum EmptyMediaContent {
    Children(Vec<AnyElement>),
    Icon(IconName),
}

/// Optional illustration, avatar, or icon displayed above an Empty title.
#[derive(IntoElement)]
pub struct EmptyMedia {
    style: StyleRefinement,
    variant: EmptyMediaVariant,
    content: EmptyMediaContent,
}

impl EmptyMedia {
    /// Creates an unframed media region for custom children.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            variant: EmptyMediaVariant::Default,
            content: EmptyMediaContent::Children(Vec::new()),
        }
    }

    /// Creates an icon media region using preset-aware dimensions.
    pub fn icon(icon: IconName) -> Self {
        Self {
            style: StyleRefinement::default(),
            variant: EmptyMediaVariant::Icon,
            content: EmptyMediaContent::Icon(icon),
        }
    }

    /// Selects the media surface treatment.
    pub fn variant(mut self, variant: EmptyMediaVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Default for EmptyMedia {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for EmptyMedia {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        match &mut self.content {
            EmptyMediaContent::Children(children) => children.extend(elements),
            EmptyMediaContent::Icon(_) => {
                self.content = EmptyMediaContent::Children(elements.into_iter().collect());
            }
        }
    }
}

impl Styled for EmptyMedia {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for EmptyMedia {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = EmptyMetrics::resolve(&cx.theme().style);
        let is_icon = self.variant == EmptyMediaVariant::Icon;
        let mut media = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .mb(metrics.media_margin_bottom)
            .when(is_icon, |this| {
                this.size(metrics.icon_media_size)
                    .rounded(metrics.icon_radius)
                    .bg(cx.theme().muted)
                    .text_color(cx.theme().foreground)
            })
            .refine_style(&self.style);

        media = match self.content {
            EmptyMediaContent::Children(children) => media.children(children),
            EmptyMediaContent::Icon(icon) => {
                media.child(Icon::new(icon).with_size(metrics.icon_size))
            }
        };

        media
    }
}

/// Primary Empty-state heading.
#[derive(IntoElement)]
pub struct EmptyTitle {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl EmptyTitle {
    /// Creates an Empty title.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for EmptyTitle {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for EmptyTitle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for EmptyTitle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for EmptyTitle {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = EmptyMetrics::resolve(&cx.theme().style);

        div()
            .font_medium()
            .when(metrics.compact_title, |this| this.text_sm())
            .when(!metrics.compact_title, |this| this.text_lg())
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// Supporting muted copy for an Empty state.
#[derive(IntoElement)]
pub struct EmptyDescription {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl EmptyDescription {
    /// Creates an Empty description.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for EmptyDescription {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for EmptyDescription {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for EmptyDescription {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for EmptyDescription {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_sm()
            .line_height(relative(1.625))
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// Constrained action or input region rendered below the Empty header.
#[derive(IntoElement)]
pub struct EmptyContent {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl EmptyContent {
    /// Creates an Empty content region.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for EmptyContent {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for EmptyContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for EmptyContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for EmptyContent {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = EmptyMetrics::resolve(&cx.theme().style);

        div()
            .flex()
            .w_full()
            .max_w(px(384.))
            .min_w_0()
            .flex_col()
            .items_center()
            .gap(metrics.content_gap)
            .text_sm()
            .refine_style(&self.style)
            .children(self.children)
    }
}

#[cfg(test)]
mod tests {
    use gpui::ParentElement as _;

    use super::{Empty, EmptyMedia, EmptyMediaContent, EmptyMediaVariant, EmptyMetrics};
    use crate::{IconName, theme::StylePreset};

    #[test]
    fn empty_builder_collects_children() {
        let empty = Empty::new().child("header").child("content");

        assert_eq!(empty.children.len(), 2);
    }

    #[test]
    fn icon_media_selects_icon_variant() {
        let media = EmptyMedia::icon(IconName::Inbox);

        assert_eq!(media.variant, EmptyMediaVariant::Icon);
        assert!(matches!(
            media.content,
            EmptyMediaContent::Icon(IconName::Inbox)
        ));
    }

    #[test]
    fn empty_metrics_match_builtin_presets() {
        let vega = EmptyMetrics::resolve(&StylePreset::vega());
        let nova = EmptyMetrics::resolve(&StylePreset::nova());
        let maia = EmptyMetrics::resolve(&StylePreset::maia());

        assert_eq!(vega.root_padding, gpui::px(48.));
        assert_eq!(vega.icon_media_size, gpui::px(40.));
        assert_eq!(vega.icon_size, gpui::px(24.));
        assert_eq!(vega.root_radius, gpui::px(10.));
        assert!(!vega.compact_title);

        assert_eq!(nova.root_padding, gpui::px(24.));
        assert_eq!(nova.icon_media_size, gpui::px(32.));
        assert_eq!(nova.icon_size, gpui::px(16.));
        assert_eq!(nova.root_radius, gpui::px(10.));
        assert_eq!(nova.content_gap, gpui::px(10.));
        assert!(nova.compact_title);

        assert_eq!(maia.root_padding, gpui::px(48.));
        assert_eq!(maia.root_radius, gpui::px(14.));
        assert_eq!(maia.icon_radius, gpui::px(14.));
    }
}
