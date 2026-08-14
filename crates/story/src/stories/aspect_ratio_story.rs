use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window, div, px,
};
use hearth_gpui::{ActiveTheme as _, aspect_ratio::AspectRatio, dock::PanelControl, h_flex};

use crate::section;

pub struct AspectRatioStory {
    focus_handle: FocusHandle,
}

impl AspectRatioStory {
    /// Creates the AspectRatio story state.
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    /// Creates a shared Story entity.
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for AspectRatioStory {
    fn title() -> &'static str {
        "AspectRatio"
    }

    fn description() -> &'static str {
        "A container that preserves a width-to-height ratio."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        Some(PanelControl::Toolbar)
    }
}

impl Focusable for AspectRatioStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AspectRatioStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let radius = cx.theme().style.radii.lg;
        let foreground = cx.theme().muted_foreground;
        let background = cx.theme().muted;
        let example = |ratio: f32, width, label: &'static str| {
            AspectRatio::new(ratio)
                .w(width)
                .rounded(radius)
                .bg(background)
                .child(
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_sm()
                        .text_color(foreground)
                        .child(label),
                )
        };

        section("Ratios").child(
            h_flex()
                .w_full()
                .flex_wrap()
                .items_start()
                .gap_6()
                .child(example(16.0 / 9.0, px(360.), "16:9"))
                .child(example(21.0 / 9.0, px(420.), "21:9"))
                .child(example(1.0, px(220.), "1:1"))
                .child(example(9.0 / 16.0, px(180.), "9:16")),
        )
    }
}
