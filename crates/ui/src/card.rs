use gpui::{
    AnyElement, App, BoxShadow, ImageSource, IntoElement, ObjectFit, ParentElement, Pixels,
    RenderOnce, StyleRefinement, Styled, StyledImage as _, Window, div, img, point,
    prelude::FluentBuilder as _, px, relative,
};

use crate::{ActiveTheme as _, Density, StylePreset, StyledExt as _, box_shadow, h_flex, v_flex};

/// The spacing and title-density variant used by a [`Card`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CardSize {
    /// Uses the standard Vega Card spacing.
    #[default]
    Default,
    /// Uses the compact Vega Card spacing.
    Small,
}

/// Card geometry resolved from semantic Style Preset properties.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CardMetrics {
    spacing: Pixels,
    title_size: Pixels,
    title_line_height: f32,
    header_gap: Pixels,
    outer_radius: Pixels,
    section_radius: Pixels,
    shadow: bool,
    separated_footer: bool,
}

/// Radius refinements for each Card section that can paint against an outer edge.
struct CardEdgeRadiusStyles {
    top_media: StyleRefinement,
    bottom_media: StyleRefinement,
    header: StyleRefinement,
    footer: StyleRefinement,
}

/// Resolves Vega, Nova, and Maia Card intent without branching on preset identity.
fn card_metrics(size: CardSize, style: &StylePreset) -> CardMetrics {
    let compact = style.density == Density::Compact;
    let comfortable = style.density == Density::Comfortable;
    let spacing = match (compact, size) {
        (true, CardSize::Default) => px(16.),
        (true, CardSize::Small) => px(12.),
        (false, CardSize::Default) => px(24.),
        (false, CardSize::Small) => px(16.),
    };
    let title_size = if size == CardSize::Small && !comfortable {
        px(14.)
    } else {
        px(16.)
    };

    CardMetrics {
        spacing,
        title_size,
        title_line_height: if compact { 1.375 } else { 1.5 },
        header_gap: if comfortable { px(8.) } else { px(4.) },
        outer_radius: style.radii.xl,
        section_radius: if comfortable {
            style.radii.lg
        } else {
            style.radii.xl
        },
        shadow: style.elevation.enabled && style.density == Density::Standard,
        separated_footer: compact,
    }
}

/// Applies an explicit Card spacing override without changing size-dependent typography.
fn card_metrics_with_spacing(
    size: CardSize,
    style: &StylePreset,
    spacing: Option<Pixels>,
) -> CardMetrics {
    let mut metrics = card_metrics(size, style);
    if let Some(spacing) = spacing {
        metrics.spacing = spacing;
    }
    metrics
}

/// Returns whether a compact Card footer owns the trailing Card padding.
fn footer_removes_card_bottom_padding(
    metrics: CardMetrics,
    has_footer: bool,
    has_bottom_media: bool,
) -> bool {
    metrics.separated_footer && has_footer && !has_bottom_media
}

/// Resolves outer and section edge radii after Card-level `Styled` overrides.
fn card_edge_radius_styles(
    metrics: CardMetrics,
    card_style: &StyleRefinement,
) -> CardEdgeRadiusStyles {
    let top_left_outer = card_style
        .corner_radii
        .top_left
        .or_else(|| Some(metrics.outer_radius.into()));
    let top_right_outer = card_style
        .corner_radii
        .top_right
        .or_else(|| Some(metrics.outer_radius.into()));
    let bottom_right_outer = card_style
        .corner_radii
        .bottom_right
        .or_else(|| Some(metrics.outer_radius.into()));
    let bottom_left_outer = card_style
        .corner_radii
        .bottom_left
        .or_else(|| Some(metrics.outer_radius.into()));

    let top_left_section = card_style
        .corner_radii
        .top_left
        .or_else(|| Some(metrics.section_radius.into()));
    let top_right_section = card_style
        .corner_radii
        .top_right
        .or_else(|| Some(metrics.section_radius.into()));
    let bottom_right_section = card_style
        .corner_radii
        .bottom_right
        .or_else(|| Some(metrics.section_radius.into()));
    let bottom_left_section = card_style
        .corner_radii
        .bottom_left
        .or_else(|| Some(metrics.section_radius.into()));

    let mut top_media = StyleRefinement::default();
    top_media.corner_radii.top_left = top_left_outer;
    top_media.corner_radii.top_right = top_right_outer;

    let mut bottom_media = StyleRefinement::default();
    bottom_media.corner_radii.bottom_right = bottom_right_outer;
    bottom_media.corner_radii.bottom_left = bottom_left_outer;

    let mut header = StyleRefinement::default();
    header.corner_radii.top_left = top_left_section;
    header.corner_radii.top_right = top_right_section;

    let mut footer = StyleRefinement::default();
    footer.corner_radii.bottom_right = bottom_right_section;
    footer.corner_radii.bottom_left = bottom_left_section;

    CardEdgeRadiusStyles {
        top_media,
        bottom_media,
        header,
        footer,
    }
}

/// A static surface that groups related content and actions.
#[derive(IntoElement)]
pub struct Card {
    style: StyleRefinement,
    size: CardSize,
    spacing: Option<Pixels>,
    media: Option<CardMedia>,
    header: Option<CardHeader>,
    content: Option<CardContent>,
    footer: Option<CardFooter>,
    bottom_media: Option<CardMedia>,
}

impl Card {
    /// Creates a Card with the default Vega spacing.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            size: CardSize::Default,
            spacing: None,
            media: None,
            header: None,
            content: None,
            footer: None,
            bottom_media: None,
        }
    }

    /// Sets the Card spacing and title-density variant.
    pub fn size(mut self, size: CardSize) -> Self {
        self.size = size;
        self
    }

    /// Uses the compact Card spacing.
    pub fn small(self) -> Self {
        self.size(CardSize::Small)
    }

    /// Overrides the shared spacing used by the Card and each typed section.
    pub fn spacing(mut self, spacing: Pixels) -> Self {
        self.spacing = Some(spacing);
        self
    }

    /// Sets optional edge-to-edge media rendered before the header.
    pub fn media(mut self, media: CardMedia) -> Self {
        self.media = Some(media);
        self
    }

    /// Sets the Card header.
    pub fn header(mut self, header: CardHeader) -> Self {
        self.header = Some(header);
        self
    }

    /// Sets the primary Card content.
    pub fn content(mut self, content: CardContent) -> Self {
        self.content = Some(content);
        self
    }

    /// Sets the Card footer.
    pub fn footer(mut self, footer: CardFooter) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Sets optional media rendered after the footer using shadcn's trailing-image spacing.
    pub fn bottom_media(mut self, media: CardMedia) -> Self {
        self.bottom_media = Some(media);
        self
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Card {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Card {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = card_metrics_with_spacing(self.size, &cx.theme().style, self.spacing);
        let edge_radius_styles = card_edge_radius_styles(metrics, &self.style);
        let has_footer = self.footer.is_some();
        let has_bottom_media = self.bottom_media.is_some();
        let surface_ring = BoxShadow {
            color: cx.theme().foreground.opacity(0.1),
            offset: point(px(0.), px(0.)),
            blur_radius: px(0.),
            spread_radius: px(1.),
            inset: false,
        };
        let mut shadows = vec![surface_ring];
        if metrics.shadow {
            shadows.push(box_shadow(
                px(0.),
                px(1.),
                px(2.),
                px(0.),
                gpui::black().opacity(0.05),
            ));
        }

        v_flex()
            .gap(metrics.spacing)
            .py(metrics.spacing)
            .when(self.media.is_some(), |this| this.pt_0())
            .when(
                footer_removes_card_bottom_padding(metrics, has_footer, has_bottom_media),
                |this| this.pb_0(),
            )
            .overflow_hidden()
            .rounded(metrics.outer_radius)
            .bg(cx.theme().tokens.card)
            .text_color(cx.theme().card_foreground)
            .text_sm()
            .shadow(shadows)
            .refine_style(&self.style)
            .when_some(self.media, |this, media| {
                this.child(media.render_with_radius(edge_radius_styles.top_media))
            })
            .when_some(self.header, |this, header| {
                this.child(header.render_with_metrics(metrics, edge_radius_styles.header, cx))
            })
            .when_some(self.content, |this, content| {
                this.child(content.render_with_metrics(metrics))
            })
            .when_some(self.footer, |this, footer| {
                this.child(footer.render_with_metrics(metrics, edge_radius_styles.footer, cx))
            })
            .when_some(self.bottom_media, |this, media| {
                this.child(media.render_with_radius(edge_radius_styles.bottom_media))
            })
    }
}

enum CardMediaContent {
    Children(Vec<AnyElement>),
    Image {
        source: ImageSource,
        object_fit: ObjectFit,
    },
}

/// Media rendered against a top or bottom edge of a [`Card`].
#[derive(IntoElement)]
pub struct CardMedia {
    style: StyleRefinement,
    content: CardMediaContent,
}

impl CardMedia {
    /// Creates media from custom children painted by the CardMedia surface.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            content: CardMediaContent::Children(Vec::new()),
        }
    }

    /// Creates image media with cover fitting by default.
    pub fn image(source: impl Into<ImageSource>) -> Self {
        Self {
            style: StyleRefinement::default(),
            content: CardMediaContent::Image {
                source: source.into(),
                object_fit: ObjectFit::Cover,
            },
        }
    }

    /// Sets how image media fits its resolved bounds.
    pub fn object_fit(mut self, object_fit: ObjectFit) -> Self {
        if let CardMediaContent::Image {
            object_fit: current,
            ..
        } = &mut self.content
        {
            *current = object_fit;
        }
        self
    }

    /// Applies the outer Card radius directly because GPUI overflow masks are rectangular.
    fn render_with_radius(self, radius_style: StyleRefinement) -> AnyElement {
        match self.content {
            CardMediaContent::Children(children) => div()
                .refine_style(&self.style)
                .refine_style(&radius_style)
                .children(children)
                .into_any_element(),
            CardMediaContent::Image { source, object_fit } => img(source)
                .w_full()
                .object_fit(object_fit)
                .refine_style(&self.style)
                .refine_style(&radius_style)
                .into_any_element(),
        }
    }
}

impl Default for CardMedia {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for CardMedia {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let CardMediaContent::Children(children) = &mut self.content {
            children.extend(elements);
        }
    }
}

impl Styled for CardMedia {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CardMedia {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = card_metrics(CardSize::Default, &cx.theme().style);
        let radius_styles = card_edge_radius_styles(metrics, &StyleRefinement::default());
        self.render_with_radius(radius_styles.top_media)
    }
}

/// The leading section of a [`Card`], with an optional trailing action.
#[derive(IntoElement)]
pub struct CardHeader {
    style: StyleRefinement,
    title: Option<CardTitle>,
    description: Option<CardDescription>,
    action: Option<CardAction>,
    children: Vec<AnyElement>,
    bordered: bool,
}

impl CardHeader {
    /// Creates an empty Card header.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            title: None,
            description: None,
            action: None,
            children: Vec::new(),
            bordered: false,
        }
    }

    /// Sets the Card title slot.
    pub fn title(mut self, title: CardTitle) -> Self {
        self.title = Some(title);
        self
    }

    /// Sets the muted description below the title.
    pub fn description(mut self, description: CardDescription) -> Self {
        self.description = Some(description);
        self
    }

    /// Sets an action aligned to the upper-right of the header.
    pub fn action(mut self, action: CardAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Adds a bottom divider and preserves the standard spacing around it.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Renders the header with spacing inherited from its owning Card.
    fn render_with_metrics(
        self,
        metrics: CardMetrics,
        radius_style: StyleRefinement,
        cx: &App,
    ) -> AnyElement {
        let content = v_flex()
            .min_w_0()
            .flex_1()
            .gap(metrics.header_gap)
            .when_some(self.title, |this, title| {
                this.child(title.render_with_metrics(metrics))
            })
            .when_some(self.description, |this, description| {
                this.child(description)
            })
            .children(self.children);

        div()
            .flex()
            .flex_row()
            .items_start()
            .gap(metrics.header_gap)
            .px(metrics.spacing)
            .when(self.bordered, |this| {
                this.pb(metrics.spacing)
                    .border_b_1()
                    .border_color(cx.theme().border)
            })
            .refine_style(&self.style)
            .refine_style(&radius_style)
            .child(content)
            .when_some(self.action, |this, action| this.child(action))
            .into_any_element()
    }
}

impl Default for CardHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for CardHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CardHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CardHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = card_metrics(CardSize::Default, &cx.theme().style);
        let radius_styles = card_edge_radius_styles(metrics, &StyleRefinement::default());
        self.render_with_metrics(metrics, radius_styles.header, cx)
    }
}

/// The primary heading content of a [`CardHeader`].
#[derive(IntoElement)]
pub struct CardTitle {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl CardTitle {
    /// Creates an empty Card title.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    /// Renders the title using the owning Card's size.
    fn render_with_metrics(self, metrics: CardMetrics) -> AnyElement {
        div()
            .text_size(metrics.title_size)
            .line_height(relative(metrics.title_line_height))
            .font_medium()
            .refine_style(&self.style)
            .children(self.children)
            .into_any_element()
    }
}

impl Default for CardTitle {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for CardTitle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CardTitle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CardTitle {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.render_with_metrics(card_metrics(CardSize::Default, &cx.theme().style))
    }
}

/// Supporting text displayed below a [`CardTitle`].
#[derive(IntoElement)]
pub struct CardDescription {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl CardDescription {
    /// Creates an empty Card description.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for CardDescription {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for CardDescription {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CardDescription {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CardDescription {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A trailing action aligned within a [`CardHeader`].
#[derive(IntoElement)]
pub struct CardAction {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl CardAction {
    /// Creates an empty Card action slot.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for CardAction {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for CardAction {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CardAction {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CardAction {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex_none()
            .self_start()
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// The primary body section of a [`Card`].
#[derive(IntoElement)]
pub struct CardContent {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl CardContent {
    /// Creates an empty Card content section.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    /// Renders content with spacing inherited from its owning Card.
    fn render_with_metrics(self, metrics: CardMetrics) -> AnyElement {
        div()
            .px(metrics.spacing)
            .refine_style(&self.style)
            .children(self.children)
            .into_any_element()
    }
}

impl Default for CardContent {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for CardContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CardContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CardContent {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.render_with_metrics(card_metrics(CardSize::Default, &cx.theme().style))
    }
}

/// The trailing action or metadata section of a [`Card`].
#[derive(IntoElement)]
pub struct CardFooter {
    style: StyleRefinement,
    children: Vec<AnyElement>,
    bordered: bool,
}

impl CardFooter {
    /// Creates an empty Card footer.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
            bordered: false,
        }
    }

    /// Adds a top divider and preserves the standard spacing around it.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Renders the footer with spacing inherited from its owning Card.
    fn render_with_metrics(
        self,
        metrics: CardMetrics,
        radius_style: StyleRefinement,
        cx: &App,
    ) -> AnyElement {
        h_flex()
            .px(metrics.spacing)
            .when(metrics.separated_footer, |this| {
                this.py(metrics.spacing)
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted.opacity(0.5))
            })
            .when(self.bordered && !metrics.separated_footer, |this| {
                this.pt(metrics.spacing)
                    .border_t_1()
                    .border_color(cx.theme().border)
            })
            .refine_style(&self.style)
            .refine_style(&radius_style)
            .children(self.children)
            .into_any_element()
    }
}

impl Default for CardFooter {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for CardFooter {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CardFooter {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CardFooter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = card_metrics(CardSize::Default, &cx.theme().style);
        let radius_styles = card_edge_radius_styles(metrics, &StyleRefinement::default());
        self.render_with_metrics(metrics, radius_styles.footer, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::div;

    #[test]
    fn test_card_builder() {
        let card = Card::new()
            .small()
            .spacing(px(20.))
            .media(CardMedia::new().child(div()))
            .header(
                CardHeader::new()
                    .title(CardTitle::new().child("Title"))
                    .description(CardDescription::new().child("Description"))
                    .action(CardAction::new().child(div()))
                    .bordered(true),
            )
            .content(CardContent::new().child("Content"))
            .footer(CardFooter::new().bordered(true).child("Footer"))
            .bottom_media(CardMedia::new().child(div()));

        assert_eq!(card.size, CardSize::Small);
        assert_eq!(card.spacing, Some(px(20.)));
        assert!(card.media.is_some());
        assert!(card.header.as_ref().is_some_and(|header| {
            header.title.is_some()
                && header.description.is_some()
                && header.action.is_some()
                && header.bordered
        }));
        assert!(card.content.is_some());
        assert!(card.footer.as_ref().is_some_and(|footer| footer.bordered));
        assert!(card.bottom_media.is_some());
    }

    #[test]
    fn card_metrics_align_built_in_style_presets() {
        let vega = card_metrics(CardSize::Default, &StylePreset::vega());
        let vega_small = card_metrics(CardSize::Small, &StylePreset::vega());
        assert_eq!(vega.spacing, px(24.));
        assert_eq!(vega_small.spacing, px(16.));
        assert_eq!(vega_small.title_size, px(14.));
        assert_eq!(vega.header_gap, px(4.));
        assert_eq!(vega.title_line_height, 1.5);
        assert!(vega.shadow);
        assert!(!vega.separated_footer);

        let nova = card_metrics(CardSize::Default, &StylePreset::nova());
        let nova_small = card_metrics(CardSize::Small, &StylePreset::nova());
        assert_eq!(nova.spacing, px(16.));
        assert_eq!(nova_small.spacing, px(12.));
        assert_eq!(nova_small.title_size, px(14.));
        assert_eq!(nova.header_gap, px(4.));
        assert_eq!(nova.title_line_height, 1.375);
        assert!(!nova.shadow);
        assert!(nova.separated_footer);

        let maia = card_metrics(CardSize::Default, &StylePreset::maia());
        let maia_small = card_metrics(CardSize::Small, &StylePreset::maia());
        assert_eq!(maia.spacing, px(24.));
        assert_eq!(maia_small.spacing, px(16.));
        assert_eq!(maia_small.title_size, px(16.));
        assert_eq!(maia.header_gap, px(8.));
        assert_eq!(maia.outer_radius, StylePreset::maia().radii.xl);
        assert_eq!(maia.section_radius, StylePreset::maia().radii.lg);
        assert!(!maia.shadow);
        assert!(!maia.separated_footer);

        let custom_small =
            card_metrics_with_spacing(CardSize::Small, &StylePreset::vega(), Some(px(20.)));
        assert_eq!(custom_small.spacing, px(20.));
        assert_eq!(custom_small.title_size, px(14.));
    }

    #[test]
    fn card_edge_radii_inherit_card_corner_overrides() {
        let metrics = card_metrics(CardSize::Default, &StylePreset::vega());
        let card_style = StyleRefinement::default()
            .rounded_tl(px(3.))
            .rounded_tr(px(7.))
            .rounded_br(px(11.));

        let styles = card_edge_radius_styles(metrics, &card_style);

        assert_eq!(styles.top_media.corner_radii.top_left, Some(px(3.).into()));
        assert_eq!(styles.top_media.corner_radii.top_right, Some(px(7.).into()));
        assert_eq!(styles.header.corner_radii.top_left, Some(px(3.).into()));
        assert_eq!(styles.header.corner_radii.top_right, Some(px(7.).into()));
        assert_eq!(
            styles.bottom_media.corner_radii.bottom_right,
            Some(px(11.).into())
        );
        assert_eq!(
            styles.footer.corner_radii.bottom_right,
            Some(px(11.).into())
        );

        let square_styles =
            card_edge_radius_styles(metrics, &StyleRefinement::default().rounded(px(0.)));
        assert_eq!(
            square_styles.top_media.corner_radii.top_left,
            Some(px(0.).into())
        );
        assert_eq!(
            square_styles.header.corner_radii.top_right,
            Some(px(0.).into())
        );
        assert_eq!(
            square_styles.bottom_media.corner_radii.bottom_left,
            Some(px(0.).into())
        );
        assert_eq!(
            square_styles.footer.corner_radii.bottom_right,
            Some(px(0.).into())
        );
    }

    #[test]
    fn card_edge_radii_fall_back_to_preset_geometry() {
        let metrics = card_metrics(CardSize::Default, &StylePreset::maia());

        let styles = card_edge_radius_styles(metrics, &StyleRefinement::default());

        assert_eq!(
            styles.top_media.corner_radii.top_left,
            Some(metrics.outer_radius.into())
        );
        assert_eq!(
            styles.bottom_media.corner_radii.bottom_right,
            Some(metrics.outer_radius.into())
        );
        assert_eq!(
            styles.header.corner_radii.top_right,
            Some(metrics.section_radius.into())
        );
        assert_eq!(
            styles.footer.corner_radii.bottom_left,
            Some(metrics.section_radius.into())
        );
    }

    #[test]
    fn trailing_media_preserves_compact_card_bottom_spacing() {
        let metrics = card_metrics(CardSize::Default, &StylePreset::nova());

        assert!(footer_removes_card_bottom_padding(metrics, true, false));
        assert!(!footer_removes_card_bottom_padding(metrics, true, true));
        assert!(!footer_removes_card_bottom_padding(metrics, false, false));
    }
}
