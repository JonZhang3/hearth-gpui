// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `gap_1`, `h_5`, `items_center`, `gap_4`.
// - Removed examples using `gap_y_4`, `gap_y_2`, `gap_x_4`, `dashed`.
use crate::section;
use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled, Window,
};
use hearth_gpui::{ActiveTheme, h_flex, label::Label, separator::Separator, v_flex};

const DESCRIPTION: &str = "Hearth GPUI is a Rust GUI components for building fantastic cross-platform desktop application by using GPUI.";

pub struct SeparatorStory {
    focus_handle: gpui::FocusHandle,
}

impl super::Story for SeparatorStory {
    fn title() -> &'static str {
        "Separator"
    }

    fn description() -> &'static str {
        "A separator that can be either vertical or horizontal."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SeparatorStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
        })
    }
}

impl Focusable for SeparatorStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SeparatorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Default").child(
                    v_flex()
                        .gap_4()
                        .w_full()
                        .mt_4()
                        .child(
                            v_flex().gap_1().child("Hearth GPUI").child(
                                Label::new(DESCRIPTION)
                                    .text_color(cx.theme().muted_foreground)
                                    .text_sm(),
                            ),
                        )
                        .child(Separator::new())
                        .child("Cross-platform desktop components built with GPUI."),
                ),
            )
            .child(
                section("Orientation").child(
                    h_flex()
                        .gap_4()
                        .h_5()
                        .items_center()
                        .child("Blog")
                        .child(Separator::vertical())
                        .child("Docs")
                        .child(Separator::vertical())
                        .child("Source"),
                ),
            )
            .child(
                section("GPUI Extensions").child(
                    v_flex()
                        .gap_4()
                        .child(Separator::horizontal().label("With Label"))
                        .child(Separator::horizontal_dashed())
                        .child(Separator::horizontal_dashed().label("Dashed With Label")),
                ),
            )
    }
}
