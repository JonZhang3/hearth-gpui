use gpui::{
    AnyElement, App, Background, ElementId, InteractiveElement as _, IntoElement, ParentElement,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder, px, relative,
};
use smallvec::SmallVec;

use crate::{ActiveTheme, Density, StylePreset, StyledExt as _, v_flex};

/// Component-local geometry derived from semantic Style Preset density.
#[derive(Debug, Clone, Copy, PartialEq)]
struct GroupBoxMetrics {
    content_padding: Pixels,
    content_gap: Pixels,
    section_gap: Pixels,
    title_line_height: f32,
    radius: Pixels,
}

impl GroupBoxMetrics {
    /// Resolves GroupBox geometry without branching on a concrete preset ID.
    fn resolve(style: &StylePreset) -> Self {
        let (content_padding, content_gap, section_gap, title_line_height) = match style.density {
            Density::Compact => (px(12.), px(8.), px(8.), 1.25),
            Density::Standard => (px(16.), px(12.), px(12.), 1.375),
            Density::Comfortable => (px(20.), px(16.), px(16.), 1.5),
        };

        Self {
            content_padding,
            content_gap,
            section_gap,
            title_line_height,
            radius: style.radii.md,
        }
    }
}

/// The variant of the GroupBox.
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub enum GroupBoxVariant {
    #[default]
    Normal,
    Fill,
    Outline,
}

/// Trait to add GroupBox variant methods to elements.
pub trait GroupBoxVariants: Sized {
    /// Set the variant of the [`GroupBox`].
    fn with_variant(self, variant: GroupBoxVariant) -> Self;
    /// Set to use [`GroupBoxVariant::Normal`] to GroupBox.
    fn normal(mut self) -> Self {
        self = self.with_variant(GroupBoxVariant::Normal);
        self
    }
    /// Set to use [`GroupBoxVariant::Fill`] to GroupBox.
    fn fill(mut self) -> Self {
        self = self.with_variant(GroupBoxVariant::Fill);
        self
    }
    /// Set to use [`GroupBoxVariant::Outline`] to GroupBox.
    fn outline(mut self) -> Self {
        self = self.with_variant(GroupBoxVariant::Outline);
        self
    }
}

impl GroupBoxVariant {
    /// Create a GroupBoxVariant from a string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fill" => GroupBoxVariant::Fill,
            "outline" => GroupBoxVariant::Outline,
            _ => GroupBoxVariant::Normal,
        }
    }

    /// Convert the GroupBoxVariant to a string.
    pub fn as_str(&self) -> &str {
        match self {
            GroupBoxVariant::Normal => "normal",
            GroupBoxVariant::Fill => "fill",
            GroupBoxVariant::Outline => "outline",
        }
    }
}

/// GroupBox is a styled container element that with
/// an optional title to groups related content together.
#[derive(IntoElement)]
pub struct GroupBox {
    id: Option<ElementId>,
    variant: GroupBoxVariant,
    style: StyleRefinement,
    title_style: StyleRefinement,
    title: Option<AnyElement>,
    aria_label: Option<SharedString>,
    content_style: StyleRefinement,
    children: SmallVec<[AnyElement; 1]>,
}

impl GroupBox {
    /// Create a new GroupBox.
    pub fn new() -> Self {
        Self {
            id: None,
            variant: GroupBoxVariant::default(),
            style: StyleRefinement::default(),
            title_style: StyleRefinement::default(),
            content_style: StyleRefinement::default(),
            title: None,
            aria_label: None,
            children: SmallVec::new(),
        }
    }

    /// Set the id of the group box, default is None.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the title of the group box, default is None.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }

    /// Set the accessible name for this logical content group.
    ///
    /// An explicit `id` is required for GPUI to expose the Group node.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set the style of the title of the group box to override the default style, default is None.
    pub fn title_style(mut self, style: StyleRefinement) -> Self {
        self.title_style = style;
        self
    }

    /// Set the style of the content of the group box to override the default style, default is None.
    pub fn content_style(mut self, style: StyleRefinement) -> Self {
        self.content_style = style;
        self
    }
}

impl ParentElement for GroupBox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for GroupBox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl GroupBoxVariants for GroupBox {
    fn with_variant(mut self, variant: GroupBoxVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl RenderOnce for GroupBox {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = GroupBoxMetrics::resolve(&cx.theme().style);
        let (bg, border, has_paddings): (Option<Background>, _, _) = match self.variant {
            GroupBoxVariant::Normal => (None, None, false),
            GroupBoxVariant::Fill => (Some(cx.theme().tokens.group_box.into()), None, true),
            GroupBoxVariant::Outline => (None, Some(cx.theme().border), true),
        };

        let group = v_flex()
            .w_full()
            .min_w_0()
            .gap(metrics.section_gap)
            .refine_style(&self.style)
            .when_some(self.title, |this, title| {
                this.child(
                    div()
                        .min_w_0()
                        .text_color(cx.theme().group_box_title_foreground)
                        .line_height(relative(metrics.title_line_height))
                        .refine_style(&self.title_style)
                        .child(title),
                )
            })
            .child(
                v_flex()
                    .min_w_0()
                    .when_some(bg, |this, bg| this.bg(bg))
                    .when_some(border, |this, border| this.border_color(border).border_1())
                    .text_color(cx.theme().group_box_foreground)
                    .when(has_paddings, |this| this.p(metrics.content_padding))
                    .gap(metrics.content_gap)
                    .rounded(metrics.radius)
                    .refine_style(&self.content_style)
                    .children(self.children),
            );

        if let Some(id) = self.id {
            group
                .id(id)
                .role(Role::Group)
                .when_some(self.aria_label, |this, label| this.aria_label(label))
                .into_any_element()
        } else {
            group.into_any_element()
        }
    }
}

#[cfg(test)]
mod test {
    use super::{GroupBox, GroupBoxMetrics, GroupBoxVariant, GroupBoxVariants as _};
    use crate::StylePreset;

    #[test]
    fn test_group_variant_from_str() {
        assert_eq!(GroupBoxVariant::from_str("normal"), GroupBoxVariant::Normal);
        assert_eq!(GroupBoxVariant::from_str("fill"), GroupBoxVariant::Fill);
        assert_eq!(
            GroupBoxVariant::from_str("outline"),
            GroupBoxVariant::Outline
        );
        assert_eq!(GroupBoxVariant::from_str("other"), GroupBoxVariant::Normal);

        assert_eq!(GroupBoxVariant::from_str("FILL"), GroupBoxVariant::Fill);
        assert_eq!(
            GroupBoxVariant::from_str("OutLine"),
            GroupBoxVariant::Outline
        );

        assert_eq!(GroupBoxVariant::Normal.as_str(), "normal");
        assert_eq!(GroupBoxVariant::Fill.as_str(), "fill");
        assert_eq!(GroupBoxVariant::Outline.as_str(), "outline");
    }

    #[test]
    fn test_group_box_metrics_follow_style_density() {
        let compact = GroupBoxMetrics::resolve(&StylePreset::nova());
        let standard = GroupBoxMetrics::resolve(&StylePreset::vega());
        let comfortable = GroupBoxMetrics::resolve(&StylePreset::maia());

        assert!(compact.content_padding < standard.content_padding);
        assert!(standard.content_padding < comfortable.content_padding);
        assert!(compact.content_gap < standard.content_gap);
        assert!(standard.content_gap < comfortable.content_gap);
        assert_eq!(compact.radius, StylePreset::nova().radii.md);
        assert_eq!(standard.radius, StylePreset::vega().radii.md);
        assert_eq!(comfortable.radius, StylePreset::maia().radii.md);
    }

    #[test]
    fn test_group_box_builder() {
        let group = GroupBox::new()
            .id("preferences")
            .aria_label("Preferences")
            .outline();

        assert_eq!(group.id, Some("preferences".into()));
        assert_eq!(group.aria_label.as_deref(), Some("Preferences"));
        assert_eq!(group.variant, GroupBoxVariant::Outline);
    }
}
