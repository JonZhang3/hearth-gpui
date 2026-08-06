use std::rc::Rc;

use crate::{
    ActiveTheme, Disableable, FocusableExt as _, Selectable, Sizable, Size, StyleSized, StyledExt,
    button::ButtonIcon,
    h_flex,
    tooltip::{ManagedTooltipExt as _, Tooltip},
};
use gpui::{
    AnyElement, App, Background, ClickEvent, Corners, Div, Edges, ElementId, Hsla,
    InteractiveElement, Interactivity, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce,
    Role, SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Toggled, Window, div, prelude::FluentBuilder as _, px, relative,
};

#[derive(Default, Clone, Copy, PartialEq)]
enum ButtonRounding {
    #[default]
    Preset,
    Full,
    Custom(Pixels),
}

pub trait ButtonVariants: Sized {
    fn with_variant(self, variant: ButtonVariant) -> Self;

    /// Uses the bordered outline style.
    fn outline(self) -> Self {
        self.with_variant(ButtonVariant::Outline)
    }

    /// With the secondary style for the Button.
    fn secondary(self) -> Self {
        self.with_variant(ButtonVariant::Secondary)
    }

    /// Uses the destructive action style.
    fn destructive(self) -> Self {
        self.with_variant(ButtonVariant::Destructive)
    }

    /// With the ghost style for the Button.
    fn ghost(self) -> Self {
        self.with_variant(ButtonVariant::Ghost)
    }

    /// With the link style for the Button.
    fn link(self) -> Self {
        self.with_variant(ButtonVariant::Link)
    }
}

/// The variant of the Button.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ButtonVariant {
    #[default]
    Default,
    Outline,
    Secondary,
    Ghost,
    Destructive,
    Link,
}

impl ButtonVariant {
    #[inline]
    pub fn is_link(&self) -> bool {
        matches!(self, Self::Link)
    }

    #[inline]
    pub fn is_ghost(&self) -> bool {
        matches!(self, Self::Ghost)
    }
}

/// A Button element.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    icon: Option<ButtonIcon>,
    trailing_icon: Option<ButtonIcon>,
    label: Option<SharedString>,
    aria_label: Option<SharedString>,
    children: Vec<AnyElement>,
    disabled: bool,
    pub(crate) active: bool,
    pressed: Option<bool>,
    variant: ButtonVariant,
    rounding: ButtonRounding,
    border_corners: Corners<bool>,
    border_edges: Edges<bool>,
    size: Size,
    tooltip: Option<(
        SharedString,
        Option<(Rc<Box<dyn gpui::Action>>, Option<SharedString>)>,
    )>,
    tooltip_builder: Option<Rc<dyn Fn(&mut Window, &mut App) -> gpui::AnyView>>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    on_hover: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    tab_index: isize,
    tab_stop: bool,
}

impl From<Button> for AnyElement {
    fn from(button: Button) -> Self {
        button.into_any_element()
    }
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();

        Self {
            id: id.clone(),
            // ID must be set after div is created;
            // `dropdown_menu` uses this id to create the popup menu.
            base: div().flex_shrink_0().id(id),
            style: StyleRefinement::default(),
            icon: None,
            trailing_icon: None,
            label: None,
            aria_label: None,
            disabled: false,
            active: false,
            pressed: None,
            variant: ButtonVariant::default(),
            rounding: ButtonRounding::Preset,
            border_corners: Corners {
                top_left: true,
                top_right: true,
                bottom_right: true,
                bottom_left: true,
            },
            border_edges: Edges::all(true),
            size: Size::Medium,
            tooltip: None,
            tooltip_builder: None,
            on_click: None,
            on_hover: None,
            children: Vec::new(),
            tab_index: 0,
            tab_stop: true,
        }
    }

    /// Uses the bordered outline style.
    pub fn outline(self) -> Self {
        self.with_variant(ButtonVariant::Outline)
    }

    /// Uses the secondary style.
    pub fn secondary(self) -> Self {
        self.with_variant(ButtonVariant::Secondary)
    }

    /// Uses the destructive action style.
    pub fn destructive(self) -> Self {
        self.with_variant(ButtonVariant::Destructive)
    }

    /// Uses the low-emphasis ghost style.
    pub fn ghost(self) -> Self {
        self.with_variant(ButtonVariant::Ghost)
    }

    /// Uses the inline link style.
    pub fn link(self) -> Self {
        self.with_variant(ButtonVariant::Link)
    }

    /// Overrides the Style Preset radius with an explicit radius.
    pub fn rounded(mut self, radius: Pixels) -> Self {
        self.rounding = ButtonRounding::Custom(radius);
        self
    }

    /// Uses a pill radius resolved from the final control height.
    pub fn rounded_full(mut self) -> Self {
        self.rounding = ButtonRounding::Full;
        self
    }

    /// Set the border corners side of the Button.
    pub(crate) fn border_corners(mut self, corners: impl Into<Corners<bool>>) -> Self {
        self.border_corners = corners.into();
        self
    }

    /// Set the border edges of the Button.
    pub(crate) fn border_edges(mut self, edges: impl Into<Edges<bool>>) -> Self {
        self.border_edges = edges.into();
        self
    }

    /// Set label to the Button, if no label is set, the button will be in Icon Button mode.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the accessibility label without adding visible button text.
    ///
    /// This is required for icon-only buttons whose icon has no accessible
    /// name of its own.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set the icon of the button, if the Button have no label, the button well in Icon Button mode.
    pub fn icon(mut self, icon: impl Into<ButtonIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets an icon rendered after the label and custom content.
    pub fn trailing_icon(mut self, icon: impl Into<ButtonIcon>) -> Self {
        self.trailing_icon = Some(icon.into());
        self
    }

    /// Sets an explicit toggle-button state and exposes it as `aria-pressed`.
    ///
    /// Use this only for actions whose pressed state persists after activation.
    /// Transient Popover and menu active states should continue using
    /// [`Selectable`] without toggle semantics.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self.active = pressed;
        self
    }

    /// Set the tooltip of the button.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some((tooltip.into(), None));
        self
    }

    /// Set the tooltip of the button with action to show keybinding.
    pub fn tooltip_with_action(
        mut self,
        tooltip: impl Into<SharedString>,
        action: &dyn gpui::Action,
        context: Option<&str>,
    ) -> Self {
        self.tooltip = Some((
            tooltip.into(),
            Some((
                Rc::new(action.boxed_clone()),
                context.map(|c| c.to_string().into()),
            )),
        ));
        self
    }

    /// Add click handler.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Add hover handler, the bool parameter indicates whether the mouse is hovering.
    pub fn on_hover(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Rc::new(handler));
        self
    }

    /// Set the tab index of the button, it will be used to focus the button by tab key.
    ///
    /// Default is 0.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Set the tab stop of the button, if true, the button will be focusable by tab key.
    ///
    /// Default is true.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    #[inline]
    fn clickable(&self) -> bool {
        !self.disabled && self.on_click.is_some()
    }

    #[inline]
    fn hoverable(&self) -> bool {
        !self.disabled && self.on_hover.is_some()
    }
}

impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

// Popover currently uses `Selectable` to communicate its open visual state to
// a trigger. For Button this maps only to the transient active appearance and
// does not add toggle semantics or `aria-pressed`.
impl Selectable for Button {
    fn selected(mut self, selected: bool) -> Self {
        self.active = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.active
    }
}

impl Sizable for Button {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ButtonVariants for Button {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl InteractiveElement for Button {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style: ButtonVariant = self.variant;
        let clickable = self.clickable();
        let is_disabled = self.disabled;
        let hoverable = self.hoverable();
        let normal_style = style.normal(cx);
        let control_metrics = cx.theme().style.controls.for_size(self.size);
        let icon_size = Size::Size(control_metrics.icon_size);
        let is_icon_only = self.label.is_none()
            && self.children.is_empty()
            && (self.icon.is_some() ^ self.trailing_icon.is_some());
        let accessibility_label = self
            .aria_label
            .clone()
            .or_else(|| self.label.clone())
            .or_else(|| self.tooltip.as_ref().map(|(text, _)| text.clone()));

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);

        let rounding = match self.rounding {
            ButtonRounding::Preset => cx.theme().style.radii.md,
            ButtonRounding::Full => control_metrics.height / 2.,
            ButtonRounding::Custom(px) => px,
        };

        let element = self
            .base
            .role(Role::Button)
            .when_some(self.pressed, |this, pressed| {
                this.aria_toggled(if pressed {
                    Toggled::True
                } else {
                    Toggled::False
                })
            })
            .when_some(accessibility_label, |this, label| this.aria_label(label))
            .when(!self.disabled, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_index(self.tab_index)
                        .tab_stop(self.tab_stop),
                )
            })
            .cursor_default()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .font_medium()
            .whitespace_nowrap()
            .cursor_default()
            .when(
                cx.theme().style.elevation.enabled && normal_style.shadow,
                |this| this.shadow_xs(),
            )
            .when(is_icon_only, |this| this.size(control_metrics.height))
            .when(!is_icon_only, |this| {
                this.h(control_metrics.height)
                    .min_w(control_metrics.height)
                    .pl(if self.icon.is_some() {
                        control_metrics.icon_edge_padding
                    } else {
                        control_metrics.padding_x
                    })
                    .pr(if self.trailing_icon.is_some() {
                        control_metrics.icon_edge_padding
                    } else {
                        control_metrics.padding_x
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
            // Keep a transparent border on every variant so focus and variant
            // changes never alter the control's measured geometry.
            .when(self.border_edges.left, |this| this.border_l_1())
            .when(self.border_edges.right, |this| this.border_r_1())
            .when(self.border_edges.top, |this| this.border_t_1())
            .when(self.border_edges.bottom, |this| this.border_b_1())
            .text_color(normal_style.fg)
            .when(self.active, |this| {
                let active_style = style.active(cx);
                this.bg(active_style.bg)
                    .border_color(active_style.border)
                    .text_color(active_style.fg)
            })
            .when(!self.disabled && !self.active, |this| {
                this.border_color(normal_style.border)
                    .bg(normal_style.bg)
                    .when(normal_style.underline, |this| this.text_decoration_1())
                    .hover(|this| {
                        let hover_style = style.hovered(cx);
                        let this = this
                            .bg(hover_style.bg)
                            .border_color(hover_style.border)
                            .text_color(hover_style.fg);
                        if hover_style.underline {
                            this.text_decoration_1()
                        } else {
                            this
                        }
                    })
                    .active(|this| {
                        let active_style = style.active(cx);
                        this.bg(active_style.bg)
                            .border_color(active_style.border)
                            .text_color(active_style.fg)
                            .relative()
                            .top(px(1.))
                    })
            })
            .when(self.disabled, |this| {
                let disabled_style = style.disabled(cx);
                this.bg(disabled_style.bg)
                    .text_color(disabled_style.fg)
                    .border_color(disabled_style.border)
                    .opacity(0.5)
                    .shadow_none()
            })
            .refine_style(&self.style)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                // Stop handle any click event when disabled.
                // To avoid handle dropdown menu open when button is disabled.
                if is_disabled {
                    cx.stop_propagation();
                    return;
                }

                // Avoid focus on mouse down.
                window.prevent_default();

                // Pressing a button must not start the window-level text selection.
                crate::global_state::GlobalState::suppress_text_selection(cx);
            })
            .when_some(self.on_click, |this, on_click| {
                this.on_click(move |event, window, cx| {
                    // Stop handle any click event when disabled.
                    // To avoid handle dropdown menu open when button is disabled.
                    if !clickable {
                        cx.stop_propagation();
                        return;
                    }

                    on_click(event, window, cx);
                })
            })
            .when_some(self.on_hover.filter(|_| hoverable), |this, on_hover| {
                this.on_hover(move |hovered, window, cx| {
                    on_hover(hovered, window, cx);
                })
            })
            .child({
                h_flex()
                    .id("label")
                    .size_full()
                    .items_center()
                    .justify_center()
                    .button_text_size(self.size)
                    .gap(control_metrics.gap)
                    .when_some(self.icon, |this, icon| {
                        this.child(icon.with_size(icon_size))
                    })
                    .when_some(self.label, |this, label| {
                        this.child(div().flex_none().line_height(relative(1.)).child(label))
                    })
                    .children(self.children)
                    .when_some(self.trailing_icon, |this, icon| {
                        this.child(icon.with_size(icon_size))
                    })
            })
            .map(|this| {
                if let Some(builder) = self.tooltip_builder {
                    this.managed_tooltip(move |window, cx| builder(window, cx))
                } else if let Some((tooltip, action)) = self.tooltip {
                    this.managed_tooltip(move |window, cx| {
                        Tooltip::new(tooltip.clone())
                            .when_some(action.clone(), |this, (action, context)| {
                                this.action(
                                    action.boxed_clone().as_ref(),
                                    context.as_ref().map(|c| c.as_ref()),
                                )
                            })
                            .build(window, cx)
                    })
                } else {
                    this
                }
            })
            .focus_ring(is_focused, px(0.), window, cx);

        crate::accessibility::accessibility_state(element, false, false, self.disabled)
    }
}

struct ButtonVariantStyle {
    bg: Background,
    border: Hsla,
    fg: Hsla,
    underline: bool,
    shadow: bool,
}

impl ButtonVariant {
    /// Resolves the default Vega-compatible appearance from semantic colors.
    fn normal(&self, cx: &mut App) -> ButtonVariantStyle {
        match self {
            Self::Default => ButtonVariantStyle::new(
                cx.theme().tokens.button_primary.into(),
                cx.theme().transparent,
                cx.theme().button_primary_foreground,
            ),
            Self::Outline => ButtonVariantStyle::new(
                cx.theme().input_background().into(),
                if cx.theme().mode.is_dark() {
                    cx.theme().input
                } else {
                    cx.theme().border
                },
                cx.theme().foreground,
            )
            .shadow(),
            Self::Secondary => ButtonVariantStyle::new(
                cx.theme().tokens.button_secondary.into(),
                cx.theme().transparent,
                cx.theme().button_secondary_foreground,
            ),
            Self::Ghost => ButtonVariantStyle::new(
                cx.theme().transparent.into(),
                cx.theme().transparent,
                cx.theme().foreground,
            ),
            Self::Destructive => ButtonVariantStyle::new(
                cx.theme()
                    .danger
                    .opacity(if cx.theme().mode.is_dark() { 0.2 } else { 0.1 })
                    .into(),
                cx.theme().transparent,
                cx.theme().danger,
            ),
            Self::Link => ButtonVariantStyle::new(
                cx.theme().transparent.into(),
                cx.theme().transparent,
                cx.theme().link,
            ),
        }
    }

    /// Resolves the hover appearance without changing control geometry.
    fn hovered(&self, cx: &mut App) -> ButtonVariantStyle {
        match self {
            Self::Default => ButtonVariantStyle::new(
                cx.theme().tokens.button_primary_hover.into(),
                cx.theme().transparent,
                cx.theme().button_primary_foreground,
            ),
            Self::Outline => ButtonVariantStyle::new(
                cx.theme().tokens.button_hover.into(),
                if cx.theme().mode.is_dark() {
                    cx.theme().input
                } else {
                    cx.theme().border
                },
                cx.theme().foreground,
            )
            .shadow(),
            Self::Secondary => ButtonVariantStyle::new(
                cx.theme().tokens.button_secondary_hover.into(),
                cx.theme().transparent,
                cx.theme().button_secondary_foreground,
            ),
            Self::Ghost => ButtonVariantStyle::new(
                cx.theme().tokens.muted.into(),
                cx.theme().transparent,
                cx.theme().foreground,
            ),
            Self::Destructive => ButtonVariantStyle::new(
                cx.theme()
                    .danger
                    .opacity(if cx.theme().mode.is_dark() { 0.3 } else { 0.2 })
                    .into(),
                cx.theme().transparent,
                cx.theme().danger,
            ),
            Self::Link => ButtonVariantStyle::new(
                cx.theme().transparent.into(),
                cx.theme().transparent,
                cx.theme().link_hover,
            )
            .underline(),
        }
    }

    /// Resolves the pressed and compound-control active appearance.
    fn active(&self, cx: &mut App) -> ButtonVariantStyle {
        match self {
            Self::Default => ButtonVariantStyle::new(
                cx.theme().tokens.button_primary_active.into(),
                cx.theme().transparent,
                cx.theme().button_primary_foreground,
            ),
            Self::Outline => ButtonVariantStyle::new(
                cx.theme().tokens.button_active.into(),
                if cx.theme().mode.is_dark() {
                    cx.theme().input
                } else {
                    cx.theme().border
                },
                cx.theme().foreground,
            )
            .shadow(),
            Self::Secondary => ButtonVariantStyle::new(
                cx.theme().tokens.button_secondary_active.into(),
                cx.theme().transparent,
                cx.theme().button_secondary_foreground,
            ),
            Self::Ghost => ButtonVariantStyle::new(
                cx.theme().tokens.muted.background.opacity(0.8),
                cx.theme().transparent,
                cx.theme().foreground,
            ),
            Self::Destructive => ButtonVariantStyle::new(
                cx.theme()
                    .danger
                    .opacity(if cx.theme().mode.is_dark() { 0.4 } else { 0.3 })
                    .into(),
                cx.theme().transparent,
                cx.theme().danger,
            ),
            Self::Link => ButtonVariantStyle::new(
                cx.theme().transparent.into(),
                cx.theme().transparent,
                cx.theme().link_active,
            )
            .underline(),
        }
    }

    /// Keeps the normal semantic colors and removes elevation under disabled opacity.
    fn disabled(&self, cx: &mut App) -> ButtonVariantStyle {
        // shadcn disables the whole control at 50% opacity; retain the normal
        // semantic colors here and let the element apply that opacity once.
        let mut style = self.normal(cx);
        style.shadow = false;
        style
    }
}

impl ButtonVariantStyle {
    /// Creates a non-underlined, non-elevated style with stable border geometry.
    fn new(bg: Background, border: Hsla, fg: Hsla) -> Self {
        Self {
            bg,
            border,
            fg,
            underline: false,
            shadow: false,
        }
    }

    /// Enables the Vega outline elevation.
    fn shadow(mut self) -> Self {
        self.shadow = true;
        self
    }

    /// Enables Link underline for hover and active states.
    fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn builder_exposes_shadcn_variants_and_composition(_cx: &mut gpui::TestAppContext) {
        let button = Button::new("save")
            .outline()
            .large()
            .icon(crate::IconName::Check)
            .label("Save")
            .trailing_icon(crate::IconName::ArrowRight)
            .rounded_full();

        assert_eq!(button.variant, ButtonVariant::Outline);
        assert_eq!(button.size, Size::Large);
        assert!(button.icon.is_some());
        assert!(button.trailing_icon.is_some());
        assert!(matches!(button.rounding, ButtonRounding::Full));
    }

    #[gpui::test]
    fn disabled_button_is_not_clickable(_cx: &mut gpui::TestAppContext) {
        let enabled = Button::new("enabled").on_click(|_, _, _| {});
        let disabled = Button::new("disabled")
            .disabled(true)
            .on_click(|_, _, _| {});

        assert!(enabled.clickable());
        assert!(!disabled.clickable());
    }

    #[gpui::test]
    fn explicit_pressed_state_is_distinct_from_transient_active_state(
        _cx: &mut gpui::TestAppContext,
    ) {
        let toggle = Button::new("toggle").pressed(true);
        let popover_trigger = Button::new("popover").selected(true);

        assert_eq!(toggle.pressed, Some(true));
        assert_eq!(popover_trigger.pressed, None);
        assert!(popover_trigger.active);
    }
}
