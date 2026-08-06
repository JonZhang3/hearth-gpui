use std::rc::Rc;

use crate::{
    ActiveTheme, Disableable, FocusableExt, IconName, Selectable, Sizable, Size, StyledExt as _,
    animation::effective_motion_duration, icon::IconNamed, text::Text, tooltip::ComponentTooltip,
    v_flex,
};
use gpui::{
    Animation, AnimationExt, AnyElement, App, Div, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement,
    Styled, Toggled, Window, div, prelude::FluentBuilder as _, px, relative, svg,
};

/// A Checkbox element.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    base: Div,
    style: StyleRefinement,
    label: Option<Text>,
    children: Vec<AnyElement>,
    checked: bool,
    indeterminate: bool,
    invalid: bool,
    disabled: bool,
    size: Size,
    tab_stop: bool,
    tab_index: isize,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    tooltip: ComponentTooltip,
}

impl Checkbox {
    /// Create a new Checkbox with the given id.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style: StyleRefinement::default(),
            label: None,
            children: Vec::new(),
            checked: false,
            indeterminate: false,
            invalid: false,
            disabled: false,
            size: Size::default(),
            on_click: None,
            tab_stop: true,
            tab_index: 0,
            tooltip: ComponentTooltip::default(),
        }
    }

    /// Set tooltip text for the checkbox.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip.text = Some((tooltip.into(), None));
        self
    }

    /// Set the label for the checkbox.
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the checked state for the checkbox.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set the indeterminate state for the checkbox.
    ///
    /// Indeterminate takes precedence over `checked` for rendering and
    /// accessibility. Activating an indeterminate checkbox selects it.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    /// Set whether the checkbox value is invalid.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Set the click handler for the checkbox.
    ///
    /// The `&bool` parameter indicates the new checked state after the click.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set the tab stop for the checkbox, default is true.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Set the tab index for the checkbox, default is 0.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    fn handle_click(
        on_click: &Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
        checked: bool,
        indeterminate: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let new_checked = indeterminate || !checked;
        if let Some(f) = on_click {
            (f)(&new_checked, window, cx);
        }
    }
}

impl InteractiveElement for Checkbox {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}
impl StatefulInteractiveElement for Checkbox {}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl Disableable for Checkbox {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Checkbox {
    fn selected(self, selected: bool) -> Self {
        self.checked(selected)
    }

    fn is_selected(&self) -> bool {
        self.checked
    }
}

impl ParentElement for Checkbox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for Checkbox {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

/// Maps visual checkbox state to the AccessKit tri-state value.
fn checkbox_toggled(checked: bool, indeterminate: bool) -> Toggled {
    if indeterminate {
        Toggled::Mixed
    } else {
        checked.into()
    }
}

pub(crate) fn checkbox_check_icon(
    id: ElementId,
    size: Size,
    checked: bool,
    indeterminate: bool,
    disabled: bool,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let visual_state = if indeterminate { 2_u8 } else { checked as u8 };
    let toggle_state = window.use_keyed_state(id, cx, |_, _| visual_state);
    let color = if disabled {
        cx.theme().primary_foreground.opacity(0.5)
    } else {
        cx.theme().primary_foreground
    };

    svg()
        .absolute()
        .top_px()
        .left_px()
        .map(|this| match size {
            Size::XSmall => this.size_2(),
            Size::Small => this.size_2p5(),
            Size::Medium => this.size_3(),
            Size::Large => this.size_3p5(),
            _ => this.size_3(),
        })
        .text_color(color)
        .map(|this| match visual_state {
            1 => this.path(IconName::Check.path()),
            2 => this.path(IconName::Minus.path()),
            _ => this,
        })
        .map(|this| {
            if !disabled && visual_state != *toggle_state.read(cx) {
                let duration = cx.theme().style.motion.emphasis();
                let timer_duration = effective_motion_duration(duration, cx);
                let easing = if visual_state > 0 {
                    cx.theme().style.motion.enter_easing
                } else {
                    cx.theme().style.motion.exit_easing
                };
                cx.spawn({
                    let toggle_state = toggle_state.clone();
                    async move |cx| {
                        cx.background_executor().timer(timer_duration).await;
                        _ = toggle_state.update(cx, |this, _| *this = visual_state);
                    }
                })
                .detach();

                this.with_animation(
                    ElementId::NamedInteger("toggle".into(), visual_state as u64),
                    Animation::new(duration).with_easing(move |delta| easing.sample(delta)),
                    move |this, delta| {
                        this.opacity(if visual_state > 0 {
                            1.0 * delta
                        } else {
                            1.0 - delta
                        })
                    },
                )
                .into_any_element()
            } else {
                this.into_any_element()
            }
        })
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let indeterminate = self.indeterminate;
        let selected = checked || indeterminate;

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);

        let border_color = if self.invalid {
            cx.theme().danger
        } else if selected {
            cx.theme().primary
        } else {
            cx.theme().input
        };
        let color = if self.disabled {
            border_color.opacity(0.5)
        } else {
            border_color
        };
        let radius = cx.theme().style.radii.md.min(px(4.));
        let control_metrics = cx.theme().style.controls.for_size(self.size);

        let element = self
            .base
            .id(self.id.clone())
            .role(Role::CheckBox)
            .aria_toggled(checkbox_toggled(checked, indeterminate))
            .when_some(
                self.label.as_ref().map(|l| l.get_text(cx)),
                |this, label| this.aria_label(label),
            )
            .when(!self.disabled, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_stop(self.tab_stop)
                        .tab_index(self.tab_index),
                )
            })
            .h_flex()
            .gap(control_metrics.gap)
            .items_start()
            .line_height(relative(1.))
            .text_color(cx.theme().foreground)
            .map(|this| match self.size {
                Size::XSmall => this.text_xs(),
                Size::Small => this.text_sm(),
                Size::Medium => this.text_base(),
                Size::Large => this.text_lg(),
                _ => this,
            })
            .when(self.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .rounded(cx.theme().style.radii.md * 0.5)
            .focus_ring_color(
                is_focused,
                px(2.),
                if self.invalid {
                    cx.theme().danger
                } else {
                    cx.theme().ring
                },
                window,
                cx,
            )
            .refine_style(&self.style)
            .child(
                div()
                    .relative()
                    .size(control_metrics.icon_size)
                    .flex_shrink_0()
                    .border_1()
                    .border_color(color)
                    .rounded(radius)
                    .map(|this| match selected {
                        false => this.bg(cx.theme().input_background()),
                        true if self.disabled => this.bg(color),
                        true if self.invalid => this.bg(cx.theme().danger),
                        true => this.bg(cx.theme().tokens.primary),
                    })
                    .child(checkbox_check_icon(
                        self.id,
                        self.size,
                        checked,
                        indeterminate,
                        self.disabled,
                        window,
                        cx,
                    )),
            )
            .when(self.label.is_some() || !self.children.is_empty(), |this| {
                this.child(
                    v_flex()
                        .flex_1()
                        .overflow_hidden()
                        .line_height(relative(1.2))
                        .gap_1()
                        .map(|this| {
                            if let Some(label) = self.label {
                                this.child(
                                    div()
                                        .size_full()
                                        .text_color(cx.theme().foreground)
                                        .when(self.disabled, |this| {
                                            this.text_color(cx.theme().muted_foreground)
                                        })
                                        .line_height(relative(1.))
                                        .child(label),
                                )
                            } else {
                                this
                            }
                        })
                        .children(self.children),
                )
            })
            .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                // Avoid focus on mouse down.
                window.prevent_default();
            })
            .when(!self.disabled, |this| {
                this.on_click({
                    let on_click = self.on_click.clone();
                    move |_, window, cx| {
                        window.prevent_default();
                        Self::handle_click(&on_click, checked, indeterminate, window, cx);
                    }
                })
            })
            .map(|this| self.tooltip.apply(this));

        crate::accessibility::accessibility_state(element, self.invalid, false, self.disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indeterminate_state_is_exposed_as_mixed() {
        assert_eq!(checkbox_toggled(false, false), Toggled::False);
        assert_eq!(checkbox_toggled(true, false), Toggled::True);
        assert_eq!(checkbox_toggled(false, true), Toggled::Mixed);
        assert_eq!(checkbox_toggled(true, true), Toggled::Mixed);
    }
}
