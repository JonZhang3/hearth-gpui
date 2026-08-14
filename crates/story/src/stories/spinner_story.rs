use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window, px,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable,
    badge::{Badge, BadgeVariants as _},
    button::Button,
    h_flex,
    spinner::{Spinner, SpinnerAnimation, SpinnerVariant},
    v_flex,
};

use crate::section;

pub struct SpinnerStory {
    focus_handle: gpui::FocusHandle,
}

impl super::Story for SpinnerStory {
    fn title() -> &'static str {
        "Spinner"
    }

    fn description() -> &'static str {
        "Displays an indeterminate loading status."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SpinnerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for SpinnerStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SpinnerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .child(
                section("Basic")
                    .gap_x_6()
                    .child(Spinner::new())
                    .child(Spinner::new().large()),
            )
            .child(
                section("Variants")
                    .gap_x_6()
                    .child(Spinner::new().variant(SpinnerVariant::Circular))
                    .child(Spinner::new().variant(SpinnerVariant::Classic)),
            )
            .child(
                section("Colors")
                    .gap_x_6()
                    .child(Spinner::new())
                    .child(Spinner::new().color(cx.theme().blue))
                    .child(Spinner::new().color(cx.theme().green)),
            )
            .child(
                section("Sizes")
                    .gap_x_6()
                    .child(Spinner::new().with_size(px(12.)))
                    .child(Spinner::new().with_size(px(16.)))
                    .child(Spinner::new().with_size(px(24.)))
                    .child(Spinner::new().with_size(px(32.))),
            )
            .child(
                section("Custom Icon")
                    .gap_x_6()
                    .child(Spinner::new())
                    .child(
                        Spinner::new()
                            .icon(IconName::Loader)
                            .animation(SpinnerAnimation::LinearSpin)
                            .large()
                            .color(cx.theme().cyan),
                    ),
            )
            .child(
                section("Composition").child(
                    h_flex()
                        .gap_3()
                        .child(
                            Button::new("spinner-submit")
                                .icon(Spinner::new())
                                .label("Submitting")
                                .disabled(true),
                        )
                        .child(
                            Badge::new()
                                .outline()
                                .leading(Spinner::new().xsmall())
                                .child("Generating"),
                        ),
                ),
            )
    }
}
