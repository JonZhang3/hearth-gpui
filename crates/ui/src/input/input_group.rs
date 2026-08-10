use std::time::Instant;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    Animation, AnimationExt as _, AnyElement, App, ElementId, Entity, Focusable as _, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, Role,
    SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div, px,
};

use crate::animation::Lerp;
use crate::button::{Button, ButtonVariant, ButtonVariants};
use crate::{
    ActiveTheme as _, Density, Sizable as _, Size, StylePreset, StyledExt as _, accessibility,
    h_flex,
};

use super::input::{
    InputMotionKind, InputMotionState, InputPaintState, input_child_id, input_focus_visible,
    input_metrics, input_motion_timing, input_uses_semantic_color_motion,
};
use super::{Input, InputState};

/// Logical placement of content surrounding an InputGroup control.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputGroupAddonAlign {
    #[default]
    InlineStart,
    InlineEnd,
    BlockStart,
    BlockEnd,
}

/// Sizes supported by the compact Button adapter used inside InputGroup addons.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputGroupButtonSize {
    #[default]
    Xs,
    Sm,
    IconXs,
    IconSm,
}

/// InputGroup-specific geometry derived from semantic StylePreset properties.
#[derive(Debug, Clone, Copy, PartialEq)]
struct InputGroupMetrics {
    radius: Pixels,
    block_radius: Pixels,
    inline_padding: Pixels,
    block_padding: Pixels,
    block_edge_padding: Pixels,
    addon_padding_y: Pixels,
    addon_gap: Pixels,
    button_radius: Pixels,
    compact_disabled_opacity: bool,
}

impl InputGroupMetrics {
    /// Resolves Vega, Nova, and Maia intent without branching on preset identity.
    fn resolve(style: &StylePreset) -> Self {
        match style.density {
            Density::Standard => Self {
                radius: style.radii.md,
                block_radius: style.radii.md,
                inline_padding: style.controls.md.icon_edge_padding,
                block_padding: style.controls.md.padding_x,
                block_edge_padding: px(8.),
                addon_padding_y: px(6.),
                addon_gap: px(8.),
                button_radius: (style.radii.md - px(5.)).max(px(0.)),
                compact_disabled_opacity: false,
            },
            Density::Compact => Self {
                radius: style.radii.lg,
                block_radius: style.radii.lg,
                inline_padding: style.controls.md.icon_edge_padding,
                block_padding: style.controls.md.padding_x,
                block_edge_padding: px(8.),
                addon_padding_y: px(6.),
                addon_gap: px(8.),
                button_radius: (style.radii.lg - px(3.)).max(px(0.)),
                compact_disabled_opacity: true,
            },
            Density::Comfortable => Self {
                radius: style.radii.xl,
                block_radius: style.radii.lg,
                inline_padding: style.controls.md.padding_x,
                block_padding: style.controls.md.padding_x,
                block_edge_padding: px(12.),
                addon_padding_y: px(8.),
                addon_gap: px(8.),
                button_radius: style.radii.xl,
                compact_disabled_opacity: false,
            },
        }
    }
}

/// Content positioned before, after, above, or below an InputGroup control.
#[derive(IntoElement)]
pub struct InputGroupAddon {
    style: StyleRefinement,
    align: InputGroupAddonAlign,
    children: Vec<AnyElement>,
}

impl InputGroupAddon {
    /// Creates an empty addon at the logical inline start.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            align: InputGroupAddonAlign::InlineStart,
            children: Vec::new(),
        }
    }

    /// Positions this addon relative to the InputGroup control.
    pub fn align(mut self, align: InputGroupAddonAlign) -> Self {
        self.align = align;
        self
    }

    fn render_for(
        self,
        state: Option<Entity<InputState>>,
        disabled: bool,
        metrics: InputGroupMetrics,
        foreground: Hsla,
    ) -> AnyElement {
        let align = self.align;
        let focus_state = state.clone();

        div()
            .flex()
            .items_center()
            .justify_center()
            .gap(metrics.addon_gap)
            .py(metrics.addon_padding_y)
            .text_sm()
            .font_medium()
            .text_color(foreground)
            .when(disabled, |this| this.opacity(0.5))
            .when(align == InputGroupAddonAlign::InlineStart, |this| {
                this.pl(metrics.inline_padding)
            })
            .when(align == InputGroupAddonAlign::InlineEnd, |this| {
                this.pr(metrics.inline_padding)
            })
            .when(align == InputGroupAddonAlign::BlockStart, |this| {
                this.w_full()
                    .justify_start()
                    .px(metrics.block_padding)
                    .pt(metrics.block_edge_padding)
            })
            .when(align == InputGroupAddonAlign::BlockEnd, |this| {
                this.w_full()
                    .justify_start()
                    .px(metrics.block_padding)
                    .pb(metrics.block_edge_padding)
            })
            .refine_style(&self.style)
            .when_some(focus_state.filter(|_| !disabled), |this, state| {
                this.cursor_text()
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        state.update(cx, |state, cx| state.focus(window, cx));
                    })
            })
            .children(self.children)
            .into_any_element()
    }
}

impl Default for InputGroupAddon {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for InputGroupAddon {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for InputGroupAddon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for InputGroupAddon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = InputGroupMetrics::resolve(&cx.theme().style);
        self.render_for(None, false, metrics, cx.theme().muted_foreground)
    }
}

/// Muted helper content rendered inside an InputGroupAddon.
#[derive(Default, IntoElement)]
pub struct InputGroupText {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl InputGroupText {
    /// Creates empty helper content that can be composed with `child`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for InputGroupText {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for InputGroupText {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for InputGroupText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .gap_2()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// Applies shadcn InputGroup button geometry to an existing Button.
#[derive(IntoElement)]
pub struct InputGroupButton {
    button: Button,
    style: StyleRefinement,
    size: InputGroupButtonSize,
    variant: ButtonVariant,
}

impl InputGroupButton {
    /// Wraps a fully configured Button while defaulting its visual variant to Ghost.
    pub fn new(button: Button) -> Self {
        Self {
            button,
            style: StyleRefinement::default(),
            size: InputGroupButtonSize::Xs,
            variant: ButtonVariant::Ghost,
        }
    }

    /// Sets the compact geometry used inside the addon.
    pub fn size(mut self, size: InputGroupButtonSize) -> Self {
        self.size = size;
        self
    }
}

impl ButtonVariants for InputGroupButton {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Styled for InputGroupButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for InputGroupButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = InputGroupMetrics::resolve(&cx.theme().style);
        let (size, icon_size, icon_only) = match self.size {
            InputGroupButtonSize::Xs => (Size::XSmall, px(14.), false),
            InputGroupButtonSize::Sm => (Size::Small, px(16.), false),
            InputGroupButtonSize::IconXs => (Size::XSmall, px(14.), true),
            InputGroupButtonSize::IconSm => (Size::Small, px(16.), true),
        };
        let button = self
            .button
            .with_size(size)
            .with_variant(self.variant)
            .icon_size(icon_size)
            .rounded(metrics.button_radius)
            .shadow_none()
            .when(size == Size::XSmall && !icon_only, |this| {
                this.px(px(6.)).gap_1()
            })
            .when(icon_only, |this| this.p_0())
            .refine_style(&self.style);

        // The wrapper stops addon background focus behavior after the Button handles the event.
        div()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(button)
    }
}

/// A unified input surface containing one typed Input and any number of addons.
#[derive(IntoElement)]
pub struct InputGroup {
    id: ElementId,
    style: StyleRefinement,
    input: Option<Input>,
    state: Option<Entity<InputState>>,
    addons: Vec<InputGroupAddon>,
    disabled: bool,
    invalid: bool,
    aria_label: Option<SharedString>,
}

impl InputGroup {
    /// Creates an empty typed InputGroup with stable element identity.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            input: None,
            state: None,
            addons: Vec::new(),
            disabled: false,
            invalid: false,
            aria_label: None,
        }
    }

    /// Sets the single Input control. A later call replaces the previous control.
    pub fn input(mut self, input: Input) -> Self {
        self.state = Some(input.state().clone());
        self.disabled = input.is_disabled();
        self.invalid = input.is_invalid();
        self.input = Some(input);
        self
    }

    /// Adds one typed addon while preserving insertion order within its alignment slot.
    pub fn addon(mut self, addon: InputGroupAddon) -> Self {
        self.addons.push(addon);
        self
    }

    /// Adds multiple typed addons.
    pub fn addons(mut self, addons: impl IntoIterator<Item = InputGroupAddon>) -> Self {
        self.addons.extend(addons);
        self
    }

    /// Sets whether the group and its Input are unavailable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the shared invalid presentation and accessibility state.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets the accessible name announced for the group.
    pub fn aria_label(mut self, aria_label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }
}

impl Styled for InputGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for InputGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = InputGroupMetrics::resolve(&cx.theme().style);
        let input_metrics = input_metrics(&cx.theme().style);
        let control_metrics = cx.theme().style.controls.md;
        let state = self.state.clone();
        let focused = state
            .as_ref()
            .is_some_and(|state| state.focus_handle(cx).is_focused(window))
            && !self.disabled;
        let focus_visible = input_focus_visible(focused);
        let multi_line = state
            .as_ref()
            .is_some_and(|state| state.read(cx).is_multi_line());
        let invalid_border = cx
            .theme()
            .danger
            .opacity(if cx.theme().is_dark() { 0.5 } else { 1. });
        let border = if self.invalid {
            invalid_border
        } else if focus_visible {
            cx.theme().ring
        } else {
            cx.theme().input
        };
        let ring_visible = self.invalid || focus_visible;
        let ring_color = if self.invalid {
            cx.theme()
                .danger
                .opacity(if cx.theme().is_dark() { 0.4 } else { 0.2 })
        } else {
            cx.theme().ring.opacity(0.5)
        };
        let paint = InputPaintState {
            background: Input::surface_background(input_metrics, self.disabled, cx),
            border,
            ring: if ring_visible {
                ring_color
            } else {
                ring_color.opacity(0.)
            },
        };
        let uses_semantic_color_motion = input_uses_semantic_color_motion(&self.style);

        let mut inline_start = Vec::new();
        let mut inline_end = Vec::new();
        let mut block_start = Vec::new();
        let mut block_end = Vec::new();
        for addon in self.addons {
            let align = addon.align;
            let rendered = addon.render_for(
                state.clone(),
                self.disabled,
                metrics,
                cx.theme().muted_foreground,
            );
            match align {
                InputGroupAddonAlign::InlineStart => inline_start.push(rendered),
                InputGroupAddonAlign::InlineEnd => inline_end.push(rendered),
                InputGroupAddonAlign::BlockStart => block_start.push(rendered),
                InputGroupAddonAlign::BlockEnd => block_end.push(rendered),
            }
        }

        let root_id = self.id;
        let motion_state =
            window.use_keyed_state(input_child_id(&root_id, "motion-state"), cx, |_, _| {
                InputMotionState::new(paint)
            });
        let (motion_duration, easing) = input_motion_timing(ring_visible, cx);
        let transition = motion_state.update(cx, |motion, _| {
            motion.transition_to(
                paint,
                Instant::now(),
                motion_duration,
                easing,
                InputMotionKind::ColorsAndShadow,
            )
        });

        let has_block = !block_start.is_empty() || !block_end.is_empty();
        let group_radius = if has_block || multi_line {
            metrics.block_radius
        } else {
            metrics.radius
        };
        let mut element = div()
            .id(root_id.clone())
            .role(Role::Group)
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .relative()
            .w_full()
            .min_w_0()
            .flex()
            .items_center()
            .when(has_block, |this| this.flex_col())
            .when(!has_block && !multi_line, |this| {
                this.h(control_metrics.height)
            })
            .rounded(group_radius)
            .border_1()
            .border_color(if uses_semantic_color_motion {
                cx.theme().transparent
            } else {
                paint.border
            })
            .bg(if uses_semantic_color_motion {
                cx.theme().transparent
            } else {
                paint.background
            })
            .when(input_metrics.shadow, |this| this.shadow_xs())
            .when(self.disabled && metrics.compact_disabled_opacity, |this| {
                this.opacity(0.5)
            })
            .refine_style(&self.style);

        if uses_semantic_color_motion {
            let mut surface_style = StyleRefinement::default();
            surface_style.corner_radii = element.style().corner_radii.clone();
            surface_style.border_widths = element.style().border_widths.clone();
            let surface = div()
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .left_0()
                .bg(paint.background)
                .border_color(paint.border)
                .refine_style(&surface_style)
                .into_any_element();
            let surface = if let Some(transition) = transition.filter(|transition| {
                transition.from.background != transition.to.background
                    || transition.from.border != transition.to.border
            }) {
                let from = transition.from;
                let to = transition.to;
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .left_0()
                    .bg(from.background)
                    .border_color(from.border)
                    .refine_style(&surface_style)
                    .with_animation(
                        input_child_id(&root_id, format!("surface-{}", transition.epoch)),
                        Animation::new(transition.duration)
                            .with_easing(move |delta| easing.sample(delta)),
                        move |this, delta| {
                            this.bg(Lerp::lerp(&from.background, &to.background, delta))
                                .border_color(Lerp::lerp(&from.border, &to.border, delta))
                        },
                    )
                    .into_any_element()
            } else {
                surface
            };
            element = element.child(surface);
        }

        let ring_transition =
            transition.filter(|transition| transition.from.ring != transition.to.ring);
        if ring_visible || ring_transition.is_some() {
            let ring_width = cx.theme().style.focus.ring_width;
            let ring_outset = ring_width + cx.theme().style.focus.ring_offset;
            let ring_style = Input::outer_ring_geometry(element.style(), ring_outset, window);
            let ring = div()
                .absolute()
                .top(-ring_outset)
                .right(-ring_outset)
                .bottom(-ring_outset)
                .left(-ring_outset)
                .border(ring_width)
                .border_color(paint.ring)
                .refine_style(&ring_style);
            let ring = if let Some(transition) = ring_transition {
                let from = transition.from;
                let to = transition.to;
                ring.with_animation(
                    input_child_id(&root_id, format!("ring-{}", transition.epoch)),
                    Animation::new(transition.duration)
                        .with_easing(move |delta| easing.sample(delta)),
                    move |this, delta| this.border_color(Lerp::lerp(&from.ring, &to.ring, delta)),
                )
                .into_any_element()
            } else {
                ring.into_any_element()
            };
            element = element.child(ring);
        }

        let input = self.input.map(|input| {
            input
                .appearance(false)
                .bordered(false)
                .focus_bordered(false)
                .disabled(self.disabled)
                .invalid(self.invalid)
                .flex_1()
                .min_w_0()
                .when(!inline_start.is_empty(), |this| this.pl(px(6.)))
                .when(!inline_end.is_empty(), |this| this.pr(px(6.)))
                .when(!block_end.is_empty(), |this| this.pt(px(12.)))
                .when(!block_start.is_empty(), |this| this.pb(px(12.)))
                .into_any_element()
        });
        let control_row = h_flex()
            .w_full()
            .min_w_0()
            .items_center()
            .children(inline_start)
            .children(input)
            .children(inline_end);
        element = element
            .children(block_start)
            .child(control_row)
            .children(block_end);

        accessibility::accessibility_state(element, self.invalid, false, self.disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, Context, Render, TestAppContext};

    struct EmptyView;

    impl Render for EmptyView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    #[gpui::test]
    fn input_group_builder_preserves_typed_slots(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let _ = cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| InputState::new(window, cx).multi_line(true));
            let group = InputGroup::new("builder")
                .input(Input::new(&state).disabled(true).invalid(true))
                .addon(
                    InputGroupAddon::new()
                        .align(InputGroupAddonAlign::BlockStart)
                        .child(InputGroupText::new().child("Label")),
                )
                .addon(
                    InputGroupAddon::new()
                        .align(InputGroupAddonAlign::InlineEnd)
                        .child("Suffix"),
                )
                .aria_label("Account input");

            assert!(group.input.is_some());
            assert_eq!(group.state.as_ref(), Some(&state));
            assert_eq!(group.addons.len(), 2);
            assert!(group.disabled);
            assert!(group.invalid);
            assert_eq!(group.aria_label.as_deref(), Some("Account input"));
            assert!(state.read(cx).is_multi_line());
            EmptyView
        });
    }

    #[test]
    fn addon_and_button_variants_use_shadcn_defaults() {
        let addon = InputGroupAddon::new();
        assert_eq!(addon.align, InputGroupAddonAlign::InlineStart);

        let button = InputGroupButton::new(Button::new("action"));
        assert_eq!(button.size, InputGroupButtonSize::Xs);
        assert_eq!(button.variant, ButtonVariant::Ghost);
    }

    #[test]
    fn input_group_metrics_match_builtin_shadcn_presets() {
        let vega = InputGroupMetrics::resolve(&StylePreset::vega());
        assert_eq!(vega.radius, px(8.));
        assert_eq!(vega.block_radius, px(8.));
        assert_eq!(vega.inline_padding, px(8.));
        assert_eq!(vega.block_padding, px(10.));
        assert_eq!(vega.button_radius, px(3.));
        assert!(!vega.compact_disabled_opacity);

        let nova = InputGroupMetrics::resolve(&StylePreset::nova());
        assert_eq!(nova.radius, px(8.));
        assert_eq!(nova.block_radius, px(8.));
        assert_eq!(nova.button_radius, px(5.));
        assert!(nova.compact_disabled_opacity);

        let maia = InputGroupMetrics::resolve(&StylePreset::maia());
        assert_eq!(maia.radius, px(18.));
        assert_eq!(maia.block_radius, px(14.));
        assert_eq!(maia.inline_padding, px(12.));
        assert_eq!(maia.block_padding, px(12.));
        assert_eq!(maia.block_edge_padding, px(12.));
        assert_eq!(maia.button_radius, px(18.));
    }
}
