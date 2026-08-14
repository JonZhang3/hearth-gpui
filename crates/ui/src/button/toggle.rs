use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Axis, Corners, Edges, ElementId, FocusHandle,
    Hsla, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Pixels, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Toggled, Window, accesskit,
    div, prelude::FluentBuilder as _, px, relative,
};
use smallvec::{SmallVec, smallvec};

use crate::{
    ActiveTheme, Disableable, Icon, Sizable, Size, StyledExt, animation::Lerp, theme::MotionEasing,
    tooltip::ComponentTooltip,
};

/// Visual style of a [`Toggle`] and its group items.
#[derive(Default, Copy, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ToggleVariant {
    #[default]
    Ghost,
    Outline,
}

/// Convenience builders shared by Toggle and ToggleGroup.
pub trait ToggleVariants: Sized {
    fn with_variant(self, variant: ToggleVariant) -> Self;

    /// Uses the transparent shadcn default variant.
    fn ghost(self) -> Self {
        self.with_variant(ToggleVariant::Ghost)
    }

    /// Uses the bordered shadcn outline variant.
    fn outline(self) -> Self {
        self.with_variant(ToggleVariant::Outline)
    }
}

/// Renderable semantic paint values retained across controlled-state rerenders.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TogglePaintState {
    border: Hsla,
    foreground: Hsla,
    ring: Hsla,
}

#[derive(Debug, Clone, Copy)]
struct ActiveToggleTransition {
    from: TogglePaintState,
    target: TogglePaintState,
    started_at: Instant,
    duration: Duration,
    easing: MotionEasing,
}

#[derive(Debug, Clone, Copy)]
struct ToggleTransition {
    from: TogglePaintState,
    to: TogglePaintState,
    duration: Duration,
    epoch: u64,
}

/// Persistent motion state that retargets from the currently visible value.
#[derive(Debug, Clone, Copy)]
struct ToggleMotionState {
    target: TogglePaintState,
    active: Option<ActiveToggleTransition>,
    epoch: u64,
}

impl ToggleMotionState {
    /// Creates stable initial state without animating the first render.
    fn new(target: TogglePaintState) -> Self {
        Self {
            target,
            active: None,
            epoch: 0,
        }
    }

    /// Samples the currently visible paint and clears completed motion.
    fn current(&mut self, now: Instant) -> TogglePaintState {
        let Some(active) = self.active else {
            return self.target;
        };
        let elapsed = now.saturating_duration_since(active.started_at);
        let linear_delta = if active.duration.is_zero() {
            1.
        } else {
            elapsed.as_secs_f32() / active.duration.as_secs_f32()
        };
        let current = interpolate_toggle_paint(
            active.from,
            active.target,
            active.easing.sample(linear_delta),
        );
        if linear_delta >= 1. {
            self.active = None;
            active.target
        } else {
            current
        }
    }

    /// Retargets motion from the sampled value, including rapid reversals.
    fn transition_to(
        &mut self,
        target: TogglePaintState,
        now: Instant,
        duration: Duration,
        easing: MotionEasing,
    ) -> Option<ToggleTransition> {
        let previous_active = self.active;
        let target_unchanged = self.target == target;
        let current = self.current(now);
        if target_unchanged && self.active.is_none() {
            return None;
        }

        self.target = target;
        self.epoch = self.epoch.wrapping_add(1);
        let duration = previous_active
            .map(|active| {
                let elapsed = now
                    .saturating_duration_since(active.started_at)
                    .min(active.duration);
                if target_unchanged {
                    active.duration.saturating_sub(elapsed)
                } else if target == active.from {
                    elapsed
                } else {
                    duration
                }
            })
            .unwrap_or(duration);

        if duration.is_zero() || current == target {
            self.active = None;
            return None;
        }

        self.active = Some(ActiveToggleTransition {
            from: current,
            target,
            started_at: now,
            duration,
            easing,
        });
        Some(ToggleTransition {
            from: current,
            to: target,
            duration,
            epoch: self.epoch,
        })
    }
}

fn interpolate_toggle_paint(
    from: TogglePaintState,
    to: TogglePaintState,
    delta: f32,
) -> TogglePaintState {
    TogglePaintState {
        border: Lerp::lerp(&from.border, &to.border, delta),
        foreground: Lerp::lerp(&from.foreground, &to.foreground, delta),
        ring: Lerp::lerp(&from.ring, &to.ring, delta),
    }
}

/// Derives internal IDs without flattening structural caller IDs to text.
fn toggle_child_id(id: &ElementId, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), name.into())
}

/// Mirrors shadcn's `focus-visible` behavior.
fn toggle_focus_visible(focused: bool, last_input_was_keyboard: bool) -> bool {
    focused && last_input_was_keyboard
}

/// A controlled two-state button aligned with shadcn Toggle semantics.
#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    style: StyleRefinement,
    checked: bool,
    size: Size,
    variant: ToggleVariant,
    disabled: bool,
    invalid: bool,
    tab_stop: bool,
    tab_index: isize,
    border_corners: Corners<bool>,
    border_edges: Edges<bool>,
    aria_label: Option<SharedString>,
    icon: Option<Icon>,
    trailing_icon: Option<Icon>,
    label: Option<SharedString>,
    children: SmallVec<[AnyElement; 1]>,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    focus_handle: Option<FocusHandle>,
    on_key_down: Option<Rc<dyn Fn(&KeyDownEvent, &mut Window, &mut App) + 'static>>,
    tooltip: ComponentTooltip,
}

impl Toggle {
    /// Creates a controlled Toggle with a stable element ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            checked: false,
            size: Size::default(),
            variant: ToggleVariant::default(),
            disabled: false,
            invalid: false,
            tab_stop: true,
            tab_index: 0,
            border_corners: Corners {
                top_left: true,
                top_right: true,
                bottom_left: true,
                bottom_right: true,
            },
            border_edges: Edges::all(true),
            aria_label: None,
            icon: None,
            trailing_icon: None,
            label: None,
            children: smallvec![],
            on_click: None,
            focus_handle: None,
            on_key_down: None,
            tooltip: ComponentTooltip::default(),
        }
    }

    /// Sets tooltip text and uses it as the fallback accessible name.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip.text = Some((tooltip.into(), None));
        self
    }

    /// Sets the visible text label and accessible name.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.aria_label = Some(label.clone());
        self.label = Some(label);
        self
    }

    /// Sets an accessible name independently from visible content.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets the leading icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the trailing icon.
    pub fn trailing_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.trailing_icon = Some(icon.into());
        self
    }

    /// Sets the controlled pressed state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Sets the invalid accessibility and destructive-ring state.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets whether this Toggle participates in sequential keyboard focus.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Sets the keyboard tab index.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Sets the activation handler, receiving the requested next checked state.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    fn border_corners(mut self, corners: Corners<bool>) -> Self {
        self.border_corners = corners;
        self
    }

    fn border_edges(mut self, edges: Edges<bool>) -> Self {
        self.border_edges = edges;
        self
    }

    fn focus_handle(mut self, focus_handle: FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle);
        self
    }

    fn on_group_key_down(
        mut self,
        handler: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_key_down = Some(Rc::new(handler));
        self
    }
}

impl ToggleVariants for Toggle {
    fn with_variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl ParentElement for Toggle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Disableable for Toggle {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Sizable for Toggle {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Toggle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Toggle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let disabled = self.disabled;
        let invalid = self.invalid;
        let interactive = !disabled && self.on_click.is_some();
        let focus_handle = self.focus_handle.clone().unwrap_or_else(|| {
            window
                .use_keyed_state(toggle_child_id(&self.id, "focus"), cx, |_, cx| {
                    cx.focus_handle()
                })
                .read(cx)
                .clone()
        });
        let focus_visible = toggle_focus_visible(
            focus_handle.is_focused(window),
            window.last_input_was_keyboard(),
        );
        let metrics = cx.theme().style.controls.for_size(self.size);
        let rounding = cx.theme().style.radii.md;
        let ring_width = cx.theme().style.focus.ring_width;
        let ring_inset = ring_width + cx.theme().style.focus.ring_offset;
        let invalid_border = cx
            .theme()
            .danger
            .opacity(if cx.theme().is_dark() { 0.5 } else { 1. });
        let border = if invalid {
            invalid_border
        } else if focus_visible {
            cx.theme().ring
        } else if self.variant == ToggleVariant::Outline {
            cx.theme().input
        } else {
            cx.theme().transparent
        };
        let ring_color = if invalid {
            cx.theme()
                .danger
                .opacity(if cx.theme().is_dark() { 0.4 } else { 0.2 })
        } else {
            cx.theme().ring.opacity(0.5)
        };
        let background = if checked {
            cx.theme().muted
        } else {
            cx.theme().transparent
        };
        let paint = TogglePaintState {
            border,
            foreground: cx.theme().foreground,
            ring: if invalid || focus_visible {
                ring_color
            } else {
                ring_color.opacity(0.)
            },
        };
        let semantic_motion = self.style.background.is_none() && self.style.border_color.is_none();
        let duration = if cx.reduce_motion() || !semantic_motion {
            Duration::ZERO
        } else {
            cx.theme().style.motion.normal()
        };
        let easing = cx.theme().style.motion.move_easing;
        let motion_state =
            window.use_keyed_state(toggle_child_id(&self.id, "paint-motion"), cx, |_, _| {
                ToggleMotionState::new(paint)
            });
        let transition = motion_state.update(cx, |state, _| {
            state.transition_to(paint, Instant::now(), duration, easing)
        });
        let show_ring = invalid
            || focus_visible
            || transition.is_some_and(|segment| segment.from.ring.a > 0. || segment.to.ring.a > 0.);
        let icon_only = self.label.is_none()
            && self.children.is_empty()
            && (self.icon.is_some() ^ self.trailing_icon.is_some());
        let accessible_label = self
            .aria_label
            .clone()
            .or_else(|| self.tooltip.text.as_ref().map(|(text, _)| text.clone()));
        let on_click = self.on_click.clone();

        let ring = show_ring.then(|| {
            let ring = div()
                .absolute()
                .top(-ring_inset)
                .right(-ring_inset)
                .bottom(-ring_inset)
                .left(-ring_inset)
                .border(ring_width)
                .border_color(paint.ring)
                .rounded(rounding + ring_width);
            if let Some(transition) = transition.filter(|_| semantic_motion) {
                let animation_id = toggle_child_id(&self.id, format!("ring-{}", transition.epoch));
                ring.with_animation(
                    animation_id,
                    Animation::new(transition.duration)
                        .with_easing(move |delta| easing.sample(delta)),
                    move |this, delta| {
                        let paint = interpolate_toggle_paint(transition.from, transition.to, delta);
                        this.border_color(paint.ring)
                    },
                )
                .into_any_element()
            } else {
                ring.into_any_element()
            }
        });
        let surface = div()
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .left_0()
            .when(self.border_corners.top_left, |this| {
                this.rounded_tl(rounding)
            })
            .when(self.border_corners.top_right, |this| {
                this.rounded_tr(rounding)
            })
            .when(self.border_corners.bottom_left, |this| {
                this.rounded_bl(rounding)
            })
            .when(self.border_corners.bottom_right, |this| {
                this.rounded_br(rounding)
            })
            .when(self.border_edges.left, |this| this.border_l_1())
            .when(self.border_edges.right, |this| this.border_r_1())
            .when(self.border_edges.top, |this| this.border_t_1())
            .when(self.border_edges.bottom, |this| this.border_b_1())
            // Checked background changes are immediate; only border and ring paint animate.
            .bg(if semantic_motion {
                background
            } else {
                cx.theme().transparent
            })
            .border_color(if semantic_motion {
                paint.border
            } else {
                cx.theme().transparent
            })
            .when_some(ring, |this, ring| this.child(ring));
        let surface = if let Some(transition) = transition.filter(|_| semantic_motion) {
            let animation_id = toggle_child_id(&self.id, format!("paint-{}", transition.epoch));
            surface
                .with_animation(
                    animation_id,
                    Animation::new(transition.duration)
                        .with_easing(move |delta| easing.sample(delta)),
                    move |this, delta| {
                        let paint = interpolate_toggle_paint(transition.from, transition.to, delta);
                        this.border_color(paint.border)
                    },
                )
                .into_any_element()
        } else {
            surface.into_any_element()
        };

        let control = div()
            .id(self.id.clone())
            .role(Role::Button)
            .aria_toggled(if checked {
                Toggled::True
            } else {
                Toggled::False
            })
            .when_some(accessible_label, |this, label| this.aria_label(label))
            .when(interactive, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_stop(self.tab_stop)
                        .tab_index(self.tab_index),
                )
            })
            .relative()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .whitespace_nowrap()
            .font_medium()
            .text_sm()
            .gap(metrics.gap)
            .when(icon_only, |this| this.size(metrics.height))
            .when(!icon_only, |this| {
                this.h(metrics.height)
                    .min_w(metrics.height)
                    .pl(if self.icon.is_some() {
                        metrics.icon_edge_padding
                    } else {
                        metrics.padding_x
                    })
                    .pr(if self.trailing_icon.is_some() {
                        metrics.icon_edge_padding
                    } else {
                        metrics.padding_x
                    })
            })
            .when(self.border_corners.top_left, |this| {
                this.rounded_tl(rounding)
            })
            .when(self.border_corners.top_right, |this| {
                this.rounded_tr(rounding)
            })
            .when(self.border_corners.bottom_left, |this| {
                this.rounded_bl(rounding)
            })
            .when(self.border_corners.bottom_right, |this| {
                this.rounded_br(rounding)
            })
            .when(self.border_edges.left, |this| this.border_l_1())
            .when(self.border_edges.right, |this| this.border_r_1())
            .when(self.border_edges.top, |this| this.border_t_1())
            .when(self.border_edges.bottom, |this| this.border_b_1())
            .bg(if semantic_motion {
                cx.theme().transparent
            } else {
                background
            })
            .border_color(if semantic_motion {
                cx.theme().transparent
            } else {
                paint.border
            })
            .text_color(paint.foreground)
            .when(!disabled && !checked, |this| {
                this.hover(|this| this.bg(cx.theme().muted).text_color(cx.theme().foreground))
            })
            .when(disabled, |this| this.opacity(0.5).shadow_none())
            .refine_style(&self.style)
            .child(surface)
            .when_some(self.icon, |this, icon| {
                this.child(icon.with_size(Size::Size(metrics.icon_size)))
            })
            .when_some(self.label, |this, label| {
                this.child(div().flex_none().line_height(relative(1.)).child(label))
            })
            .children(self.children)
            .when_some(self.trailing_icon, |this, icon| {
                this.child(icon.with_size(Size::Size(metrics.icon_size)))
            })
            .when(interactive, |this| {
                this.on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
                    // Pointer activation must not produce a keyboard focus-visible ring.
                    window.prevent_default();
                    crate::global_state::GlobalState::suppress_text_selection(cx);
                })
            })
            .when(interactive, |this| {
                this.on_click(move |_, window, cx| {
                    window.prevent_default();
                    if let Some(on_click) = on_click.as_ref() {
                        on_click(&!checked, window, cx);
                    }
                })
            })
            .when_some(self.on_key_down, |this, on_key_down| {
                this.on_key_down(move |event, window, cx| on_key_down(event, window, cx))
            });

        let control = self.tooltip.apply(&self.id, control);
        crate::accessibility::accessibility_state(control, invalid, false, disabled)
    }
}

/// Selection behavior of a ToggleGroup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToggleGroupMode {
    Single,
    #[default]
    Multiple,
}

/// Controlled value of a ToggleGroup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToggleGroupSelection {
    Single(Option<SharedString>),
    Multiple(Vec<SharedString>),
}

impl Default for ToggleGroupSelection {
    fn default() -> Self {
        Self::Multiple(Vec::new())
    }
}

impl ToggleGroupSelection {
    fn contains(&self, value: &SharedString) -> bool {
        match self {
            Self::Single(selected) => selected.as_ref() == Some(value),
            Self::Multiple(selected) => selected.contains(value),
        }
    }

    fn toggled(&self, mode: ToggleGroupMode, value: &SharedString) -> Self {
        match mode {
            ToggleGroupMode::Single => Self::Single((!self.contains(value)).then(|| value.clone())),
            ToggleGroupMode::Multiple => {
                let mut selected = match self {
                    Self::Multiple(selected) => selected.clone(),
                    Self::Single(selected) => selected.iter().cloned().collect(),
                };
                if let Some(ix) = selected.iter().position(|candidate| candidate == value) {
                    selected.remove(ix);
                } else {
                    selected.push(value.clone());
                }
                Self::Multiple(selected)
            }
        }
    }
}

/// Resolves an axis-aware roving-focus target within the enabled item sequence.
fn toggle_group_focus_target(
    key: &str,
    orientation: Axis,
    position: usize,
    len: usize,
) -> Option<usize> {
    if len == 0 || position >= len {
        return None;
    }
    match key {
        "home" => Some(0),
        "end" => Some(len - 1),
        "left" if orientation == Axis::Horizontal => Some((position + len - 1) % len),
        "right" if orientation == Axis::Horizontal => Some((position + 1) % len),
        "up" if orientation == Axis::Vertical => Some((position + len - 1) % len),
        "down" if orientation == Axis::Vertical => Some((position + 1) % len),
        _ => None,
    }
}

/// A typed item owned by a [`ToggleGroup`].
pub struct ToggleGroupItem {
    value: SharedString,
    toggle: Toggle,
}

impl ToggleGroupItem {
    /// Creates an item with a stable selection value.
    pub fn new(value: impl Into<SharedString>) -> Self {
        let value = value.into();
        Self {
            toggle: Toggle::new(value.clone()),
            value,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.toggle = self.toggle.label(label);
        self
    }

    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.toggle = self.toggle.aria_label(label);
        self
    }

    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.toggle = self.toggle.icon(icon);
        self
    }

    pub fn trailing_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.toggle = self.toggle.trailing_icon(icon);
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.toggle = self.toggle.tooltip(tooltip);
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.toggle = self.toggle.invalid(invalid);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.toggle = self.toggle.disabled(disabled);
        self
    }

    /// Adds an item-level activation side effect without replacing the group change callback.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.toggle = self.toggle.on_click(handler);
        self
    }
}

impl ParentElement for ToggleGroupItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.toggle.extend(elements);
    }
}

impl Styled for ToggleGroupItem {
    fn style(&mut self) -> &mut StyleRefinement {
        self.toggle.style()
    }
}

/// A controlled single- or multiple-selection group of Toggle buttons.
#[derive(IntoElement)]
pub struct ToggleGroup {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    variant: ToggleVariant,
    disabled: bool,
    mode: ToggleGroupMode,
    selection: ToggleGroupSelection,
    orientation: Axis,
    spacing: Pixels,
    aria_label: Option<SharedString>,
    items: Vec<ToggleGroupItem>,
    on_change: Option<Rc<dyn Fn(&ToggleGroupSelection, &mut Window, &mut App) + 'static>>,
}

impl ToggleGroup {
    /// Creates a horizontal multiple-selection group with shadcn's default 8px spacing.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            size: Size::default(),
            variant: ToggleVariant::default(),
            disabled: false,
            mode: ToggleGroupMode::Multiple,
            selection: ToggleGroupSelection::default(),
            orientation: Axis::Horizontal,
            spacing: px(8.),
            aria_label: None,
            items: Vec::new(),
            on_change: None,
        }
    }

    pub fn mode(mut self, mode: ToggleGroupMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn selection(mut self, selection: ToggleGroupSelection) -> Self {
        self.selection = selection;
        self
    }

    pub fn orientation(mut self, orientation: Axis) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn spacing(mut self, spacing: Pixels) -> Self {
        self.spacing = spacing.max(px(0.));
        self
    }

    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    pub fn child(mut self, item: ToggleGroupItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn children(mut self, items: impl IntoIterator<Item = ToggleGroupItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&ToggleGroupSelection, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Sizable for ToggleGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ToggleVariants for ToggleGroup {
    fn with_variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Disableable for ToggleGroup {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for ToggleGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ToggleGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let joined = self.spacing == px(0.);
        let items_len = self.items.len();
        let joined_outline = joined && self.variant == ToggleVariant::Outline;
        let group_border = if cx.theme().mode.is_dark() {
            cx.theme().input
        } else {
            cx.theme().border
        };
        let group_rounding = cx.theme().style.radii.md;
        let item_focus_handles = self
            .items
            .iter()
            .enumerate()
            .map(|(ix, _)| {
                window
                    .use_keyed_state(
                        toggle_child_id(&self.id, format!("item-{ix}-focus")),
                        cx,
                        |_, cx| cx.focus_handle(),
                    )
                    .read(cx)
                    .clone()
            })
            .collect::<Vec<_>>();
        let enabled_indexes = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(ix, item)| (!self.disabled && !item.toggle.disabled).then_some(ix))
            .collect::<Vec<_>>();
        let preferred_tab_index = enabled_indexes
            .iter()
            .copied()
            .find(|ix| self.selection.contains(&self.items[*ix].value))
            .or_else(|| enabled_indexes.first().copied());
        let group_id = self.id.clone();
        let orientation = self.orientation;
        let selection = self.selection.clone();
        let mode = self.mode;
        let on_change = self.on_change.clone();
        let variant = self.variant;
        let size = self.size;
        let disabled = self.disabled;

        let element = div()
            .id(group_id)
            .role(Role::Group)
            .aria_orientation(if orientation == Axis::Horizontal {
                accesskit::Orientation::Horizontal
            } else {
                accesskit::Orientation::Vertical
            })
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .flex()
            .when(orientation == Axis::Horizontal, |this| {
                this.flex_row().items_center()
            })
            .when(orientation == Axis::Vertical, |this| {
                this.flex_col().items_stretch()
            })
            .gap(self.spacing)
            .relative()
            .refine_style(&self.style)
            .children(self.items.into_iter().enumerate().map(|(ix, mut item)| {
                let value = item.value.clone();
                let selected = selection.contains(&value);
                let item_disabled = disabled || item.toggle.disabled;
                let item_handler = item.toggle.on_click.take();
                let group_handler = on_change.clone();
                let current_selection = selection.clone();
                let focus_handle = item_focus_handles[ix].clone();
                let enabled_focus_handles = enabled_indexes
                    .iter()
                    .map(|index| item_focus_handles[*index].clone())
                    .collect::<Vec<_>>();
                let enabled_position = enabled_indexes.iter().position(|index| *index == ix);
                let corners = if !joined || items_len == 1 {
                    Corners {
                        top_left: true,
                        top_right: true,
                        bottom_left: true,
                        bottom_right: true,
                    }
                } else if orientation == Axis::Horizontal {
                    Corners {
                        top_left: ix == 0,
                        top_right: ix + 1 == items_len,
                        bottom_left: ix == 0,
                        bottom_right: ix + 1 == items_len,
                    }
                } else {
                    Corners {
                        top_left: ix == 0,
                        top_right: ix == 0,
                        bottom_left: ix + 1 == items_len,
                        bottom_right: ix + 1 == items_len,
                    }
                };
                let edges = if !joined || items_len == 1 {
                    Edges::all(true)
                } else if orientation == Axis::Horizontal {
                    Edges {
                        left: ix == 0,
                        right: true,
                        top: true,
                        bottom: true,
                    }
                } else {
                    Edges {
                        left: true,
                        right: true,
                        top: ix == 0,
                        bottom: true,
                    }
                };

                let toggle = item
                    .toggle
                    .checked(selected)
                    .disabled(item_disabled)
                    .with_size(size)
                    .with_variant(variant)
                    .border_corners(corners)
                    // The joined outline is painted by the group. Removing item borders avoids
                    // exposing the transparent layout border between adjacent controls.
                    .border_edges(if joined_outline {
                        Edges::all(false)
                    } else {
                        edges
                    })
                    .focus_handle(focus_handle)
                    .tab_stop(preferred_tab_index == Some(ix))
                    .on_group_key_down(move |event, window, cx| {
                        if event.keystroke.modifiers.modified() {
                            return;
                        }
                        let Some(position) = enabled_position else {
                            return;
                        };
                        let target = toggle_group_focus_target(
                            event.keystroke.key.as_str(),
                            orientation,
                            position,
                            enabled_focus_handles.len(),
                        )
                        .and_then(|target| enabled_focus_handles.get(target));
                        if let Some(target) = target {
                            window.prevent_default();
                            cx.stop_propagation();
                            target.focus(window, cx);
                        }
                    })
                    .on_click(move |next, window, cx| {
                        if let Some(item_handler) = item_handler.as_ref() {
                            item_handler(next, window, cx);
                        }
                        if let Some(group_handler) = group_handler.as_ref() {
                            let next = current_selection.toggled(mode, &value);
                            group_handler(&next, window, cx);
                        }
                    });

                let separator = (joined_outline && ix + 1 < items_len).then(|| {
                    div()
                        .absolute()
                        .bg(group_border)
                        .when(orientation == Axis::Horizontal, |this| {
                            this.top_0().right_0().bottom_0().w(px(1.))
                        })
                        .when(orientation == Axis::Vertical, |this| {
                            this.right_0().bottom_0().left_0().h(px(1.))
                        })
                });

                div()
                    .relative()
                    .flex()
                    .when(orientation == Axis::Vertical, |this| {
                        this.flex_col().items_stretch()
                    })
                    .child(toggle)
                    .when_some(separator, |this, separator| this.child(separator))
            }))
            .when(joined_outline, |this| {
                // Paint one non-layout-affecting outline around the complete connected group.
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .right_0()
                        .bottom_0()
                        .left_0()
                        .border_1()
                        .border_color(group_border)
                        .rounded(group_rounding),
                )
            });

        crate::accessibility::accessibility_state(element, false, false, disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IconName;
    use gpui::{
        AppContext as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render, TestAppContext,
        VisualTestContext, point,
    };
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    #[gpui::test]
    fn toggle_builder_exposes_aligned_states(_cx: &mut gpui::TestAppContext) {
        let toggle = Toggle::new("complex-toggle")
            .label("Enable Feature")
            .icon(IconName::Check)
            .trailing_icon(IconName::ArrowRight)
            .checked(true)
            .invalid(true)
            .outline()
            .large()
            .tab_stop(false)
            .tab_index(2)
            .disabled(false)
            .on_click(|_, _, _| {});

        assert_eq!(toggle.label.as_deref(), Some("Enable Feature"));
        assert_eq!(toggle.aria_label.as_deref(), Some("Enable Feature"));
        assert!(toggle.icon.is_some());
        assert!(toggle.trailing_icon.is_some());
        assert!(toggle.checked);
        assert!(toggle.invalid);
        assert_eq!(toggle.variant, ToggleVariant::Outline);
        assert_eq!(toggle.size, Size::Large);
        assert!(!toggle.tab_stop);
        assert_eq!(toggle.tab_index, 2);
    }

    #[test]
    fn selection_supports_single_and_multiple_toggle_rules() {
        let bold: SharedString = "bold".into();
        let italic: SharedString = "italic".into();
        let empty = ToggleGroupSelection::Single(None);
        let selected = empty.toggled(ToggleGroupMode::Single, &bold);
        assert_eq!(selected, ToggleGroupSelection::Single(Some(bold.clone())));
        assert_eq!(
            selected.toggled(ToggleGroupMode::Single, &bold),
            ToggleGroupSelection::Single(None)
        );

        let selected = ToggleGroupSelection::Multiple(vec![bold.clone()]);
        assert_eq!(
            selected.toggled(ToggleGroupMode::Multiple, &italic),
            ToggleGroupSelection::Multiple(vec![bold.clone(), italic])
        );
        assert_eq!(
            selected.toggled(ToggleGroupMode::Multiple, &bold),
            ToggleGroupSelection::Multiple(Vec::new())
        );
    }

    #[test]
    fn group_roving_focus_is_axis_aware_and_wraps() {
        assert_eq!(
            toggle_group_focus_target("left", Axis::Horizontal, 0, 3),
            Some(2)
        );
        assert_eq!(
            toggle_group_focus_target("right", Axis::Horizontal, 2, 3),
            Some(0)
        );
        assert_eq!(
            toggle_group_focus_target("up", Axis::Vertical, 0, 3),
            Some(2)
        );
        assert_eq!(
            toggle_group_focus_target("down", Axis::Vertical, 2, 3),
            Some(0)
        );
        assert_eq!(
            toggle_group_focus_target("down", Axis::Horizontal, 1, 3),
            None
        );
        assert_eq!(
            toggle_group_focus_target("home", Axis::Vertical, 2, 3),
            Some(0)
        );
        assert_eq!(
            toggle_group_focus_target("end", Axis::Vertical, 0, 3),
            Some(2)
        );
        assert_eq!(toggle_group_focus_target("end", Axis::Vertical, 0, 0), None);
    }

    #[test]
    fn motion_state_starts_stable_and_reverses_from_current_value() {
        let off = TogglePaintState {
            border: Hsla::transparent_black(),
            foreground: Hsla::black(),
            ring: Hsla::transparent_black(),
        };
        let on = TogglePaintState {
            border: Hsla::red(),
            foreground: Hsla::white(),
            ring: Hsla::red(),
        };
        let now = Instant::now();
        let duration = Duration::from_millis(150);
        let mut state = ToggleMotionState::new(off);

        assert!(
            state
                .transition_to(off, now, duration, MotionEasing::Linear)
                .is_none()
        );
        state
            .transition_to(on, now, duration, MotionEasing::Linear)
            .expect("forward transition");
        let reverse = state
            .transition_to(
                off,
                now + Duration::from_millis(75),
                duration,
                MotionEasing::Linear,
            )
            .expect("reverse transition");
        assert_eq!(reverse.duration, Duration::from_millis(75));
        assert_eq!(reverse.to, off);
        assert_ne!(reverse.from, on);
    }

    #[test]
    fn reduced_motion_reaches_target_immediately() {
        let off = TogglePaintState {
            border: Hsla::transparent_black(),
            foreground: Hsla::black(),
            ring: Hsla::transparent_black(),
        };
        let on = TogglePaintState {
            border: Hsla::red(),
            foreground: Hsla::white(),
            ring: Hsla::red(),
        };
        let mut state = ToggleMotionState::new(off);
        assert!(
            state
                .transition_to(on, Instant::now(), Duration::ZERO, MotionEasing::Linear)
                .is_none()
        );
        assert_eq!(state.target, on);
        assert!(state.active.is_none());
    }

    #[test]
    fn focus_ring_is_keyboard_only() {
        assert!(toggle_focus_visible(true, true));
        assert!(!toggle_focus_visible(true, false));
        assert!(!toggle_focus_visible(false, true));
    }

    #[test]
    fn internal_ids_preserve_structural_identity() {
        let structured = ElementId::NamedInteger("toggle".into(), 1);
        let textual = ElementId::Name("toggle-1".into());
        assert_eq!(structured.to_string(), textual.to_string());
        assert_ne!(
            toggle_child_id(&structured, "paint-motion"),
            toggle_child_id(&textual, "paint-motion")
        );
    }

    struct KeyboardFixture {
        calls: Arc<AtomicUsize>,
        checked: Arc<Mutex<bool>>,
    }

    impl Render for KeyboardFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            let checked = self.checked.clone();
            div().child(Toggle::new("keyboard-toggle").label("Bold").on_click(
                move |value, _, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    *checked.lock().unwrap() = *value;
                },
            ))
        }
    }

    #[gpui::test]
    fn space_activates_once_and_ignores_key_repeat(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let checked = Arc::new(Mutex::new(false));
        let captured_calls = calls.clone();
        let captured_checked = checked.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| KeyboardFixture { calls, checked });
            crate::Root::new(fixture, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
        });
        let space = Keystroke::parse("space").expect("space must be a valid keystroke");
        cx.simulate_event(KeyDownEvent {
            keystroke: space.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyDownEvent {
            keystroke: space.clone(),
            is_held: true,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke: space });
        cx.run_until_parked();

        assert_eq!(captured_calls.load(Ordering::SeqCst), 1);
        assert!(*captured_checked.lock().unwrap());
    }

    #[gpui::test]
    fn pointer_click_uses_the_same_single_activation_path(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let checked = Arc::new(Mutex::new(false));
        let captured_calls = calls.clone();
        let captured_checked = checked.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| KeyboardFixture { calls, checked });
            crate::Root::new(fixture, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        cx.simulate_click(point(px(18.), px(18.)), Modifiers::default());
        cx.run_until_parked();

        assert_eq!(captured_calls.load(Ordering::SeqCst), 1);
        assert!(*captured_checked.lock().unwrap());
    }
}
