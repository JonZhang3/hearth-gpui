use gpui::{
    AnyElement, App, IntoElement, ParentElement, Pixels, RenderOnce, StyleRefinement, Styled,
    Window, prelude::FluentBuilder as _, px,
};
use smallvec::SmallVec;

use crate::{ActiveTheme, Density, StylePreset, StyledExt, h_flex};

/// Component-local geometry derived from the active Style Preset.
#[derive(Debug, Clone, Copy, PartialEq)]
struct StatusBarMetrics {
    padding_x: Pixels,
    padding_y: Pixels,
    item_gap: Pixels,
    region_gap: Pixels,
    min_height: Pixels,
}

impl StatusBarMetrics {
    /// Resolves StatusBar density without coupling the component to preset IDs.
    fn resolve(style: &StylePreset) -> Self {
        let (padding_x, padding_y, item_gap, region_gap) = match style.density {
            Density::Compact => (px(6.), px(2.), px(4.), px(6.)),
            Density::Standard => (px(8.), px(4.), px(8.), px(8.)),
            Density::Comfortable => (px(12.), px(6.), px(12.), px(12.)),
        };

        Self {
            padding_x,
            padding_y,
            item_gap,
            region_gap,
            min_height: style.controls.xs.height + padding_y * 2.,
        }
    }
}

/// A horizontal status bar, usually placed at the bottom of a window or pane.
///
/// It is split into three regions — `left`, `center`, and `right`. This mirrors
/// the status bars found in native UI frameworks (Windows `StatusStrip`, WPF
/// `StatusBar`, macOS `NSStatusBar`): a container that holds a row of items
/// aligned to either end.
///
/// Each region accepts any [`IntoElement`], so a string, an [`Icon`](crate::Icon),
/// a ghost `Button`, a vertical `Separator`, a custom layout, etc. can be passed
/// directly. Use a plain string for a non-interactive label.
///
/// `left` and `right` pin items to each end. `child`/`children` add to the
/// center region, whose alignment follows the pinned ends: centered with both
/// `left` and `right`, end-aligned with only `left`, and start-aligned
/// otherwise (only `right`, or neither — like a plain container).
///
/// ```
/// use gpui_component::status_bar::StatusBar;
///
/// let _ = StatusBar::new().left("Ln 1, Col 1").right("UTF-8");
/// ```
#[derive(IntoElement)]
pub struct StatusBar {
    style: StyleRefinement,
    left: SmallVec<[AnyElement; 1]>,
    right: SmallVec<[AnyElement; 1]>,
    children: SmallVec<[AnyElement; 1]>,
}

impl StatusBar {
    /// Create a new, empty [`StatusBar`].
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            left: SmallVec::new(),
            right: SmallVec::new(),
            children: SmallVec::new(),
        }
    }

    /// Append an element to the left region. Call multiple times to add more.
    pub fn left(mut self, child: impl IntoElement) -> Self {
        self.left.push(child.into_any_element());
        self
    }

    /// Append an element to the right region. Call multiple times to add more.
    pub fn right(mut self, child: impl IntoElement) -> Self {
        self.right.push(child.into_any_element());
        self
    }
}

/// `child` / `children` add to the center region, so a `StatusBar` without
/// `left`/`right` items behaves like a plain container.
impl ParentElement for StatusBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for StatusBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // The center aligns by which ends are pinned: centered with both left
        // and right, end-aligned with only left, otherwise start-aligned (only
        // right, or neither) — so a bar with just `child`s reads like a container.
        let has_left = !self.left.is_empty();
        let has_right = !self.right.is_empty();
        let metrics = StatusBarMetrics::resolve(&cx.theme().style);
        let pinned_region = || {
            h_flex()
                .min_w_0()
                .flex_shrink_0()
                .items_center()
                .gap(metrics.item_gap)
        };

        h_flex()
            .w_full()
            .min_w_0()
            .min_h(metrics.min_height)
            .flex_shrink_0()
            .items_center()
            .gap(metrics.region_gap)
            .py(metrics.padding_y)
            .px(metrics.padding_x)
            .border_t_1()
            .border_color(cx.theme().status_bar_border)
            .bg(cx.theme().tokens.status_bar)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .when(has_left, |this| {
                this.child(pinned_region().children(self.left))
            })
            .child(
                h_flex()
                    .min_w_0()
                    .flex_1()
                    .overflow_hidden()
                    .items_center()
                    .gap(metrics.item_gap)
                    .when(has_left && has_right, |this| this.justify_center())
                    .when(has_left && !has_right, |this| this.justify_end())
                    .children(self.children),
            )
            .when(has_right, |this| {
                this.child(pinned_region().children(self.right))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{StatusBar, StatusBarMetrics};
    use crate::StylePreset;
    use gpui::ParentElement as _;

    #[test]
    fn status_bar_metrics_follow_preset_density() {
        let compact = StatusBarMetrics::resolve(&StylePreset::nova());
        let standard = StatusBarMetrics::resolve(&StylePreset::vega());
        let comfortable = StatusBarMetrics::resolve(&StylePreset::maia());

        assert!(compact.padding_x < standard.padding_x);
        assert!(standard.padding_x < comfortable.padding_x);
        assert!(compact.padding_y < standard.padding_y);
        assert!(standard.padding_y < comfortable.padding_y);
        assert!(compact.item_gap < standard.item_gap);
        assert!(standard.item_gap < comfortable.item_gap);
        assert!(compact.min_height < standard.min_height);
        assert!(standard.min_height < comfortable.min_height);
    }

    #[test]
    fn status_bar_builder_preserves_regions() {
        let bar = StatusBar::new().left("left").child("center").right("right");

        assert_eq!(bar.left.len(), 1);
        assert_eq!(bar.children.len(), 1);
        assert_eq!(bar.right.len(), 1);
    }
}
