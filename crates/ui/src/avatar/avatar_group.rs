use gpui::{
    AnyElement, BoxShadow, Div, InteractiveElement, Interactivity, IntoElement, ParentElement as _,
    Pixels, RenderOnce, SharedString, StyleRefinement, Styled, div, point,
    prelude::FluentBuilder as _, px,
};

use crate::{ActiveTheme, AvatarSizeMetrics, Icon, Sizable, Size, StyledExt as _};

use super::Avatar;

enum AvatarGroupCountContent {
    Text(SharedString),
    Icon(Box<Icon>),
}

/// A text or icon item displayed at the end of an [`AvatarGroup`].
pub struct AvatarGroupCount {
    base: Div,
    style: StyleRefinement,
    content: AvatarGroupCountContent,
}

impl AvatarGroupCount {
    /// Creates a text count such as `+3`.
    pub fn text(value: impl Into<SharedString>) -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            content: AvatarGroupCountContent::Text(value.into()),
        }
    }

    /// Creates an icon count item.
    pub fn icon(icon: impl Into<Icon>) -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            content: AvatarGroupCountContent::Icon(Box::new(icon.into())),
        }
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
            .relative()
            .size(metrics.diameter)
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(background)
            .text_color(foreground)
            .shadow(vec![ring])
            .refine_style(&self.style)
            .map(|this| match self.content {
                AvatarGroupCountContent::Text(text) => {
                    this.text_size(metrics.fallback_text_size).child(text)
                }
                AvatarGroupCountContent::Icon(icon) => {
                    this.child(icon.with_size(metrics.count_icon_size))
                }
            })
            .into_any_element()
    }
}

impl Styled for AvatarGroupCount {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// A compact, ordered collection of uniformly sized Avatars.
#[derive(IntoElement)]
pub struct AvatarGroup {
    base: Div,
    style: StyleRefinement,
    avatars: Vec<Avatar>,
    count: Option<AvatarGroupCount>,
    size: Size,
}

impl AvatarGroup {
    /// Creates an empty AvatarGroup.
    pub fn new() -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            avatars: Vec::new(),
            count: None,
            size: Size::Medium,
        }
    }

    /// Adds an Avatar while preserving insertion order.
    pub fn avatar(mut self, avatar: Avatar) -> Self {
        self.avatars.push(avatar);
        self
    }

    /// Adds multiple Avatars while preserving insertion order.
    pub fn avatars(mut self, avatars: impl IntoIterator<Item = Avatar>) -> Self {
        self.avatars.extend(avatars);
        self
    }

    /// Sets the explicit trailing count or action item.
    pub fn count(mut self, count: AvatarGroupCount) -> Self {
        self.count = Some(count);
        self
    }
}

impl Default for AvatarGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl Sizable for AvatarGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for AvatarGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for AvatarGroup {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for AvatarGroup {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let avatar_metrics = cx.theme().style.avatars;
        let metrics = avatar_metrics.for_size(self.size);
        let avatar_count = self.avatars.len();

        self.base
            .h_flex()
            .refine_style(&self.style)
            .children(self.avatars.into_iter().enumerate().map(|(index, avatar)| {
                avatar
                    .with_size(self.size)
                    .grouped()
                    .when(index > 0, |this| this.ml(-avatar_metrics.group_overlap))
            }))
            .when_some(self.count, |this, count| {
                this.child(
                    div()
                        .when(avatar_count > 0, |this| {
                            this.ml(-avatar_metrics.group_overlap)
                        })
                        .child(count.render_with_metrics(
                            metrics,
                            avatar_metrics.group_ring_width,
                            cx.theme().background,
                            cx.theme().muted,
                            cx.theme().muted_foreground,
                        )),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avatar::AvatarFallback;

    #[gpui::test]
    fn avatar_group_builder_preserves_all_items(_cx: &mut gpui::TestAppContext) {
        let group = AvatarGroup::new()
            .avatar(Avatar::new("alice", "Alice").fallback(AvatarFallback::text("A")))
            .avatar(Avatar::new("bob", "Bob").fallback(AvatarFallback::text("B")))
            .avatar(Avatar::new("charlie", "Charlie").fallback(AvatarFallback::text("C")))
            .count(AvatarGroupCount::text("+3"))
            .large();

        assert_eq!(group.avatars.len(), 3);
        assert!(group.count.is_some());
        assert_eq!(group.size, Size::Large);
    }
}
