use std::{
    ops::Range,
    sync::{Arc, Mutex},
};

use gpui::{
    AbsoluteLength, AnyElement, App, AvailableSpace, Bounds, DefiniteLength, Element, ElementId,
    GlobalElementId, HighlightStyle, ImageSource, InspectorElementId, InteractiveElement as _,
    IntoElement, LayoutId, Length, LineFragment as WrapLineFragment, ObjectFit, ParentElement as _,
    Pixels, Role, ShapedLine, SharedString, SharedUri, Size, StatefulInteractiveElement as _,
    StyleRefinement, Styled, StyledImage as _, TextRun, TextStyle, WhiteSpace, Window, div, img,
    point, prelude::FluentBuilder as _, px, relative, size,
};

use crate::{ActiveTheme as _, StyledExt as _, tooltip::Tooltip};

use super::{
    inline::{Inline, InlineLink, InlineSelectionSink, InlineState},
    node::LinkMark,
};
use crate::text::{MarkdownInlineKind, MarkdownLinkHandler, MarkdownTextStyle};

const IMAGE_LEN: usize = 1;

/// Classify absolute Markdown image URLs as remote resources and relative paths as embedded assets.
fn markdown_image_source(url: &SharedUri) -> ImageSource {
    url.as_ref().into()
}

pub(super) struct InlineFlow {
    id: ElementId,
    items: Vec<InlineFlowItem>,
    selection_state: Option<Arc<Mutex<InlineState>>>,
    link_handler: MarkdownLinkHandler,
    layout_cache: InlineFlowLayoutCache,
    semantic_styles: Vec<(MarkdownInlineKind, MarkdownTextStyle)>,
    layout_state: InlineFlowLayoutState,
}

/// Layout and paint properties for an atomic inline text box.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct InlineBoxStyle {
    pub(super) background: Option<gpui::Hsla>,
    pub(super) padding_x: Pixels,
    pub(super) padding_y: Pixels,
    pub(super) margin_x: Pixels,
    pub(super) margin_y: Pixels,
    pub(super) corner_radius: Pixels,
    pub(super) border_width: Pixels,
    pub(super) border_color: Option<gpui::Hsla>,
    pub(super) font_family: Option<SharedString>,
    pub(super) font_size: Option<Pixels>,
    pub(super) line_height: Option<Pixels>,
}

impl InlineBoxStyle {
    fn outer_width(&self, content: Pixels) -> Pixels {
        content + (self.padding_x + self.margin_x + self.border_width) * 2.
    }

    fn outer_height(&self, content: Pixels) -> Pixels {
        content + (self.padding_y + self.margin_y + self.border_width) * 2.
    }
}

pub(super) enum InlineFlowItem {
    Text {
        state: Arc<Mutex<InlineState>>,
        paragraph_range: Range<usize>,
        text: SharedString,
        links: Vec<InlineLink>,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
        link_hover_style: Option<Arc<MarkdownTextStyle>>,
        box_style: Option<InlineBoxStyle>,
    },
    Image {
        url: SharedUri,
        source: Option<ImageSource>,
        sizing: InlineImageSizing,
        link: Option<LinkMark>,
        title: String,
        width: Option<DefiniteLength>,
        height: Option<DefiniteLength>,
        style: Box<StyleRefinement>,
    },
    Custom {
        element: Option<AnyElement>,
        text: SharedString,
    },
}

/// Controls whether an image behaves like an inline glyph or a standalone content block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum InlineImageSizing {
    #[default]
    Compact,
    Intrinsic,
}

#[derive(Clone, Default)]
pub(crate) struct InlineFlowLayoutCache {
    layout: Arc<Mutex<Option<(InlineFlowLayoutKey, Arc<InlineFlowLayout>)>>>,
    image_measurements:
        Arc<Mutex<Option<(InlineImageMeasurementKey, Vec<Option<MeasuredImageLayout>>)>>>,
}

impl std::fmt::Debug for InlineFlowLayoutCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("InlineFlowLayoutCache").finish()
    }
}

impl InlineFlowLayoutCache {
    /// Return the cached layout for an identical measurement input or replace the single entry.
    fn get_or_insert_with(
        &self,
        key: InlineFlowLayoutKey,
        build: impl FnOnce() -> InlineFlowLayout,
    ) -> Arc<InlineFlowLayout> {
        if let Ok(cache) = self.layout.lock()
            && let Some((cached_key, layout)) = cache.as_ref()
            && cached_key == &key
        {
            return layout.clone();
        }

        let layout = Arc::new(build());
        if let Ok(mut cache) = self.layout.lock() {
            *cache = Some((key, layout.clone()));
        }
        layout
    }

    /// Reuses intrinsic image measurements while image inputs and typography are unchanged.
    fn image_measurements(
        &self,
        key: InlineImageMeasurementKey,
        measure: impl FnOnce() -> Vec<Option<MeasuredImageLayout>>,
    ) -> Vec<Option<MeasuredImageLayout>> {
        if let Ok(cache) = self.image_measurements.lock()
            && let Some((cached_key, layouts)) = cache.as_ref()
            && cached_key == &key
        {
            return layouts.clone();
        }
        let layouts = measure();
        let measurements_are_stable = layouts
            .iter()
            .flatten()
            .all(|layout| layout.intrinsic_resolved);
        if measurements_are_stable && let Ok(mut cache) = self.image_measurements.lock() {
            *cache = Some((key, layouts.clone()));
        }
        layouts
    }
}

#[derive(Clone, Debug, PartialEq)]
struct InlineImageMeasurementKey {
    images: Vec<(
        usize,
        SharedUri,
        Option<DefiniteLength>,
        Option<DefiniteLength>,
        InlineImageSizing,
        Box<StyleRefinement>,
    )>,
    line_height: Pixels,
    rem_size: Pixels,
}

#[derive(Clone, Debug, PartialEq)]
struct InlineFlowLayoutKey {
    semantic_styles: Vec<(MarkdownInlineKind, MarkdownTextStyle)>,
    resource_policy: crate::text::MarkdownResourcePolicy,
    image_layouts: Vec<Option<MeasuredImageLayout>>,
    custom_sizes: Vec<(usize, Size<Pixels>)>,
    text_style: TextStyle,
    wrap_width: Option<Pixels>,
    line_height: Pixels,
    rem_size: Pixels,
}

#[derive(Clone, Default)]
pub(crate) struct InlineFlowLayoutState {
    layout: Arc<Mutex<Option<Arc<InlineFlowLayout>>>>,
    origin: Arc<Mutex<Option<gpui::Point<Pixels>>>>,
}

impl InlineFlowLayoutState {
    /// Returns the window-space bounds occupied by the complete flow.
    pub(crate) fn bounds(&self) -> Option<Bounds<Pixels>> {
        let (layout, origin) = self.snapshot()?;
        Some(Bounds::new(origin, layout.size))
    }

    /// Returns the synthetic inline-flow offset nearest to a window position.
    pub(crate) fn index_for_position(&self, position: gpui::Point<Pixels>) -> usize {
        let Some((layout, origin)) = self.snapshot() else {
            return 0;
        };
        let local = position - origin;
        let mut previous_end = 0;
        for fragment in &layout.fragments {
            let (fragment_origin, fragment_size, range) = fragment.geometry();
            let bounds = Bounds::new(fragment_origin, fragment_size);
            if local.y < bounds.top() {
                return previous_end;
            }
            if local.y <= bounds.bottom() {
                if local.x <= bounds.left() {
                    return range.start;
                }
                if local.x <= bounds.right() {
                    return match fragment {
                        PositionedFragment::Text {
                            shaped_line,
                            flow_range,
                            ..
                        } => {
                            flow_range.start
                                + shaped_line
                                    .index_for_x(local.x - bounds.left())
                                    .unwrap_or(flow_range.len())
                        }
                        PositionedFragment::Image { .. } | PositionedFragment::Custom { .. } => {
                            if local.x < bounds.center().x {
                                range.start
                            } else {
                                range.end
                            }
                        }
                    };
                }
                previous_end = range.end;
            } else {
                previous_end = range.end;
            }
        }
        previous_end
    }

    /// Returns a window position and visual line height for a synthetic flow offset.
    pub(crate) fn position_for_index(&self, index: usize) -> Option<(gpui::Point<Pixels>, Pixels)> {
        let (layout, origin) = self.snapshot()?;
        let fragment = layout.fragments.iter().find(|fragment| {
            let range = fragment.geometry().2;
            range.contains(&index) || index == range.end
        })?;
        let (fragment_origin, fragment_size, range) = fragment.geometry();
        let x = match fragment {
            PositionedFragment::Text {
                shaped_line,
                flow_range,
                ..
            } => shaped_line.x_for_index(index.saturating_sub(flow_range.start)),
            PositionedFragment::Image { .. } | PositionedFragment::Custom { .. } => {
                if index >= range.end {
                    fragment_size.width
                } else {
                    Pixels::ZERO
                }
            }
        };
        Some((
            origin + point(fragment_origin.x + x, fragment_origin.y),
            fragment_size.height,
        ))
    }

    /// Returns paint bounds for the selected part of the synthetic inline flow.
    pub(crate) fn selection_bounds(&self, range: Range<usize>) -> Vec<Bounds<Pixels>> {
        let Some((layout, origin)) = self.snapshot() else {
            return Vec::new();
        };
        layout
            .fragments
            .iter()
            .filter_map(|fragment| {
                let (fragment_origin, fragment_size, fragment_range) = fragment.geometry();
                let start = range.start.max(fragment_range.start);
                let end = range.end.min(fragment_range.end);
                if start >= end {
                    return None;
                }
                let (left, right) = match fragment {
                    PositionedFragment::Text {
                        shaped_line,
                        flow_range,
                        ..
                    } => (
                        shaped_line.x_for_index(start - flow_range.start),
                        shaped_line.x_for_index(end - flow_range.start),
                    ),
                    PositionedFragment::Image { .. } | PositionedFragment::Custom { .. } => {
                        (Pixels::ZERO, fragment_size.width)
                    }
                };
                Some(Bounds::from_corners(
                    origin + point(fragment_origin.x + left, fragment_origin.y),
                    origin
                        + point(
                            fragment_origin.x + right,
                            fragment_origin.y + fragment_size.height,
                        ),
                ))
            })
            .collect()
    }

    fn snapshot(&self) -> Option<(Arc<InlineFlowLayout>, gpui::Point<Pixels>)> {
        let layout = self.layout.lock().ok()?.clone()?;
        let origin = self.origin.lock().ok()?.as_ref().copied()?;
        Some((layout, origin))
    }
}

#[derive(Clone, Default)]
struct InlineFlowLayout {
    fragments: Vec<PositionedFragment>,
    size: Size<Pixels>,
}

#[derive(Clone)]
enum PositionedFragment {
    Text {
        item_ix: usize,
        origin: gpui::Point<Pixels>,
        size: Size<Pixels>,
        source_range: Range<usize>,
        text: SharedString,
        links: Vec<InlineLink>,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
        link_hover_style: Option<Arc<MarkdownTextStyle>>,
        box_style: Box<Option<InlineBoxStyle>>,
        shaped_line: Arc<ShapedLine>,
        line_height: Pixels,
        flow_range: Range<usize>,
    },
    Image {
        item_ix: usize,
        origin: gpui::Point<Pixels>,
        size: Size<Pixels>,
        base_size: Size<Pixels>,
        source_range: Range<usize>,
    },
    Custom {
        item_ix: usize,
        origin: gpui::Point<Pixels>,
        size: Size<Pixels>,
        source_range: Range<usize>,
    },
}

impl PositionedFragment {
    fn geometry(&self) -> (gpui::Point<Pixels>, Size<Pixels>, Range<usize>) {
        match self {
            Self::Text {
                origin,
                size,
                flow_range,
                ..
            } => (*origin, *size, flow_range.clone()),
            Self::Image {
                origin,
                size,
                source_range,
                ..
            }
            | Self::Custom {
                origin,
                size,
                source_range,
                ..
            } => (*origin, *size, source_range.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum MeasureItem {
    Text {
        text: SharedString,
        links: Vec<InlineLink>,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
        link_hover_style: Option<Arc<MarkdownTextStyle>>,
        box_style: Option<InlineBoxStyle>,
    },
    Image {
        url: SharedUri,
        width: Option<DefiniteLength>,
        height: Option<DefiniteLength>,
        sizing: InlineImageSizing,
        style: Box<StyleRefinement>,
    },
    Custom {
        size: Size<Pixels>,
        len: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MeasuredImageLayout {
    /// Intrinsic or Markdown-attribute size applied before semantic refinement.
    base_size: Size<Pixels>,
    /// Final GPUI layout size, including semantic dimensions and box-model styles.
    outer_size: Size<Pixels>,
    /// Whether every dimension that depends on the decoded resource is final.
    intrinsic_resolved: bool,
}

struct LineFragmentLayout {
    item_ix: usize,
    kind: LineFragmentKind,
    size: Size<Pixels>,
    source_range: Range<usize>,
    flow_range: Range<usize>,
}

enum LineFragmentKind {
    Text {
        text: SharedString,
        links: Vec<InlineLink>,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
        link_hover_style: Option<Arc<MarkdownTextStyle>>,
        box_style: Option<InlineBoxStyle>,
        shaped_line: Arc<ShapedLine>,
        line_height: Pixels,
    },
    Image {
        base_size: Size<Pixels>,
    },
    Custom,
}

impl InlineFlow {
    pub(super) fn new(id: impl Into<ElementId>, items: Vec<InlineFlowItem>) -> Self {
        Self {
            id: id.into(),
            items,
            selection_state: None,
            link_handler: MarkdownLinkHandler::default(),
            layout_cache: InlineFlowLayoutCache::default(),
            semantic_styles: Vec::new(),
            layout_state: InlineFlowLayoutState::default(),
        }
    }

    /// Store all fragment selections in one paragraph-level state.
    pub(super) fn selection_state(mut self, state: Arc<Mutex<InlineState>>) -> Self {
        self.selection_state = Some(state);
        self
    }

    /// Apply the view-level resource policy and optional link callback.
    pub(super) fn link_handler(mut self, handler: MarkdownLinkHandler) -> Self {
        self.link_handler = handler;
        self
    }

    /// Reuse paragraph layout while its content, style, and available width remain unchanged.
    pub(super) fn layout_cache(mut self, cache: InlineFlowLayoutCache) -> Self {
        self.layout_cache = cache;
        self
    }

    /// Store the compact semantic style input used by inline measurement.
    pub(super) fn semantic_styles(
        mut self,
        styles: Vec<(MarkdownInlineKind, MarkdownTextStyle)>,
    ) -> Self {
        self.semantic_styles = styles;
        self
    }

    /// Shares positioned fragment geometry with the owning Markdown renderer.
    pub(crate) fn layout_state(mut self, state: InlineFlowLayoutState) -> Self {
        self.layout_state = state;
        self
    }

    fn image_element(
        ix: usize,
        source: ImageSource,
        link: &Option<LinkMark>,
        title: &str,
        base_size: Size<Pixels>,
        style: &StyleRefinement,
        link_handler: MarkdownLinkHandler,
    ) -> AnyElement {
        img(source)
            .id(ix)
            .object_fit(ObjectFit::Contain)
            .max_w(relative(1.))
            .w(base_size.width)
            .h(base_size.height)
            .refine_style(style)
            .when_some(
                link.clone()
                    .filter(|link| link_handler.policy.allows_link(&link.url)),
                |this, link| {
                    let title = title.to_string();
                    let link_handler = link_handler.clone();
                    this.cursor_pointer()
                        .tooltip(move |window, cx| Tooltip::new(title.clone()).build(window, cx))
                        .on_click(move |_, window, cx| {
                            link_handler.activate(&link.url, window, cx);
                        })
                },
            )
            .into_any_element()
    }

    /// Resolve fragment-local selection state and persistent source-item hover state.
    fn text_fragment_states(
        item: &InlineFlowItem,
    ) -> Option<(Arc<Mutex<InlineState>>, Arc<Mutex<InlineState>>)> {
        let InlineFlowItem::Text { state, .. } = item else {
            return None;
        };

        // Fragment state is paint-local. Paragraph selection is persisted separately.
        let selection_state = Arc::new(Mutex::new(InlineState::default()));
        Some((selection_state, state.clone()))
    }
}

impl IntoElement for InlineFlow {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for InlineFlow {
    type RequestLayoutState = InlineFlowLayoutState;
    type PrepaintState = Vec<AnyElement>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let measure_items = self
            .items
            .iter_mut()
            .map(|item| match item {
                InlineFlowItem::Custom { element, text } => {
                    let size = element
                        .as_mut()
                        .map(|element| {
                            element.layout_as_root(AvailableSpace::min_size(), window, cx)
                        })
                        .unwrap_or_default();
                    MeasureItem::Custom {
                        size,
                        len: text.len().max(1),
                    }
                }
                item => MeasureItem::from(&*item),
            })
            .collect::<Vec<_>>();
        let line_height = window.line_height();
        let rem_size = window.rem_size();
        let image_key = InlineImageMeasurementKey {
            images: measure_items
                .iter()
                .enumerate()
                .filter_map(|(ix, item)| match item {
                    MeasureItem::Image {
                        url,
                        width,
                        height,
                        sizing,
                        style,
                    } => Some((ix, url.clone(), *width, *height, *sizing, style.clone())),
                    MeasureItem::Text { .. } | MeasureItem::Custom { .. } => None,
                })
                .collect(),
            line_height,
            rem_size,
        };
        let image_layouts = self.layout_cache.image_measurements(image_key, || {
            measure_items
                .iter()
                .enumerate()
                .map(|(ix, item)| match item {
                    MeasureItem::Image {
                        url,
                        width,
                        height,
                        sizing,
                        style,
                    } => Some(measure_image_layout(
                        ix,
                        url,
                        *width,
                        *height,
                        *sizing,
                        style,
                        line_height,
                        rem_size,
                        window,
                        cx,
                    )),
                    MeasureItem::Text { .. } | MeasureItem::Custom { .. } => None,
                })
                .collect()
        });
        let layout_state = self.layout_state.clone();
        let layout_ref = layout_state.layout.clone();
        let layout_cache = self.layout_cache.clone();
        let semantic_styles = self.semantic_styles.clone();
        let resource_policy = self.link_handler.policy;
        let custom_sizes = measure_items
            .iter()
            .enumerate()
            .filter_map(|(ix, item)| match item {
                MeasureItem::Custom { size, .. } => Some((ix, *size)),
                MeasureItem::Text { .. } | MeasureItem::Image { .. } => None,
            })
            .collect::<Vec<_>>();

        let layout_id = window.request_measured_layout(Default::default(), {
            move |known_dimensions, available_space, window, _cx| {
                let text_style = window.text_style();
                let wrap_width = if text_style.white_space == WhiteSpace::Normal {
                    known_dimensions.width.or(match available_space.width {
                        AvailableSpace::Definite(width) => Some(width),
                        _ => None,
                    })
                } else {
                    None
                };
                let key = InlineFlowLayoutKey {
                    semantic_styles: semantic_styles.clone(),
                    resource_policy,
                    image_layouts: image_layouts.clone(),
                    custom_sizes: custom_sizes.clone(),
                    text_style: text_style.clone(),
                    wrap_width,
                    line_height,
                    rem_size,
                };
                let layout = layout_cache.get_or_insert_with(key, || {
                    layout_flow(
                        &measure_items,
                        &image_layouts,
                        &text_style,
                        wrap_width,
                        window,
                    )
                });
                let size = layout.size;
                if let Ok(mut state) = layout_ref.lock() {
                    *state = Some(layout);
                }
                size
            }
        });

        (layout_id, layout_state)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Ok(mut origin) = request_layout.origin.lock() {
            *origin = Some(bounds.origin);
        }
        let fragments = request_layout
            .layout
            .lock()
            .ok()
            .and_then(|layout| layout.clone())
            .unwrap_or_default();
        let mut elements = Vec::with_capacity(fragments.fragments.len());

        for fragment in fragments.fragments.iter().cloned() {
            match fragment {
                PositionedFragment::Text {
                    item_ix,
                    origin,
                    size: fragment_size,
                    source_range,
                    text,
                    links,
                    highlights,
                    link_hover_style,
                    box_style,
                    shaped_line,
                    line_height,
                    ..
                } => {
                    let Some((state, hover_state)) =
                        Self::text_fragment_states(&self.items[item_ix])
                    else {
                        continue;
                    };
                    let accessible_text = match &self.items[item_ix] {
                        InlineFlowItem::Text { text, .. } => text.clone(),
                        _ => text.clone(),
                    };
                    let keyboard_link = (source_range.start == 0)
                        .then(|| links.first())
                        .flatten()
                        .filter(|link| link.range.start == 0)
                        .cloned();
                    if let Ok(mut state) = state.lock() {
                        state.set_text(text);
                    }

                    let paragraph_range = match &self.items[item_ix] {
                        InlineFlowItem::Text {
                            paragraph_range, ..
                        } => {
                            (paragraph_range.start + source_range.start)
                                ..(paragraph_range.start + source_range.end)
                        }
                        InlineFlowItem::Image { .. } | InlineFlowItem::Custom { .. } => continue,
                    };

                    let mut inline = Inline::new("text", state, links, highlights)
                        .link_hover_style(link_hover_style)
                        .link_handler(self.link_handler.clone())
                        .precomputed_line(shaped_line, line_height)
                        .hover_state(hover_state, elements.len());
                    if let Some(selection_state) = &self.selection_state {
                        inline = inline.selection_sink(InlineSelectionSink::new(
                            selection_state.clone(),
                            paragraph_range,
                            bounds,
                        ));
                    }
                    let inline = inline.into_any_element();
                    let mut element = if let Some(box_style) = *box_style {
                        div()
                            .id(elements.len())
                            .flex()
                            .items_center()
                            .size_full()
                            .mx(box_style.margin_x)
                            .my(box_style.margin_y)
                            .px(box_style.padding_x)
                            .py(box_style.padding_y)
                            .border(box_style.border_width)
                            .when_some(box_style.border_color, |this, color| {
                                this.border_color(color)
                            })
                            .when_some(box_style.font_family.clone(), |this, family| {
                                this.font_family(family)
                            })
                            .when_some(box_style.font_size, |this, size| this.text_size(size))
                            .when_some(box_style.line_height, |this, height| {
                                this.line_height(height)
                            })
                            .rounded(box_style.corner_radius)
                            .when_some(box_style.background, |this, background| this.bg(background))
                            .child(inline)
                            .into_any_element()
                    } else {
                        inline
                    };
                    if let Some(link) = keyboard_link
                        .filter(|link| self.link_handler.policy.allows_link(&link.mark.url))
                    {
                        let focus_id = ElementId::NamedChild(
                            Arc::new(self.id.clone()),
                            format!("link-{}", link.id).into(),
                        );
                        let focus_handle = window
                            .use_keyed_state(focus_id.clone(), cx, |_, cx| cx.focus_handle())
                            .read(cx)
                            .clone();
                        let focus_visible =
                            focus_handle.is_focused(window) && window.last_input_was_keyboard();
                        let handler = self.link_handler.clone();
                        let url = link.mark.url.clone();
                        element = div()
                            .id(focus_id)
                            .role(Role::Link)
                            .aria_label(accessible_text)
                            .track_focus(&focus_handle.tab_stop(true))
                            .when(focus_visible, |this| {
                                this.bg(cx.theme().accent.opacity(0.35))
                            })
                            .on_key_down(move |event, window, cx| {
                                if !event.keystroke.modifiers.modified()
                                    && event.keystroke.key == "enter"
                                {
                                    window.prevent_default();
                                    handler.activate(&url, window, cx);
                                }
                            })
                            .child(element)
                            .into_any_element();
                    }
                    element.prepaint_as_root(
                        bounds.origin + origin,
                        size(
                            AvailableSpace::Definite(fragment_size.width),
                            AvailableSpace::Definite(fragment_size.height),
                        ),
                        window,
                        cx,
                    );
                    elements.push(element);
                }
                PositionedFragment::Image {
                    item_ix,
                    origin,
                    size: fragment_size,
                    base_size,
                    source_range: _,
                } => {
                    let InlineFlowItem::Image {
                        url,
                        source,
                        link,
                        title,
                        style,
                        ..
                    } = &self.items[item_ix]
                    else {
                        continue;
                    };
                    let mut element = Self::image_element(
                        elements.len(),
                        source.clone().unwrap_or_else(|| markdown_image_source(url)),
                        link,
                        title.as_str(),
                        base_size,
                        style.as_ref(),
                        self.link_handler.clone(),
                    );
                    element.prepaint_as_root(
                        bounds.origin + origin,
                        size(
                            AvailableSpace::Definite(fragment_size.width),
                            AvailableSpace::Definite(fragment_size.height),
                        ),
                        window,
                        cx,
                    );
                    elements.push(element);
                }
                PositionedFragment::Custom {
                    item_ix,
                    origin,
                    size: fragment_size,
                    source_range: _,
                } => {
                    let InlineFlowItem::Custom { element, .. } = &mut self.items[item_ix] else {
                        continue;
                    };
                    let Some(mut element) = element.take() else {
                        continue;
                    };
                    element.prepaint_as_root(
                        bounds.origin + origin,
                        size(
                            AvailableSpace::Definite(fragment_size.width),
                            AvailableSpace::Definite(fragment_size.height),
                        ),
                        window,
                        cx,
                    );
                    elements.push(element);
                }
            }
        }

        elements
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(selection_state) = &self.selection_state
            && let Ok(mut state) = selection_state.lock()
        {
            state.selection = None;
        }
        for element in prepaint {
            element.paint(window, cx);
        }
    }
}

impl From<&InlineFlowItem> for MeasureItem {
    fn from(item: &InlineFlowItem) -> Self {
        match item {
            InlineFlowItem::Text {
                state: _,
                text,
                links,
                highlights,
                link_hover_style,
                box_style,
                ..
            } => MeasureItem::Text {
                text: text.clone(),
                links: links.clone(),
                highlights: highlights.clone(),
                link_hover_style: link_hover_style.clone(),
                box_style: box_style.clone(),
            },
            InlineFlowItem::Image {
                url,
                width,
                height,
                sizing,
                style,
                ..
            } => MeasureItem::Image {
                url: url.clone(),
                width: *width,
                height: *height,
                sizing: *sizing,
                style: style.clone(),
            },
            InlineFlowItem::Custom { text, .. } => MeasureItem::Custom {
                size: Size::default(),
                len: text.len().max(1),
            },
        }
    }
}

impl MeasureItem {
    fn len(&self) -> usize {
        match self {
            MeasureItem::Text { text, .. } => text.len(),
            MeasureItem::Image { .. } => IMAGE_LEN,
            MeasureItem::Custom { len, .. } => *len,
        }
    }
}

fn layout_flow(
    items: &[MeasureItem],
    image_layouts: &[Option<MeasuredImageLayout>],
    text_style: &TextStyle,
    wrap_width: Option<Pixels>,
    window: &mut Window,
) -> InlineFlowLayout {
    let line_height = window.line_height();
    let rem_size = window.rem_size();
    let total_len = items.iter().map(MeasureItem::len).sum::<usize>();
    if total_len == 0 {
        return InlineFlowLayout::default();
    }

    let line_ranges = line_ranges(items, image_layouts, text_style, wrap_width, window);
    let font_size = text_style.font_size.to_pixels(rem_size);
    let mut fragments = Vec::new();
    let mut max_width = Pixels::ZERO;
    let mut y = Pixels::ZERO;

    for line_range in line_ranges {
        let mut line_fragments = Vec::new();
        let mut line_width = Pixels::ZERO;
        let mut actual_line_height = line_height;
        let mut item_start = 0;

        for (item_ix, item) in items.iter().enumerate() {
            let item_end = item_start + item.len();
            if item_end <= line_range.start {
                item_start = item_end;
                continue;
            }
            if item_start >= line_range.end {
                break;
            }

            match item {
                MeasureItem::Text {
                    text,
                    links,
                    highlights,
                    link_hover_style,
                    box_style,
                } => {
                    let local_start = line_range.start.max(item_start) - item_start;
                    let local_end = line_range.end.min(item_end) - item_start;
                    if local_start < local_end {
                        let subtext = SharedString::from(text[local_start..local_end].to_string());
                        let highlights =
                            slice_ranges(highlights, local_start, local_end, |range, style| {
                                (range, *style)
                            });
                        let links = slice_links(links, local_start, local_end);
                        let item_text_style = text_style_for_box(text_style, box_style.as_ref());
                        let item_font_size = box_style
                            .as_ref()
                            .and_then(|style| style.font_size)
                            .unwrap_or(font_size);
                        let runs =
                            runs_for_highlights(&subtext, &item_text_style, highlights.clone());
                        let shaped_line =
                            Arc::new(shape_line(subtext.clone(), item_font_size, &runs, window));
                        let width = box_style
                            .as_ref()
                            .map(|style| style.outer_width(shaped_line.width()))
                            .unwrap_or_else(|| shaped_line.width());
                        let content_line_height = box_style
                            .as_ref()
                            .and_then(|style| style.line_height)
                            .unwrap_or(line_height);
                        let height = box_style
                            .as_ref()
                            .map(|style| style.outer_height(content_line_height))
                            .unwrap_or(content_line_height);
                        actual_line_height = actual_line_height.max(height);
                        line_width += width;
                        line_fragments.push(LineFragmentLayout {
                            item_ix,
                            kind: LineFragmentKind::Text {
                                text: subtext,
                                links,
                                highlights,
                                link_hover_style: link_hover_style.clone(),
                                box_style: box_style.clone(),
                                shaped_line,
                                line_height: content_line_height,
                            },
                            size: size(width, height),
                            source_range: local_start..local_end,
                            flow_range: (item_start + local_start)..(item_start + local_end),
                        });
                    }
                }
                MeasureItem::Image { sizing, .. } => {
                    if line_range.start <= item_start && item_end <= line_range.end {
                        let mut image_layout =
                            image_layouts[item_ix].expect("image should be measured before layout");
                        if *sizing == InlineImageSizing::Intrinsic
                            && let Some(wrap_width) = wrap_width
                            && image_layout.outer_size.width > wrap_width
                        {
                            let scale = wrap_width / image_layout.outer_size.width;
                            image_layout.base_size.width *= scale;
                            image_layout.base_size.height *= scale;
                            image_layout.outer_size.width *= scale;
                            image_layout.outer_size.height *= scale;
                        }
                        line_width += image_layout.outer_size.width;
                        actual_line_height = actual_line_height.max(image_layout.outer_size.height);
                        line_fragments.push(LineFragmentLayout {
                            item_ix,
                            kind: LineFragmentKind::Image {
                                base_size: image_layout.base_size,
                            },
                            size: image_layout.outer_size,
                            source_range: 0..IMAGE_LEN,
                            flow_range: item_start..item_end,
                        });
                    }
                }
                MeasureItem::Custom { size, .. } => {
                    if line_range.start <= item_start && item_end <= line_range.end {
                        line_width += size.width;
                        actual_line_height = actual_line_height.max(size.height);
                        line_fragments.push(LineFragmentLayout {
                            item_ix,
                            kind: LineFragmentKind::Custom,
                            size: *size,
                            source_range: 0..item.len(),
                            flow_range: item_start..item_end,
                        });
                    }
                }
            }

            item_start = item_end;
        }

        let mut x = Pixels::ZERO;
        for fragment in line_fragments {
            let origin = point(x, y + (actual_line_height - fragment.size.height) / 2.);
            let positioned = match fragment.kind {
                LineFragmentKind::Text {
                    text,
                    links,
                    highlights,
                    link_hover_style,
                    box_style,
                    shaped_line,
                    line_height,
                } => PositionedFragment::Text {
                    item_ix: fragment.item_ix,
                    origin,
                    size: fragment.size,
                    source_range: fragment.source_range,
                    text,
                    links,
                    highlights,
                    link_hover_style,
                    box_style: Box::new(box_style),
                    shaped_line,
                    line_height,
                    flow_range: fragment.flow_range,
                },
                LineFragmentKind::Image { base_size } => PositionedFragment::Image {
                    item_ix: fragment.item_ix,
                    origin,
                    size: fragment.size,
                    base_size,
                    source_range: fragment.flow_range,
                },
                LineFragmentKind::Custom => PositionedFragment::Custom {
                    item_ix: fragment.item_ix,
                    origin,
                    size: fragment.size,
                    source_range: fragment.flow_range,
                },
            };
            x += fragment.size.width;
            fragments.push(positioned);
        }

        max_width = max_width.max(line_width);
        y += actual_line_height;
    }

    InlineFlowLayout {
        fragments,
        size: size(max_width, y),
    }
}

/// Resolve the same atomic inline typography for wrapping, shaping, and painting.
fn text_style_for_box(base: &TextStyle, box_style: Option<&InlineBoxStyle>) -> TextStyle {
    let mut style = base.clone();
    let Some(box_style) = box_style else {
        return style;
    };
    if let Some(font_family) = &box_style.font_family {
        style.font_family = font_family.clone();
    }
    if let Some(font_size) = box_style.font_size {
        style.font_size = font_size.into();
    }
    if let Some(line_height) = box_style.line_height {
        style.line_height = line_height.into();
    }
    style
}

fn line_ranges(
    items: &[MeasureItem],
    image_layouts: &[Option<MeasuredImageLayout>],
    text_style: &TextStyle,
    wrap_width: Option<Pixels>,
    window: &mut Window,
) -> Vec<Range<usize>> {
    let hard_lines = hard_line_ranges(items);
    let Some(wrap_width) = wrap_width else {
        return hard_lines;
    };
    let rem_size = window.rem_size();
    let font_size = text_style.font_size.to_pixels(rem_size);
    let mut wrapper = window
        .text_system()
        .line_wrapper(text_style.font(), font_size);

    let mut ranges = Vec::new();
    for hard_line in hard_lines {
        if hard_line.is_empty() {
            ranges.push(hard_line);
            continue;
        }

        let wrap_fragments = wrap_fragments_for_range(
            items,
            image_layouts,
            text_style,
            font_size,
            &hard_line,
            window,
        );
        let boundaries = wrapper
            .wrap_line(&wrap_fragments, wrap_width)
            .map(|boundary| hard_line.start + boundary.ix.min(hard_line.len()))
            .collect::<Vec<_>>();
        let mut start = hard_line.start;
        for end in boundaries {
            if start < end {
                ranges.push(start..end);
            }
            start = end;
        }
        if start < hard_line.end {
            ranges.push(start..hard_line.end);
        }
    }

    ranges
}

/// Split source items at hard line breaks while keeping global byte offsets.
fn hard_line_ranges(items: &[MeasureItem]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut item_start = 0;
    let mut line_start = 0;

    for item in items {
        if let MeasureItem::Text { text, .. } = item {
            for (offset, character) in text.char_indices() {
                if character == '\n' {
                    let newline = item_start + offset;
                    ranges.push(line_start..newline);
                    line_start = newline + character.len_utf8();
                }
            }
        }
        item_start += item.len();
    }
    ranges.push(line_start..item_start);
    ranges
}

/// Build GPUI wrapping fragments for one newline-free source range.
fn wrap_fragments_for_range<'a>(
    items: &'a [MeasureItem],
    image_layouts: &[Option<MeasuredImageLayout>],
    text_style: &TextStyle,
    font_size: Pixels,
    range: &Range<usize>,
    window: &mut Window,
) -> Vec<WrapLineFragment<'a>> {
    let mut fragments = Vec::new();
    let mut item_start = 0;

    for (ix, item) in items.iter().enumerate() {
        let item_end = item_start + item.len();
        if item_end <= range.start {
            item_start = item_end;
            continue;
        }
        if item_start >= range.end {
            break;
        }

        match item {
            MeasureItem::Text {
                text,
                highlights,
                box_style,
                ..
            } => {
                let local_start = range.start.max(item_start) - item_start;
                let local_end = range.end.min(item_end) - item_start;
                let subtext = &text[local_start..local_end];
                if let Some(box_style) = box_style {
                    let highlights =
                        slice_ranges(highlights, local_start, local_end, |range, style| {
                            (range, *style)
                        });
                    let item_text_style = text_style_for_box(text_style, Some(box_style));
                    let item_font_size = box_style.font_size.unwrap_or(font_size);
                    let runs = runs_for_highlights(subtext, &item_text_style, highlights);
                    let shaped_line = shape_line(subtext.into(), item_font_size, &runs, window);
                    fragments.push(WrapLineFragment::element(
                        box_style.outer_width(shaped_line.width()),
                        subtext.len(),
                    ));
                } else {
                    fragments.push(WrapLineFragment::text(subtext));
                }
            }
            MeasureItem::Image { .. } => {
                fragments.push(WrapLineFragment::element(
                    image_layouts[ix]
                        .expect("image should be measured before wrapping")
                        .outer_size
                        .width,
                    IMAGE_LEN,
                ));
            }
            MeasureItem::Custom { size, len } => {
                fragments.push(WrapLineFragment::element(size.width, *len));
            }
        }
        item_start = item_end;
    }

    fragments
}

/// Measure an inline image with the same base dimensions and semantic style used for painting.
///
/// Keeping `base_size` separate prevents padding, border, or margin from being applied twice when
/// the already-measured image is reconstructed during prepaint.
#[allow(clippy::too_many_arguments)]
fn measure_image_layout(
    ix: usize,
    url: &SharedUri,
    width: Option<DefiniteLength>,
    height: Option<DefiniteLength>,
    sizing: InlineImageSizing,
    style: &StyleRefinement,
    line_height: Pixels,
    rem_size: Pixels,
    window: &mut Window,
    cx: &mut App,
) -> MeasuredImageLayout {
    let requires_intrinsic = width.is_none() || height.is_none();
    let intrinsic_size = if !requires_intrinsic {
        None
    } else {
        intrinsic_image_size(ix, url, width, height, window, cx)
    };
    let base_size = image_size(width, height, intrinsic_size, sizing, line_height, rem_size);
    // A layout-only container avoids loading the image twice while applying the same GPUI box
    // model as the painted image. Intrinsic image dimensions have already been resolved above.
    let mut element = gpui::div()
        .w(base_size.width)
        .h(base_size.height)
        .refine_style(style)
        .into_any_element();
    let mut outer_size = element.layout_as_root(AvailableSpace::min_size(), window, cx);
    let margin_base = AbsoluteLength::Pixels(outer_size.width);
    let margin_to_pixels = |margin: Option<Length>| match margin {
        Some(Length::Definite(length)) => length.to_pixels(margin_base, rem_size),
        Some(Length::Auto) | None => Pixels::ZERO,
    };
    outer_size.width += margin_to_pixels(style.margin.left) + margin_to_pixels(style.margin.right);
    outer_size.height += margin_to_pixels(style.margin.top) + margin_to_pixels(style.margin.bottom);

    MeasuredImageLayout {
        base_size,
        outer_size,
        intrinsic_resolved: !requires_intrinsic || intrinsic_size.is_some(),
    }
}

fn intrinsic_image_size(
    ix: usize,
    url: &SharedUri,
    width: Option<DefiniteLength>,
    height: Option<DefiniteLength>,
    window: &mut Window,
    cx: &mut App,
) -> Option<Size<Pixels>> {
    let mut element = img(markdown_image_source(url))
        .id(ix)
        .object_fit(ObjectFit::Contain)
        .max_w(relative(1.))
        .when_some(width, |this, width| this.w(width))
        .when_some(height, |this, height| this.h(height))
        .into_any_element();
    let measured_size = element.layout_as_root(AvailableSpace::min_size(), window, cx);

    if measured_size.width <= Pixels::ZERO || measured_size.height <= Pixels::ZERO {
        None
    } else {
        Some(measured_size)
    }
}

fn image_size(
    width: Option<DefiniteLength>,
    height: Option<DefiniteLength>,
    intrinsic_size: Option<Size<Pixels>>,
    sizing: InlineImageSizing,
    line_height: Pixels,
    rem_size: Pixels,
) -> Size<Pixels> {
    let base_size = AbsoluteLength::Pixels(line_height);
    match (width, height) {
        (Some(width), Some(height)) => size(
            width.to_pixels(base_size, rem_size),
            height.to_pixels(base_size, rem_size),
        ),
        (Some(width), None) => {
            let width = width.to_pixels(base_size, rem_size);
            let height = intrinsic_size
                .and_then(|intrinsic_size| {
                    (intrinsic_size.width > Pixels::ZERO && intrinsic_size.height > Pixels::ZERO)
                        .then(|| width * (intrinsic_size.height / intrinsic_size.width))
                })
                .unwrap_or(line_height);
            size(width, height)
        }
        (None, Some(height)) => {
            let height = height.to_pixels(base_size, rem_size);
            let width = intrinsic_size
                .and_then(|intrinsic_size| {
                    (intrinsic_size.width > Pixels::ZERO && intrinsic_size.height > Pixels::ZERO)
                        .then(|| height * (intrinsic_size.width / intrinsic_size.height))
                })
                .unwrap_or(height);
            size(width, height)
        }
        (None, None) => match sizing {
            InlineImageSizing::Compact => inline_image_size_for_line(intrinsic_size, line_height),
            InlineImageSizing::Intrinsic => {
                intrinsic_size.unwrap_or_else(|| inline_image_size_for_line(None, line_height))
            }
        },
    }
}

fn inline_image_size_for_line(
    intrinsic_size: Option<Size<Pixels>>,
    line_height: Pixels,
) -> Size<Pixels> {
    let height = line_height * 0.75;
    let aspect_ratio = intrinsic_size
        .and_then(|intrinsic_size| {
            (intrinsic_size.width > Pixels::ZERO && intrinsic_size.height > Pixels::ZERO)
                .then(|| intrinsic_size.width / intrinsic_size.height)
        })
        .unwrap_or(1.);

    size((height * aspect_ratio).max(px(1.)), height.max(px(1.)))
}

fn runs_for_highlights(
    text: &str,
    default_style: &TextStyle,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
) -> Vec<TextRun> {
    let mut runs = Vec::new();
    let mut ix = 0;

    for (range, highlight) in highlights {
        if ix < range.start {
            runs.push(default_style.clone().to_run(range.start - ix));
        }
        runs.push(
            default_style
                .clone()
                .highlight(highlight)
                .to_run(range.len()),
        );
        ix = range.end;
    }

    if ix < text.len() {
        runs.push(default_style.to_run(text.len() - ix));
    }

    runs
}

fn shape_line(
    text: SharedString,
    font_size: Pixels,
    runs: &[TextRun],
    window: &mut Window,
) -> ShapedLine {
    debug_assert!(
        !text.contains('\n'),
        "InlineFlow must split hard lines before shaping"
    );
    window.text_system().shape_line(text, font_size, runs, None)
}

fn slice_ranges<T, U>(
    ranges: &[(Range<usize>, T)],
    start: usize,
    end: usize,
    map: impl Fn(Range<usize>, &T) -> U,
) -> Vec<U> {
    ranges
        .iter()
        .filter_map(|(range, value)| {
            let clipped_start = range.start.max(start);
            let clipped_end = range.end.min(end);
            (clipped_start < clipped_end)
                .then(|| map((clipped_start - start)..(clipped_end - start), value))
        })
        .collect()
}

/// Clip links to a wrapped fragment while preserving their source-item identity.
fn slice_links(links: &[InlineLink], start: usize, end: usize) -> Vec<InlineLink> {
    links
        .iter()
        .filter_map(|link| {
            let clipped_start = link.range.start.max(start);
            let clipped_end = link.range.end.min(end);
            (clipped_start < clipped_end).then(|| InlineLink {
                id: link.id,
                range: (clipped_start - start)..(clipped_end - start),
                mark: link.mark.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        AppContext as _, Context, Render, Resource, TestAppContext, VisualTestContext, div, point,
    };
    use std::cell::Cell;

    struct InlineFlowTestRoot;

    impl Render for InlineFlowTestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    struct ImageMeasureProbe {
        style: StyleRefinement,
    }

    struct InlineBoxMeasureProbe {
        text: SharedString,
        box_style: Option<InlineBoxStyle>,
    }

    #[test]
    fn inline_flow_layout_cache_reuses_matching_inputs_and_replaces_changed_width() {
        let cache = InlineFlowLayoutCache::default();
        let key = InlineFlowLayoutKey {
            semantic_styles: Vec::new(),
            resource_policy: crate::text::MarkdownResourcePolicy::Trusted,
            image_layouts: Vec::new(),
            custom_sizes: Vec::new(),
            text_style: TextStyle::default(),
            wrap_width: Some(px(320.)),
            line_height: px(20.),
            rem_size: px(16.),
        };
        let mut builds = 0;

        let first = cache.get_or_insert_with(key.clone(), || {
            builds += 1;
            InlineFlowLayout::default()
        });
        let second = cache.get_or_insert_with(key.clone(), || {
            builds += 1;
            InlineFlowLayout::default()
        });

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(builds, 1);

        cache.get_or_insert_with(
            InlineFlowLayoutKey {
                wrap_width: Some(px(240.)),
                ..key
            },
            || {
                builds += 1;
                InlineFlowLayout::default()
            },
        );
        assert_eq!(builds, 2);
    }

    #[test]
    fn image_measurement_cache_waits_for_intrinsic_dimensions_then_reuses_them() {
        let cache = InlineFlowLayoutCache::default();
        let key = InlineImageMeasurementKey {
            images: Vec::new(),
            line_height: px(20.),
            rem_size: px(16.),
        };
        let measurements = Cell::new(0);
        let layout = |intrinsic_resolved| MeasuredImageLayout {
            base_size: size(px(20.), px(20.)),
            outer_size: size(px(20.), px(20.)),
            intrinsic_resolved,
        };

        cache.image_measurements(key.clone(), || {
            measurements.set(measurements.get() + 1);
            vec![Some(layout(false))]
        });
        cache.image_measurements(key.clone(), || {
            measurements.set(measurements.get() + 1);
            vec![Some(layout(true))]
        });
        cache.image_measurements(key, || {
            measurements.set(measurements.get() + 1);
            vec![Some(layout(true))]
        });

        assert_eq!(measurements.get(), 2);
    }

    #[test]
    fn intrinsic_sizing_preserves_block_dimensions_while_compact_sizing_uses_line_height() {
        let intrinsic = size(px(640.), px(320.));
        let block = image_size(
            None,
            None,
            Some(intrinsic),
            InlineImageSizing::Intrinsic,
            px(20.),
            px(16.),
        );
        let inline = image_size(
            None,
            None,
            Some(intrinsic),
            InlineImageSizing::Compact,
            px(20.),
            px(16.),
        );

        assert_eq!(block, intrinsic);
        assert_eq!(inline, size(px(30.), px(15.)));
    }

    #[test]
    fn markdown_image_source_preserves_embedded_and_remote_resource_kinds() {
        let embedded: SharedUri = "icons/heart.svg".into();
        assert!(matches!(
            markdown_image_source(&embedded),
            ImageSource::Resource(Resource::Embedded(path)) if path.as_ref() == "icons/heart.svg"
        ));

        let remote: SharedUri = "https://example.com/image.svg".into();
        assert!(matches!(
            markdown_image_source(&remote),
            ImageSource::Resource(Resource::Uri(uri))
                if uri.as_ref() == "https://example.com/image.svg"
        ));
    }

    impl IntoElement for ImageMeasureProbe {
        type Element = Self;

        fn into_element(self) -> Self::Element {
            self
        }
    }

    impl Element for ImageMeasureProbe {
        type RequestLayoutState = MeasuredImageLayout;
        type PrepaintState = ();

        fn id(&self) -> Option<ElementId> {
            None
        }

        fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
            None
        }

        fn request_layout(
            &mut self,
            _id: Option<&GlobalElementId>,
            _inspector_id: Option<&InspectorElementId>,
            window: &mut Window,
            cx: &mut App,
        ) -> (LayoutId, Self::RequestLayoutState) {
            let url: SharedUri = "https://example.com/image.png".into();
            let measured = measure_image_layout(
                0,
                &url,
                Some(px(20.).into()),
                Some(px(10.).into()),
                InlineImageSizing::Compact,
                &self.style,
                px(20.),
                window.rem_size(),
                window,
                cx,
            );
            (window.request_layout(Default::default(), [], cx), measured)
        }

        fn prepaint(
            &mut self,
            _id: Option<&GlobalElementId>,
            _inspector_id: Option<&InspectorElementId>,
            _bounds: Bounds<Pixels>,
            _request_layout: &mut Self::RequestLayoutState,
            _window: &mut Window,
            _cx: &mut App,
        ) {
        }

        fn paint(
            &mut self,
            _id: Option<&GlobalElementId>,
            _inspector_id: Option<&InspectorElementId>,
            _bounds: Bounds<Pixels>,
            _request_layout: &mut Self::RequestLayoutState,
            _prepaint: &mut Self::PrepaintState,
            _window: &mut Window,
            _cx: &mut App,
        ) {
        }
    }

    impl IntoElement for InlineBoxMeasureProbe {
        type Element = Self;

        fn into_element(self) -> Self::Element {
            self
        }
    }

    impl Element for InlineBoxMeasureProbe {
        type RequestLayoutState = Size<Pixels>;
        type PrepaintState = ();

        fn id(&self) -> Option<ElementId> {
            None
        }

        fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
            None
        }

        fn request_layout(
            &mut self,
            _id: Option<&GlobalElementId>,
            _inspector_id: Option<&InspectorElementId>,
            window: &mut Window,
            cx: &mut App,
        ) -> (LayoutId, Self::RequestLayoutState) {
            let items = vec![MeasureItem::Text {
                text: self.text.clone(),
                links: vec![],
                highlights: vec![],
                link_hover_style: None,
                box_style: self.box_style.clone(),
            }];
            let layout = layout_flow(&items, &[None], &window.text_style(), None, window);
            (
                window.request_layout(Default::default(), [], cx),
                layout.size,
            )
        }

        fn prepaint(
            &mut self,
            _id: Option<&GlobalElementId>,
            _inspector_id: Option<&InspectorElementId>,
            _bounds: Bounds<Pixels>,
            _request_layout: &mut Self::RequestLayoutState,
            _window: &mut Window,
            _cx: &mut App,
        ) {
        }

        fn paint(
            &mut self,
            _id: Option<&GlobalElementId>,
            _inspector_id: Option<&InspectorElementId>,
            _bounds: Bounds<Pixels>,
            _request_layout: &mut Self::RequestLayoutState,
            _prepaint: &mut Self::PrepaintState,
            _window: &mut Window,
            _cx: &mut App,
        ) {
        }
    }

    #[test]
    fn wrapped_link_fragments_preserve_the_same_hover_identity() {
        let links = vec![InlineLink {
            id: 7,
            range: 2..12,
            mark: LinkMark {
                url: "https://example.com".into(),
                ..Default::default()
            },
        }];

        let first = slice_links(&links, 0, 6);
        let second = slice_links(&links, 6, 14);

        assert_eq!(first[0].id, 7);
        assert_eq!(first[0].range, 2..6);
        assert_eq!(second[0].id, 7);
        assert_eq!(second[0].range, 0..6);
    }

    #[test]
    fn hard_line_ranges_exclude_newlines_and_preserve_empty_lines() {
        let items = vec![MeasureItem::Text {
            text: "first\n\nsecond".into(),
            links: vec![],
            highlights: vec![],
            link_hover_style: None,
            box_style: None,
        }];

        assert_eq!(hard_line_ranges(&items), vec![0..5, 6..6, 7..13]);
    }

    #[test]
    fn wrapped_fragments_reuse_persistent_hover_state_across_frames() {
        let source_state = Arc::new(Mutex::new(InlineState::default()));
        let item = InlineFlowItem::Text {
            state: source_state.clone(),
            paragraph_range: 0..12,
            text: "wrapped text".into(),
            links: vec![],
            highlights: vec![],
            link_hover_style: None,
            box_style: None,
        };

        let (first_selection, first_hover) =
            InlineFlow::text_fragment_states(&item).expect("text fragment states");
        let (second_selection, second_hover) =
            InlineFlow::text_fragment_states(&item).expect("text fragment states");

        assert!(!Arc::ptr_eq(&first_selection, &second_selection));
        assert!(Arc::ptr_eq(&first_hover, &source_state));
        assert!(Arc::ptr_eq(&second_hover, &source_state));
    }

    #[gpui::test]
    fn inline_box_horizontal_padding_contributes_to_layout(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| InlineFlowTestRoot);
            crate::Root::new(content, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        let draw_text = |box_style, cx: &mut VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(100.)), |_, _| {
                InlineBoxMeasureProbe {
                    text: "code".into(),
                    box_style,
                }
            })
            .0
        };

        let plain = draw_text(None, cx);
        let boxed = draw_text(
            Some(InlineBoxStyle {
                background: None,
                padding_x: px(4.),
                corner_radius: px(6.),
                ..Default::default()
            }),
            cx,
        );

        assert_eq!(boxed.width, plain.width + px(8.));
        assert_eq!(boxed.height, plain.height);
    }

    #[gpui::test]
    fn inline_box_vertical_box_model_contributes_to_layout(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| InlineFlowTestRoot);
            crate::Root::new(content, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        let draw_text = |box_style, cx: &mut VisualTestContext| {
            cx.draw(point(px(0.), px(0.)), size(px(300.), px(100.)), |_, _| {
                InlineBoxMeasureProbe {
                    text: "code".into(),
                    box_style,
                }
            })
            .0
        };

        let plain = draw_text(None, cx);
        let boxed = draw_text(
            Some(InlineBoxStyle {
                padding_y: px(2.),
                margin_y: px(1.),
                border_width: px(1.),
                ..Default::default()
            }),
            cx,
        );

        assert_eq!(boxed.width, plain.width + px(2.));
        assert_eq!(boxed.height, plain.height + px(8.));
    }

    #[gpui::test]
    fn inline_flow_shapes_streamed_hard_lines_independently(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| InlineFlowTestRoot);
            crate::Root::new(content, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        let draw_text = |text: &'static str, cx: &mut VisualTestContext| {
            cx.draw(
                point(px(0.), px(0.)),
                size(px(300.), px(100.)),
                move |_, _| InlineBoxMeasureProbe {
                    text: text.into(),
                    box_style: Some(InlineBoxStyle {
                        background: None,
                        padding_x: px(4.),
                        corner_radius: px(6.),
                        ..Default::default()
                    }),
                },
            )
            .0
        };
        let single_line = draw_text("pending link", cx);
        let measured = draw_text("pending link\nnext frame", cx);

        assert_eq!(measured.height, single_line.height * 2.);
    }

    #[test]
    fn inline_image_without_explicit_size_scales_intrinsic_ratio_to_line_height() {
        let line_height = px(20.);
        let intrinsic_size = size(px(160.), px(40.));

        let measured = inline_image_size_for_line(Some(intrinsic_size), line_height);

        assert_eq!(measured, size(px(60.), px(15.)));
    }

    #[test]
    fn inline_image_without_intrinsic_size_uses_compact_square_fallback() {
        let measured = inline_image_size_for_line(None, px(20.));

        assert_eq!(measured, size(px(15.), px(15.)));
    }

    #[gpui::test]
    fn semantic_image_layout_includes_dimensions_and_box_model(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| InlineFlowTestRoot);
            crate::Root::new(content, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        let (dimensions, _) = cx.draw(point(px(0.), px(0.)), size(px(300.), px(100.)), |_, _| {
            ImageMeasureProbe {
                style: StyleRefinement::default().w(px(48.)).h(px(24.)),
            }
        });
        let (boxed, _) = cx.draw(point(px(0.), px(0.)), size(px(300.), px(100.)), |_, _| {
            ImageMeasureProbe {
                style: StyleRefinement::default()
                    .w(px(48.))
                    .h(px(24.))
                    .p(px(4.))
                    .border(px(1.))
                    .m(px(3.)),
            }
        });

        assert_eq!(dimensions.base_size, size(px(20.), px(10.)));
        assert_eq!(dimensions.outer_size, size(px(48.), px(24.)));
        assert!(boxed.outer_size.width > dimensions.outer_size.width);
        assert!(boxed.outer_size.height > dimensions.outer_size.height);
    }
}
