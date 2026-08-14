// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `read_only`, `aria_label`.
// - Added or exposed behavior through `rating_metrics`, `rating_after_click`, `rating_after_key`,
//   `read_only`, `aria_label`, `builder_retains_requested_value_and_accessibility_options`,
//   `click_selects_lower_star_and_toggles_current_star_down`, `keyboard_navigation_is_bounded` and
//   1 more.
// - Reworked Rating around accessibility semantics and ARIA state, semantic Style Preset geometry
//   and density, keyboard navigation and activation behavior, focus-visible and focus restoration
//   behavior.
use std::rc::Rc;

use crate::{
    ActiveTheme, Density, Disableable, Icon, IconName, Sizable, Size, StylePreset, StyledExt,
    accessibility::accessibility_state, h_flex,
};
use gpui::{
    App, ClickEvent, ElementId, Hsla, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, div, prelude::FluentBuilder as _, px,
};

/// Geometry resolved from semantic Style Preset metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RatingMetrics {
    icon_edge: Pixels,
    item_padding: Pixels,
    gap: Pixels,
    radius: Pixels,
}

/// Resolves Rating geometry without branching on a Style Preset ID.
fn rating_metrics(size: Size, style: &StylePreset) -> RatingMetrics {
    let control = style.controls.for_size(size);
    let icon_edge = match size {
        Size::XSmall => px(12.),
        Size::Small => px(14.),
        Size::Medium => px(16.),
        Size::Large => px(24.),
        Size::Size(edge) => edge,
    };

    RatingMetrics {
        icon_edge,
        item_padding: control.gap * 0.5,
        gap: match style.density {
            Density::Compact => px(0.),
            Density::Standard | Density::Comfortable => px(1.),
        },
        radius: style.radii.md,
    }
}

/// Returns the value selected by clicking a star.
fn rating_after_click(current: usize, star: usize) -> usize {
    if current == star {
        star.saturating_sub(1)
    } else {
        star
    }
}

/// Resolves slider-style keyboard navigation for a Rating.
fn rating_after_key(key: &str, current: usize, max: usize) -> Option<usize> {
    match key {
        "left" | "down" => Some(current.saturating_sub(1)),
        "right" | "up" => Some(current.saturating_add(1).min(max)),
        "home" => Some(0),
        "end" => Some(max),
        _ => None,
    }
}

/// A controlled or internally managed star Rating element.
#[derive(IntoElement)]
pub struct Rating {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    disabled: bool,
    read_only: bool,
    value: usize,
    max: usize,
    color: Option<Hsla>,
    aria_label: SharedString,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
}

impl Rating {
    /// Creates a Rating with a stable element ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            size: Size::Medium,
            disabled: false,
            read_only: false,
            value: 0,
            max: 5,
            color: None,
            aria_label: "Rating".into(),
            on_click: None,
        }
    }

    /// Sets the star size.
    pub fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Disables interaction and exposes the disabled state to assistive technology.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Makes the value non-interactive while preserving its normal visual emphasis.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Sets the accessible name exposed by the slider node.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = label.into();
        self
    }

    /// Overrides the active star color. The default is the Color Theme's yellow.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the current value. Rendering clamps it to the configured maximum.
    pub fn value(mut self, value: usize) -> Self {
        self.value = value;
        self
    }

    /// Sets the maximum number of stars.
    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }

    /// Sets the handler invoked after pointer or keyboard value changes.
    pub fn on_click(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Styled for Rating {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Rating {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Rating {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Persistent interaction state retained for one keyed Rating instance.
struct RatingState {
    external_value: usize,
    value: usize,
    hovered_value: usize,
    focus_handle: gpui::FocusHandle,
}

impl RenderOnce for Rating {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let max = self.max;
        let external_value = self.value.min(max);
        let disabled = self.disabled;
        let read_only = self.read_only;
        let interactive = !disabled && !read_only && max > 0;
        let active_color = self.color.unwrap_or(cx.theme().yellow);
        let inactive_color = cx.theme().muted_foreground.opacity(0.45);
        let style = cx.theme().style.as_ref();
        let metrics = rating_metrics(self.size, style);
        let ring_width = style.focus.ring_width;
        let ring_inset = ring_width + style.focus.ring_offset;
        let on_click = self.on_click.clone();

        let state = window.use_keyed_state(id.clone(), cx, |_, cx| RatingState {
            external_value,
            value: external_value,
            hovered_value: 0,
            focus_handle: cx.focus_handle(),
        });

        // Keep internal interaction state synchronized with controlled value and max changes.
        if state.read(cx).external_value != external_value
            || state.read(cx).value > max
            || (!interactive && state.read(cx).hovered_value > 0)
        {
            state.update(cx, |state, _| {
                state.external_value = external_value;
                state.value = external_value;
                state.hovered_value = if interactive {
                    state.hovered_value.min(max)
                } else {
                    0
                };
            });
        }

        let (value, hovered_value, focus_handle) = {
            let state = state.read(cx);
            (state.value, state.hovered_value, state.focus_handle.clone())
        };
        let displayed_value = if interactive && hovered_value > 0 {
            hovered_value
        } else {
            value
        };
        let focus_visible =
            interactive && focus_handle.is_focused(window) && window.last_input_was_keyboard();

        let ring = focus_visible.then(|| {
            div()
                .absolute()
                .top(-ring_inset)
                .right(-ring_inset)
                .bottom(-ring_inset)
                .left(-ring_inset)
                .border(ring_width)
                .border_color(cx.theme().ring.opacity(0.5))
                .rounded(metrics.radius)
        });

        let mut element = h_flex()
            .id(id)
            .role(Role::Slider)
            .aria_label(self.aria_label)
            .aria_value(format!("{value} of {max}"))
            .aria_numeric_value(value as f64)
            .aria_min_numeric_value(0.0)
            .aria_max_numeric_value(max as f64)
            .aria_numeric_value_step(1.0)
            .aria_orientation(gpui::accesskit::Orientation::Horizontal)
            .relative()
            .flex_nowrap()
            .gap(metrics.gap)
            .rounded(metrics.radius)
            .when_some(ring, |this, ring| this.child(ring))
            .when(disabled, |this| this.opacity(0.5))
            .refine_style(&self.style)
            .when(interactive, |this| {
                this.track_focus(&focus_handle.clone().tab_stop(true))
            })
            .when(interactive, |this| {
                let state = state.clone();
                this.on_hover(move |hovered, _, cx| {
                    if !hovered && state.read(cx).hovered_value != 0 {
                        state.update(cx, |state, cx| {
                            state.hovered_value = 0;
                            cx.notify();
                        });
                    }
                })
            })
            .when(interactive, |this| {
                let focus_handle = focus_handle.clone();
                this.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    crate::global_state::GlobalState::suppress_text_selection(cx);
                    focus_handle.focus(window, cx);
                })
            })
            .when(interactive, |this| {
                let state = state.clone();
                let on_click = on_click.clone();
                this.on_key_down(move |event: &KeyDownEvent, window, cx| {
                    if event.keystroke.modifiers.modified() {
                        return;
                    }
                    let current = state.read(cx).value;
                    let Some(next) = rating_after_key(event.keystroke.key.as_str(), current, max)
                    else {
                        return;
                    };

                    window.prevent_default();
                    cx.stop_propagation();
                    if next == current {
                        return;
                    }
                    state.update(cx, |state, cx| {
                        state.value = next;
                        state.hovered_value = 0;
                        cx.notify();
                    });
                    if let Some(on_click) = on_click.as_ref() {
                        on_click(&next, window, cx);
                    }
                })
            });

        for star in 1..=max {
            let filled = star <= displayed_value;
            let mut item = div()
                .id(star)
                .p(metrics.item_padding)
                .flex_none()
                .flex_shrink_0()
                .text_color(if filled { active_color } else { inactive_color })
                .child(
                    Icon::new(if filled {
                        IconName::StarFill
                    } else {
                        IconName::Star
                    })
                    .with_size(metrics.icon_edge),
                );

            if interactive {
                let hover_state = state.clone();
                item = item.on_mouse_move(move |_, _, cx| {
                    if hover_state.read(cx).hovered_value != star {
                        hover_state.update(cx, |state, cx| {
                            state.hovered_value = star;
                            cx.notify();
                        });
                    }
                });

                let click_state = state.clone();
                let click_handler = on_click.clone();
                item = item.on_click(move |_: &ClickEvent, window, cx| {
                    window.prevent_default();
                    let current = click_state.read(cx).value;
                    let next = rating_after_click(current, star);
                    if next == current {
                        return;
                    }
                    click_state.update(cx, |state, cx| {
                        state.value = next;
                        state.hovered_value = 0;
                        cx.notify();
                    });
                    if let Some(on_click) = click_handler.as_ref() {
                        on_click(&next, window, cx);
                    }
                });
            }

            element = element.child(item);
        }

        accessibility_state(element, false, read_only, disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_retains_requested_value_and_accessibility_options() {
        let rating = Rating::new("rating")
            .max(3)
            .value(5)
            .read_only(true)
            .aria_label("Product rating");

        assert_eq!(rating.value, 5);
        assert_eq!(rating.max, 3);
        assert!(rating.read_only);
        assert_eq!(rating.aria_label.as_ref(), "Product rating");
    }

    #[test]
    fn click_selects_lower_star_and_toggles_current_star_down() {
        assert_eq!(rating_after_click(5, 3), 3);
        assert_eq!(rating_after_click(3, 3), 2);
        assert_eq!(rating_after_click(0, 1), 1);
    }

    #[test]
    fn keyboard_navigation_is_bounded() {
        assert_eq!(rating_after_key("left", 0, 5), Some(0));
        assert_eq!(rating_after_key("right", 5, 5), Some(5));
        assert_eq!(rating_after_key("home", 3, 5), Some(0));
        assert_eq!(rating_after_key("end", 3, 5), Some(5));
        assert_eq!(rating_after_key("space", 3, 5), None);
    }

    #[test]
    fn metrics_follow_semantic_style_values() {
        let vega = StylePreset::vega();
        let nova = StylePreset::nova();
        let maia = StylePreset::maia();

        assert_eq!(rating_metrics(Size::Medium, &vega).radius, vega.radii.md);
        assert_eq!(rating_metrics(Size::Medium, &nova).radius, nova.radii.md);
        assert_eq!(rating_metrics(Size::Medium, &maia).radius, maia.radii.md);
        assert!(
            rating_metrics(Size::Medium, &nova).item_padding
                <= rating_metrics(Size::Medium, &maia).item_padding
        );
    }
}
