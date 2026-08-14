// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `informative`.
// - Added or exposed behavior through `informative_container`, `informative`, `edge`,
//   `resolve_edge`, `render_element`, `named_sizes_and_inherited_size_resolve_consistently`,
//   `clone_preserves_transformation_style_and_accessibility`,
//   `informative_icon_exposes_image_semantics`.
// - Removed or replaced `text_color`.
// - Reworked Icon around accessibility semantics and ARIA state.
use crate::{Sizable, Size, StyledExt};
use gpui::{
    AnyElement, App, AppContext, Context, Div, ElementId, Entity, InteractiveElement, IntoElement,
    ParentElement, Radians, Render, RenderOnce, Role, SharedString, Stateful,
    StatefulInteractiveElement, StyleRefinement, Styled, Transformation, Window, div, svg,
};
use gpui_component_macros::icon_named;

/// Types implementing this trait can automatically be converted to [`Icon`].
///
/// This allows you to implement a custom version of [`IconName`] that functions as a drop-in
/// replacement for other UI components.
pub trait IconNamed {
    /// Returns the embedded path of the icon.
    fn path(self) -> SharedString;
}

impl<T: IconNamed> From<T> for Icon {
    fn from(value: T) -> Self {
        Icon::build(value)
    }
}

// Generate `IconName` from the icons that `gpui-component-assets` ships.
// The `$VAR` form resolves to the absolute path published by the assets
// crate's `build.rs` (via cargo's `links` mechanism) and re-exported by
// our own `build.rs`. See `gpui_component_macros::icon_named!`'s doc
// comment for the full mechanism.
icon_named!(IconName, "$GPUI_COMPONENT_DEFAULT_ICONS_DIR");

impl IconName {
    /// Return the icon as a Entity<Icon>
    pub fn view(self, cx: &mut App) -> Entity<Icon> {
        Icon::build(self).view(cx)
    }
}

impl From<IconName> for AnyElement {
    fn from(val: IconName) -> Self {
        Icon::build(val).into_any_element()
    }
}

impl RenderOnce for IconName {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        Icon::build(self)
    }
}

#[derive(Clone, IntoElement)]
pub struct Icon {
    style: StyleRefinement,
    path: SharedString,
    size: Option<Size>,
    transformation: Option<Transformation>,
    accessibility: Option<(ElementId, SharedString)>,
}

/// Adds image semantics to the container used by an informative Icon.
fn informative_container(element: Div, id: ElementId, label: SharedString) -> Stateful<Div> {
    element.id(id).role(Role::Image).aria_label(label)
}

impl Default for Icon {
    fn default() -> Self {
        Self {
            style: StyleRefinement::default(),
            path: "".into(),
            size: None,
            transformation: None,
            accessibility: None,
        }
    }
}

impl Icon {
    pub fn new(icon: impl Into<Icon>) -> Self {
        icon.into()
    }

    /// Creates a standalone informative icon exposed as an accessible image.
    ///
    /// Icons created with [`Icon::new`] remain decorative by default because
    /// controls such as Button and Alert provide their own accessible name.
    pub fn informative(
        id: impl Into<ElementId>,
        icon: impl Into<Icon>,
        label: impl Into<SharedString>,
    ) -> Self {
        let mut icon = icon.into();
        icon.accessibility = Some((id.into(), label.into()));
        icon
    }

    fn build(name: impl IconNamed) -> Self {
        Self::default().path(name.path())
    }

    /// Set the icon path of the Assets bundle
    ///
    /// For example: `icons/foo.svg`
    pub fn path(mut self, path: impl Into<SharedString>) -> Self {
        self.path = path.into();
        self
    }

    #[cfg(any(target_os = "macos", target_os = "windows", test))]
    pub(crate) fn path_ref(&self) -> &SharedString {
        &self.path
    }

    /// Create a new view for the icon
    pub fn view(self, cx: &mut App) -> Entity<Icon> {
        cx.new(|_| self)
    }

    /// Applies a paint transformation without changing layout bounds.
    pub fn transform(mut self, transformation: Transformation) -> Self {
        self.transformation = Some(transformation);
        self
    }

    /// Creates an empty icon slot that preserves the resolved Icon dimensions.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Rotate the icon by the given angle
    pub fn rotate(mut self, radians: impl Into<Radians>) -> Self {
        self.transformation = Some(Transformation::rotate(radians));
        self
    }

    /// Resolves the inherited or explicitly requested square Icon edge.
    fn edge(&self, window: &Window) -> gpui::Pixels {
        Self::resolve_edge(
            self.size,
            window.text_style().font_size.to_pixels(window.rem_size()),
        )
    }

    /// Resolves named sizes while keeping the default coupled to surrounding text.
    fn resolve_edge(size: Option<Size>, inherited: gpui::Pixels) -> gpui::Pixels {
        match size {
            Some(Size::Size(edge)) => edge,
            Some(Size::XSmall) => gpui::px(12.),
            Some(Size::Small) => gpui::px(14.),
            Some(Size::Medium) => gpui::px(16.),
            Some(Size::Large) => gpui::px(24.),
            None => inherited,
        }
    }

    /// Builds the same visual and accessibility tree for RenderOnce and Entity rendering.
    fn render_element(&self, window: &Window) -> AnyElement {
        let edge = self.edge(window);
        let inherited_color = window.text_style().color;
        let accessibility = self.accessibility.clone();

        if let Some((id, label)) = accessibility {
            let mut graphic = svg().size_full().path(self.path.clone());
            if let Some(transformation) = self.transformation {
                graphic = graphic.with_transformation(transformation);
            }
            let mut container = div()
                .flex_none()
                .flex_shrink_0()
                .size(edge)
                .text_color(inherited_color)
                .refine_style(&self.style);
            if !self.path.is_empty() {
                container = container.child(graphic);
            }
            return informative_container(container, id, label).into_any_element();
        }

        if self.path.is_empty() {
            let placeholder = div()
                .flex_none()
                .flex_shrink_0()
                .size(edge)
                .refine_style(&self.style);
            return placeholder.into_any_element();
        }

        let mut icon = svg()
            .flex_none()
            .flex_shrink_0()
            .size(edge)
            .text_color(inherited_color)
            .path(self.path.clone())
            .refine_style(&self.style);
        if let Some(transformation) = self.transformation {
            icon = icon.with_transformation(transformation);
        }

        icon.into_any_element()
    }
}

impl Styled for Icon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Icon {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.render_element(window)
    }
}

impl From<Icon> for AnyElement {
    fn from(val: Icon) -> Self {
        val.into_any_element()
    }
}

impl Render for Icon {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.render_element(window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Element as _, px, radians};

    #[test]
    fn named_sizes_and_inherited_size_resolve_consistently() {
        assert_eq!(Icon::resolve_edge(None, px(18.)), px(18.));
        assert_eq!(Icon::resolve_edge(Some(Size::XSmall), px(18.)), px(12.));
        assert_eq!(Icon::resolve_edge(Some(Size::Small), px(18.)), px(14.));
        assert_eq!(Icon::resolve_edge(Some(Size::Medium), px(18.)), px(16.));
        assert_eq!(Icon::resolve_edge(Some(Size::Large), px(18.)), px(24.));
        assert_eq!(
            Icon::resolve_edge(Some(Size::Size(px(20.))), px(18.)),
            px(20.)
        );
    }

    #[test]
    fn clone_preserves_transformation_style_and_accessibility() {
        let transformation = Transformation::rotate(radians(std::f32::consts::FRAC_PI_4));
        let icon = Icon::informative(
            "status-icon",
            Icon::new(IconName::Info)
                .large()
                .w(px(32.))
                .transform(transformation),
            "Information",
        );
        let clone = icon.clone();

        assert_eq!(clone.path, icon.path);
        assert_eq!(clone.size, Some(Size::Large));
        assert_eq!(clone.transformation, Some(transformation));
        assert_eq!(clone.style.size.width, icon.style.size.width);
        assert_eq!(clone.accessibility, icon.accessibility);
    }

    #[test]
    fn informative_icon_exposes_image_semantics() {
        let icon = Icon::informative("accessible-icon", IconName::CircleCheck, "Ready");
        let (id, label) = icon
            .accessibility
            .clone()
            .expect("informative Icon must retain accessibility metadata");
        let semantic_icon = informative_container(div(), id, label);
        let mut node = gpui::accesskit::Node::new(Role::Image);
        semantic_icon.write_a11y_info(&mut node);

        assert_eq!(semantic_icon.a11y_role(), Some(Role::Image));
        assert_eq!(node.label(), Some("Ready"));
        assert!(Icon::new(IconName::CircleCheck).accessibility.is_none());
    }
}
