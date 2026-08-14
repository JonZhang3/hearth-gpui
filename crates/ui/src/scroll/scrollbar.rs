// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added or exposed behavior through `resolve`, `cross_inset`, `visible_duration`,
//   `scrollbar_axis_geometry`, `metrics_follow_semantic_style_presets`,
//   `geometry_rejects_non_overflowing_content`, `geometry_stays_inside_tiny_containers`,
//   `geometry_clamps_out_of_range_offsets` and 2 more.
// - Reworked Scrollbar around interruptible and reduced-motion-aware transitions, semantic Style
//   Preset geometry and density.
use std::{cell::Cell, ops::Deref, panic::Location, rc::Rc};

use instant::{Duration, Instant};

use crate::{ActiveTheme, AxisExt, MotionEasing, StylePreset};
use gpui::{
    Anchor, App, Axis, Background, Bounds, ContentMask, CursorStyle, Element, ElementId,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId, ListState,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Position, ScrollHandle,
    ScrollWheelEvent, Size, Style, UniformListScrollHandle, Window, fill, point, px, relative,
    size,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Desktop-sized pointer target used by the overlaid scrollbar layer.
const WIDTH: Pixels = px(4. * 2. + 8.);
const TRACK_WIDTH: Pixels = px(10.);
const MIN_THUMB_SIZE: Pixels = px(48.);
const THUMB_WIDTH: Pixels = px(6.);
const THUMB_ACTIVE_WIDTH: Pixels = px(8.);
const AUTO_HIDE_DELAY: Duration = Duration::from_secs(2);

/// Component-local presentation resolved from semantic Style Preset metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ScrollbarMetrics {
    hit_width: Pixels,
    track_width: Pixels,
    min_thumb_size: Pixels,
    thumb_width: Pixels,
    active_thumb_width: Pixels,
    thumb_end_inset: Pixels,
    thumb_radius: Pixels,
    track_radius: Pixels,
    fade_delay: Duration,
    fade_duration: Duration,
    fade_easing: MotionEasing,
}

impl ScrollbarMetrics {
    /// Resolves scrollbar paint metrics while preserving a generous desktop hit target.
    fn resolve(style: &StylePreset) -> Self {
        Self {
            hit_width: WIDTH,
            track_width: TRACK_WIDTH,
            min_thumb_size: MIN_THUMB_SIZE,
            thumb_width: THUMB_WIDTH,
            active_thumb_width: THUMB_ACTIVE_WIDTH,
            thumb_end_inset: px(1.),
            thumb_radius: style.radii.sm,
            track_radius: style.radii.sm,
            fade_delay: AUTO_HIDE_DELAY,
            fade_duration: style.motion.slow(),
            fade_easing: style.motion.exit_easing,
        }
    }

    /// Returns the centered cross-axis inset for a visual inside the hit target.
    fn cross_inset(self, visual_width: Pixels) -> Pixels {
        ((self.hit_width - visual_width) / 2.).max(px(0.))
    }

    /// Returns the total time for which an auto-hidden scrollbar remains mounted.
    fn visible_duration(self) -> Duration {
        self.fade_delay + self.fade_duration
    }
}

#[derive(Debug, Clone, Copy)]
struct ScrollbarPaintStyle {
    thumb: Background,
    track: Background,
    thumb_width: Pixels,
    thumb_radius: Pixels,
}

/// Stable one-axis geometry shared by prepaint and regression tests.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ScrollbarAxisGeometry {
    available_length: Pixels,
    thumb_length: Pixels,
    thumb_start: Pixels,
}

/// Resolves thumb geometry and rejects non-scrollable or degenerate layouts.
fn scrollbar_axis_geometry(
    container_size: Pixels,
    scroll_area_size: Pixels,
    scroll_position: Pixels,
    margin_end: Pixels,
    min_thumb_size: Pixels,
) -> Option<ScrollbarAxisGeometry> {
    let available_length = (container_size - margin_end).max(px(0.));
    let scroll_range = scroll_area_size - container_size;
    if available_length <= px(0.) || scroll_range <= px(0.) || scroll_area_size <= px(0.) {
        return None;
    }

    let thumb_length = (container_size / scroll_area_size * available_length)
        .max(min_thumb_size.min(available_length))
        .min(available_length);
    let scroll_progress = (-scroll_position / scroll_range).clamp(0., 1.);
    let thumb_start = scroll_progress * (available_length - thumb_length);

    Some(ScrollbarAxisGeometry {
        available_length,
        thumb_length,
        thumb_start,
    })
}

/// Scrollbar show mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, Default, JsonSchema)]
pub enum ScrollbarShow {
    /// Show scrollbar when scrolling, will fade out after idle.
    #[default]
    Scrolling,
    /// Show scrollbar on hover.
    Hover,
    /// Always show scrollbar.
    Always,
}

impl ScrollbarShow {
    fn is_hover(&self) -> bool {
        matches!(self, Self::Hover)
    }

    fn is_always(&self) -> bool {
        matches!(self, Self::Always)
    }
}

/// A trait for scroll handles that can get and set offset.
pub trait ScrollbarHandle: 'static {
    /// Get the current offset of the scroll handle.
    fn offset(&self) -> Point<Pixels>;
    /// Set the offset of the scroll handle.
    fn set_offset(&self, offset: Point<Pixels>);
    /// The full size of the content, including padding.
    fn content_size(&self) -> Size<Pixels>;
    /// Called when start dragging the scrollbar thumb.
    fn start_drag(&self) {}
    /// Called when end dragging the scrollbar thumb.
    fn end_drag(&self) {}
}

impl ScrollbarHandle for ScrollHandle {
    fn offset(&self) -> Point<Pixels> {
        self.offset()
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.set_offset(offset);
    }

    fn content_size(&self) -> Size<Pixels> {
        (self.max_offset() + self.bounds().size.into()).into()
    }
}

impl ScrollbarHandle for UniformListScrollHandle {
    fn offset(&self) -> Point<Pixels> {
        self.0.borrow().base_handle.offset()
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.0.borrow_mut().base_handle.set_offset(offset)
    }

    fn content_size(&self) -> Size<Pixels> {
        let base_handle = &self.0.borrow().base_handle;
        (base_handle.max_offset() + base_handle.bounds().size.into()).into()
    }
}

impl ScrollbarHandle for ListState {
    fn offset(&self) -> Point<Pixels> {
        self.scroll_px_offset_for_scrollbar()
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.set_offset_from_scrollbar(offset);
    }

    fn content_size(&self) -> Size<Pixels> {
        self.viewport_bounds().size + self.max_offset_for_scrollbar().into()
    }

    fn start_drag(&self) {
        self.scrollbar_drag_started();
    }

    fn end_drag(&self) {
        self.scrollbar_drag_ended();
    }
}

#[doc(hidden)]
#[derive(Debug, Clone)]
struct ScrollbarState(Rc<Cell<ScrollbarStateInner>>);

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
struct ScrollbarStateInner {
    hovered_axis: Option<Axis>,
    hovered_on_thumb: Option<Axis>,
    dragged_axis: Option<Axis>,
    drag_pos: Point<Pixels>,
    last_scroll_offset: Point<Pixels>,
    last_scroll_time: Option<Instant>,
    // Last update offset
    last_update: Instant,
    idle_timer_scheduled: bool,
}

impl Default for ScrollbarState {
    fn default() -> Self {
        Self(Rc::new(Cell::new(ScrollbarStateInner {
            hovered_axis: None,
            hovered_on_thumb: None,
            dragged_axis: None,
            drag_pos: point(px(0.), px(0.)),
            last_scroll_offset: point(px(0.), px(0.)),
            last_scroll_time: None,
            last_update: Instant::now(),
            idle_timer_scheduled: false,
        })))
    }
}

impl Deref for ScrollbarState {
    type Target = Rc<Cell<ScrollbarStateInner>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ScrollbarStateInner {
    fn with_drag_pos(&self, axis: Axis, pos: Point<Pixels>) -> Self {
        let mut state = *self;
        if axis.is_vertical() {
            state.drag_pos.y = pos.y;
        } else {
            state.drag_pos.x = pos.x;
        }

        state.dragged_axis = Some(axis);
        state
    }

    fn with_unset_drag_pos(&self) -> Self {
        let mut state = *self;
        state.dragged_axis = None;
        state
    }

    fn with_hovered(&self, axis: Option<Axis>) -> Self {
        let mut state = *self;
        state.hovered_axis = axis;
        if axis.is_some() {
            state.last_scroll_time = Some(Instant::now());
        }
        state
    }

    fn with_hovered_on_thumb(&self, axis: Option<Axis>, visible_duration: Duration) -> Self {
        let mut state = *self;
        state.hovered_on_thumb = axis;
        // A hidden scrolling-only scrollbar must not become visible from pointer hover.
        if axis.is_some() && self.is_scrollbar_visible(visible_duration) {
            state.last_scroll_time = Some(Instant::now());
        }
        state
    }

    fn with_last_scroll(
        &self,
        last_scroll_offset: Point<Pixels>,
        last_scroll_time: Option<Instant>,
    ) -> Self {
        let mut state = *self;
        state.last_scroll_offset = last_scroll_offset;
        state.last_scroll_time = last_scroll_time;
        state
    }

    fn with_last_scroll_time(&self, t: Option<Instant>) -> Self {
        let mut state = *self;
        state.last_scroll_time = t;
        state
    }

    fn with_last_update(&self, t: Instant) -> Self {
        let mut state = *self;
        state.last_update = t;
        state
    }

    fn with_idle_timer_scheduled(&self, scheduled: bool) -> Self {
        let mut state = *self;
        state.idle_timer_scheduled = scheduled;
        state
    }

    fn is_scrollbar_visible(&self, visible_duration: Duration) -> bool {
        // On drag
        if self.dragged_axis.is_some() {
            return true;
        }

        if let Some(last_time) = self.last_scroll_time {
            Instant::now().duration_since(last_time) < visible_duration
        } else {
            false
        }
    }
}

/// Scrollbar axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarAxis {
    /// Vertical scrollbar.
    Vertical,
    /// Horizontal scrollbar.
    Horizontal,
    /// Show both vertical and horizontal scrollbars.
    Both,
}

impl From<Axis> for ScrollbarAxis {
    fn from(axis: Axis) -> Self {
        match axis {
            Axis::Vertical => Self::Vertical,
            Axis::Horizontal => Self::Horizontal,
        }
    }
}

impl ScrollbarAxis {
    /// Return true if the scrollbar axis is vertical.
    #[inline]
    pub fn is_vertical(&self) -> bool {
        matches!(self, Self::Vertical)
    }

    /// Return true if the scrollbar axis is horizontal.
    #[inline]
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Self::Horizontal)
    }

    /// Return true if the scrollbar axis is both vertical and horizontal.
    #[inline]
    pub fn is_both(&self) -> bool {
        matches!(self, Self::Both)
    }

    /// Return true if the scrollbar has vertical axis.
    #[inline]
    pub fn has_vertical(&self) -> bool {
        matches!(self, Self::Vertical | Self::Both)
    }

    /// Return true if the scrollbar has horizontal axis.
    #[inline]
    pub fn has_horizontal(&self) -> bool {
        matches!(self, Self::Horizontal | Self::Both)
    }

    #[inline]
    fn all(&self) -> Vec<Axis> {
        match self {
            Self::Vertical => vec![Axis::Vertical],
            Self::Horizontal => vec![Axis::Horizontal],
            // This should keep Horizontal first, Vertical is the primary axis
            // if Vertical not need display, then Horizontal will not keep right margin.
            Self::Both => vec![Axis::Horizontal, Axis::Vertical],
        }
    }
}

/// Scrollbar control for scroll-area or a uniform-list.
pub struct Scrollbar {
    pub(crate) id: ElementId,
    axis: ScrollbarAxis,
    scrollbar_show: Option<ScrollbarShow>,
    scroll_handle: Rc<dyn ScrollbarHandle>,
    scroll_size: Option<Size<Pixels>>,
    /// Maximum frames per second for scrolling by drag. Default is 120 FPS.
    ///
    /// This is used to limit the update rate of the scrollbar when it is
    /// being dragged for some complex interactions for reducing CPU usage.
    max_fps: usize,
}

impl Scrollbar {
    /// Create a new scrollbar.
    ///
    /// This will have both vertical and horizontal scrollbars.
    #[track_caller]
    pub fn new<H: ScrollbarHandle + Clone>(scroll_handle: &H) -> Self {
        let caller = Location::caller();
        Self {
            id: ElementId::CodeLocation(*caller),
            axis: ScrollbarAxis::Both,
            scrollbar_show: None,
            scroll_handle: Rc::new(scroll_handle.clone()),
            max_fps: 120,
            scroll_size: None,
        }
    }

    /// Create with horizontal scrollbar.
    #[track_caller]
    pub fn horizontal<H: ScrollbarHandle + Clone>(scroll_handle: &H) -> Self {
        Self::new(scroll_handle).axis(ScrollbarAxis::Horizontal)
    }

    /// Create with vertical scrollbar.
    #[track_caller]
    pub fn vertical<H: ScrollbarHandle + Clone>(scroll_handle: &H) -> Self {
        Self::new(scroll_handle).axis(ScrollbarAxis::Vertical)
    }

    /// Set a specific element id, default is the [`Location::caller`].
    ///
    /// NOTE: In most cases, you don't need to set a specific id for scrollbar.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the scrollbar show mode [`ScrollbarShow`], if not set use the `cx.theme().scrollbar_show`.
    pub fn scrollbar_show(mut self, scrollbar_show: ScrollbarShow) -> Self {
        self.scrollbar_show = Some(scrollbar_show);
        self
    }

    /// Set a special scroll size of the content area, default is None.
    ///
    /// Default will sync the `content_size` from `scroll_handle`.
    pub fn scroll_size(mut self, scroll_size: Size<Pixels>) -> Self {
        self.scroll_size = Some(scroll_size);
        self
    }

    /// Set scrollbar axis.
    pub fn axis(mut self, axis: impl Into<ScrollbarAxis>) -> Self {
        self.axis = axis.into();
        self
    }

    /// Set maximum frames per second for scrolling by drag. Default is 120 FPS.
    ///
    /// If you have very high CPU usage, consider reducing this value to improve performance.
    ///
    /// Available values: 30..120
    pub(crate) fn max_fps(mut self, max_fps: usize) -> Self {
        self.max_fps = max_fps.clamp(30, 120);
        self
    }

    // Get the width of the scrollbar.
    pub(crate) const fn width() -> Pixels {
        WIDTH
    }

    fn style_for_active(cx: &App, metrics: ScrollbarMetrics) -> ScrollbarPaintStyle {
        ScrollbarPaintStyle {
            thumb: cx.theme().tokens.scrollbar_thumb_hover.into(),
            track: cx.theme().tokens.scrollbar.into(),
            thumb_width: metrics.active_thumb_width,
            thumb_radius: metrics.thumb_radius.min(metrics.active_thumb_width / 2.),
        }
    }

    fn style_for_hovered_thumb(cx: &App, metrics: ScrollbarMetrics) -> ScrollbarPaintStyle {
        Self::style_for_active(cx, metrics)
    }

    fn style_for_hovered_bar(cx: &App, metrics: ScrollbarMetrics) -> ScrollbarPaintStyle {
        ScrollbarPaintStyle {
            thumb: cx.theme().tokens.scrollbar_thumb.into(),
            track: cx.theme().tokens.scrollbar.into(),
            thumb_width: metrics.active_thumb_width,
            thumb_radius: metrics.thumb_radius.min(metrics.active_thumb_width / 2.),
        }
    }

    fn style_for_normal(&self, cx: &App, metrics: ScrollbarMetrics) -> ScrollbarPaintStyle {
        let scrollbar_show = self.scrollbar_show.unwrap_or(cx.theme().scrollbar_show);
        let thumb_width = match scrollbar_show {
            ScrollbarShow::Scrolling => metrics.thumb_width,
            _ => metrics.active_thumb_width,
        };

        ScrollbarPaintStyle {
            thumb: cx.theme().tokens.scrollbar_thumb.into(),
            track: gpui::transparent_black().into(),
            thumb_width,
            thumb_radius: metrics.thumb_radius.min(thumb_width / 2.),
        }
    }

    fn style_for_idle(&self, cx: &App, metrics: ScrollbarMetrics) -> ScrollbarPaintStyle {
        let scrollbar_show = self.scrollbar_show.unwrap_or(cx.theme().scrollbar_show);
        let thumb_width = match scrollbar_show {
            ScrollbarShow::Scrolling => metrics.thumb_width,
            _ => metrics.active_thumb_width,
        };

        ScrollbarPaintStyle {
            thumb: gpui::transparent_black().into(),
            track: gpui::transparent_black().into(),
            thumb_width,
            thumb_radius: metrics.thumb_radius.min(thumb_width / 2.),
        }
    }
}

impl IntoElement for Scrollbar {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[doc(hidden)]
pub struct PrepaintState {
    hitbox: Hitbox,
    scrollbar_state: ScrollbarState,
    states: Vec<AxisPrepaintState>,
}

#[doc(hidden)]
pub struct AxisPrepaintState {
    axis: Axis,
    bar_hitbox: Hitbox,
    bounds: Bounds<Pixels>,
    track_bounds: Bounds<Pixels>,
    track_radius: Pixels,
    track_bg: Background,
    thumb_radius: Pixels,
    thumb_bounds: Bounds<Pixels>,
    // Bounds of thumb to be rendered.
    thumb_fill_bounds: Bounds<Pixels>,
    thumb_bg: Background,
    scroll_size: Pixels,
    container_size: Pixels,
    thumb_size: Pixels,
}

impl Element for Scrollbar {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<gpui::ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.position = Position::Absolute;
        style.flex_grow = 1.0;
        style.flex_shrink = 1.0;
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();

        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let hitbox = window.with_content_mask(Some(ContentMask { bounds }), |window| {
            window.insert_hitbox(bounds, HitboxBehavior::Normal)
        });

        let state = window
            .use_state(cx, |_, _| ScrollbarState::default())
            .read(cx)
            .clone();

        let mut states = vec![];
        let metrics = ScrollbarMetrics::resolve(&cx.theme().style);
        let scroll_size = self
            .scroll_size
            .unwrap_or(self.scroll_handle.content_size());
        let horizontal_overflows =
            self.axis.has_horizontal() && scroll_size.width > hitbox.size.width;
        let vertical_overflows =
            self.axis.has_vertical() && scroll_size.height > hitbox.size.height;

        for axis in self.axis.all().into_iter() {
            let is_vertical = axis.is_vertical();
            let (scroll_area_size, container_size, scroll_position) = if is_vertical {
                (
                    scroll_size.height,
                    hitbox.size.height,
                    self.scroll_handle.offset().y,
                )
            } else {
                (
                    scroll_size.width,
                    hitbox.size.width,
                    self.scroll_handle.offset().x,
                )
            };

            let axis_overflows = if is_vertical {
                vertical_overflows
            } else {
                horizontal_overflows
            };
            if !axis_overflows {
                continue;
            }

            // Keep the two tracks out of the shared corner only when both axes overflow.
            let margin_end = if horizontal_overflows && vertical_overflows {
                metrics.hit_width
            } else {
                px(0.)
            };
            let Some(geometry) = scrollbar_axis_geometry(
                container_size,
                scroll_area_size,
                scroll_position,
                margin_end,
                metrics.min_thumb_size,
            ) else {
                continue;
            };
            let available_length = geometry.available_length;
            let thumb_length = geometry.thumb_length;
            let thumb_start = geometry.thumb_start;

            let bounds = Bounds {
                origin: if is_vertical {
                    point(
                        hitbox.origin.x + hitbox.size.width - metrics.hit_width,
                        hitbox.origin.y,
                    )
                } else {
                    point(
                        hitbox.origin.x,
                        hitbox.origin.y + hitbox.size.height - metrics.hit_width,
                    )
                },
                size: gpui::Size {
                    width: if is_vertical {
                        metrics.hit_width
                    } else {
                        available_length
                    },
                    height: if is_vertical {
                        available_length
                    } else {
                        metrics.hit_width
                    },
                },
            };

            let track_cross_inset = metrics.cross_inset(metrics.track_width);
            let track_bounds = if is_vertical {
                Bounds::from_anchor_and_size(
                    Anchor::TopRight,
                    bounds.top_right() + point(-track_cross_inset, px(0.)),
                    size(metrics.track_width, bounds.size.height),
                )
            } else {
                Bounds::from_anchor_and_size(
                    Anchor::BottomLeft,
                    bounds.bottom_left() + point(px(0.), -track_cross_inset),
                    size(bounds.size.width, metrics.track_width),
                )
            };

            let scrollbar_show = self.scrollbar_show.unwrap_or(cx.theme().scrollbar_show);
            let is_always_to_show = scrollbar_show.is_always();
            let is_hover_to_show = scrollbar_show.is_hover();
            let is_hovered_on_bar = state.get().hovered_axis == Some(axis);
            let is_hovered_on_thumb = state.get().hovered_on_thumb == Some(axis);
            let is_offset_changed = state.get().last_scroll_offset != self.scroll_handle.offset();

            let paint_style = if state.get().dragged_axis == Some(axis) {
                Self::style_for_active(cx, metrics)
            } else if is_hover_to_show && (is_hovered_on_bar || is_hovered_on_thumb) {
                if is_hovered_on_thumb {
                    Self::style_for_hovered_thumb(cx, metrics)
                } else {
                    Self::style_for_hovered_bar(cx, metrics)
                }
            } else if is_offset_changed {
                self.style_for_normal(cx, metrics)
            } else if is_always_to_show {
                if is_hovered_on_thumb {
                    Self::style_for_hovered_thumb(cx, metrics)
                } else {
                    Self::style_for_hovered_bar(cx, metrics)
                }
            } else {
                let mut idle_state = self.style_for_idle(cx, metrics);
                if let Some(last_time) = state.get().last_scroll_time {
                    let elapsed = Instant::now().duration_since(last_time);
                    if is_hovered_on_bar {
                        state.set(state.get().with_last_scroll_time(Some(Instant::now())));
                        idle_state = if is_hovered_on_thumb {
                            Self::style_for_hovered_thumb(cx, metrics)
                        } else {
                            Self::style_for_hovered_bar(cx, metrics)
                        };
                    } else if elapsed < metrics.fade_delay {
                        idle_state.thumb = cx.theme().tokens.scrollbar_thumb.into();

                        if !state.get().idle_timer_scheduled {
                            let state = state.clone();
                            state.set(state.get().with_idle_timer_scheduled(true));
                            let current_view = window.current_view();
                            let next_delay = metrics.fade_delay - elapsed;
                            window
                                .spawn(cx, async move |cx| {
                                    cx.background_executor().timer(next_delay).await;
                                    state.set(state.get().with_idle_timer_scheduled(false));
                                    cx.update(|_, cx| cx.notify(current_view)).ok();
                                })
                                .detach();
                        }
                    } else if elapsed < metrics.visible_duration() {
                        let progress = (elapsed - metrics.fade_delay).as_secs_f32()
                            / metrics.fade_duration.as_secs_f32();
                        let opacity = 1. - metrics.fade_easing.sample(progress);
                        let thumb: Background = cx.theme().tokens.scrollbar_thumb.into();
                        idle_state.thumb = thumb.opacity(opacity);

                        window.request_animation_frame();
                    }
                }

                idle_state
            };

            let thumb_cross_inset = metrics.cross_inset(paint_style.thumb_width);
            let thumb_bounds = if is_vertical {
                Bounds::from_anchor_and_size(
                    Anchor::TopRight,
                    bounds.top_right() + point(px(0.), thumb_start),
                    size(metrics.hit_width, thumb_length),
                )
            } else {
                Bounds::from_anchor_and_size(
                    Anchor::BottomLeft,
                    bounds.bottom_left() + point(thumb_start, px(0.)),
                    size(thumb_length, metrics.hit_width),
                )
            };

            let end_inset = metrics.thumb_end_inset.min(thumb_length / 2.);
            let thumb_fill_length = (thumb_length - end_inset * 2.).max(px(0.));
            let thumb_fill_bounds = if is_vertical {
                Bounds::from_anchor_and_size(
                    Anchor::TopRight,
                    bounds.top_right() + point(-thumb_cross_inset, thumb_start + end_inset),
                    size(paint_style.thumb_width, thumb_fill_length),
                )
            } else {
                Bounds::from_anchor_and_size(
                    Anchor::BottomLeft,
                    bounds.bottom_left() + point(thumb_start + end_inset, -thumb_cross_inset),
                    size(thumb_fill_length, paint_style.thumb_width),
                )
            };

            let bar_hitbox = window.with_content_mask(Some(ContentMask { bounds }), |window| {
                window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal)
            });

            states.push(AxisPrepaintState {
                axis,
                bar_hitbox,
                bounds,
                track_bounds,
                track_radius: metrics.track_radius.min(metrics.track_width / 2.),
                track_bg: paint_style.track,
                thumb_radius: paint_style.thumb_radius,
                thumb_bounds,
                thumb_fill_bounds,
                thumb_bg: paint_style.thumb,
                scroll_size: scroll_area_size,
                container_size,
                thumb_size: thumb_length,
            })
        }

        PrepaintState {
            hitbox,
            states,
            scrollbar_state: state,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let scrollbar_state = &prepaint.scrollbar_state;
        let scrollbar_show = self.scrollbar_show.unwrap_or(cx.theme().scrollbar_show);
        let metrics = ScrollbarMetrics::resolve(&cx.theme().style);
        let view_id = window.current_view();
        let hitbox_bounds = prepaint.hitbox.bounds;
        let is_visible = scrollbar_state
            .get()
            .is_scrollbar_visible(metrics.visible_duration())
            || scrollbar_show.is_always();
        let is_hover_to_show = scrollbar_show.is_hover();

        // Update last_scroll_time when offset is changed.
        if self.scroll_handle.offset() != scrollbar_state.get().last_scroll_offset {
            scrollbar_state.set(
                scrollbar_state
                    .get()
                    .with_last_scroll(self.scroll_handle.offset(), Some(Instant::now())),
            );
            cx.notify(view_id);
        }

        window.with_content_mask(
            Some(ContentMask {
                bounds: hitbox_bounds,
            }),
            |window| {
                for state in prepaint.states.iter() {
                    let axis = state.axis;
                    let bounds = state.bounds;
                    let thumb_bounds = state.thumb_bounds;
                    let scroll_area_size = state.scroll_size;
                    let container_size = state.container_size;
                    let thumb_size = state.thumb_size;
                    let is_vertical = axis.is_vertical();

                    window.set_cursor_style(CursorStyle::default(), &state.bar_hitbox);

                    window.paint_layer(hitbox_bounds, |cx| {
                        cx.paint_quad(
                            fill(state.track_bounds, state.track_bg)
                                .corner_radii(state.track_radius),
                        );
                        cx.paint_quad(
                            fill(state.thumb_fill_bounds, state.thumb_bg)
                                .corner_radii(state.thumb_radius),
                        );
                    });

                    window.on_mouse_event({
                        let state = scrollbar_state.clone();
                        let scroll_handle = self.scroll_handle.clone();

                        move |event: &ScrollWheelEvent, phase, _, cx| {
                            if phase.bubble() && hitbox_bounds.contains(&event.position) {
                                if scroll_handle.offset() != state.get().last_scroll_offset {
                                    state.set(state.get().with_last_scroll(
                                        scroll_handle.offset(),
                                        Some(Instant::now()),
                                    ));
                                    cx.notify(view_id);
                                }
                            }
                        }
                    });

                    let safe_range = (-scroll_area_size + container_size)..px(0.);

                    if is_hover_to_show || is_visible {
                        window.on_mouse_event({
                            let state = scrollbar_state.clone();
                            let scroll_handle = self.scroll_handle.clone();

                            move |event: &MouseDownEvent, phase, _, cx| {
                                if phase.bubble() && bounds.contains(&event.position) {
                                    cx.stop_propagation();

                                    if thumb_bounds.contains(&event.position) {
                                        // click on the thumb bar, set the drag position
                                        let pos = event.position - thumb_bounds.origin;

                                        scroll_handle.start_drag();
                                        state.set(state.get().with_drag_pos(axis, pos));

                                        cx.notify(view_id);
                                    } else {
                                        // click on the scrollbar, jump to the position
                                        // Set the thumb bar center to the click position
                                        let offset = scroll_handle.offset();
                                        let thumb_travel = if is_vertical {
                                            bounds.size.height - thumb_size
                                        } else {
                                            bounds.size.width - thumb_size
                                        };
                                        if thumb_travel <= px(0.) {
                                            return;
                                        }
                                        let percentage = (if is_vertical {
                                            event.position.y - thumb_size / 2. - bounds.origin.y
                                        } else {
                                            event.position.x - thumb_size / 2. - bounds.origin.x
                                        } / thumb_travel)
                                            .clamp(0., 1.);

                                        if is_vertical {
                                            scroll_handle.set_offset(point(
                                                offset.x,
                                                (-(scroll_area_size - container_size) * percentage)
                                                    .clamp(safe_range.start, safe_range.end),
                                            ));
                                        } else {
                                            scroll_handle.set_offset(point(
                                                (-(scroll_area_size - container_size) * percentage)
                                                    .clamp(safe_range.start, safe_range.end),
                                                offset.y,
                                            ));
                                        }
                                    }
                                }
                            }
                        });
                    }

                    window.on_mouse_event({
                        let scroll_handle = self.scroll_handle.clone();
                        let state = scrollbar_state.clone();
                        let max_fps_duration = Duration::from_millis((1000 / self.max_fps) as u64);

                        move |event: &MouseMoveEvent, _, _, cx| {
                            let mut notify = false;
                            // When is hover to show mode or it was visible,
                            // we need to update the hovered state and increase the last_scroll_time.
                            let need_hover_to_update = is_hover_to_show || is_visible;
                            // Update hovered state for scrollbar
                            if bounds.contains(&event.position) && need_hover_to_update {
                                if state.get().hovered_axis != Some(axis) {
                                    state.set(state.get().with_hovered(Some(axis)));
                                    notify = true;
                                }
                            } else if state.get().hovered_axis == Some(axis) {
                                state.set(state.get().with_hovered(None));
                                notify = true;
                            }

                            // Update hovered state for scrollbar thumb
                            if thumb_bounds.contains(&event.position) {
                                if state.get().hovered_on_thumb != Some(axis) {
                                    state.set(state.get().with_hovered_on_thumb(
                                        Some(axis),
                                        metrics.visible_duration(),
                                    ));
                                    notify = true;
                                }
                            } else {
                                if state.get().hovered_on_thumb == Some(axis) {
                                    state.set(
                                        state.get().with_hovered_on_thumb(
                                            None,
                                            metrics.visible_duration(),
                                        ),
                                    );
                                    notify = true;
                                }
                            }

                            // Move thumb position on dragging
                            if state.get().dragged_axis == Some(axis) && event.dragging() {
                                // Stop the event propagation to avoid selecting text or other side effects.
                                cx.stop_propagation();

                                // drag_pos is the position of the mouse down event
                                // We need to keep the thumb bar still at the origin down position
                                let drag_pos = state.get().drag_pos;
                                let thumb_travel = if is_vertical {
                                    bounds.size.height - thumb_size
                                } else {
                                    bounds.size.width - thumb_size
                                };
                                if thumb_travel <= px(0.) {
                                    return;
                                }

                                let percentage = (if is_vertical {
                                    (event.position.y - drag_pos.y - bounds.origin.y) / thumb_travel
                                } else {
                                    (event.position.x - drag_pos.x - bounds.origin.x) / thumb_travel
                                })
                                .clamp(0., 1.);

                                let offset = if is_vertical {
                                    point(
                                        scroll_handle.offset().x,
                                        (-(scroll_area_size - container_size) * percentage)
                                            .clamp(safe_range.start, safe_range.end),
                                    )
                                } else {
                                    point(
                                        (-(scroll_area_size - container_size) * percentage)
                                            .clamp(safe_range.start, safe_range.end),
                                        scroll_handle.offset().y,
                                    )
                                };

                                if (scroll_handle.offset().y - offset.y).abs() > px(1.)
                                    || (scroll_handle.offset().x - offset.x).abs() > px(1.)
                                {
                                    // Limit update rate
                                    if state.get().last_update.elapsed() > max_fps_duration {
                                        scroll_handle.set_offset(offset);
                                        state.set(state.get().with_last_update(Instant::now()));
                                        notify = true;
                                    }
                                }
                            }

                            if notify {
                                cx.notify(view_id);
                            }
                        }
                    });

                    window.on_mouse_event({
                        let state = scrollbar_state.clone();
                        let scroll_handle = self.scroll_handle.clone();

                        move |_event: &MouseUpEvent, phase, _, cx| {
                            if phase.bubble() && state.get().dragged_axis == Some(axis) {
                                scroll_handle.end_drag();
                                state.set(state.get().with_unset_drag_pos());
                                cx.notify(view_id);
                            }
                        }
                    });
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_follow_semantic_style_presets() {
        for style in [
            StylePreset::vega(),
            StylePreset::nova(),
            StylePreset::maia(),
        ] {
            let metrics = ScrollbarMetrics::resolve(&style);

            assert_eq!(metrics.thumb_radius, style.radii.sm);
            assert_eq!(metrics.track_radius, style.radii.sm);
            assert_eq!(metrics.fade_duration, style.motion.slow());
            assert_eq!(metrics.fade_easing, style.motion.exit_easing);
            assert!(metrics.hit_width >= metrics.track_width);
        }
    }

    #[test]
    fn geometry_rejects_non_overflowing_content() {
        assert_eq!(
            scrollbar_axis_geometry(px(100.), px(100.), px(0.), px(0.), MIN_THUMB_SIZE),
            None
        );
    }

    #[test]
    fn geometry_stays_inside_tiny_containers() {
        let geometry =
            scrollbar_axis_geometry(px(12.), px(120.), px(-108.), px(8.), MIN_THUMB_SIZE)
                .expect("overflowing content should produce geometry");

        assert_eq!(geometry.available_length, px(4.));
        assert_eq!(geometry.thumb_length, px(4.));
        assert_eq!(geometry.thumb_start, px(0.));
    }

    #[test]
    fn geometry_clamps_out_of_range_offsets() {
        let start =
            scrollbar_axis_geometry(px(100.), px(400.), px(50.), px(0.), MIN_THUMB_SIZE).unwrap();
        let end =
            scrollbar_axis_geometry(px(100.), px(400.), px(-500.), px(0.), MIN_THUMB_SIZE).unwrap();

        assert_eq!(start.thumb_start, px(0.));
        assert_eq!(end.thumb_start, end.available_length - end.thumb_length);
    }

    #[test]
    fn hidden_thumb_hover_does_not_refresh_scroll_activity() {
        let visible_duration = Duration::from_secs(3);
        let state = ScrollbarState::default().get();
        let hovered = state.with_hovered_on_thumb(Some(Axis::Vertical), visible_duration);

        assert_eq!(hovered.hovered_on_thumb, Some(Axis::Vertical));
        assert_eq!(hovered.last_scroll_time, None);
        assert!(!hovered.is_scrollbar_visible(visible_duration));
    }

    #[test]
    fn visible_thumb_hover_keeps_scrollbar_active() {
        let visible_duration = Duration::from_secs(3);
        let active = ScrollbarState::default()
            .get()
            .with_last_scroll_time(Some(Instant::now()));
        let hovered = active.with_hovered_on_thumb(Some(Axis::Vertical), visible_duration);

        assert_eq!(hovered.hovered_on_thumb, Some(Axis::Vertical));
        assert!(hovered.last_scroll_time.is_some());
        assert!(hovered.is_scrollbar_visible(visible_duration));
    }
}
