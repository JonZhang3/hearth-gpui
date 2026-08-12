use std::sync::Arc;

use gpui::{
    AnyElement, App, Axis, ElementId, Half, InteractiveElement as _, IntoElement, ParentElement,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _, px, relative,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, AxisExt, Density, Icon, Sizable, Size, StylePreset, StyledExt as _,
    accessibility::accessibility_state_with_current, stepper::trigger::StepperTrigger,
};

/// Geometry resolved from semantic Style Preset metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct StepperMetrics {
    pub indicator_size: Pixels,
    pub icon_size: Pixels,
    pub stacked_gap: Pixels,
    pub inline_gap: Pixels,
    pub connector_width: Pixels,
    pub connector_gap: Pixels,
}

impl StepperMetrics {
    /// Resolves Stepper geometry without branching on a preset identifier.
    pub(super) fn resolve(size: Size, style: &StylePreset) -> Self {
        let control = style.controls.for_size(size);
        let indicator_size = match size {
            Size::Size(edge) => edge.max(px(0.)),
            _ => {
                let ratio = match style.density {
                    Density::Compact => 0.625,
                    Density::Standard => 2. / 3.,
                    Density::Comfortable => 0.75,
                };
                control.height * ratio
            }
        };
        let density_gap = match style.density {
            Density::Compact => px(2.),
            Density::Standard => px(4.),
            Density::Comfortable => px(6.),
        };

        Self {
            indicator_size,
            icon_size: control.icon_size.min((indicator_size - px(4.)).max(px(0.))),
            stacked_gap: density_gap,
            inline_gap: density_gap + px(2.),
            connector_width: match style.density {
                Density::Compact => px(1.),
                Density::Standard | Density::Comfortable => px(2.),
            },
            connector_gap: density_gap,
        }
    }
}

/// Creates stable child IDs without flattening structured caller IDs.
fn stepper_child_id(id: &ElementId, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), name.into())
}

/// A step item within a [`Stepper`].
#[derive(IntoElement)]
pub struct StepperItem {
    owner_id: ElementId,
    step: usize,
    checked_step: Option<usize>,
    size_of_set: usize,
    style: StyleRefinement,
    icon: Option<Icon>,
    label: Option<SharedString>,
    aria_label: Option<SharedString>,
    children: Vec<AnyElement>,
    layout: Axis,
    disabled: bool,
    size: Size,
    is_last: bool,
    text_center: bool,
    interactive: bool,
    on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl StepperItem {
    pub fn new() -> Self {
        Self {
            owner_id: "stepper".into(),
            step: 0,
            checked_step: Some(0),
            size_of_set: 1,
            style: StyleRefinement::default(),
            icon: None,
            label: None,
            aria_label: None,
            layout: Axis::Horizontal,
            disabled: false,
            size: Size::default(),
            is_last: false,
            text_center: false,
            interactive: false,
            children: Vec::new(),
            on_click: None,
        }
    }

    /// Sets the visible text and accessible name of the step.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the accessible name for custom step content.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set the icon of the stepper item.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set disabled state of the stepper item.
    ///
    /// Will override the stepper's disabled state if set to true.
    ///
    /// Default is false.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub(super) fn text_center(mut self, center: bool) -> Self {
        self.text_center = center;
        self
    }

    pub(super) fn owner_id(mut self, id: ElementId) -> Self {
        self.owner_id = id;
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

    pub(super) fn size_of_set(mut self, size: usize) -> Self {
        self.size_of_set = size;
        self
    }

    pub(super) fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub(super) fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    pub(super) fn is_last(mut self, is_last: bool) -> Self {
        self.is_last = is_last;
        self
    }

    pub(super) fn on_click<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl ParentElement for StepperItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for StepperItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for StepperItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for StepperItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_current = self.checked_step == Some(self.step);
        let is_passed = self.checked_step.is_some_and(|checked| self.step < checked);
        let metrics = StepperMetrics::resolve(self.size, &cx.theme().style);
        let accessible_label = self
            .aria_label
            .clone()
            .or_else(|| self.label.clone())
            .unwrap_or_else(|| t!("Stepper.step", step = self.step + 1).into());
        let item_id = stepper_child_id(&self.owner_id, format!("item-{}", self.step));
        let trigger_id = stepper_child_id(&self.owner_id, format!("trigger-{}", self.step));
        let disabled = self.disabled;

        let item = div()
            .id(item_id)
            .role(Role::ListItem)
            .aria_position_in_set(self.step + 1)
            .aria_size_of_set(self.size_of_set)
            .aria_label(accessible_label.clone())
            .relative()
            .min_w_0()
            .when(self.layout.is_horizontal(), |this| this.h_flex())
            .when(self.layout.is_vertical(), |this| this.v_flex())
            .when(!self.is_last, |this| this.flex_1())
            .when(self.text_center, |this| this.flex_1().justify_center())
            .items_start()
            .refine_style(&self.style)
            .child(
                StepperTrigger::new()
                    .id(trigger_id)
                    .icon(self.icon)
                    .label(self.label)
                    .aria_label(accessible_label)
                    .metrics(metrics)
                    .step(self.step)
                    .with_size(self.size)
                    .checked_step(self.checked_step)
                    .text_center(self.text_center)
                    .layout(self.layout)
                    .disabled(self.disabled)
                    .interactive(self.interactive)
                    .children(self.children)
                    .when_some(self.on_click, |this, on_click| {
                        this.on_click(move |window, cx| on_click(window, cx))
                    }),
            )
            .when(!self.is_last, |this| {
                this.child(
                    StepperSeparator::new()
                        .layout(self.layout)
                        .text_center(self.text_center)
                        .metrics(metrics)
                        .checked(is_passed),
                )
            });

        accessibility_state_with_current(
            item,
            false,
            false,
            disabled,
            is_current.then_some(gpui::accesskit::AriaCurrent::Step),
        )
    }
}

/// A separator between stepper items.
///
/// Default is `absolute` positioned.
#[derive(IntoElement)]
struct StepperSeparator {
    checked: bool,
    metrics: StepperMetrics,
    layout: Axis,
    style: StyleRefinement,
    text_center: bool,
}

impl StepperSeparator {
    fn new() -> Self {
        Self {
            checked: false,
            metrics: StepperMetrics::resolve(Size::default(), &StylePreset::vega()),
            layout: Axis::Horizontal,
            style: StyleRefinement::default(),
            text_center: false,
        }
    }

    fn text_center(mut self, center: bool) -> Self {
        self.text_center = center;
        self
    }

    fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    fn metrics(mut self, metrics: StepperMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

impl Styled for StepperSeparator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for StepperSeparator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let icon_size = self.metrics.indicator_size;
        let text_center = self.text_center;
        let separator_wide = self.metrics.connector_width;
        let gap = self.metrics.connector_gap;
        let cross_axis_offset = (icon_size - separator_wide) / 2.;

        div()
            .absolute()
            .flex_1()
            .when(self.layout.is_horizontal(), |this| {
                this.h(separator_wide).mt(cross_axis_offset).map(|this| {
                    if !text_center {
                        this.ml(icon_size + gap).mr(gap).left_0().right_0()
                    } else {
                        this.mx(icon_size.half() + gap)
                            .left(relative(0.5))
                            .right(relative(-0.5))
                    }
                })
            })
            .when(self.layout.is_vertical(), |this| {
                this.w(separator_wide).ml(cross_axis_offset).map(|this| {
                    if !text_center {
                        this.mt(icon_size + gap).mb(gap).top_0().bottom_0()
                    } else {
                        this.my(icon_size.half() + gap)
                            .top(relative(0.5))
                            .bottom(relative(-0.5))
                    }
                })
            })
            .refine_style(&self.style)
            .bg(cx.theme().border)
            .when(self.checked, |this| this.bg(cx.theme().tokens.primary))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepper_metrics_follow_builtin_preset_density_and_custom_size() {
        let nova = StepperMetrics::resolve(Size::Medium, &StylePreset::nova());
        let vega = StepperMetrics::resolve(Size::Medium, &StylePreset::vega());
        let maia = StepperMetrics::resolve(Size::Medium, &StylePreset::maia());

        assert!(nova.indicator_size < vega.indicator_size);
        assert!(vega.indicator_size < maia.indicator_size);
        assert!(nova.connector_width < vega.connector_width);
        assert!(nova.stacked_gap < maia.stacked_gap);

        let custom = StepperMetrics::resolve(Size::Size(px(30.)), &StylePreset::vega());
        assert_eq!(custom.indicator_size, px(30.));
    }

    #[gpui::test]
    fn stepper_item_builder_preserves_public_configuration(_cx: &mut gpui::TestAppContext) {
        let item = StepperItem::new()
            .label("Shipping")
            .aria_label("Shipping step")
            .icon(crate::IconName::Inbox)
            .disabled(true)
            .large();

        assert_eq!(item.label.as_deref(), Some("Shipping"));
        assert_eq!(item.aria_label.as_deref(), Some("Shipping step"));
        assert!(item.icon.is_some());
        assert!(item.disabled);
        assert_eq!(item.size, Size::Large);
    }
}
