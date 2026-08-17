use std::{
    ops::Range,
    sync::{Arc, Mutex},
};

use gpui::{
    AbsoluteLength, AnyElement, App, AvailableSpace, Bounds, DefiniteLength, Element, ElementId,
    GlobalElementId, HighlightStyle, ImageSource, InspectorElementId, InteractiveElement as _,
    IntoElement, LayoutId, Length, LineFragment as WrapLineFragment, ObjectFit, ParentElement as _,
    Pixels, ShapedLine, SharedString, SharedUri, Size, StatefulInteractiveElement as _,
    StyleRefinement, Styled, StyledImage as _, TextRun, TextStyle, WhiteSpace, Window, div, img,
    point, prelude::FluentBuilder as _, px, relative, size,
};

use crate::{StyledExt as _, WindowExt as _, tooltip::Tooltip};

use super::{
    inline::{Inline, InlineLink, InlineSelectionSink, InlineState},
    node::LinkMark,
};
use crate::text::MarkdownTextStyle;

const IMAGE_LEN: usize = 1;

/// Classify absolute Markdown image URLs as remote resources and relative paths as embedded assets.
fn markdown_image_source(url: &SharedUri) -> ImageSource {
    url.as_ref().into()
}

pub(super) struct InlineFlow {
    id: ElementId,
    items: Vec<InlineFlowItem>,
    selection_state: Option<Arc<Mutex<InlineState>>>,
}

/// Layout and paint properties for an atomic inline text box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct InlineBoxStyle {
    pub(super) background: Option<gpui::Hsla>,
    pub(super) padding_x: Pixels,
    pub(super) corner_radius: Pixels,
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
        link: Option<LinkMark>,
        title: String,
        width: Option<DefiniteLength>,
        height: Option<DefiniteLength>,
        style: Box<StyleRefinement>,
    },
}

#[derive(Default)]
pub(crate) struct InlineFlowLayoutState {
    layout: Arc<Mutex<Option<InlineFlowLayout>>>,
}

#[derive(Default)]
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
        box_style: Option<InlineBoxStyle>,
    },
    Image {
        item_ix: usize,
        origin: gpui::Point<Pixels>,
        size: Size<Pixels>,
        base_size: Size<Pixels>,
    },
}

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
        style: Box<StyleRefinement>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MeasuredImageLayout {
    /// Intrinsic or Markdown-attribute size applied before semantic refinement.
    base_size: Size<Pixels>,
    /// Final GPUI layout size, including semantic dimensions and box-model styles.
    outer_size: Size<Pixels>,
}

struct LineFragmentLayout {
    item_ix: usize,
    kind: LineFragmentKind,
    size: Size<Pixels>,
    source_range: Range<usize>,
}

enum LineFragmentKind {
    Text {
        text: SharedString,
        links: Vec<InlineLink>,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
        link_hover_style: Option<Arc<MarkdownTextStyle>>,
        box_style: Option<InlineBoxStyle>,
    },
    Image {
        base_size: Size<Pixels>,
    },
}

impl InlineFlow {
    pub(super) fn new(id: impl Into<ElementId>, items: Vec<InlineFlowItem>) -> Self {
        Self {
            id: id.into(),
            items,
            selection_state: None,
        }
    }

    /// Store all fragment selections in one paragraph-level state.
    pub(super) fn selection_state(mut self, state: Arc<Mutex<InlineState>>) -> Self {
        self.selection_state = Some(state);
        self
    }

    fn image_element(
        ix: usize,
        url: &SharedUri,
        link: &Option<LinkMark>,
        title: &str,
        base_size: Size<Pixels>,
        style: &StyleRefinement,
    ) -> AnyElement {
        img(markdown_image_source(url))
            .id(ix)
            .object_fit(ObjectFit::Contain)
            .max_w(relative(1.))
            .w(base_size.width)
            .h(base_size.height)
            .refine_style(style)
            .when_some(
                link.clone()
                    .filter(|link| link.url.as_ref() != crate::text::streaming::PENDING_LINK_URL),
                |this, link| {
                    let title = title.to_string();
                    this.cursor_pointer()
                        .tooltip(move |window, cx| Tooltip::new(title.clone()).build(window, cx))
                        .on_click(move |_, window, cx| {
                            window.end_text_selection(cx);
                            cx.stop_propagation();
                            cx.open_url(&link.url);
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
        let measure_items = self.items.iter().map(MeasureItem::from).collect::<Vec<_>>();
        let line_height = window.line_height();
        let rem_size = window.rem_size();
        let image_layouts = measure_items
            .iter()
            .enumerate()
            .map(|(ix, item)| match item {
                MeasureItem::Image {
                    url,
                    width,
                    height,
                    style,
                } => Some(measure_image_layout(
                    ix,
                    url,
                    *width,
                    *height,
                    style,
                    line_height,
                    rem_size,
                    window,
                    cx,
                )),
                MeasureItem::Text { .. } => None,
            })
            .collect::<Vec<_>>();
        let layout_state = InlineFlowLayoutState::default();
        let layout_ref = layout_state.layout.clone();

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
                let layout = layout_flow(
                    &measure_items,
                    &image_layouts,
                    &text_style,
                    wrap_width,
                    window,
                );
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
        let fragments = request_layout
            .layout
            .lock()
            .ok()
            .and_then(|layout| layout.as_ref().map(|layout| layout.fragments.clone()))
            .unwrap_or_default();
        let mut elements = Vec::with_capacity(fragments.len());

        for fragment in fragments {
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
                    ..
                } => {
                    let Some((state, hover_state)) =
                        Self::text_fragment_states(&self.items[item_ix])
                    else {
                        continue;
                    };
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
                        InlineFlowItem::Image { .. } => continue,
                    };

                    let mut inline = Inline::new("text", state, links, highlights)
                        .link_hover_style(link_hover_style)
                        .hover_state(hover_state, elements.len());
                    if let Some(selection_state) = &self.selection_state {
                        inline = inline.selection_sink(InlineSelectionSink::new(
                            selection_state.clone(),
                            paragraph_range,
                            bounds,
                        ));
                    }
                    let inline = inline.into_any_element();
                    let mut element = if let Some(box_style) = box_style {
                        div()
                            .id(elements.len())
                            .flex()
                            .items_center()
                            .size_full()
                            .px(box_style.padding_x)
                            .rounded(box_style.corner_radius)
                            .when_some(box_style.background, |this, background| this.bg(background))
                            .child(inline)
                            .into_any_element()
                    } else {
                        inline
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
                PositionedFragment::Image {
                    item_ix,
                    origin,
                    size: fragment_size,
                    base_size,
                } => {
                    let InlineFlowItem::Image {
                        url,
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
                        url,
                        link,
                        title.as_str(),
                        base_size,
                        style.as_ref(),
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
                box_style: *box_style,
            },
            InlineFlowItem::Image {
                url,
                width,
                height,
                style,
                ..
            } => MeasureItem::Image {
                url: url.clone(),
                width: *width,
                height: *height,
                style: style.clone(),
            },
        }
    }
}

impl MeasureItem {
    fn len(&self) -> usize {
        match self {
            MeasureItem::Text { text, .. } => text.len(),
            MeasureItem::Image { .. } => IMAGE_LEN,
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
                        let runs = runs_for_highlights(&subtext, text_style, highlights.clone());
                        let shaped_line = shape_line(subtext.clone(), font_size, &runs, window);
                        let width = shaped_line.width()
                            + box_style
                                .map(|style| style.padding_x * 2.)
                                .unwrap_or_default();
                        line_width += width;
                        line_fragments.push(LineFragmentLayout {
                            item_ix,
                            kind: LineFragmentKind::Text {
                                text: subtext,
                                links,
                                highlights,
                                link_hover_style: link_hover_style.clone(),
                                box_style: *box_style,
                            },
                            size: size(width, line_height),
                            source_range: local_start..local_end,
                        });
                    }
                }
                MeasureItem::Image { .. } => {
                    if line_range.start <= item_start && item_end <= line_range.end {
                        let image_layout =
                            image_layouts[item_ix].expect("image should be measured before layout");
                        line_width += image_layout.outer_size.width;
                        actual_line_height = actual_line_height.max(image_layout.outer_size.height);
                        line_fragments.push(LineFragmentLayout {
                            item_ix,
                            kind: LineFragmentKind::Image {
                                base_size: image_layout.base_size,
                            },
                            size: image_layout.outer_size,
                            source_range: 0..IMAGE_LEN,
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
                } => PositionedFragment::Text {
                    item_ix: fragment.item_ix,
                    origin,
                    size: fragment.size,
                    source_range: fragment.source_range,
                    text,
                    links,
                    highlights,
                    link_hover_style,
                    box_style,
                },
                LineFragmentKind::Image { base_size } => PositionedFragment::Image {
                    item_ix: fragment.item_ix,
                    origin,
                    size: fragment.size,
                    base_size,
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
                    let runs = runs_for_highlights(subtext, text_style, highlights);
                    let shaped_line = shape_line(subtext.into(), font_size, &runs, window);
                    fragments.push(WrapLineFragment::element(
                        shaped_line.width() + box_style.padding_x * 2.,
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
    style: &StyleRefinement,
    line_height: Pixels,
    rem_size: Pixels,
    window: &mut Window,
    cx: &mut App,
) -> MeasuredImageLayout {
    let intrinsic_size = if width.is_some() && height.is_some() {
        None
    } else {
        intrinsic_image_size(ix, url, width, height, window, cx)
    };
    let base_size = image_size(width, height, intrinsic_size, line_height, rem_size);
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
        (None, None) => inline_image_size_for_line(intrinsic_size, line_height),
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
                box_style: self.box_style,
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
            }),
            cx,
        );

        assert_eq!(boxed.width, plain.width + px(8.));
        assert_eq!(boxed.height, plain.height);
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
