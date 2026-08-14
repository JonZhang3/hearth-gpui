// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `aria_label`, `max_w_md`, `gap_3`, `large`, `gap_2`, `w_full` and 4 more.
// - Reworked Progress story around accessibility semantics and ARIA state.
use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled, Task,
    Window, div, prelude::FluentBuilder as _, px,
};
use hearth_gpui::{
    ActiveTheme, IconName, Selectable, Sizable, StyledExt,
    button::Button,
    h_flex,
    progress::{Progress, ProgressCircle},
    v_flex,
};
use std::time::Duration;

use crate::section;

pub struct ProgressStory {
    focus_handle: gpui::FocusHandle,
    value: f32,
    loading: bool,
    _task: Option<Task<()>>,
}

impl super::Story for ProgressStory {
    fn title() -> &'static str {
        "Progress"
    }

    fn description() -> &'static str {
        "Displays an indicator showing the completion progress of a task, typically displayed as a progress bar."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ProgressStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            value: 25.,
            loading: false,
            _task: None,
        }
    }

    /// Updates the displayed value and refreshes all progress examples.
    pub fn set_value(&mut self, value: f32, cx: &mut Context<Self>) {
        self.value = value;
        cx.notify();
    }

    fn start_animation(&mut self, cx: &mut Context<Self>) {
        self.value = 0.;

        self._task = Some(cx.spawn({
            let entity = cx.entity();
            async move |_, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(15))
                        .await;

                    let mut need_break = false;
                    _ = entity.update(cx, |this, cx| {
                        this.value = (this.value + 2.).min(100.);
                        cx.notify();

                        if this.value >= 100. {
                            this._task = None;
                            need_break = true;
                        }
                    });

                    if need_break {
                        break;
                    }
                }
            }
        }));
    }
}

impl Focusable for ProgressStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProgressStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .child(
                h_flex()
                    .w_full()
                    .gap_3()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("button-1").small().label("0%").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.set_value(0., cx);
                                }),
                            ))
                            .child(Button::new("button-2").small().label("25%").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.set_value(25., cx);
                                }),
                            ))
                            .child(Button::new("button-3").small().label("75%").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.set_value(75., cx);
                                }),
                            ))
                            .child(Button::new("button-4").small().label("100%").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.set_value(100., cx);
                                }),
                            ))
                            .child(
                                Button::new("circle-animation-button")
                                    .small()
                                    .icon(IconName::Play)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.start_animation(cx);
                                    })),
                            )
                            .child(
                                Button::new("loading-toggle-button")
                                    .small()
                                    .label("Loading")
                                    .selected(self.loading)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.loading = !this.loading;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("circle-button-5")
                                    .icon(IconName::Minus)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_value((this.value - 1.).max(0.), cx);
                                    })),
                            )
                            .child(
                                Button::new("circle-button-6")
                                    .icon(IconName::Plus)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_value((this.value + 1.).min(100.), cx);
                                    })),
                            ),
                    ),
            )
            .child(
                section("Progress Bar").max_w_md().child(
                    Progress::new("progress-1")
                        .aria_label("Task progress")
                        .value(self.value)
                        .loading(self.loading),
                ),
            )
            .child(
                section("Progress Sizes").max_w_md().child(
                    v_flex()
                        .gap_3()
                        .child(
                            Progress::new("progress-size-xs")
                                .aria_label("Extra small progress")
                                .value(self.value)
                                .xsmall(),
                        )
                        .child(
                            Progress::new("progress-size-sm")
                                .aria_label("Small progress")
                                .value(self.value)
                                .small(),
                        )
                        .child(
                            Progress::new("progress-size-md")
                                .aria_label("Medium progress")
                                .value(self.value),
                        )
                        .child(
                            Progress::new("progress-size-lg")
                                .aria_label("Large progress")
                                .value(self.value)
                                .large(),
                        ),
                ),
            )
            .child(
                section("Progress with Label").max_w_md().child(
                    v_flex()
                        .gap_2()
                        .child(
                            h_flex()
                                .w_full()
                                .justify_between()
                                .text_sm()
                                .child(div().font_medium().child("Uploading"))
                                .child(
                                    div()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("{}%", self.value as i32)),
                                ),
                        )
                        .child(
                            Progress::new("progress-labeled")
                                .aria_label("Upload progress")
                                .aria_value(if self.loading {
                                    "Loading".to_string()
                                } else {
                                    format!("{} percent", self.value as i32)
                                })
                                .value(self.value)
                                .loading(self.loading),
                        ),
                ),
            )
            .child(
                section("Custom Style").max_w_md().child(
                    Progress::new("progress-2")
                        .aria_label("Custom progress")
                        .value(32.)
                        .loading(self.loading)
                        .h(px(16.))
                        .rounded(px(2.))
                        .color(cx.theme().green_light)
                        .border_2()
                        .border_color(cx.theme().green),
                ),
            )
            .child(
                section("Circle Progress").max_w_md().child(
                    ProgressCircle::new("circle-progress-1")
                        .value(self.value)
                        .loading(self.loading)
                        .size_20()
                        .when(!self.loading, |this| {
                            this.child(
                                v_flex()
                                    .size_full()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .child(format!("{}%", self.value))
                                            .text_color(cx.theme().progress_bar),
                                    )
                                    .child(div().child("Loading").text_xs()),
                            )
                        }),
                ),
            )
            .child(
                section("With size").max_w_md().child(
                    h_flex()
                        .gap_2()
                        .child(
                            ProgressCircle::new("circle-progress-2")
                                .value(self.value)
                                .loading(self.loading)
                                .large(),
                        )
                        .child(
                            ProgressCircle::new("circle-progress-3")
                                .value(self.value)
                                .loading(self.loading),
                        )
                        .child(
                            ProgressCircle::new("circle-progress-4")
                                .value(self.value)
                                .loading(self.loading)
                                .small(),
                        )
                        .child(
                            ProgressCircle::new("circle-progress-5")
                                .value(self.value)
                                .loading(self.loading)
                                .xsmall(),
                        ),
                ),
            )
            .child(
                section("With Label").max_w_md().child(
                    h_flex()
                        .gap_2()
                        .child(
                            ProgressCircle::new("circle-progress-6")
                                .color(cx.theme().primary)
                                .value(self.value)
                                .loading(self.loading)
                                .size_4(),
                        )
                        .child("Downloading..."),
                ),
            )
            .child(
                section("Circle with Color").max_w_md().child(
                    ProgressCircle::new("circle-progress-7")
                        .color(cx.theme().yellow)
                        .value(self.value)
                        .loading(self.loading)
                        .size_12(),
                ),
            )
    }
}
