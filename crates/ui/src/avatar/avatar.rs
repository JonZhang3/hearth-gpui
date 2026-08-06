use gpui::{
    AnyElement, App, BoxShadow, Div, ElementId, ImageSource, InteractiveElement, Interactivity,
    IntoElement, ObjectFit, ParentElement as _, Pixels, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, StyledImage as _, Window, div, img,
    point, prelude::FluentBuilder as _, px,
};

use crate::{ActiveTheme, AvatarSizeMetrics, Icon, IconName, Sizable, Size, StyledExt};

/// The image slot rendered by an [`Avatar`].
pub struct AvatarImage {
    source: ImageSource,
    style: StyleRefinement,
}

impl AvatarImage {
    /// Creates an Avatar image from a GPUI image source.
    pub fn new(source: impl Into<ImageSource>) -> Self {
        Self {
            source: source.into(),
            style: StyleRefinement::default(),
        }
    }
}

impl Styled for AvatarImage {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

#[derive(Clone)]
enum AvatarFallbackContent {
    Text(SharedString),
    Icon(Box<Icon>),
}

/// The loading and error fallback rendered by an [`Avatar`].
#[derive(Clone)]
pub struct AvatarFallback {
    content: AvatarFallbackContent,
    style: StyleRefinement,
}

impl AvatarFallback {
    /// Creates a text fallback, typically one or two initials.
    pub fn text(value: impl Into<SharedString>) -> Self {
        Self {
            content: AvatarFallbackContent::Text(value.into()),
            style: StyleRefinement::default(),
        }
    }

    /// Creates an icon fallback.
    pub fn icon(icon: impl Into<Icon>) -> Self {
        Self {
            content: AvatarFallbackContent::Icon(Box::new(icon.into())),
            style: StyleRefinement::default(),
        }
    }

    fn render_with_metrics(
        self,
        metrics: AvatarSizeMetrics,
        background: gpui::Hsla,
        foreground: gpui::Hsla,
    ) -> AnyElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(background)
            .text_color(foreground)
            .refine_style(&self.style)
            .map(|this| match self.content {
                AvatarFallbackContent::Text(text) => {
                    this.text_size(metrics.fallback_text_size).child(text)
                }
                AvatarFallbackContent::Icon(icon) => {
                    this.child(icon.with_size(metrics.fallback_icon_size))
                }
            })
            .into_any_element()
    }
}

impl Styled for AvatarFallback {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// A status or icon marker anchored to the lower-right of an [`Avatar`].
pub struct AvatarBadge {
    base: Div,
    style: StyleRefinement,
    icon: Option<Icon>,
}

impl AvatarBadge {
    /// Creates an empty status-dot badge.
    pub fn new() -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            icon: None,
        }
    }

    /// Adds an icon to the badge. Small and extra-small Avatars hide it.
    pub fn child(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    fn render_with_metrics(
        self,
        metrics: AvatarSizeMetrics,
        ring_width: Pixels,
        ring_color: gpui::Hsla,
        background: gpui::Hsla,
        foreground: gpui::Hsla,
    ) -> AnyElement {
        let ring = BoxShadow {
            color: ring_color,
            offset: point(px(0.), px(0.)),
            blur_radius: px(0.),
            spread_radius: ring_width,
            inset: false,
        };

        self.base
            .absolute()
            .right_0()
            .bottom_0()
            .size(metrics.badge_size)
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(background)
            .text_color(foreground)
            .shadow(vec![ring])
            .refine_style(&self.style)
            .when_some(
                self.icon.zip(metrics.badge_icon_size),
                |this, (icon, icon_size)| this.child(icon.with_size(icon_size)),
            )
            .into_any_element()
    }
}

impl Default for AvatarBadge {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for AvatarBadge {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// A user or organization image with explicit image, fallback, and badge slots.
#[derive(IntoElement)]
pub struct Avatar {
    base: Div,
    style: StyleRefinement,
    id: Option<ElementId>,
    accessibility_label: Option<SharedString>,
    image: Option<AvatarImage>,
    fallback: Option<AvatarFallback>,
    badge: Option<AvatarBadge>,
    size: Size,
    grouped: bool,
}

impl Avatar {
    /// Creates a semantic Avatar with an accessible name.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            id: Some(id.into()),
            accessibility_label: Some(label.into()),
            image: None,
            fallback: None,
            badge: None,
            size: Size::Medium,
            grouped: false,
        }
    }

    /// Creates a decorative Avatar that is omitted from the accessibility tree.
    pub fn decorative() -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            id: None,
            accessibility_label: None,
            image: None,
            fallback: None,
            badge: None,
            size: Size::Medium,
            grouped: false,
        }
    }

    /// Sets the image slot.
    pub fn image(mut self, image: AvatarImage) -> Self {
        self.image = Some(image);
        self
    }

    /// Sets the loading and error fallback slot.
    pub fn fallback(mut self, fallback: AvatarFallback) -> Self {
        self.fallback = Some(fallback);
        self
    }

    /// Sets the lower-right badge slot.
    pub fn badge(mut self, badge: AvatarBadge) -> Self {
        self.badge = Some(badge);
        self
    }

    pub(super) fn grouped(mut self) -> Self {
        self.grouped = true;
        self
    }
}

impl Sizable for Avatar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Avatar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for Avatar {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for Avatar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let avatar_metrics = cx.theme().style.avatars;
        let metrics = avatar_metrics.for_size(self.size);
        let fallback = self
            .fallback
            .unwrap_or_else(|| AvatarFallback::icon(IconName::User));
        let fallback_background = cx.theme().muted;
        let fallback_foreground = cx.theme().muted_foreground;
        let mut media_style = StyleRefinement::default();
        media_style.corner_radii = self.style.corner_radii.clone();

        let media = match self.image {
            Some(image) => {
                let loading_fallback = fallback.clone();
                let error_fallback = fallback;
                div().size_full().child(
                    img(image.source)
                        .size_full()
                        .rounded_full()
                        .object_fit(ObjectFit::Cover)
                        .with_loading(move || {
                            loading_fallback.clone().render_with_metrics(
                                metrics,
                                fallback_background,
                                fallback_foreground,
                            )
                        })
                        .with_fallback(move || {
                            error_fallback.clone().render_with_metrics(
                                metrics,
                                fallback_background,
                                fallback_foreground,
                            )
                        })
                        .refine_style(&image.style),
                )
            }
            None => div().size_full().child(fallback.render_with_metrics(
                metrics,
                fallback_background,
                fallback_foreground,
            )),
        };

        let group_ring = BoxShadow {
            color: cx.theme().background,
            offset: point(px(0.), px(0.)),
            blur_radius: px(0.),
            spread_radius: avatar_metrics.group_ring_width,
            inset: false,
        };
        let accessibility = self.id.zip(self.accessibility_label);

        let avatar = self
            .base
            .relative()
            .size(metrics.diameter)
            .flex_shrink_0()
            .rounded_full()
            .refine_style(&self.style)
            .child(
                div()
                    .size_full()
                    .rounded_full()
                    .overflow_hidden()
                    .refine_style(&media_style)
                    .child(media),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded_full()
                    .border(avatar_metrics.outline_width)
                    .border_color(cx.theme().border)
                    .refine_style(&media_style),
            )
            .when(self.grouped, |this| this.shadow(vec![group_ring]))
            .when_some(self.badge, |this, badge| {
                this.child(badge.render_with_metrics(
                    metrics,
                    avatar_metrics.group_ring_width,
                    cx.theme().background,
                    cx.theme().primary,
                    cx.theme().primary_foreground,
                ))
            });

        match accessibility {
            Some((id, label)) => avatar
                .id(id)
                .role(Role::Image)
                .aria_label(label)
                .into_any_element(),
            None => avatar.into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn avatar_builder_preserves_semantic_slots(_cx: &mut gpui::TestAppContext) {
        let avatar = Avatar::new("jason", "Jason Lee")
            .image(AvatarImage::new("avatar.png"))
            .fallback(AvatarFallback::text("JL"))
            .badge(AvatarBadge::new().child(IconName::Plus))
            .large();

        assert_eq!(avatar.accessibility_label, Some("Jason Lee".into()));
        assert!(avatar.image.is_some());
        assert!(avatar.fallback.is_some());
        assert!(avatar.badge.is_some());
        assert_eq!(avatar.size, Size::Large);
    }

    #[gpui::test]
    fn decorative_avatar_has_no_accessible_label(_cx: &mut gpui::TestAppContext) {
        assert!(Avatar::decorative().accessibility_label.is_none());
    }
}
