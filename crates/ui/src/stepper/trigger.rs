// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added or exposed behavior through `id`, `label`, `aria_label`, `metrics`, `interactive`.
// - Removed or replaced `icon_size`.
// - Reworked Trigger around accessibility semantics and ARIA state, semantic Style Preset geometry
//   and density, keyboard navigation and activation behavior, focus-visible and focus restoration
//   behavior.
use std::rc::Rc;

use gpui::{
    AnyElement, App, Axis, ElementId, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{
    ActiveTheme as _, AxisExt, FocusableExt as _, Icon, Sizable as _, Size, StyleSized,
    StyledExt as _, accessibility::accessibility_state, stepper::item::StepperMetrics,
};

/// The trigger part of a stepper item.
#[derive(IntoElement)]
pub(super) struct StepperTrigger {
    id: ElementId,
    step: usize,
    checked_step: Option<usize>,
    style: StyleRefinement,
    icon: Option<Icon>,
    label: Option<SharedString>,
    aria_label: Option<SharedString>,
    metrics: StepperMetrics,
    children: Vec<AnyElement>,
    layout: Axis,
    disabled: bool,
    interactive: bool,
    text_center: bool,
    size: Size,
    on_click: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl StepperTrigger {
    pub(super) fn new() -> Self {
        Self {
            id: "stepper-trigger".into(),
            step: 0,
            checked_step: Some(0),
            icon: None,
            label: None,
            aria_label: None,
            metrics: StepperMetrics::resolve(Size::default(), &crate::StylePreset::vega()),
            layout: Axis::Horizontal,
            disabled: false,
            interactive: false,
            size: Size::default(),
            children: Vec::new(),
            text_center: false,
            style: StyleRefinement::default(),
            on_click: None,
        }
    }

    pub(super) fn id(mut self, id: ElementId) -> Self {
        self.id = id;
        self
    }

    pub(super) fn step(mut self, ix: usize) -> Self {
        self.step = ix;
        self
    }

    pub(super) fn checked_step(mut self, checked_step: Option<usize>) -> Self {
        self.checked_step = checked_step;
        self
    }

    pub(super) fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    pub(super) fn icon(mut self, icon: Option<impl Into<Icon>>) -> Self {
        self.icon = icon.map(|i| i.into());
        self
    }

    pub(super) fn label(mut self, label: Option<SharedString>) -> Self {
        self.label = label;
        self
    }

    pub(super) fn aria_label(mut self, label: SharedString) -> Self {
        self.aria_label = Some(label);
        self
    }

    pub(super) fn metrics(mut self, metrics: StepperMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    pub(super) fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub(super) fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub(super) fn with_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub(super) fn text_center(mut self, center: bool) -> Self {
        self.text_center = center;
        self
    }

    pub(super) fn on_click<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl Styled for StepperTrigger {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for StepperTrigger {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for StepperTrigger {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_checked = self
            .checked_step
            .is_some_and(|checked| self.step <= checked);
        let interactive = self.interactive && self.on_click.is_some() && !self.disabled;
        let focus_handle = interactive.then(|| {
            window
                .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
                .read(cx)
                .clone()
        });
        let focus_visible = focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_focused(window) && window.last_input_was_keyboard());
        let on_click = self.on_click.clone();
        let indicator = div()
            .id("indicator")
            .size(self.metrics.indicator_size)
            .overflow_hidden()
            .flex()
            .flex_shrink_0()
            .rounded_full()
            .items_center()
            .justify_center()
            .bg(cx.theme().tokens.secondary)
            .when(interactive && !is_checked, |this| {
                this.hover(|this| this.bg(cx.theme().tokens.secondary_hover))
                    .active(|this| this.bg(cx.theme().tokens.secondary_active))
            })
            .text_color(cx.theme().secondary_foreground)
            .when(is_checked, |this| {
                this.bg(cx.theme().tokens.primary)
                    .text_color(cx.theme().primary_foreground)
            })
            .when(self.size != Size::XSmall, |this| {
                this.child(if let Some(icon) = self.icon {
                    icon.with_size(self.metrics.icon_size).into_any_element()
                } else {
                    div().child(format!("{}", self.step + 1)).into_any_element()
                })
            })
            .focus_ring(focus_visible, px(0.), window, cx);

        let element = div()
            .id(self.id)
            .when(interactive, |this| this.role(Role::Button))
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .when_some(focus_handle.as_ref(), |this, handle| {
                this.track_focus(&handle.clone().tab_stop(true))
            })
            .min_w_0()
            .when(self.layout.is_horizontal(), |this| {
                this.v_flex().gap(self.metrics.stacked_gap)
            })
            .when(self.layout.is_vertical(), |this| {
                this.h_flex().gap(self.metrics.inline_gap)
            })
            .items_start()
            .when(self.text_center, |this| this.items_center())
            .input_text_size(self.size.smaller())
            .refine_style(&self.style)
            .when(self.disabled, |this| this.opacity(0.5))
            .child(indicator)
            .when_some(self.label, |this, label| this.child(label))
            .children(self.children)
            .when(interactive, |this| {
                this.on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    crate::global_state::GlobalState::suppress_text_selection(cx);
                })
                .on_click({
                    let on_click = on_click.clone();
                    move |_, window, cx| {
                        if let Some(on_click) = &on_click {
                            on_click(window, cx);
                        }
                    }
                })
            });

        accessibility_state(
            element,
            false,
            !interactive && !self.disabled,
            self.disabled,
        )
    }
}
