use gpui::{
    AnyElement, App, Hsla, InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px, relative,
};

use crate::{ActiveTheme, Icon, Sizable, Size, StyledExt, h_flex, white};

/// Applies one of the shadcn-compatible visual variants to a [`Badge`].
pub trait BadgeVariants: Sized {
    /// Sets the visual variant.
    fn with_variant(self, variant: BadgeVariant) -> Self;

    /// Uses the secondary surface.
    fn secondary(self) -> Self {
        self.with_variant(BadgeVariant::Secondary)
    }

    /// Uses the destructive status surface.
    fn destructive(self) -> Self {
        self.with_variant(BadgeVariant::Destructive)
    }

    /// Uses the bordered outline surface.
    fn outline(self) -> Self {
        self.with_variant(BadgeVariant::Outline)
    }

    /// Uses the low-emphasis ghost surface.
    fn ghost(self) -> Self {
        self.with_variant(BadgeVariant::Ghost)
    }

    /// Uses the inline link treatment without adding interaction semantics.
    fn link(self) -> Self {
        self.with_variant(BadgeVariant::Link)
    }
}

/// The visual variant of an inline [`Badge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    #[default]
    Default,
    Secondary,
    Destructive,
    Outline,
    Ghost,
    Link,
}

/// A compact, non-interactive label for status or metadata.
#[derive(IntoElement)]
pub struct Badge {
    style: StyleRefinement,
    variant: BadgeVariant,
    leading: Option<AnyElement>,
    trailing: Option<AnyElement>,
    children: Vec<AnyElement>,
}

impl Badge {
    /// Creates a badge with the default variant.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            variant: BadgeVariant::Default,
            leading: None,
            trailing: None,
            children: Vec::new(),
        }
    }

    /// Sets content in the compact leading slot.
    pub fn leading(mut self, element: impl IntoElement) -> Self {
        self.leading = Some(element.into_any_element());
        self
    }

    /// Sets content in the compact trailing slot.
    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing = Some(element.into_any_element());
        self
    }
}

impl BadgeVariants for Badge {
    fn with_variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl ParentElement for Badge {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Badge {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let has_leading = self.leading.is_some();
        let has_trailing = self.trailing.is_some();
        let (background, foreground, border) = match self.variant {
            BadgeVariant::Default => (
                cx.theme().primary,
                cx.theme().primary_foreground,
                cx.theme().transparent,
            ),
            BadgeVariant::Secondary => (
                cx.theme().secondary,
                cx.theme().secondary_foreground,
                cx.theme().transparent,
            ),
            BadgeVariant::Destructive => (
                cx.theme()
                    .danger
                    .opacity(if cx.theme().is_dark() { 0.2 } else { 0.1 }),
                cx.theme().danger,
                cx.theme().transparent,
            ),
            BadgeVariant::Outline => (
                cx.theme().transparent,
                cx.theme().foreground,
                cx.theme().border,
            ),
            BadgeVariant::Ghost | BadgeVariant::Link => (
                cx.theme().transparent,
                if self.variant == BadgeVariant::Link {
                    cx.theme().primary
                } else {
                    cx.theme().foreground
                },
                cx.theme().transparent,
            ),
        };

        h_flex()
            .h(px(20.))
            .flex_shrink_0()
            .justify_center()
            .gap_1()
            .pl(if has_leading { px(6.) } else { px(8.) })
            .pr(if has_trailing { px(6.) } else { px(8.) })
            .overflow_hidden()
            .whitespace_nowrap()
            .rounded(px(10.))
            .border_1()
            .border_color(border)
            .bg(background)
            .text_color(foreground)
            .text_xs()
            .font_medium()
            .map(|this| match self.variant {
                BadgeVariant::Ghost => this.hover(|this| {
                    this.bg(cx
                        .theme()
                        .muted
                        .opacity(if cx.theme().is_dark() { 0.5 } else { 1.0 }))
                        .text_color(cx.theme().muted_foreground)
                }),
                BadgeVariant::Link => this.hover(|this| this.text_decoration_1()),
                _ => this,
            })
            .refine_style(&self.style)
            .when_some(self.leading, |this, leading| {
                this.child(
                    h_flex()
                        .size(px(12.))
                        .flex_none()
                        .justify_center()
                        .child(leading),
                )
            })
            .children(self.children)
            .when_some(self.trailing, |this, trailing| {
                this.child(
                    h_flex()
                        .size(px(12.))
                        .flex_none()
                        .justify_center()
                        .child(trailing),
                )
            })
    }
}

#[derive(Default, Clone)]
enum OverlayBadgeContent {
    #[default]
    Number,
    Dot,
    Icon(Box<Icon>),
}

/// A count, dot, or icon overlaid on a target element.
#[derive(IntoElement)]
pub struct OverlayBadge {
    style: StyleRefinement,
    count: usize,
    max: usize,
    content: OverlayBadgeContent,
    children: Vec<AnyElement>,
    color: Option<Hsla>,
    size: Size,
}

impl OverlayBadge {
    /// Creates an empty numeric overlay badge.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            count: 0,
            max: 99,
            content: OverlayBadgeContent::Number,
            color: None,
            children: Vec::new(),
            size: Size::default(),
        }
    }

    /// Uses a status dot in the upper-right corner.
    pub fn dot(mut self) -> Self {
        self.content = OverlayBadgeContent::Dot;
        self
    }

    /// Sets the displayed count. A zero count hides the numeric badge.
    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    /// Uses an icon badge in the lower-right corner.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.content = OverlayBadgeContent::Icon(Box::new(icon.into()));
        self
    }

    /// Sets the largest count displayed without a trailing plus sign.
    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }

    /// Overrides the badge surface color.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl ParentElement for OverlayBadge {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for OverlayBadge {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for OverlayBadge {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for OverlayBadge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let visible = match self.content {
            OverlayBadgeContent::Number => self.count > 0,
            OverlayBadgeContent::Dot | OverlayBadgeContent::Icon(_) => true,
        };

        let (size, text_size) = match self.size {
            Size::Large => (px(24.), px(14.)),
            Size::Medium | Size::Size(_) => (px(16.), px(10.)),
            Size::Small | Size::XSmall => (px(10.), px(8.)),
        };

        div()
            .relative()
            .refine_style(&self.style)
            .children(self.children)
            .when(visible, |this| {
                this.child(
                    h_flex()
                        .absolute()
                        .justify_center()
                        .items_center()
                        .rounded_full()
                        .bg(self.color.unwrap_or(cx.theme().red))
                        .text_color(white())
                        .text_size(text_size)
                        .map(|this| match self.content {
                            OverlayBadgeContent::Dot => this.top_0().right_0().size(px(6.)),
                            OverlayBadgeContent::Number => {
                                let count = format_overlay_count(self.count, self.max);
                                let (top, right) = match self.size {
                                    Size::Large => (px(2.), -px(count.len() as f32)),
                                    Size::Medium | Size::Size(_) => {
                                        (-px(3.), -px(3.) * count.len())
                                    }
                                    Size::Small | Size::XSmall => (-px(4.), -px(4.) * count.len()),
                                };

                                this.top(top)
                                    .right(right)
                                    .py_0p5()
                                    .px_0p5()
                                    .min_w_3p5()
                                    .text_size(px(10.))
                                    .line_height(relative(1.))
                                    .child(count)
                            }
                            OverlayBadgeContent::Icon(icon) => this
                                .right_0()
                                .bottom_0()
                                .size(size)
                                .border_1()
                                .border_color(cx.theme().background)
                                .child(*icon),
                        }),
                )
            })
    }
}

fn format_overlay_count(count: usize, max: usize) -> String {
    if count > max {
        format!("{max}+")
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::div;

    #[test]
    fn test_badge_builder() {
        let badge = Badge::new()
            .secondary()
            .leading(div())
            .child("Badge")
            .trailing(div());

        assert_eq!(badge.variant, BadgeVariant::Secondary);
        assert!(badge.leading.is_some());
        assert!(badge.trailing.is_some());
        assert_eq!(badge.children.len(), 1);
    }

    #[test]
    fn test_badge_variants() {
        assert_eq!(Badge::new().variant, BadgeVariant::Default);
        assert_eq!(
            Badge::new().destructive().variant,
            BadgeVariant::Destructive
        );
        assert_eq!(Badge::new().outline().variant, BadgeVariant::Outline);
        assert_eq!(Badge::new().ghost().variant, BadgeVariant::Ghost);
        assert_eq!(Badge::new().link().variant, BadgeVariant::Link);
    }

    #[test]
    fn test_overlay_badge_builder() {
        let badge = OverlayBadge::new().count(120).max(99).large().child(div());

        assert_eq!(badge.count, 120);
        assert_eq!(badge.max, 99);
        assert_eq!(badge.size, Size::Large);
        assert_eq!(badge.children.len(), 1);
    }

    #[test]
    fn test_overlay_count_formatting() {
        assert_eq!(format_overlay_count(0, 99), "0");
        assert_eq!(format_overlay_count(99, 99), "99");
        assert_eq!(format_overlay_count(100, 99), "99+");
    }
}
