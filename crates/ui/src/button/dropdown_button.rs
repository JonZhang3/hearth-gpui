use std::sync::Arc;

use gpui::{
    Anchor, App, Axis, Context, ElementId, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Window, div, prelude::FluentBuilder,
};
use gpui::{Pixels, px};
use rust_i18n::t;

use crate::{
    Disableable, IconName, Selectable, Sizable, Size, StyledExt as _,
    menu::{DropdownMenu, PopupMenu},
    tooltip::ComponentTooltip,
};

use super::{
    Button, ButtonVariant, ButtonVariants,
    button_group::{group_corners, group_edges},
};

/// Derives a stable menu-trigger identity without colliding across DropdownButton instances.
fn menu_trigger_id(id: &ElementId) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), "menu-trigger".into())
}

#[derive(IntoElement)]
pub struct DropdownButton {
    id: ElementId,
    style: StyleRefinement,
    button: Option<Button>,
    menu:
        Option<Box<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static>>,
    selected: bool,
    disabled: bool,
    // The button props
    variant: ButtonVariant,
    size: Size,
    radius: Option<Pixels>,
    anchor: Anchor,
    tooltip: ComponentTooltip,
    aria_label: Option<SharedString>,
    menu_aria_label: Option<SharedString>,
}

impl DropdownButton {
    /// Create a new DropdownButton.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            button: None,
            menu: None,
            selected: false,
            disabled: false,
            variant: ButtonVariant::default(),
            size: Size::default(),
            radius: None,
            anchor: Anchor::TopRight,
            tooltip: ComponentTooltip::default(),
            aria_label: None,
            menu_aria_label: None,
        }
    }

    /// Set tooltip text for the dropdown button.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip.text = Some((tooltip.into(), None));
        self
    }

    /// Set the accessible name of the composite button group.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set the accessible name of the dropdown-menu trigger segment.
    pub fn menu_aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.menu_aria_label = Some(label.into());
        self
    }

    /// Set the left button of the dropdown button.
    pub fn button(mut self, button: Button) -> Self {
        self.button = Some(button);
        self
    }

    /// Set the dropdown menu of the button.
    pub fn dropdown_menu(
        mut self,
        menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.menu = Some(Box::new(menu));
        self
    }

    /// Set the dropdown menu of the button with anchor corner.
    pub fn dropdown_menu_with_anchor(
        mut self,
        anchor: impl Into<Anchor>,
        menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.menu = Some(Box::new(menu));
        self.anchor = anchor.into();
        self
    }

    /// Set the rounded style of the button.
    pub fn rounded(mut self, radius: Pixels) -> Self {
        self.radius = Some(radius);
        self
    }
}

impl Disableable for DropdownButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for DropdownButton {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl Sizable for DropdownButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ButtonVariants for DropdownButton {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Selectable for DropdownButton {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for DropdownButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let menu_trigger_id = menu_trigger_id(&self.id);
        let menu_aria_label = self
            .menu_aria_label
            .unwrap_or_else(|| t!("DropdownButton.more_options").into());

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .h_flex()
            .refine_style(&self.style)
            .when_some(self.button, |this, button| {
                this.child(
                    button
                        .when_some(self.radius, |this, radius| this.rounded(radius))
                        .border_corners(group_corners(Axis::Horizontal, true, false))
                        .border_edges(group_edges(Axis::Horizontal, true))
                        .pressed_offset(false)
                        .selected(self.selected)
                        .disabled(self.disabled)
                        .with_size(self.size)
                        .with_variant(self.variant),
                )
                .when_some(self.menu, |this, menu| {
                    this.child(
                        Button::new(menu_trigger_id)
                            .icon(IconName::ChevronDown)
                            .aria_label(menu_aria_label)
                            .when_some(self.radius, |this, radius| this.rounded(radius))
                            .border_corners(group_corners(Axis::Horizontal, false, true))
                            .border_edges(group_edges(Axis::Horizontal, false))
                            .pressed_offset(false)
                            .selected(self.selected)
                            .disabled(self.disabled)
                            .with_size(self.size)
                            .with_variant(self.variant)
                            .dropdown_menu_with_anchor(self.anchor, menu)
                            .side_offset(px(4.)),
                    )
                })
            })
            .map(|this| self.tooltip.apply(&self.id, this))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn test_dropdown_button_builder(_cx: &mut gpui::TestAppContext) {
        let button = Button::new("inner").label("Action");
        let dropdown = DropdownButton::new("complex-dropdown")
            .aria_label("Document actions")
            .menu_aria_label("Open document options")
            .button(button)
            .outline()
            .large()
            .disabled(false)
            .selected(false)
            .rounded(gpui::px(8.))
            .dropdown_menu_with_anchor(Anchor::BottomLeft, |menu, _, _| menu);

        assert!(dropdown.button.is_some());
        assert_eq!(dropdown.variant, ButtonVariant::Outline);
        assert_eq!(dropdown.size, Size::Large);
        assert!(!dropdown.disabled);
        assert!(!dropdown.selected);
        assert_eq!(dropdown.radius, Some(gpui::px(8.)));
        assert!(dropdown.menu.is_some());
        assert_eq!(dropdown.anchor, Anchor::BottomLeft);
        assert_eq!(dropdown.aria_label, Some("Document actions".into()));
        assert_eq!(
            dropdown.menu_aria_label,
            Some("Open document options".into())
        );
    }

    #[test]
    fn menu_trigger_ids_preserve_parent_identity() {
        let first: ElementId = ("dropdown", 1_u32).into();
        let second: ElementId = "dropdown-1".into();

        assert_ne!(menu_trigger_id(&first), menu_trigger_id(&second));
    }
}
