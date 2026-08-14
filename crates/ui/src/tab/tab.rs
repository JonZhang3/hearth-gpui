// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added or exposed behavior through `tab_child_id`, `current`, `transition_to`, `base_height`,
//   `state_id`, `focus_handle`, `on_group_key_down`,
//   `pill_foreground_reverses_from_the_sampled_color` and 1 more.
// - Reworked Tab around accessibility semantics and ARIA state, interruptible and
//   reduced-motion-aware transitions, semantic Style Preset geometry and density, keyboard
//   navigation and activation behavior, focus-visible and focus restoration behavior.
// - Replaced legacy radius access with `Theme.style.radii.md`.
use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::animation::Lerp;
use crate::{ActiveTheme, Icon, IconName, Selectable, Sizable, Size, StyledExt, h_flex};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Background, ClickEvent, Div, Edges, ElementId,
    FocusHandle, Hsla, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div, px,
    relative,
};

use crate::styled::FocusableExt as _;
use crate::theme::MotionEasing;

/// Creates a structural child ID without flattening the caller's ElementId.
pub(super) fn tab_child_id(id: &ElementId, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), name.into())
}

#[derive(Debug, Clone, Copy)]
struct ActiveTabForegroundTransition {
    from: Hsla,
    target: Hsla,
    started_at: Instant,
    duration: Duration,
    easing: MotionEasing,
}

#[derive(Debug, Clone, Copy)]
struct TabForegroundTransition {
    from: Hsla,
    target: Hsla,
    duration: Duration,
    epoch: u64,
}

/// Retains the sampled Pill foreground color across rapid selection changes.
#[derive(Debug, Clone, Copy)]
struct TabForegroundMotionState {
    target: Hsla,
    active: Option<ActiveTabForegroundTransition>,
    epoch: u64,
}

impl TabForegroundMotionState {
    fn new(target: Hsla) -> Self {
        Self {
            target,
            active: None,
            epoch: 0,
        }
    }

    /// Returns the currently visible foreground and clears completed motion.
    fn current(&mut self, now: Instant) -> Hsla {
        let Some(active) = self.active else {
            return self.target;
        };
        let elapsed = now.saturating_duration_since(active.started_at);
        let linear_delta = if active.duration.is_zero() {
            1.
        } else {
            elapsed.as_secs_f32() / active.duration.as_secs_f32()
        };
        let current = Lerp::lerp(
            &active.from,
            &active.target,
            active.easing.sample(linear_delta),
        );
        if linear_delta >= 1. {
            self.active = None;
            active.target
        } else {
            current
        }
    }

    /// Retargets from the currently sampled foreground without a reversal jump.
    fn transition_to(
        &mut self,
        target: Hsla,
        now: Instant,
        duration: Duration,
        easing: MotionEasing,
    ) -> Option<TabForegroundTransition> {
        let current = self.current(now);
        if self.target == target {
            return None;
        }

        self.target = target;
        self.epoch = self.epoch.wrapping_add(1);
        if duration.is_zero() || current == target {
            self.active = None;
            return None;
        }
        self.active = Some(ActiveTabForegroundTransition {
            from: current,
            target,
            started_at: now,
            duration,
            easing,
        });
        Some(TabForegroundTransition {
            from: current,
            target,
            duration,
            epoch: self.epoch,
        })
    }
}

/// Tab variants.
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub enum TabVariant {
    #[default]
    Tab,
    Outline,
    Pill,
    Segmented,
    Underline,
}

impl TabVariant {
    /// Resolves the compact tab control height from the active control metrics.
    fn base_height(size: Size, cx: &App) -> Pixels {
        let controls = cx.theme().style.controls;
        match size {
            Size::XSmall => (controls.xs.height - px(4.)).max(px(0.)),
            Size::Small => controls.xs.height,
            Size::Medium => controls.sm.height,
            Size::Large => controls.md.height,
            Size::Size(height) => height,
        }
    }

    pub(super) fn height(&self, size: Size, cx: &App) -> Pixels {
        let height = Self::base_height(size, cx);
        if *self != TabVariant::Underline {
            return height;
        }

        height
            + match size {
                Size::XSmall | Size::Small => px(6.),
                Size::Large => px(8.),
                Size::Medium | Size::Size(_) => px(4.),
            }
    }

    pub(super) fn inner_height(&self, size: Size, cx: &App) -> Pixels {
        let outer_height = self.height(size, cx);
        let inset = match (self, size) {
            (TabVariant::Segmented, Size::XSmall) => px(4.),
            (TabVariant::Segmented, Size::Small) => px(6.),
            (TabVariant::Segmented, _) => px(8.),
            (TabVariant::Underline, Size::XSmall) => px(6.),
            (TabVariant::Underline, Size::Small) => px(8.),
            (TabVariant::Underline, Size::Large) => px(12.),
            (TabVariant::Underline, _) => px(10.),
            (TabVariant::Outline | TabVariant::Pill, Size::Medium) => px(6.),
            (TabVariant::Tab | TabVariant::Outline | TabVariant::Pill, Size::Large) => px(0.),
            _ => px(2.),
        };
        (outer_height - inset).max(px(0.))
    }

    /// Default px(12) to match panel px_3, See [`crate::dock::TabPanel`]
    fn inner_paddings(&self, size: Size, cx: &App) -> Edges<Pixels> {
        let controls = cx.theme().style.controls.for_size(size);
        let mut padding_x = controls.padding_x
            + match size {
                Size::Medium => px(2.),
                Size::Large => controls.gap,
                _ => px(0.),
            };

        if matches!(self, TabVariant::Underline) {
            padding_x = px(0.);
        }

        Edges {
            left: padding_x,
            right: padding_x,
            ..Default::default()
        }
    }

    fn inner_margins(&self, size: Size) -> Edges<Pixels> {
        match size {
            Size::XSmall => match self {
                TabVariant::Underline => Edges {
                    top: px(1.),
                    bottom: px(2.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            Size::Small => match self {
                TabVariant::Underline => Edges {
                    top: px(2.),
                    bottom: px(3.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            Size::Large => match self {
                TabVariant::Underline => Edges {
                    top: px(5.),
                    bottom: px(6.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            _ => match self {
                TabVariant::Underline => Edges {
                    top: px(3.),
                    bottom: px(4.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
        }
    }

    fn normal(&self, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().foreground,
                bg: cx.theme().transparent.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().transparent.into(),
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
        }
    }

    fn hovered(&self, selected: bool, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().secondary_foreground,
                bg: cx.theme().tokens.secondary_hover.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().secondary_foreground,
                bg: cx.theme().tokens.secondary.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: if selected {
                    cx.theme().tokens.background.into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().transparent.into(),
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
        }
    }

    fn selected(&self, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().tokens.tab_active.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().primary,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().primary,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().primary_foreground,
                bg: cx.theme().tokens.primary.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().tokens.background.into(),
                shadow: true,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                border_color: cx.theme().primary,
                ..Default::default()
            },
        }
    }

    fn disabled(&self, selected: bool, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent.into(),
                border_color: if selected {
                    cx.theme().border
                } else {
                    cx.theme().transparent
                },
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: if selected {
                    cx.theme().primary
                } else {
                    cx.theme().border
                },
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: if selected {
                    cx.theme().primary_foreground.opacity(0.5)
                } else {
                    cx.theme().muted_foreground
                },
                bg: if selected {
                    cx.theme().primary.opacity(0.5).into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().tokens.tab_bar.into(),
                inner_bg: if selected {
                    cx.theme().tokens.background.into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent.into(),
                border_color: if selected {
                    cx.theme().border
                } else {
                    cx.theme().transparent
                },
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    pub(super) fn tab_bar_radius(&self, size: Size, cx: &App) -> Pixels {
        if *self != TabVariant::Segmented {
            return px(0.);
        }

        match size {
            Size::XSmall | Size::Small => cx.theme().style.radii.md,
            Size::Large => cx.theme().style.radii.lg,
            _ => cx.theme().style.radii.lg,
        }
    }

    fn radius(&self, size: Size, cx: &App) -> Pixels {
        match self {
            TabVariant::Outline | TabVariant::Pill => px(99.),
            TabVariant::Segmented => match size {
                Size::XSmall | Size::Small => cx.theme().style.radii.md,
                Size::Large => cx.theme().style.radii.lg,
                _ => cx.theme().style.radii.lg,
            },
            _ => px(0.),
        }
    }

    pub(super) fn inner_radius(&self, size: Size, cx: &App) -> Pixels {
        match self {
            TabVariant::Segmented => match size {
                Size::Large => self.tab_bar_radius(size, cx) - px(3.),
                _ => self.tab_bar_radius(size, cx) - px(2.),
            },
            _ => px(0.),
        }
    }
}

#[allow(dead_code)]
struct TabStyle {
    borders: Edges<Pixels>,
    border_color: Hsla,
    bg: Background,
    fg: Hsla,
    shadow: bool,
    inner_bg: Background,
}

impl Default for TabStyle {
    fn default() -> Self {
        TabStyle {
            borders: Edges::all(px(0.)),
            border_color: gpui::transparent_white(),
            bg: gpui::transparent_white().into(),
            fg: gpui::transparent_white(),
            shadow: false,
            inner_bg: gpui::transparent_white().into(),
        }
    }
}

/// A Tab element for the [`super::TabBar`].
#[derive(IntoElement)]
pub struct Tab {
    ix: usize,
    base: Div,
    pub(super) state_id: Option<ElementId>,
    pub(super) label: Option<SharedString>,
    aria_label: Option<SharedString>,
    pub(super) icon: Option<Icon>,
    prefix: Option<AnyElement>,
    pub(super) tab_bar_prefix: Option<bool>,
    suffix: Option<AnyElement>,
    children: Vec<AnyElement>,
    variant: TabVariant,
    size: Size,
    pub(super) disabled: bool,
    pub(super) selected: bool,
    pub(super) focus_handle: Option<FocusHandle>,
    pub(super) tab_stop: bool,
    pub(super) on_key_down: Option<Rc<dyn Fn(&KeyDownEvent, &mut Window, &mut App) + 'static>>,
    pub(super) position_in_set: Option<usize>,
    pub(super) size_of_set: Option<usize>,
    pub(super) indicator_active: bool,
    pub(super) indicator_ready: bool,
    pub(super) on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl From<&'static str> for Tab {
    fn from(label: &'static str) -> Self {
        Self::new().label(label)
    }
}

impl From<String> for Tab {
    fn from(label: String) -> Self {
        Self::new().label(label)
    }
}

impl From<SharedString> for Tab {
    fn from(label: SharedString) -> Self {
        Self::new().label(label)
    }
}

impl From<Icon> for Tab {
    fn from(icon: Icon) -> Self {
        Self::default().icon(icon)
    }
}

impl From<IconName> for Tab {
    fn from(icon_name: IconName) -> Self {
        Self::default().icon(Icon::new(icon_name))
    }
}

impl Default for Tab {
    fn default() -> Self {
        Self {
            ix: 0,
            base: div(),
            state_id: None,
            label: None,
            aria_label: None,
            icon: None,
            tab_bar_prefix: None,
            children: Vec::new(),
            disabled: false,
            selected: false,
            focus_handle: None,
            tab_stop: false,
            on_key_down: None,
            position_in_set: None,
            size_of_set: None,
            indicator_active: false,
            indicator_ready: true,
            prefix: None,
            suffix: None,
            variant: TabVariant::default(),
            size: Size::default(),
            on_click: None,
        }
    }
}

impl Tab {
    /// Create a new tab with a label.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set label for the tab.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the accessible label for the tab.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    fn a11y_label(&self) -> Option<SharedString> {
        self.aria_label.clone().or_else(|| self.label.clone())
    }

    /// Set icon for the tab.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set Tab Variant.
    pub fn with_variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Use Pill variant.
    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    /// Use outline variant.
    pub fn outline(mut self) -> Self {
        self.variant = TabVariant::Outline;
        self
    }

    /// Use Segmented variant.
    pub fn segmented(mut self) -> Self {
        self.variant = TabVariant::Segmented;
        self
    }

    /// Use Underline variant.
    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
    }

    /// Set the left side of the tab
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set the right side of the tab
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set disabled state to the tab, default false.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the click handler for the tab.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Set index to the tab.
    pub(crate) fn ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self
    }

    /// Set if the tab bar has a prefix.
    pub(crate) fn tab_bar_prefix(mut self, tab_bar_prefix: bool) -> Self {
        self.tab_bar_prefix = Some(tab_bar_prefix);
        self
    }

    /// Injects the structural state identity owned by the parent TabBar.
    pub(super) fn state_id(mut self, id: ElementId) -> Self {
        self.state_id = Some(id);
        self
    }

    /// Injects the roving-focus state owned by the parent TabBar.
    pub(super) fn focus_handle(mut self, focus_handle: FocusHandle, tab_stop: bool) -> Self {
        self.focus_handle = Some(focus_handle);
        self.tab_stop = tab_stop;
        self
    }

    /// Injects horizontal TabBar keyboard navigation.
    pub(super) fn on_group_key_down(
        mut self,
        handler: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_key_down = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Tab {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Selectable for Tab {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl InteractiveElement for Tab {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Tab {}

impl Styled for Tab {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl Sizable for Tab {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Tab {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut tab_style = if self.selected {
            self.variant.selected(cx)
        } else {
            self.variant.normal(cx)
        };
        let mut hover_style = self.variant.hovered(self.selected, cx);
        if self.disabled {
            tab_style = self.variant.disabled(self.selected, cx);
            hover_style = self.variant.disabled(self.selected, cx);
        }
        let tab_bar_prefix = self.tab_bar_prefix.unwrap_or_default();
        if !tab_bar_prefix {
            if self.ix == 0 && self.variant == TabVariant::Tab {
                tab_style.borders.left = px(0.);
                hover_style.borders.left = px(0.);
            }
        }
        let radius = self.variant.radius(self.size, cx);
        let inner_radius = self.variant.inner_radius(self.size, cx);
        let inner_paddings = self.variant.inner_paddings(self.size, cx);
        let inner_margins = self.variant.inner_margins(self.size);
        let inner_height = self.variant.inner_height(self.size, cx);
        let height = self.variant.height(self.size, cx);
        let aria_label = self.a11y_label();
        let focus_handle = self.focus_handle.clone();
        let focus_visible = focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_focused(window))
            && window.last_input_was_keyboard();

        let segmented_indicator_active =
            self.variant == TabVariant::Segmented && self.indicator_active;
        let has_inline_inner_bg =
            self.selected && segmented_indicator_active && !self.indicator_ready;
        let inline_inner_bg = tab_style.inner_bg;
        let (inner_bg, hover_inner_bg) = if segmented_indicator_active && self.indicator_ready {
            (cx.theme().transparent.into(), cx.theme().transparent.into())
        } else if has_inline_inner_bg {
            (inline_inner_bg, inline_inner_bg)
        } else {
            (tab_style.inner_bg, hover_style.inner_bg)
        };
        let inner_shadow =
            tab_style.shadow && !segmented_indicator_active && cx.theme().style.elevation.enabled;

        // When a sliding indicator is active and ready, it alone represents the
        // selected state. Suppress the selected tab's own active background/border
        // so the two don't overlap during the switch animation (Segmented already
        // does this for its `inner_bg` above). Skip disabled tabs so a
        // disabled-selected tab keeps its dimmed styling instead of the
        // full-strength indicator color.
        let suppress_active_visual =
            self.selected && !self.disabled && self.indicator_active && self.indicator_ready;
        // Pill paints its active state via the outer `bg`.
        let outer_bg = if suppress_active_visual && self.variant == TabVariant::Pill {
            cx.theme().transparent.into()
        } else {
            tab_style.bg
        };
        // Underline paints its active state via the bottom `border_color`.
        let outer_border_color = if suppress_active_visual && self.variant == TabVariant::Underline
        {
            cx.theme().transparent
        } else {
            tab_style.border_color
        };

        // Pill foreground motion follows the same semantic timing as the
        // indicator and resumes from its sampled color after an interruption.
        let animate_fg = !self.disabled
            && self.variant == TabVariant::Pill
            && self.indicator_active
            && self.indicator_ready;
        let foreground_transition = if animate_fg {
            self.state_id.as_ref().and_then(|state_id| {
                let state = window.use_keyed_state(
                    tab_child_id(state_id, "foreground-state"),
                    cx,
                    |_, _| TabForegroundMotionState::new(tab_style.fg),
                );
                let duration = if cx.reduce_motion() {
                    Duration::ZERO
                } else {
                    cx.theme().style.motion.slow()
                };
                let easing = cx.theme().style.motion.move_easing;
                state.update(cx, |state, _| {
                    state.transition_to(tab_style.fg, Instant::now(), duration, easing)
                })
            })
        } else {
            None
        };

        let inner_content = h_flex()
            .flex_1()
            .h(inner_height)
            .line_height(relative(1.))
            .whitespace_nowrap()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .margins(inner_margins)
            .flex_shrink_0()
            .map(|this| match self.icon {
                Some(icon) => this
                    .w(inner_height * 1.25)
                    .child(icon.map(|this| match self.size {
                        Size::XSmall => this.size_2p5(),
                        Size::Small => this.size_3p5(),
                        Size::Large => this.size_4(),
                        _ => this.size_4(),
                    })),
                None => this
                    .paddings(inner_paddings)
                    .map(|this| match self.label {
                        Some(label) => this.child(label),
                        None => this,
                    })
                    .children(self.children),
            })
            .bg(inner_bg)
            .rounded(inner_radius)
            .when(inner_shadow, |this| this.shadow_xs())
            .hover(|this| this.bg(hover_inner_bg).rounded(inner_radius));

        let inner_element = if let (Some(state_id), Some(transition)) =
            (self.state_id.as_ref(), foreground_transition)
        {
            let easing = cx.theme().style.motion.move_easing;
            inner_content
                .with_animation(
                    tab_child_id(state_id, format!("foreground-{}", transition.epoch)),
                    Animation::new(transition.duration)
                        .with_easing(move |delta| easing.sample(delta)),
                    move |this, delta| {
                        this.text_color(Lerp::lerp(&transition.from, &transition.target, delta))
                    },
                )
                .into_any_element()
        } else {
            inner_content.into_any_element()
        };

        let tab = self
            .base
            .id(self.ix)
            .role(Role::Tab)
            .when_some(aria_label, |this, label| this.aria_label(label))
            .aria_selected(self.selected)
            .when_some(self.position_in_set, |this, position| {
                this.aria_position_in_set(position)
            })
            .when_some(self.size_of_set, |this, size| this.aria_size_of_set(size))
            .when_some(
                focus_handle.clone().filter(|_| !self.disabled),
                |this, handle| this.track_focus(&handle.tab_stop(self.tab_stop)),
            )
            .relative()
            .flex()
            .flex_wrap()
            .gap_1()
            .items_center()
            .flex_shrink_0()
            .h(height)
            .overflow_hidden()
            .text_color(tab_style.fg)
            .map(|this| match self.size {
                Size::XSmall => this.text_xs(),
                Size::Large => this.text_base(),
                _ => this.text_sm(),
            })
            .bg(outer_bg)
            .border_l(tab_style.borders.left)
            .border_r(tab_style.borders.right)
            .border_t(tab_style.borders.top)
            .border_b(tab_style.borders.bottom)
            .border_color(outer_border_color)
            .rounded(radius)
            .focus_ring(focus_visible, px(0.), window, cx)
            .hover(|this| {
                // Always register the hover style: GPUI only refreshes the cached
                // hover state while one is present. If the selected tab skipped it,
                // the stale state would keep hover colors after unselecting.
                if self.selected || self.disabled {
                    return this;
                }
                this.text_color(hover_style.fg)
                    .bg(hover_style.bg)
                    .border_l(hover_style.borders.left)
                    .border_r(hover_style.borders.right)
                    .border_t(hover_style.borders.top)
                    .border_b(hover_style.borders.bottom)
                    .border_color(hover_style.border_color)
                    .rounded(radius)
            })
            .when(has_inline_inner_bg, |this| {
                this.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top_0()
                        .bottom_0()
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .w_full()
                                .h(inner_height)
                                .bg(inline_inner_bg)
                                .rounded(inner_radius)
                                .when(
                                    tab_style.shadow && cx.theme().style.elevation.enabled,
                                    |this| this.shadow_sm(),
                                ),
                        ),
                )
            })
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(inner_element)
            .when_some(self.suffix, |this, suffix| {
                this.child(
                    div()
                        .id("suffix")
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .child(suffix),
                )
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                // Stop propagation behavior, for works on TitleBar.
                // https://github.com/longbridge/gpui-component/issues/1836
                cx.stop_propagation();
            })
            .when(!self.disabled, |this| {
                this.when_some(self.on_key_down.clone(), |this, handler| {
                    this.on_key_down(move |event, window, cx| handler(event, window, cx))
                })
                .when_some(self.on_click.clone(), |this, on_click| {
                    let focus_handle = focus_handle.clone();
                    this.on_click(move |event, window, cx| {
                        if let Some(focus_handle) = focus_handle.as_ref() {
                            focus_handle.focus(window, cx);
                        }
                        on_click(event, window, cx);
                    })
                })
            });

        crate::accessibility::accessibility_state(tab, false, false, self.disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn a11y_label_defaults_to_visible_label(_cx: &mut gpui::TestAppContext) {
        let tab = Tab::new().label("Account");

        assert_eq!(tab.a11y_label(), Some("Account".into()));
    }

    #[gpui::test]
    fn explicit_a11y_label_overrides_visible_label(_cx: &mut gpui::TestAppContext) {
        let tab = Tab::new().label("Acct").aria_label("Account settings");

        assert_eq!(tab.a11y_label(), Some("Account settings".into()));
    }

    #[test]
    fn pill_foreground_reverses_from_the_sampled_color() {
        let off = Hsla::black();
        let on = Hsla::white();
        let now = Instant::now();
        let duration = Duration::from_millis(160);
        let mut state = TabForegroundMotionState::new(off);

        state
            .transition_to(on, now, duration, MotionEasing::Linear)
            .expect("forward transition");
        let reverse = state
            .transition_to(
                off,
                now + Duration::from_millis(80),
                duration,
                MotionEasing::Linear,
            )
            .expect("reverse transition");

        assert_ne!(reverse.from, on);
        assert_ne!(reverse.from, off);
        assert_eq!(reverse.target, off);
    }

    #[test]
    fn internal_ids_preserve_structural_identity() {
        let structured = ElementId::NamedInteger("tab".into(), 1);
        let textual = ElementId::Name("tab-1".into());
        assert_eq!(structured.to_string(), textual.to_string());
        assert_ne!(
            tab_child_id(&structured, "indicator"),
            tab_child_id(&textual, "indicator")
        );
    }
}
