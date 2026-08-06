use gpui::{
    Anchor, App, Context, Edges, ElementId, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div, prelude::FluentBuilder,
};
use gpui::{Corners, Pixels};

use crate::{
    Disableable, IconName, Selectable, Sizable, Size, StyledExt as _,
    menu::{DropdownMenu, PopupMenu},
    tooltip::ComponentTooltip,
};

use super::{Button, ButtonVariant, ButtonVariants};

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
        }
    }

    /// Set tooltip text for the dropdown button.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip.text = Some((tooltip.into(), None));
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
        let rounded = self.variant.is_ghost() && !self.selected;

        div()
            .id(self.id)
            .h_flex()
            .refine_style(&self.style)
            .when_some(self.button, |this, button| {
                this.child(
                    button
                        .when_some(self.radius, |this, radius| this.rounded(radius))
                        .border_corners(Corners {
                            top_left: true,
                            top_right: rounded,
                            bottom_left: true,
                            bottom_right: rounded,
                        })
                        .border_edges(Edges {
                            left: true,
                            top: true,
                            right: true,
                            bottom: true,
                        })
                        .selected(self.selected)
                        .disabled(self.disabled)
                        .with_size(self.size)
                        .with_variant(self.variant),
                )
                .when_some(self.menu, |this, menu| {
                    this.child(
                        Button::new("popup")
                            .icon(IconName::ChevronDown)
                            .when_some(self.radius, |this, radius| this.rounded(radius))
                            .border_edges(Edges {
                                left: rounded,
                                top: true,
                                right: true,
                                bottom: true,
                            })
                            .border_corners(Corners {
                                top_left: rounded,
                                top_right: true,
                                bottom_left: rounded,
                                bottom_right: true,
                            })
                            .selected(self.selected)
                            .disabled(self.disabled)
                            .with_size(self.size)
                            .with_variant(self.variant)
                            .dropdown_menu_with_anchor(self.anchor, menu),
                    )
                })
            })
            .map(|this| self.tooltip.apply(this))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn test_dropdown_button_builder(_cx: &mut gpui::TestAppContext) {
        let button = Button::new("inner").label("Action");
        let dropdown = DropdownButton::new("complex-dropdown")
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
    }
}
