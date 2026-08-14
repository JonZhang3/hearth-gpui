// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public types: `TooltipSide`, `TooltipAlign`, `TooltipTrigger`.
// - Added public methods: `new`, `trigger`, `text`, `content`, `side`, `align`, `side_offset`,
//   `align_offset` and 4 more.
// - Added or exposed behavior through `motion_offset`, `opposite`, `resolve`, `begin_open`,
//   `request_hide_after`, `tooltip_arrow`, `show`, `hide_if_inactive` and 24 more.
// - Removed or replaced `managed_tooltip`.
// - Reworked Tooltip around accessibility semantics and ARIA state, interruptible and
//   reduced-motion-aware transitions, semantic Style Preset geometry and density, keyboard
//   navigation and activation behavior, focus-visible and focus restoration behavior.
use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::{
    Action, AnyElement, AnyView, App, AppContext, Bounds, Context, Display, Element, ElementId,
    FocusHandle, GlobalElementId, Half, InspectorElementId, InteractiveElement as _, IntoElement,
    KeyDownEvent, Keystroke, LayoutId, MouseButton, ParentElement, PathBuilder, Pixels, Point,
    Position, Render, RenderOnce, SharedString, Size, StatefulInteractiveElement, Style,
    StyleRefinement, Styled, Task, Window, canvas, deferred, div, point, prelude::FluentBuilder,
    px,
};

use crate::{
    ActiveTheme, ElementExt, StyledExt,
    animation::{OverlayLifecycle, OverlayPhase, Transition, effective_motion_duration},
    h_flex,
    kbd::Kbd,
    root::Root,
    text::Text,
    theme::Density,
};

pub(crate) fn init(_cx: &mut App) {
    // No app-level init needed — TooltipOverlay is per-window via Root.
}

const DEFAULT_SHOW_DELAY: Duration = Duration::from_millis(500);
const DEFAULT_HIDE_DELAY: Duration = Duration::ZERO;
const GRACE_PERIOD: Duration = Duration::from_millis(300);
const TOOLTIP_WINDOW_MARGIN: Pixels = px(4.);
const TOOLTIP_MOTION_DISTANCE: Pixels = px(8.);
const TOOLTIP_ARROW_SIZE: Pixels = px(10.);
/// Visible Arrow depth outside the Surface after the shadcn attachment transform.
const TOOLTIP_ARROW_PROTRUSION: Pixels = px(4.);

/// Physical side on which Tooltip content is displayed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipSide {
    #[default]
    Top,
    Right,
    Bottom,
    Left,
}

impl TooltipSide {
    /// Returns the enter translation from the trigger toward the final surface.
    fn motion_offset(self, distance: Pixels) -> Point<Pixels> {
        match self {
            Self::Top => point(px(0.), distance),
            Self::Right => point(-distance, px(0.)),
            Self::Bottom => point(px(0.), -distance),
            Self::Left => point(distance, px(0.)),
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Right => Self::Left,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
        }
    }
}

/// Cross-axis alignment of Tooltip content relative to its trigger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipAlign {
    Start,
    #[default]
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TooltipOptions {
    side: TooltipSide,
    align: TooltipAlign,
    side_offset: Pixels,
    align_offset: Pixels,
    show_delay: Duration,
    hide_delay: Duration,
    show_arrow: bool,
    arrow_color: Option<gpui::Hsla>,
}

impl Default for TooltipOptions {
    fn default() -> Self {
        Self {
            side: TooltipSide::Top,
            align: TooltipAlign::Center,
            side_offset: px(4.),
            align_offset: px(0.),
            show_delay: DEFAULT_SHOW_DELAY,
            hide_delay: DEFAULT_HIDE_DELAY,
            show_arrow: true,
            arrow_color: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TooltipMetrics {
    radius: Pixels,
}

impl TooltipMetrics {
    /// Resolves component-local geometry from semantic density and radius metrics.
    fn resolve(cx: &App) -> Self {
        let style = &cx.theme().style;
        Self {
            radius: match style.density {
                Density::Compact | Density::Standard => style.radii.md,
                Density::Comfortable => style.radii.lg,
            },
        }
    }
}

// ── Tooltip surface ─────────────────────────────────────────────────────────

enum TooltipContext {
    Text(Text),
    Element(Box<dyn Fn(&mut Window, &mut App) -> AnyElement>),
}

/// A Tooltip element that can display text or custom content,
/// with optional key binding information.
pub struct Tooltip {
    style: StyleRefinement,
    content: TooltipContext,
    key_binding: Option<Keystroke>,
    action: Option<(Box<dyn Action>, Option<SharedString>)>,
}

impl Tooltip {
    /// Create a Tooltip with a text content.
    pub fn new(text: impl Into<Text>) -> Self {
        Self {
            style: StyleRefinement::default(),
            content: TooltipContext::Text(text.into()),
            key_binding: None,
            action: None,
        }
    }

    /// Create a Tooltip with a custom element.
    pub fn element<E, F>(builder: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        Self {
            style: StyleRefinement::default(),
            key_binding: None,
            action: None,
            content: TooltipContext::Element(Box::new(move |window, cx| {
                builder(window, cx).into_any_element()
            })),
        }
    }

    /// Set Action to display key binding information for the tooltip if it exists.
    pub fn action(mut self, action: &dyn Action, context: Option<&str>) -> Self {
        self.action = Some((action.boxed_clone(), context.map(SharedString::new)));
        self
    }

    /// Sets an explicit platform-aware key binding for the tooltip.
    pub fn key_binding(mut self, key_binding: Option<Keystroke>) -> Self {
        self.key_binding = key_binding;
        self
    }

    /// Build the tooltip and return it as an `AnyView`.
    pub fn build(self, _: &mut Window, cx: &mut App) -> AnyView {
        cx.new(|_| self).into()
    }
}

impl FluentBuilder for Tooltip {}
impl Styled for Tooltip {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let metrics = TooltipMetrics::resolve(cx);
        let key_binding = if let Some(key_binding) = &self.key_binding {
            Some(Kbd::from_keystroke(key_binding.clone()))
        } else {
            if let Some((action, context)) = &self.action {
                Kbd::binding_for_action(
                    action.as_ref(),
                    context.as_ref().map(|s| s.as_ref()),
                    window,
                )
            } else {
                None
            }
        };
        let has_key_binding = key_binding.is_some();
        let kbd_background_opacity = if cx.theme().mode.is_dark() { 0.1 } else { 0.2 };

        div().child(
            h_flex()
                .id("tooltip-content")
                .debug_selector(|| "tooltip-content".into())
                .role(gpui::accesskit::Role::Tooltip)
                .font_family(cx.theme().font_family.clone())
                .max_w(px(320.))
                .bg(cx.theme().foreground)
                .text_color(cx.theme().background)
                .rounded(metrics.radius)
                .pl_3()
                .pr(if has_key_binding { px(6.) } else { px(12.) })
                .py_1p5()
                .text_xs()
                .gap_1p5()
                .refine_style(&self.style)
                .map(|this| {
                    this.child(
                        div()
                            .min_w_0()
                            .flex_shrink_1()
                            .debug_selector(|| "tooltip-text".into())
                            .map(|this| match self.content {
                                TooltipContext::Text(ref text) => this.child(text.clone()),
                                TooltipContext::Element(ref builder) => {
                                    this.child(builder(window, cx))
                                }
                            }),
                    )
                })
                .when_some(key_binding, |this, kbd| {
                    this.child(
                        div()
                            .flex_shrink_0()
                            .debug_selector(|| "tooltip-kbd".into())
                            .child(
                                kbd.appearance(false)
                                    .h_5()
                                    .min_w_5()
                                    .justify_center()
                                    .px_1()
                                    .rounded(cx.theme().style.radii.sm)
                                    .bg(cx.theme().background.opacity(kbd_background_opacity))
                                    .text_color(cx.theme().background)
                                    .font_medium(),
                            ),
                    )
                }),
        )
    }
}

// ── Managed tooltip system ──────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
struct TooltipOverlayPosition {
    bounds: Bounds<Pixels>,
    side: TooltipSide,
    arrow_offset: Pixels,
}

fn tooltip_overlay_position(
    trigger_bounds: Bounds<Pixels>,
    tooltip_size: Size<Pixels>,
    viewport_size: Size<Pixels>,
    margin: Pixels,
    options: TooltipOptions,
) -> TooltipOverlayPosition {
    let arrow_extent = if options.show_arrow {
        TOOLTIP_ARROW_PROTRUSION
    } else {
        px(0.)
    };
    let side_gap = options.side_offset + arrow_extent;

    let available = |side| match side {
        TooltipSide::Top => trigger_bounds.top() - margin,
        TooltipSide::Right => viewport_size.width - margin - trigger_bounds.right(),
        TooltipSide::Bottom => viewport_size.height - margin - trigger_bounds.bottom(),
        TooltipSide::Left => trigger_bounds.left() - margin,
    };
    let required = |side| match side {
        TooltipSide::Top | TooltipSide::Bottom => tooltip_size.height + side_gap,
        TooltipSide::Right | TooltipSide::Left => tooltip_size.width + side_gap,
    };

    let opposite = options.side.opposite();
    let side = if available(options.side) >= required(options.side)
        || available(options.side) >= available(opposite)
    {
        options.side
    } else {
        opposite
    };

    let cross_axis_origin =
        |trigger_start: Pixels, trigger_size: Pixels, tooltip_extent: Pixels| -> Pixels {
            match options.align {
                TooltipAlign::Start => trigger_start + options.align_offset,
                TooltipAlign::Center => {
                    trigger_start + trigger_size.half() - tooltip_extent.half()
                        + options.align_offset
                }
                TooltipAlign::End => {
                    trigger_start + trigger_size - tooltip_extent + options.align_offset
                }
            }
        };

    let origin = match side {
        TooltipSide::Top => point(
            cross_axis_origin(
                trigger_bounds.left(),
                trigger_bounds.size.width,
                tooltip_size.width,
            ),
            trigger_bounds.top() - tooltip_size.height - side_gap,
        ),
        TooltipSide::Right => point(
            trigger_bounds.right() + side_gap,
            cross_axis_origin(
                trigger_bounds.top(),
                trigger_bounds.size.height,
                tooltip_size.height,
            ),
        ),
        TooltipSide::Bottom => point(
            cross_axis_origin(
                trigger_bounds.left(),
                trigger_bounds.size.width,
                tooltip_size.width,
            ),
            trigger_bounds.bottom() + side_gap,
        ),
        TooltipSide::Left => point(
            trigger_bounds.left() - tooltip_size.width - side_gap,
            cross_axis_origin(
                trigger_bounds.top(),
                trigger_bounds.size.height,
                tooltip_size.height,
            ),
        ),
    };
    let bounds = clamp_tooltip_bounds(Bounds::new(origin, tooltip_size), viewport_size, margin);
    let arrow_offset = match side {
        TooltipSide::Top | TooltipSide::Bottom => (trigger_bounds.center().x - bounds.left())
            .max(TOOLTIP_ARROW_SIZE)
            .min((tooltip_size.width - TOOLTIP_ARROW_SIZE).max(TOOLTIP_ARROW_SIZE)),
        TooltipSide::Right | TooltipSide::Left => (trigger_bounds.center().y - bounds.top())
            .max(TOOLTIP_ARROW_SIZE)
            .min((tooltip_size.height - TOOLTIP_ARROW_SIZE).max(TOOLTIP_ARROW_SIZE)),
    };

    TooltipOverlayPosition {
        bounds,
        side,
        arrow_offset,
    }
}

fn clamp_tooltip_bounds(
    mut bounds: Bounds<Pixels>,
    viewport_size: Size<Pixels>,
    margin: Pixels,
) -> Bounds<Pixels> {
    let right_limit = (viewport_size.width - margin).max(margin);
    let bottom_limit = (viewport_size.height - margin).max(margin);

    if bounds.right() > right_limit {
        bounds.origin.x -= bounds.right() - right_limit;
    }
    if bounds.left() < margin {
        bounds.origin.x = margin;
    }

    if bounds.bottom() > bottom_limit {
        bounds.origin.y -= bounds.bottom() - bottom_limit;
    }
    if bounds.top() < margin {
        bounds.origin.y = margin;
    }

    bounds
}

struct TooltipOverlayPositioner {
    trigger_bounds: Bounds<Pixels>,
    options: TooltipOptions,
    placement: Rc<Cell<Option<TooltipOverlayPosition>>>,
    children: Vec<AnyElement>,
}

struct TooltipOverlayPositionerState {
    child_layout_ids: Vec<LayoutId>,
}

fn tooltip_overlay_positioner(
    trigger_bounds: Bounds<Pixels>,
    options: TooltipOptions,
    placement: Rc<Cell<Option<TooltipOverlayPosition>>>,
) -> TooltipOverlayPositioner {
    TooltipOverlayPositioner {
        trigger_bounds,
        options,
        placement,
        children: Vec::new(),
    }
}

impl ParentElement for TooltipOverlayPositioner {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Element for TooltipOverlayPositioner {
    type RequestLayoutState = TooltipOverlayPositionerState;
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
        let child_layout_ids = self
            .children
            .iter_mut()
            .map(|child| child.request_layout(window, cx))
            .collect::<Vec<_>>();

        let layout_id = window.request_layout(
            Style {
                position: Position::Absolute,
                display: Display::Flex,
                ..Style::default()
            },
            child_layout_ids.iter().copied(),
            cx,
        );

        (
            layout_id,
            TooltipOverlayPositionerState { child_layout_ids },
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if request_layout.child_layout_ids.is_empty() {
            return;
        }

        let mut child_min: Point<Pixels> = point(Pixels::MAX, Pixels::MAX);
        let mut child_max = Point::default();
        for child_layout_id in &request_layout.child_layout_ids {
            let child_bounds = window.layout_bounds(*child_layout_id);
            child_min = child_min.min(&child_bounds.origin);
            child_max = child_max.max(&child_bounds.bottom_right());
        }

        let tooltip_size: Size<Pixels> = (child_max - child_min).into();
        let client_inset = window.client_inset().unwrap_or(px(0.));
        let tooltip_position = tooltip_overlay_position(
            self.trigger_bounds,
            tooltip_size,
            window.viewport_size(),
            TOOLTIP_WINDOW_MARGIN + client_inset,
            self.options,
        );
        if self.placement.get() != Some(tooltip_position) {
            self.placement.set(Some(tooltip_position));
            window.request_animation_frame();
        }

        let offset = tooltip_position.bounds.origin - bounds.origin;
        let offset = point(offset.x.round(), offset.y.round());

        window.with_element_offset(offset, |window| {
            for child in &mut self.children {
                child.prepaint(window, cx);
            }
        });
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        for child in &mut self.children {
            child.paint(window, cx);
        }
    }
}

impl IntoElement for TooltipOverlayPositioner {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Content for a managed tooltip.
#[derive(Clone)]
pub(crate) struct TooltipContent {
    pub build: Rc<dyn Fn(&mut Window, &mut App) -> AnyView>,
    pub trigger_bounds: Bounds<Pixels>,
    options: TooltipOptions,
}

/// Manages tooltip lifecycle: delay, grace period, animations, and rendering.
///
/// A single instance lives in [`Root`] per window. [`TooltipTrigger`] instances
/// submit measured content and share its grace-period lifecycle.
pub struct TooltipOverlay {
    content: Option<TooltipContent>,
    epoch: usize,
    had_recent_tooltip: bool,
    animation_epoch: usize,
    lifecycle: OverlayLifecycle,
    /// Placement resolved during prepaint before the visible enter transition starts.
    placement: Rc<Cell<Option<TooltipOverlayPosition>>>,

    _show_task: Option<Task<()>>,
    _hide_task: Option<Task<()>>,
}

impl TooltipOverlay {
    pub fn new() -> Self {
        Self {
            content: None,
            epoch: 0,
            had_recent_tooltip: false,
            animation_epoch: 0,
            lifecycle: OverlayLifecycle::default(),
            placement: Rc::new(Cell::new(None)),
            _show_task: None,
            _hide_task: None,
        }
    }

    fn next_epoch(&mut self) -> usize {
        self.epoch += 1;
        self.epoch
    }

    /// Starts an interruptible enter transition for the current content.
    fn begin_open(&mut self, cx: &mut Context<Self>) {
        let Some(transition) = self.lifecycle.begin_open() else {
            return;
        };
        let duration = effective_motion_duration(cx.theme().style.motion.fast(), cx);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(duration).await;
            let _ = this.update(cx, |this, cx| {
                if this.lifecycle.complete_open(transition) {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Requests a tooltip and applies the configured pointer or focus delay.
    pub(crate) fn request_show(
        &mut self,
        content: TooltipContent,
        delay: Duration,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._hide_task = None;

        let was_visible = self.content.is_some();
        let in_grace = self.had_recent_tooltip;

        if was_visible || in_grace || delay.is_zero() {
            self.content = Some(content);
            self._show_task = None;
            self.animation_epoch += 1;
            self.lifecycle = OverlayLifecycle::default();
            self.placement.set(None);
            cx.notify();
        } else {
            let epoch = self.next_epoch();
            let content = content.clone();
            self._show_task = Some(cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(delay).await;
                let _ = this.update_in(cx, |this, _, cx| {
                    if this.epoch != epoch {
                        return;
                    }

                    this.content = Some(content);
                    this.animation_epoch += 1;
                    this.lifecycle = OverlayLifecycle::default();
                    this.placement.set(None);
                    cx.notify();
                });
            }));
        }
    }

    /// Schedules closing while preserving the mounted exit lifecycle.
    pub(crate) fn request_hide_after(
        &mut self,
        delay: Duration,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._show_task = None;
        if delay.is_zero() {
            self.request_hide(window, cx);
            return;
        }

        let epoch = self.next_epoch();
        self._hide_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.epoch == epoch {
                    this.request_hide(window, cx);
                }
            });
        }));
    }

    /// Request hiding the current tooltip. Starts a brief grace period so that
    /// moving to another tooltip-bearing element feels instant.
    pub(crate) fn request_hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Cancel any pending show
        self._show_task = None;

        if self.content.is_none() {
            return;
        }
        if self.lifecycle.phase() == OverlayPhase::Closed {
            // A tooltip dismissed before its prepaint placement never became visible,
            // so it must unmount immediately instead of waiting for an exit transition.
            self.content = None;
            self.placement.set(None);
            self.next_epoch();
            cx.notify();
            return;
        }
        let Some(transition) = self.lifecycle.begin_close() else {
            return;
        };

        let epoch = self.next_epoch();
        self.had_recent_tooltip = true;
        self.animation_epoch += 1;
        cx.notify();
        let exit_duration = effective_motion_duration(cx.theme().style.motion.fast(), cx);

        self._hide_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(exit_duration).await;
            let closed = this
                .update_in(cx, |this, _, cx| {
                    if this.epoch != epoch || !this.lifecycle.complete_close(transition) {
                        return false;
                    }
                    this.content = None;
                    this.placement.set(None);
                    cx.notify();
                    true
                })
                .unwrap_or(false);
            if !closed {
                return;
            }

            cx.background_executor().timer(GRACE_PERIOD).await;
            let _ = this.update_in(cx, |this, _, cx| {
                if this.epoch == epoch {
                    this.had_recent_tooltip = false;
                    cx.notify();
                }
            });
        }));
    }

    pub(crate) fn hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.request_hide(window, cx);
    }

    #[cfg(test)]
    fn clear_state(&mut self) -> bool {
        let changed = self.content.is_some()
            || self.had_recent_tooltip
            || self._show_task.is_some()
            || self._hide_task.is_some();

        self.content = None;
        self.had_recent_tooltip = false;
        self._show_task = None;
        self._hide_task = None;
        self.lifecycle = OverlayLifecycle::default();
        self.placement.set(None);

        changed
    }
}

impl Render for TooltipOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(content) = self.content.as_ref().cloned() else {
            return div().into_any_element();
        };

        let content_view = (content.build)(window, cx);
        let trigger_bounds = content.trigger_bounds;
        let options = content.options;
        let Some(placement) = self.placement.get() else {
            // Measure and resolve flipping before mounting the visible enter animation.
            return deferred(
                tooltip_overlay_positioner(trigger_bounds, options, self.placement.clone())
                    .child(div().opacity(0.).child(content_view)),
            )
            .with_priority(2)
            .into_any_element();
        };

        if self.lifecycle.phase() == OverlayPhase::Closed {
            self.begin_open(cx);
        }

        let animation_epoch = self.animation_epoch;
        let closing = self.lifecycle.phase() == OverlayPhase::Closing;
        let motion_offset = placement.side.motion_offset(TOOLTIP_MOTION_DISTANCE);
        let motion_id = animation_epoch as u64 * 4
            + match placement.side {
                TooltipSide::Top => 0,
                TooltipSide::Right => 1,
                TooltipSide::Bottom => 2,
                TooltipSide::Left => 3,
            };
        let arrow_color = options.arrow_color.unwrap_or(cx.theme().foreground);
        let arrow = options
            .show_arrow
            .then(|| tooltip_arrow(placement.side, placement.arrow_offset, arrow_color));

        deferred(
            tooltip_overlay_positioner(trigger_bounds, options, self.placement.clone()).child(
                div()
                    .relative()
                    .child(content_view)
                    .when_some(arrow, |this, arrow| this.child(arrow))
                    .when(closing, |this| {
                        this.child(div().absolute().top_0().left_0().size_full().occlude())
                    })
                    .map(|el| {
                        if closing {
                            let transition = Transition::new(cx.theme().style.motion.fast())
                                .ease_token(cx.theme().style.motion.exit_easing)
                                .fade(1.0, 0.0);
                            match placement.side {
                                TooltipSide::Top | TooltipSide::Bottom => transition
                                    .slide_y(px(0.), motion_offset.y)
                                    .apply(
                                        el,
                                        ElementId::NamedInteger("tooltip-exit".into(), motion_id),
                                    )
                                    .into_any_element(),
                                TooltipSide::Right | TooltipSide::Left => transition
                                    .slide_x(px(0.), motion_offset.x)
                                    .apply(
                                        el,
                                        ElementId::NamedInteger("tooltip-exit".into(), motion_id),
                                    )
                                    .into_any_element(),
                            }
                        } else {
                            let transition = Transition::new(cx.theme().style.motion.fast())
                                .ease_token(cx.theme().style.motion.enter_easing)
                                .fade(0.0, 1.0);
                            match placement.side {
                                TooltipSide::Top | TooltipSide::Bottom => transition
                                    .slide_y(motion_offset.y, px(0.))
                                    .apply(
                                        el,
                                        ElementId::NamedInteger("tooltip-enter".into(), motion_id),
                                    )
                                    .into_any_element(),
                                TooltipSide::Right | TooltipSide::Left => transition
                                    .slide_x(motion_offset.x, px(0.))
                                    .apply(
                                        el,
                                        ElementId::NamedInteger("tooltip-enter".into(), motion_id),
                                    )
                                    .into_any_element(),
                            }
                        }
                    }),
            ),
        )
        .with_priority(2)
        .into_any_element()
    }
}

/// Paints the shadcn arrow as a diamond attached to the resolved surface side.
fn tooltip_arrow(side: TooltipSide, cross_offset: Pixels, color: gpui::Hsla) -> AnyElement {
    let half = TOOLTIP_ARROW_SIZE.half();
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let mut path = PathBuilder::fill();
            let center = bounds.center();
            let radius = px(0.6);
            path.move_to(point(center.x - radius, bounds.top() + radius));
            path.curve_to(
                point(center.x + radius, bounds.top() + radius),
                point(center.x, bounds.top()),
            );
            path.line_to(point(bounds.right() - radius, center.y - radius));
            path.curve_to(
                point(bounds.right() - radius, center.y + radius),
                point(bounds.right(), center.y),
            );
            path.line_to(point(center.x + radius, bounds.bottom() - radius));
            path.curve_to(
                point(center.x - radius, bounds.bottom() - radius),
                point(center.x, bounds.bottom()),
            );
            path.line_to(point(bounds.left() + radius, center.y + radius));
            path.curve_to(
                point(bounds.left() + radius, center.y - radius),
                point(bounds.left(), center.y),
            );
            path.close();
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        },
    )
    .absolute()
    .size(TOOLTIP_ARROW_SIZE)
    .map(|this| match side {
        TooltipSide::Top => this
            .left(cross_offset - half)
            .bottom(-TOOLTIP_ARROW_PROTRUSION),
        TooltipSide::Right => this
            .left(-TOOLTIP_ARROW_PROTRUSION)
            .top(cross_offset - half),
        TooltipSide::Bottom => this
            .left(cross_offset - half)
            .top(-TOOLTIP_ARROW_PROTRUSION),
        TooltipSide::Left => this
            .right(-TOOLTIP_ARROW_PROTRUSION)
            .top(cross_offset - half),
    })
    .into_any_element()
}

// ── Compositional trigger ────────────────────────────────────────────────────

struct TooltipTriggerState {
    focus_handle: FocusHandle,
    focus_observed: bool,
    hovered: bool,
    focused: bool,
    trigger_bounds: Bounds<Pixels>,
    build: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyView>>,
    options: TooltipOptions,
}

impl TooltipTriggerState {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            focus_observed: false,
            hovered: false,
            focused: false,
            trigger_bounds: Bounds::default(),
            build: None,
            options: TooltipOptions::default(),
        }
    }

    /// Shows the current content from the latest measured trigger bounds.
    fn show(&self, delay: Duration, window: &mut Window, cx: &mut Context<Self>) {
        let Some(build) = self.build.clone() else {
            return;
        };
        let Some(overlay) = Root::tooltip_overlay(window, cx) else {
            return;
        };
        overlay.update(cx, |overlay, cx| {
            overlay.request_show(
                TooltipContent {
                    build,
                    trigger_bounds: self.trigger_bounds,
                    options: self.options,
                },
                delay,
                window,
                cx,
            );
        });
    }

    /// Hides only when neither pointer hover nor keyboard focus owns visibility.
    fn hide_if_inactive(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.hovered || self.focused {
            return;
        }
        let Some(overlay) = Root::tooltip_overlay(window, cx) else {
            return;
        };
        overlay.update(cx, |overlay, cx| {
            overlay.request_hide_after(self.options.hide_delay, window, cx);
        });
    }

    fn on_focus_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focused = true;
        self.show(Duration::ZERO, window, cx);
    }

    fn on_focus_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focused = false;
        self.hide_if_inactive(window, cx);
    }
}

impl Render for TooltipTriggerState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Composes a trigger with managed Tooltip content and placement behavior.
#[derive(IntoElement)]
pub struct TooltipTrigger {
    id: ElementId,
    trigger: Option<AnyElement>,
    build: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyView>>,
    options: TooltipOptions,
    aria_description: Option<SharedString>,
}

impl TooltipTrigger {
    /// Creates a managed Tooltip trigger with stable per-window state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            build: None,
            options: TooltipOptions::default(),
            aria_description: None,
        }
    }

    /// Sets the trigger subtree while preserving its own activation behavior.
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Sets plain text content and exposes it as the trigger description.
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        let text = text.into();
        self.aria_description = Some(text.clone());
        self.build = Some(Rc::new(move |window, cx| {
            Tooltip::new(text.clone()).build(window, cx)
        }));
        self
    }

    /// Sets custom Tooltip content.
    pub fn content<F>(mut self, build: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> AnyView + 'static,
    {
        self.build = Some(Rc::new(build));
        self
    }

    /// Places the surface on the requested physical side, with automatic flipping.
    pub fn side(mut self, side: TooltipSide) -> Self {
        self.options.side = side;
        self
    }

    /// Aligns the surface on the cross axis of its resolved side.
    pub fn align(mut self, align: TooltipAlign) -> Self {
        self.options.align = align;
        self
    }

    /// Sets the distance between the Arrow tip and Trigger.
    pub fn side_offset(mut self, offset: Pixels) -> Self {
        self.options.side_offset = offset;
        self
    }

    /// Offsets the resolved cross-axis alignment.
    pub fn align_offset(mut self, offset: Pixels) -> Self {
        self.options.align_offset = offset;
        self
    }

    /// Sets the pointer-hover delay; keyboard focus always opens immediately.
    pub fn show_delay(mut self, delay: Duration) -> Self {
        self.options.show_delay = delay;
        self
    }

    /// Sets the delay before beginning the exit lifecycle.
    pub fn hide_delay(mut self, delay: Duration) -> Self {
        self.options.hide_delay = delay;
        self
    }

    /// Enables or disables the placement-aware Arrow.
    pub fn show_arrow(mut self, show: bool) -> Self {
        self.options.show_arrow = show;
        self
    }

    /// Overrides the Arrow color independently from the Surface.
    pub fn arrow_color(mut self, color: impl Into<gpui::Hsla>) -> Self {
        self.options.arrow_color = Some(color.into());
        self
    }
}

impl RenderOnce for TooltipTrigger {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state =
            window.use_keyed_state(self.id.clone(), cx, |_, cx| TooltipTriggerState::new(cx));
        state.update(cx, |state, cx| {
            state.build = self.build.clone();
            state.options = self.options;
            if !state.focus_observed {
                cx.on_focus_in(
                    &state.focus_handle,
                    window,
                    TooltipTriggerState::on_focus_in,
                )
                .detach();
                cx.on_focus_out(&state.focus_handle, window, |state, _, window, cx| {
                    state.on_focus_out(window, cx);
                })
                .detach();
                state.focus_observed = true;
            }
        });

        let focus_handle = state.read(cx).focus_handle.clone();
        let Some(trigger) = self.trigger else {
            return div().id(self.id);
        };

        div()
            .id(self.id)
            .flex_none()
            .w_auto()
            .h_auto()
            .track_focus(&focus_handle)
            .when_some(self.aria_description, |this, description| {
                this.aria_description(description)
            })
            .on_prepaint({
                let state = state.clone();
                move |bounds, _, cx| {
                    state.update(cx, |state, _| state.trigger_bounds = bounds);
                }
            })
            .on_hover(window.listener_for(&state, |state, hovered, window, cx| {
                state.hovered = *hovered;
                if *hovered {
                    state.show(state.options.show_delay, window, cx);
                } else {
                    state.hide_if_inactive(window, cx);
                }
            }))
            .on_mouse_down(MouseButton::Left, {
                let state = state.clone();
                move |_, window, cx| {
                    state.update(cx, |state, _| {
                        state.hovered = false;
                        state.focused = false;
                    });
                    if let Some(overlay) = Root::tooltip_overlay(window, cx) {
                        overlay.update(cx, |overlay, cx| overlay.hide(window, cx));
                    }
                }
            })
            .on_key_down({
                let state = state.clone();
                move |event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        state.update(cx, |state, _| {
                            state.hovered = false;
                            state.focused = false;
                        });
                        if let Some(overlay) = Root::tooltip_overlay(window, cx) {
                            overlay.update(cx, |overlay, cx| overlay.hide(window, cx));
                        }
                    }
                }
            })
            .child(trigger)
    }
}

// ── Shared tooltip state for components ─────────────────────────────────────

/// Shared tooltip state that components (Button, Switch, Checkbox, Radio, etc.)
/// can embed to get `.tooltip()` support with minimal boilerplate.
#[derive(Default)]
pub(crate) struct ComponentTooltip {
    pub text: Option<(
        SharedString,
        Option<(Rc<Box<dyn Action>>, Option<SharedString>)>,
    )>,
    pub builder: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyView>>,
}

impl ComponentTooltip {
    /// Wraps a component control in the shared pointer and focus-aware trigger engine.
    pub fn apply<E>(self, id: &ElementId, el: E) -> AnyElement
    where
        E: StatefulInteractiveElement + IntoElement,
    {
        let trigger_id =
            ElementId::NamedChild(std::sync::Arc::new(id.clone()), "tooltip-trigger".into());
        if let Some(builder) = self.builder {
            TooltipTrigger::new(trigger_id)
                .trigger(el)
                .content(move |window, cx| builder(window, cx))
                .into_any_element()
        } else if let Some((text, action)) = self.text {
            TooltipTrigger::new(trigger_id)
                .trigger(el.aria_description(text.clone()))
                .text(text.clone())
                .content(move |window, cx| {
                    Tooltip::new(text.clone())
                        .when_some(action.clone(), |this, (action, context)| {
                            this.action(
                                action.boxed_clone().as_ref(),
                                context.as_ref().map(|c| c.as_ref()),
                            )
                        })
                        .build(window, cx)
                })
                .into_any_element()
        } else {
            el.into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Entity, TestAppContext, VisualTestContext, size};

    struct TooltipTestHost(Entity<Tooltip>);

    impl Render for TooltipTestHost {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().absolute().flex().child(self.0.clone())
        }
    }

    fn test_content(bounds: Bounds<Pixels>) -> TooltipContent {
        TooltipContent {
            build: Rc::new(|window, cx| Tooltip::new("Test tooltip").build(window, cx)),
            trigger_bounds: bounds,
            options: TooltipOptions::default(),
        }
    }

    fn test_options(side: TooltipSide) -> TooltipOptions {
        TooltipOptions {
            side,
            side_offset: px(0.),
            show_arrow: false,
            ..TooltipOptions::default()
        }
    }

    fn test_bounds(x: f32, y: f32, width: f32, height: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(width), px(height)))
    }

    fn test_size(width: f32, height: f32) -> Size<Pixels> {
        size(px(width), px(height))
    }

    fn draw_window(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
    }

    #[test]
    fn tooltip_overlay_clear_state_resets_active_tooltip() {
        let mut overlay = TooltipOverlay::new();

        overlay.content = Some(test_content(test_bounds(10., 10., 40., 20.)));
        overlay.had_recent_tooltip = true;
        overlay._show_task = Some(Task::ready(()));

        assert!(overlay.clear_state());
        assert!(overlay.content.is_none());
        assert!(!overlay.had_recent_tooltip);
        assert!(overlay._show_task.is_none());
        assert!(overlay._hide_task.is_none());
        assert!(overlay.placement.get().is_none());
    }

    #[gpui::test]
    fn tooltip_reopen_during_exit_preserves_latest_content(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (overlay, cx) = cx.add_window_view(|_, _| TooltipOverlay::new());
        let first = test_content(test_bounds(10., 10., 40., 20.));
        let latest = test_content(test_bounds(80., 10., 40., 20.));

        overlay.update_in(cx, |overlay, window, cx| {
            overlay.had_recent_tooltip = true;
            overlay.request_show(first, Duration::ZERO, window, cx);
            overlay.placement.set(Some(TooltipOverlayPosition {
                bounds: test_bounds(10., 10., 80., 30.),
                side: TooltipSide::Top,
                arrow_offset: px(40.),
            }));
            overlay.begin_open(cx);
            overlay.request_hide(window, cx);
            overlay.request_show(latest, Duration::ZERO, window, cx);
            overlay.placement.set(Some(TooltipOverlayPosition {
                bounds: test_bounds(80., 10., 80., 30.),
                side: TooltipSide::Top,
                arrow_offset: px(40.),
            }));
            overlay.begin_open(cx);
        });
        cx.background_executor
            .advance_clock(Duration::from_millis(150));
        cx.run_until_parked();

        assert_eq!(
            overlay.read_with(cx, |overlay, _| overlay.lifecycle.phase()),
            OverlayPhase::Open
        );
        assert_eq!(
            overlay.read_with(cx, |overlay, _| overlay
                .content
                .as_ref()
                .unwrap()
                .trigger_bounds),
            test_bounds(80., 10., 40., 20.)
        );
    }

    #[gpui::test]
    fn tooltip_dismissed_before_placement_unmounts_immediately(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (overlay, cx) = cx.add_window_view(|_, _| TooltipOverlay::new());

        overlay.update_in(cx, |overlay, window, cx| {
            overlay.request_show(
                test_content(test_bounds(10., 10., 40., 20.)),
                Duration::ZERO,
                window,
                cx,
            );
            overlay.request_hide(window, cx);
        });

        assert!(overlay.read_with(cx, |overlay, _| overlay.content.is_none()));
        assert_eq!(
            overlay.read_with(cx, |overlay, _| overlay.lifecycle.phase()),
            OverlayPhase::Closed
        );
    }

    #[gpui::test]
    fn tooltip_reduced_motion_completes_lifecycle_without_delay(cx: &mut TestAppContext) {
        cx.update(crate::init);
        cx.update(|cx| cx.set_reduce_motion(true));
        let (overlay, cx) = cx.add_window_view(|_, _| TooltipOverlay::new());

        overlay.update_in(cx, |overlay, window, cx| {
            overlay.request_show(
                test_content(test_bounds(10., 10., 40., 20.)),
                Duration::ZERO,
                window,
                cx,
            );
            overlay.placement.set(Some(TooltipOverlayPosition {
                bounds: test_bounds(10., 10., 80., 30.),
                side: TooltipSide::Top,
                arrow_offset: px(40.),
            }));
            overlay.begin_open(cx);
        });
        cx.run_until_parked();
        assert_eq!(
            overlay.read_with(cx, |overlay, _| overlay.lifecycle.phase()),
            OverlayPhase::Open
        );

        overlay.update_in(cx, |overlay, window, cx| {
            overlay.request_hide(window, cx);
        });
        cx.run_until_parked();
        assert!(overlay.read_with(cx, |overlay, _| overlay.content.is_none()));
    }

    #[test]
    fn tooltip_overlay_position_prefers_above_when_space_allows() {
        let trigger_bounds = test_bounds(100., 80., 80., 24.);
        let position = tooltip_overlay_position(
            trigger_bounds,
            test_size(120., 30.),
            test_size(300., 200.),
            TOOLTIP_WINDOW_MARGIN,
            test_options(TooltipSide::Top),
        );

        assert_eq!(position.side, TooltipSide::Top);
        assert_eq!(position.bounds.origin.x, px(80.));
        assert_eq!(position.bounds.origin.y, px(50.));
        assert_eq!(position.bounds.bottom(), trigger_bounds.top());
    }

    #[test]
    fn tooltip_overlay_position_flips_below_near_top_edge() {
        let trigger_bounds = test_bounds(24., 4., 120., 32.);
        let position = tooltip_overlay_position(
            trigger_bounds,
            test_size(240., 32.),
            test_size(520., 260.),
            TOOLTIP_WINDOW_MARGIN,
            test_options(TooltipSide::Top),
        );

        assert_eq!(position.side, TooltipSide::Bottom);
        assert_eq!(position.bounds.top(), trigger_bounds.bottom());
        assert!(position.bounds.top() >= trigger_bounds.bottom());
    }

    #[test]
    fn tooltip_motion_points_toward_the_trigger() {
        assert_eq!(
            TooltipSide::Top.motion_offset(px(6.)),
            point(px(0.), px(6.))
        );
        assert_eq!(
            TooltipSide::Right.motion_offset(px(6.)),
            point(px(-6.), px(0.))
        );
        assert_eq!(
            TooltipSide::Bottom.motion_offset(px(6.)),
            point(px(0.), px(-6.))
        );
        assert_eq!(
            TooltipSide::Left.motion_offset(px(6.)),
            point(px(6.), px(0.))
        );
    }

    #[test]
    fn tooltip_overlay_position_clamps_horizontal_edges() {
        let trigger_bounds = test_bounds(4., 80., 24., 24.);
        let position = tooltip_overlay_position(
            trigger_bounds,
            test_size(120., 30.),
            test_size(300., 200.),
            TOOLTIP_WINDOW_MARGIN,
            test_options(TooltipSide::Top),
        );

        assert_eq!(position.side, TooltipSide::Top);
        assert_eq!(position.bounds.left(), TOOLTIP_WINDOW_MARGIN);
    }

    #[test]
    fn tooltip_overlay_position_uses_larger_side_when_neither_side_fits() {
        let trigger_bounds = test_bounds(120., 20., 40., 20.);
        let position = tooltip_overlay_position(
            trigger_bounds,
            test_size(160., 120.),
            test_size(300., 100.),
            TOOLTIP_WINDOW_MARGIN,
            test_options(TooltipSide::Top),
        );

        assert_eq!(position.side, TooltipSide::Bottom);
        assert_eq!(position.bounds.top(), TOOLTIP_WINDOW_MARGIN);
        assert_eq!(position.bounds.left(), px(60.));
    }

    #[test]
    fn tooltip_overlay_position_supports_horizontal_sides() {
        let trigger_bounds = test_bounds(120., 80., 40., 20.);
        let right = tooltip_overlay_position(
            trigger_bounds,
            test_size(80., 30.),
            test_size(320., 220.),
            TOOLTIP_WINDOW_MARGIN,
            test_options(TooltipSide::Right),
        );
        let left = tooltip_overlay_position(
            trigger_bounds,
            test_size(80., 30.),
            test_size(320., 220.),
            TOOLTIP_WINDOW_MARGIN,
            test_options(TooltipSide::Left),
        );

        assert_eq!(right.side, TooltipSide::Right);
        assert_eq!(right.bounds.left(), trigger_bounds.right());
        assert_eq!(left.side, TooltipSide::Left);
        assert_eq!(left.bounds.right(), trigger_bounds.left());
    }

    #[test]
    fn tooltip_overlay_position_applies_cross_axis_alignment() {
        let trigger_bounds = test_bounds(100., 80., 80., 24.);
        let mut start_options = test_options(TooltipSide::Top);
        start_options.align = TooltipAlign::Start;
        let mut end_options = start_options;
        end_options.align = TooltipAlign::End;

        let start = tooltip_overlay_position(
            trigger_bounds,
            test_size(40., 30.),
            test_size(320., 220.),
            TOOLTIP_WINDOW_MARGIN,
            start_options,
        );
        let end = tooltip_overlay_position(
            trigger_bounds,
            test_size(40., 30.),
            test_size(320., 220.),
            TOOLTIP_WINDOW_MARGIN,
            end_options,
        );

        assert_eq!(start.bounds.left(), trigger_bounds.left());
        assert_eq!(end.bounds.right(), trigger_bounds.right());
    }

    #[test]
    fn tooltip_arrow_protrusion_preserves_side_offset_on_every_side() {
        let trigger = test_bounds(150., 100., 40., 20.);
        let tooltip_size = test_size(60., 30.);
        let viewport = test_size(400., 300.);
        let side_offset = px(4.);

        for side in [
            TooltipSide::Top,
            TooltipSide::Right,
            TooltipSide::Bottom,
            TooltipSide::Left,
        ] {
            let position = tooltip_overlay_position(
                trigger,
                tooltip_size,
                viewport,
                TOOLTIP_WINDOW_MARGIN,
                TooltipOptions {
                    side,
                    side_offset,
                    ..TooltipOptions::default()
                },
            );

            match side {
                TooltipSide::Top => assert_eq!(
                    position.bounds.bottom() + TOOLTIP_ARROW_PROTRUSION + side_offset,
                    trigger.top()
                ),
                TooltipSide::Right => assert_eq!(
                    position.bounds.left() - TOOLTIP_ARROW_PROTRUSION - side_offset,
                    trigger.right()
                ),
                TooltipSide::Bottom => assert_eq!(
                    position.bounds.top() - TOOLTIP_ARROW_PROTRUSION - side_offset,
                    trigger.bottom()
                ),
                TooltipSide::Left => assert_eq!(
                    position.bounds.right() + TOOLTIP_ARROW_PROTRUSION + side_offset,
                    trigger.left()
                ),
            }
        }
    }

    #[gpui::test]
    fn tooltip_long_text_wraps_without_clipping_the_kbd(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, cx| {
            TooltipTestHost(cx.new(|_| {
                Tooltip::new(
                    "This is a deliberately long tooltip label that must wrap before its keyboard shortcut is clipped.",
                )
                .key_binding(Some(Keystroke::parse("ctrl-shift-delete").unwrap()))
            }))
        });
        let cx: &mut VisualTestContext = cx;
        draw_window(cx);

        let surface = cx.debug_bounds("tooltip-content").unwrap();
        let text = cx.debug_bounds("tooltip-text").unwrap();
        let kbd = cx.debug_bounds("tooltip-kbd").unwrap();

        assert!(surface.size.width <= px(320.));
        assert!(text.size.height > px(20.));
        assert!(kbd.right() <= surface.right());
    }

    #[gpui::test]
    fn tooltip_short_text_keeps_content_width(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) =
            cx.add_window_view(|_, cx| TooltipTestHost(cx.new(|_| Tooltip::new("Short tooltip"))));
        let cx: &mut VisualTestContext = cx;
        draw_window(cx);

        assert!(cx.debug_bounds("tooltip-content").unwrap().size.width < px(320.));
    }
}
