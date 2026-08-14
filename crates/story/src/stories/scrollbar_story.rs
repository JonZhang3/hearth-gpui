use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, Styled, UniformListScrollHandle, Window, div, px, uniform_list,
};
use gpui_component::{
    ActiveTheme as _, Selectable,
    button::{Button, ButtonGroup},
    h_flex,
    scroll::ScrollableElement,
    v_flex,
};

pub struct ScrollbarStory {
    focus_handle: FocusHandle,
    item_count: usize,
    size_mode: usize,
    scroll_handle: UniformListScrollHandle,
}

const ITEM_HEIGHT: Pixels = px(50.);

impl ScrollbarStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            item_count: 5_000,
            size_mode: 0,
            scroll_handle: UniformListScrollHandle::new(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn change_test_cases(&mut self, n: usize, cx: &mut Context<Self>) {
        self.size_mode = n;
        self.item_count = match n {
            0 => 5_000,
            1 => 100,
            2 => 500_000,
            _ => 5,
        };
        cx.notify();
    }

    fn render_buttons(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex().gap_2().justify_between().child(
            h_flex().gap_2().child(
                ButtonGroup::new("test-cases")
                    .outline()
                    .compact()
                    .child(
                        Button::new("test-0")
                            .label("5K items")
                            .selected(self.size_mode == 0),
                    )
                    .child(
                        Button::new("test-1")
                            .label("100 items")
                            .selected(self.size_mode == 1),
                    )
                    .child(
                        Button::new("test-2")
                            .label("500K items")
                            .selected(self.size_mode == 2),
                    )
                    .child(
                        Button::new("test-3")
                            .label("5 items")
                            .selected(self.size_mode == 3),
                    )
                    .on_click(cx.listener(|view, clicks: &Vec<usize>, _, cx| {
                        if clicks.contains(&0) {
                            view.change_test_cases(0, cx)
                        } else if clicks.contains(&1) {
                            view.change_test_cases(1, cx)
                        } else if clicks.contains(&2) {
                            view.change_test_cases(2, cx)
                        } else if clicks.contains(&3) {
                            view.change_test_cases(3, cx)
                        }
                    })),
            ),
        )
    }
}

impl super::Story for ScrollbarStory {
    fn title() -> &'static str {
        "Scrollbar"
    }

    fn description() -> &'static str {
        "Add scrollbar to a scrollable element."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ScrollbarStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ScrollbarStory {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .size_full()
            .gap_4()
            .child(self.render_buttons(cx))
            .child({
                div()
                    .relative()
                    .border_1()
                    .border_color(cx.theme().border)
                    .flex_1()
                    .child(
                        uniform_list("list", self.item_count, {
                            move |visible_range, _, cx| {
                                let mut elements = Vec::with_capacity(visible_range.len());
                                for ix in visible_range {
                                    elements.push(
                                        div()
                                            .h(ITEM_HEIGHT)
                                            .pt_1()
                                            .items_center()
                                            .justify_center()
                                            .text_sm()
                                            .child(
                                                div()
                                                    .p_2()
                                                    .bg(cx.theme().secondary)
                                                    .child(format!("Item {ix}")),
                                            ),
                                    );
                                }
                                elements
                            }
                        })
                        .py_1()
                        .px_3()
                        .size_full()
                        .track_scroll(&self.scroll_handle),
                    )
                    .vertical_scrollbar(&self.scroll_handle)
            })
            .child(
                h_flex()
                    .gap_4()
                    .h(px(140.))
                    .child(
                        h_flex()
                            .id("horizontal-scrollbar-example")
                            .flex_1()
                            .h_full()
                            .gap_2()
                            .p_3()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().style.radii.md)
                            .overflow_x_scrollbar()
                            .children((0..12).map(|ix| {
                                div()
                                    .min_w(px(120.))
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(cx.theme().style.radii.sm)
                                    .bg(cx.theme().secondary)
                                    .child(format!("Card {ix}"))
                            })),
                    )
                    .child(
                        div()
                            .id("both-axis-scrollbar-example")
                            .flex_1()
                            .h_full()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().style.radii.md)
                            .overflow_scrollbar()
                            .child(
                                div()
                                    .w(px(900.))
                                    .h(px(360.))
                                    .p_4()
                                    .bg(cx.theme().secondary)
                                    .child("Two-axis scroll area"),
                            ),
                    ),
            )
    }
}
