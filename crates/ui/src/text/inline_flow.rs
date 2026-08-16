use std::{
    ops::Range,
    sync::{Arc, Mutex},
};

use gpui::{
    AbsoluteLength, AnyElement, App, AvailableSpace, Bounds, DefiniteLength, Element, ElementId,
    GlobalElementId, HighlightStyle, InspectorElementId, InteractiveElement as _, IntoElement,
    LayoutId, Length, LineFragment as WrapLineFragment, ObjectFit, Pixels, ShapedLine,
    SharedString, SharedUri, Size, StatefulInteractiveElement as _, StyleRefinement, Styled,
    StyledImage as _, TextRun, TextStyle, WhiteSpace, Window, img, point,
    prelude::FluentBuilder as _, px, relative, size,
};

use crate::{StyledExt as _, WindowExt as _, tooltip::Tooltip};

use super::{
    inline::{Inline, InlineLink, InlineState},
    node::LinkMark,
};
use crate::text::MarkdownTextStyle;

const IMAGE_LEN: usize = 1;

pub(super) struct InlineFlow {
    id: ElementId,
    items: Vec<InlineFlowItem>,
}

pub(super) enum InlineFlowItem {
    Text {
        state: Arc<Mutex<InlineState>>,
        text: SharedString,
        links: Vec<InlineLink>,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
        link_hover_style: Option<MarkdownTextStyle>,
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
        link_hover_style: Option<MarkdownTextStyle>,
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
        link_hover_style: Option<MarkdownTextStyle>,
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
        link_hover_style: Option<MarkdownTextStyle>,
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
        }
    }

    fn image_element(
        ix: usize,
        url: &SharedUri,
        link: &Option<LinkMark>,
        title: &str,
        base_size: Size<Pixels>,
        style: &StyleRefinement,
    ) -> AnyElement {
        img(url.clone())
            .id(ix)
            .object_fit(ObjectFit::Contain)
            .max_w(relative(1.))
            .w(base_size.width)
            .h(base_size.height)
            .refine_style(style)
            .when_some(link.clone(), |this, link| {
                let title = title.to_string();
                this.cursor_pointer()
                    .tooltip(move |window, cx| Tooltip::new(title.clone()).build(window, cx))
                    .on_click(move |_, window, cx| {
                        window.end_text_selection(cx);
                        cx.stop_propagation();
                        cx.open_url(&link.url);
                    })
            })
            .into_any_element()
    }

    /// Resolve fragment-local selection state and persistent source-item hover state.
    fn text_fragment_states(
        item: &InlineFlowItem,
        source_range: &Range<usize>,
    ) -> Option<(Arc<Mutex<InlineState>>, Arc<Mutex<InlineState>>)> {
        let InlineFlowItem::Text {
            state,
            text: source,
            ..
        } = item
        else {
            return None;
        };

        let selection_state = if source_range == &(0..source.len()) {
            state.clone()
        } else {
            Arc::new(Mutex::new(InlineState::default()))
        };
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
                    ..
                } => {
                    let Some((state, hover_state)) =
                        Self::text_fragment_states(&self.items[item_ix], &source_range)
                    else {
                        continue;
                    };
                    if let Ok(mut state) = state.lock() {
                        state.set_text(text);
                    }

                    let mut element = Inline::new(elements.len(), state, links, highlights)
                        .link_hover_style(link_hover_style)
                        .hover_state(hover_state, elements.len())
                        .into_any_element();
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
                ..
            } => MeasureItem::Text {
                text: text.clone(),
                links: links.clone(),
                highlights: highlights.clone(),
                link_hover_style: link_hover_style.clone(),
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
                        let width = shaped_line.width();
                        line_width += width;
                        line_fragments.push(LineFragmentLayout {
                            item_ix,
                            kind: LineFragmentKind::Text {
                                text: subtext,
                                links,
                                highlights,
                                link_hover_style: link_hover_style.clone(),
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
                } => PositionedFragment::Text {
                    item_ix: fragment.item_ix,
                    origin,
                    size: fragment.size,
                    source_range: fragment.source_range,
                    text,
                    links,
                    highlights,
                    link_hover_style,
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
    let total_len = items.iter().map(MeasureItem::len).sum::<usize>();
    let Some(wrap_width) = wrap_width else {
        return std::iter::once(0..total_len).collect();
    };
    let rem_size = window.rem_size();

    let wrap_fragments = items
        .iter()
        .enumerate()
        .map(|(ix, item)| match item {
            MeasureItem::Text { text, .. } => WrapLineFragment::text(text),
            MeasureItem::Image { .. } => WrapLineFragment::element(
                image_layouts[ix]
                    .expect("image should be measured before wrapping")
                    .outer_size
                    .width,
                IMAGE_LEN,
            ),
        })
        .collect::<Vec<_>>();
    let font_size = text_style.font_size.to_pixels(rem_size);
    let mut wrapper = window
        .text_system()
        .line_wrapper(text_style.font(), font_size);
    let boundaries = wrapper
        .wrap_line(&wrap_fragments, wrap_width)
        .map(|boundary| boundary.ix.min(total_len))
        .collect::<Vec<_>>();
    let mut ranges = Vec::with_capacity(boundaries.len() + 1);
    let mut start = 0;

    for end in boundaries {
        if start < end {
            ranges.push(start..end);
        }
        start = end;
    }

    if start < total_len {
        ranges.push(start..total_len);
    }

    ranges
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
    let mut element = img(url.clone())
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
    use gpui::{AppContext as _, Context, Render, TestAppContext, VisualTestContext, div, point};

    struct InlineFlowTestRoot;

    impl Render for InlineFlowTestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    struct ImageMeasureProbe {
        style: StyleRefinement,
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
    fn wrapped_fragments_reuse_persistent_hover_state_across_frames() {
        let source_state = Arc::new(Mutex::new(InlineState::default()));
        let item = InlineFlowItem::Text {
            state: source_state.clone(),
            text: "wrapped text".into(),
            links: vec![],
            highlights: vec![],
            link_hover_style: None,
        };

        let (first_selection, first_hover) =
            InlineFlow::text_fragment_states(&item, &(0..7)).expect("text fragment states");
        let (second_selection, second_hover) =
            InlineFlow::text_fragment_states(&item, &(0..7)).expect("text fragment states");

        assert!(!Arc::ptr_eq(&first_selection, &second_selection));
        assert!(Arc::ptr_eq(&first_hover, &source_state));
        assert!(Arc::ptr_eq(&second_hover, &source_state));
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
