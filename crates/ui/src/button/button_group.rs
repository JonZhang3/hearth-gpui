// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public types: `ButtonGroupText`, `ButtonGroupSeparator`.
// - Added public methods: `new`, `group`, `text`, `separator`, `orientation`, `aria_label`.
// - Added or exposed behavior through `extend`, `render_for`, `group`, `text`, `separator`,
//   `orientation`, `aria_label`, `group_corners` and 3 more.
// - Removed or replaced `with_variant`, `test_button_group_builder`.
// - Reworked Button Group around accessibility semantics and ARIA state, semantic Style Preset
//   geometry and density.
use gpui::{
    AnyElement, App, Axis, Corners, Edges, ElementId, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use std::{cell::Cell, rc::Rc};

use crate::{
    ActiveTheme as _, Disableable, Selectable as _, Sizable, Size, StyledExt as _,
    button::{Button, ButtonVariant},
};

/// Content that visually participates in a button group without being interactive.
#[derive(IntoElement)]
pub struct ButtonGroupText {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl ButtonGroupText {
    /// Creates a text item from any GPUI element.
    pub fn new(content: impl IntoElement) -> Self {
        Self {
            style: StyleRefinement::default(),
            children: vec![content.into_any_element()],
        }
    }
}

impl Styled for ButtonGroupText {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ButtonGroupText {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ButtonGroupText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = cx.theme().style.controls.md;
        div()
            .h(metrics.height)
            .px(metrics.padding_x)
            .flex()
            .items_center()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted)
            .text_color(cx.theme().muted_foreground)
            .text_sm()
            .font_medium()
            .rounded(cx.theme().style.radii.md)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// A visual separator between sections of a button group.
#[derive(Default, IntoElement)]
pub struct ButtonGroupSeparator {
    style: StyleRefinement,
}

impl ButtonGroupSeparator {
    /// Creates a separator whose direction follows its parent group.
    pub fn new() -> Self {
        Self::default()
    }

    fn render_for(self, orientation: Axis, cx: &App) -> AnyElement {
        div()
            .bg(cx.theme().border)
            .when(orientation == Axis::Horizontal, |this| {
                this.w(px(1.)).self_stretch()
            })
            .when(orientation == Axis::Vertical, |this| {
                this.h(px(1.)).w_full()
            })
            .refine_style(&self.style)
            .into_any_element()
    }
}

impl Styled for ButtonGroupSeparator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ButtonGroupSeparator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.render_for(Axis::Horizontal, cx)
    }
}

enum ButtonGroupItem {
    Button(Box<Button>),
    Group(Box<ButtonGroup>),
    Text(ButtonGroupText),
    Separator(ButtonGroupSeparator),
}

/// Composes related actions while preserving each child's own behavior.
#[derive(IntoElement)]
pub struct ButtonGroup {
    id: ElementId,
    style: StyleRefinement,
    items: Vec<ButtonGroupItem>,
    orientation: Axis,
    aria_label: Option<SharedString>,
    legacy_variant: Option<ButtonVariant>,
    legacy_size: Option<Size>,
    legacy_disabled: bool,
    legacy_multiple: bool,
    legacy_on_click: Option<Box<dyn Fn(&Vec<usize>, &mut Window, &mut App) + 'static>>,
}

impl ButtonGroup {
    /// Creates a horizontal action group.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            items: Vec::new(),
            orientation: Axis::Horizontal,
            aria_label: None,
            legacy_variant: None,
            legacy_size: None,
            legacy_disabled: false,
            legacy_multiple: false,
            legacy_on_click: None,
        }
    }

    /// Adds a button and preserves its click handler and accessibility state.
    pub fn child(mut self, child: Button) -> Self {
        self.items.push(ButtonGroupItem::Button(Box::new(child)));
        self
    }

    /// Adds multiple buttons.
    pub fn children(mut self, children: impl IntoIterator<Item = Button>) -> Self {
        for child in children {
            self = self.child(child);
        }
        self
    }

    /// Adds a nested group as a separate action cluster.
    pub fn group(mut self, group: ButtonGroup) -> Self {
        self.items.push(ButtonGroupItem::Group(Box::new(group)));
        self
    }

    /// Adds non-interactive group text.
    pub fn text(mut self, text: ButtonGroupText) -> Self {
        self.items.push(ButtonGroupItem::Text(text));
        self
    }

    /// Adds a separator following the group orientation.
    pub fn separator(mut self, separator: ButtonGroupSeparator) -> Self {
        self.items.push(ButtonGroupItem::Separator(separator));
        self
    }

    /// Sets the group orientation.
    pub fn orientation(mut self, orientation: Axis) -> Self {
        self.orientation = orientation;
        self
    }

    /// Legacy alias for [`Self::orientation`].
    #[doc(hidden)]
    pub fn layout(self, orientation: Axis) -> Self {
        self.orientation(orientation)
    }

    /// Legacy style propagation retained while internal stories migrate.
    #[doc(hidden)]
    pub fn outline(mut self) -> Self {
        self.legacy_variant = Some(ButtonVariant::Outline);
        self
    }

    /// Legacy compact mode now resolves to the Vega extra-small metrics.
    #[doc(hidden)]
    pub fn compact(mut self) -> Self {
        self.legacy_size = Some(Size::XSmall);
        self
    }

    /// Legacy group selection mode retained for internal story controls.
    #[doc(hidden)]
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.legacy_multiple = multiple;
        self
    }

    /// Legacy group callback retained for internal story controls.
    #[doc(hidden)]
    pub fn on_click(
        mut self,
        handler: impl Fn(&Vec<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.legacy_on_click = Some(Box::new(handler));
        self
    }

    /// Sets an accessible name for the group.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }
}

impl Sizable for ButtonGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.legacy_size = Some(size.into());
        self
    }
}

impl Disableable for ButtonGroup {
    fn disabled(mut self, disabled: bool) -> Self {
        self.legacy_disabled = disabled;
        self
    }
}

impl Styled for ButtonGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ButtonGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let orientation = self.orientation;
        let has_nested_group = self
            .items
            .iter()
            .any(|item| matches!(item, ButtonGroupItem::Group(_)));
        let item_count = self.items.len();
        let button_items = self
            .items
            .iter()
            .map(|item| matches!(item, ButtonGroupItem::Button(_)))
            .collect::<Vec<_>>();
        let mut rendered = Vec::with_capacity(item_count);
        let state = Rc::new(Cell::new(None));
        let selected_indices = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| match item {
                ButtonGroupItem::Button(button) if button.is_selected() => Some(index),
                _ => None,
            })
            .collect::<Vec<_>>();

        for (index, item) in self.items.into_iter().enumerate() {
            let previous_is_button = index > 0 && button_items[index - 1];
            let next_is_button = index + 1 < item_count && button_items[index + 1];
            let element = match item {
                ButtonGroupItem::Button(button) => {
                    let first = !previous_is_button;
                    let last = !next_is_button;
                    (*button)
                        .border_corners(group_corners(orientation, first, last))
                        .border_edges(group_edges(orientation, first))
                        .when_some(self.legacy_variant, |this, variant| {
                            crate::button::ButtonVariants::with_variant(this, variant)
                        })
                        .when_some(self.legacy_size, |this, size| this.with_size(size))
                        .when(self.legacy_disabled, |button| button.disabled(true))
                        .when(self.legacy_on_click.is_some(), |this| {
                            let state = Rc::clone(&state);
                            let pressed = this.is_selected();
                            this.pressed(pressed)
                                .on_click(move |_, _, _| state.set(Some(index)))
                        })
                        .into_any_element()
                }
                ButtonGroupItem::Group(group) => group
                    .when(self.legacy_disabled, |group| group.disabled(true))
                    .into_any_element(),
                ButtonGroupItem::Text(text) => text.into_any_element(),
                ButtonGroupItem::Separator(separator) => separator.render_for(orientation, cx),
            };
            rendered.push(element);
        }

        div()
            .id(self.id)
            .role(Role::Group)
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .flex()
            .when(orientation == Axis::Vertical, |this| this.flex_col())
            .when(has_nested_group, |this| this.gap(px(8.)))
            .refine_style(&self.style)
            .children(rendered)
            .when_some(
                self.legacy_on_click.filter(|_| !self.legacy_disabled),
                move |this, on_click| {
                    this.on_click(move |_, window, cx| {
                        let mut indices = selected_indices.clone();
                        if let Some(index) = state.get() {
                            if self.legacy_multiple {
                                if let Some(position) =
                                    indices.iter().position(|item| *item == index)
                                {
                                    indices.remove(position);
                                } else {
                                    indices.push(index);
                                }
                            } else {
                                indices.clear();
                                indices.push(index);
                            }
                        }
                        on_click(&indices, window, cx);
                    })
                },
            )
    }
}

pub(crate) fn group_corners(orientation: Axis, first: bool, last: bool) -> Corners<bool> {
    if orientation == Axis::Vertical {
        Corners {
            top_left: first,
            top_right: first,
            bottom_left: last,
            bottom_right: last,
        }
    } else {
        Corners {
            top_left: first,
            top_right: last,
            bottom_left: first,
            bottom_right: last,
        }
    }
}

pub(crate) fn group_edges(orientation: Axis, first: bool) -> Edges<bool> {
    if orientation == Axis::Vertical {
        Edges {
            left: true,
            top: first,
            right: true,
            bottom: true,
        }
    } else {
        Edges {
            left: first,
            top: true,
            right: true,
            bottom: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn button_group_accepts_composable_items(_cx: &mut gpui::TestAppContext) {
        let group = ButtonGroup::new("actions")
            .aria_label("Message actions")
            .child(Button::new("archive").label("Archive"))
            .separator(ButtonGroupSeparator::new())
            .text(ButtonGroupText::new("More"))
            .group(ButtonGroup::new("more").child(Button::new("report").label("Report")));

        assert_eq!(group.items.len(), 4);
        assert_eq!(group.aria_label.as_deref(), Some("Message actions"));
    }

    #[gpui::test]
    fn disabled_state_remains_reversible_for_nested_groups(_cx: &mut gpui::TestAppContext) {
        let group = ButtonGroup::new("parent")
            .group(ButtonGroup::new("nested").child(Button::new("action")))
            .disabled(true)
            .disabled(false);

        let ButtonGroupItem::Group(nested) = &group.items[0] else {
            panic!("expected nested button group");
        };
        assert!(!group.legacy_disabled);
        assert!(!nested.legacy_disabled);
    }
}
