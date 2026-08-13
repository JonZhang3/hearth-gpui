use gpui::{
    AnyElement, App, ElementId, FocusHandle, Hsla, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{ActiveTheme as _, Icon, StyledExt as _, h_flex};

/// Ordered semantic metadata shared by chart legends and accessibility summaries.
#[derive(Clone, Default)]
pub struct ChartConfig {
    items: Vec<ChartConfigItem>,
}

impl ChartConfig {
    /// Creates an empty chart configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a series while preserving the order used by the plot and legend.
    pub fn item(mut self, item: ChartConfigItem) -> Self {
        self.items.push(item);
        self
    }

    /// Returns the ordered series metadata.
    pub fn items(&self) -> &[ChartConfigItem] {
        &self.items
    }
}

/// Semantic metadata for one chart series.
#[derive(Clone)]
pub struct ChartConfigItem {
    pub key: SharedString,
    pub label: SharedString,
    pub color: Hsla,
    pub icon: Option<Icon>,
}

impl ChartConfigItem {
    /// Creates series metadata with a stable key, display label, and semantic color.
    pub fn new(
        key: impl Into<SharedString>,
        label: impl Into<SharedString>,
        color: impl Into<Hsla>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            color: color.into(),
            icon: None,
        }
    }

    /// Uses an icon in legends instead of the default color marker.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// A category and its screen-reader friendly values.
#[derive(Clone)]
pub struct ChartAccessibilityItem {
    pub label: SharedString,
    pub values: Vec<(SharedString, SharedString)>,
}

impl ChartAccessibilityItem {
    /// Creates one accessible data category.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            values: Vec::new(),
        }
    }

    /// Appends a named value to this category.
    pub fn value(mut self, label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        self.values.push((label.into(), value.into()));
        self
    }

    fn accessible_label(&self) -> String {
        let values = self
            .values
            .iter()
            .map(|(label, value)| format!("{label}: {value}"))
            .collect::<Vec<_>>()
            .join(", ");
        if values.is_empty() {
            self.label.to_string()
        } else {
            format!("{}; {values}", self.label)
        }
    }
}

/// Accessibility metadata exposed by a [`ChartContainer`].
#[derive(Clone, Default)]
pub struct ChartAccessibility {
    pub label: SharedString,
    pub description: Option<SharedString>,
    pub items: Vec<ChartAccessibilityItem>,
}

impl ChartAccessibility {
    /// Creates chart accessibility metadata with a required accessible name.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            description: None,
            items: Vec::new(),
        }
    }

    /// Sets the chart description announced after its name.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Appends an accessible data category.
    pub fn item(mut self, item: ChartAccessibilityItem) -> Self {
        self.items.push(item);
        self
    }
}

/// Styled chart host providing shadcn-equivalent typography and accessibility semantics.
#[derive(IntoElement)]
pub struct ChartContainer {
    id: ElementId,
    style: StyleRefinement,
    child: Option<AnyElement>,
    accessibility: Option<ChartAccessibility>,
}

struct ChartContainerState {
    focus_handle: FocusHandle,
    active_item: usize,
}

fn chart_active_item_after_key(key: &str, active_item: usize, item_count: usize) -> Option<usize> {
    if item_count == 0 {
        return None;
    }
    let active_item = active_item.min(item_count - 1);
    match key {
        "left" | "up" => Some(active_item.saturating_sub(1)),
        "right" | "down" => Some((active_item + 1).min(item_count - 1)),
        "home" => Some(0),
        "end" => Some(item_count - 1),
        _ => None,
    }
}

impl ChartContainer {
    /// Creates a chart host with a stable accessibility identifier.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            child: None,
            accessibility: None,
        }
    }

    /// Exposes the chart and its data to assistive technology.
    pub fn accessibility(mut self, accessibility: ChartAccessibility) -> Self {
        self.accessibility = Some(accessibility);
        self
    }
}

impl Styled for ChartContainer {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ChartContainer {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.child = elements.into_iter().next();
    }
}

impl RenderOnce for ChartContainer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| ChartContainerState {
            focus_handle: cx.focus_handle(),
            active_item: 0,
        });
        let focus_handle = state.read(cx).focus_handle.clone();
        let mut container = div()
            .id(self.id)
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_xs()
            .refine_style(&self.style)
            .children(self.child);

        if let Some(accessibility) = self.accessibility {
            let items = accessibility.items;
            let item_count = items.len();
            let active_item = state.read(cx).active_item.min(item_count.saturating_sub(1));
            let active_value = items
                .get(active_item)
                .map(ChartAccessibilityItem::accessible_label);
            let state_for_key = state.clone();
            container = container
                .role(Role::GraphicsDocument)
                .aria_label(accessibility.label)
                .when_some(active_value, |this, value| this.aria_value(value))
                .when_some(accessibility.description, |this, description| {
                    this.aria_description(description)
                })
                .when(item_count > 0, |this| {
                    this.track_focus(&focus_handle.tab_stop(true)).on_key_down(
                        move |event: &KeyDownEvent, window, cx| {
                            if event.keystroke.modifiers.modified() {
                                return;
                            }
                            let key = event.keystroke.key.as_str();
                            let Some(next_item) = chart_active_item_after_key(
                                key,
                                state_for_key.read(cx).active_item,
                                item_count,
                            ) else {
                                return;
                            };
                            state_for_key.update(cx, |state, cx| {
                                state.active_item = next_item;
                                cx.notify();
                            });
                            window.prevent_default();
                            cx.stop_propagation();
                        },
                    )
                })
                .a11y_synthetic_children(move |builder| {
                    for (index, item) in items.iter().enumerate() {
                        let mut node = gpui::accesskit::Node::new(Role::GraphicsSymbol);
                        node.set_label(item.accessible_label());
                        node.set_selected(index == active_item);
                        builder.push_child(builder.synthetic_node_id(("chart-item", index)), node);
                    }
                });
        }

        container
    }
}

/// Placement of a chart legend relative to its plot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChartLegendPosition {
    Top,
    #[default]
    Bottom,
}

/// A compact legend derived from ordered [`ChartConfig`] metadata.
#[derive(IntoElement)]
pub struct ChartLegend {
    config: ChartConfig,
    position: ChartLegendPosition,
    hide_icon: bool,
}

impl ChartLegend {
    /// Creates a legend containing every configured series.
    pub fn new(config: ChartConfig) -> Self {
        Self {
            config,
            position: ChartLegendPosition::Bottom,
            hide_icon: false,
        }
    }

    /// Places the legend above or below its associated plot.
    pub fn position(mut self, position: ChartLegendPosition) -> Self {
        self.position = position;
        self
    }

    /// Hides both custom icons and fallback color markers.
    pub fn hide_icon(mut self, hide: bool) -> Self {
        self.hide_icon = hide;
        self
    }
}

impl RenderOnce for ChartLegend {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let hide_icon = self.hide_icon;
        h_flex()
            .items_center()
            .justify_center()
            .gap_4()
            .text_xs()
            .when(self.position == ChartLegendPosition::Top, |this| {
                this.pb_3()
            })
            .when(self.position == ChartLegendPosition::Bottom, |this| {
                this.pt_3()
            })
            .children(self.config.items.into_iter().map(move |item| {
                h_flex()
                    .items_center()
                    .gap_1p5()
                    .when(!hide_icon, |this| {
                        if let Some(icon) = item.icon {
                            this.child(icon.size(px(12.)).text_color(item.color))
                        } else {
                            this.child(div().size_2().rounded(px(2.)).bg(item.color))
                        }
                    })
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(item.label),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_config_preserves_series_order() {
        let config = ChartConfig::new()
            .item(ChartConfigItem::new("desktop", "Desktop", gpui::black()))
            .item(ChartConfigItem::new("mobile", "Mobile", gpui::white()));

        assert_eq!(config.items()[0].key.as_ref(), "desktop");
        assert_eq!(config.items()[1].key.as_ref(), "mobile");
    }

    #[test]
    fn accessibility_item_formats_named_values() {
        let item = ChartAccessibilityItem::new("January").value("Desktop", "186");
        assert_eq!(item.accessible_label(), "January; Desktop: 186");
    }

    #[test]
    fn chart_keyboard_navigation_clamps_to_current_item_count() {
        assert_eq!(chart_active_item_after_key("left", 8, 3), Some(1));
        assert_eq!(chart_active_item_after_key("right", 2, 3), Some(2));
        assert_eq!(chart_active_item_after_key("home", 2, 3), Some(0));
        assert_eq!(chart_active_item_after_key("end", 0, 3), Some(2));
        assert_eq!(chart_active_item_after_key("enter", 0, 3), None);
        assert_eq!(chart_active_item_after_key("right", 0, 0), None);
    }
}
