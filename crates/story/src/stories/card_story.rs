use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, Styled, Window, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Sizable as _,
    button::Button,
    card::{
        Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardMedia,
        CardTitle,
    },
    dock::PanelControl,
    h_flex, v_flex,
};

use crate::section;

pub struct CardStory {
    focus_handle: FocusHandle,
}

impl CardStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for CardStory {
    fn title() -> &'static str {
        "Card"
    }

    fn description() -> &'static str {
        "Groups related content and actions on a shadcn Vega surface."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for CardStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Builds a representative Card at the requested density.
fn example_card(small: bool) -> Card {
    Card::new()
        .when(small, |card| card.small())
        .header(
            CardHeader::new()
                .title(CardTitle::new().child(if small { "Small Card" } else { "Default Card" }))
                .description(
                    CardDescription::new()
                        .child("Use typed sections to keep spacing consistent across the surface."),
                ),
        )
        .content(
            CardContent::new().child(
                "Card content can contain text, controls, lists, or any other GPUI element.",
            ),
        )
        .footer(
            CardFooter::new().child(
                Button::new(if small {
                    "small-card-action"
                } else {
                    "default-card-action"
                })
                .when(small, |button| button.small())
                .outline()
                .label("Action"),
            ),
        )
}

impl Render for CardStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Sizes").child(
                    h_flex()
                        .items_start()
                        .gap_4()
                        .flex_wrap()
                        .child(example_card(false).w(px(360.)))
                        .child(example_card(true).w(px(360.))),
                ),
            )
            .child(
                section("Header Action").max_w_md().child(
                    Card::new()
                        .header(
                            CardHeader::new()
                                .title(CardTitle::new().child("Meeting Notes"))
                                .description(CardDescription::new().child(
                                    "Transcript from a long client meeting whose supporting copy must shrink without overlapping the action.",
                                ))
                                .action(CardAction::new().child(
                                    Button::new("transcribe-card")
                                        .small()
                                        .outline()
                                        .label("Transcribe"),
                                )),
                        )
                        .content(CardContent::new().child(
                            "The client requested a dashboard redesign with a stronger focus on responsive layouts.",
                        )),
                ),
            )
            .child(
                section("Divided Sections").max_w_md().child(
                    Card::new()
                        .header(
                            CardHeader::new()
                                .title(CardTitle::new().child("Release Health"))
                                .description(CardDescription::new().child(
                                    "Track readiness across launch signals.",
                                ))
                                .bordered(true),
                        )
                        .content(CardContent::new().child("24 of 26 checks passed."))
                        .footer(
                            CardFooter::new()
                                .bordered(true)
                                .justify_end()
                                .gap_2()
                                .child(Button::new("review-release").label("Review")),
                        ),
                ),
            )
            .child(
                section("Edge-to-edge Media").max_w_md().child(
                    Card::new()
                        .media(
                            CardMedia::new()
                                .h(px(144.))
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .bg(cx.theme().muted)
                                .text_color(cx.theme().muted_foreground)
                                .child("Edge-to-edge media"),
                        )
                        .header(
                            CardHeader::new()
                                .title(CardTitle::new().child("Beautiful Landscape"))
                                .description(CardDescription::new().child(
                                    "Media is clipped by the Card surface without duplicating radius styles.",
                                )),
                        )
                        .footer(CardFooter::new().child(
                            Button::new("media-card-action")
                                .w_full()
                                .label("View details"),
                        )),
                ),
            )
            .child(
                section("Custom Spacing and Radius").max_w_md().child(
                    Card::new()
                        .spacing(px(20.))
                        .rounded(px(10.))
                        .header(
                            CardHeader::new()
                                .title(CardTitle::new().child("Custom Geometry"))
                                .description(CardDescription::new().child(
                                    "Card-level spacing and radius overrides propagate to every typed section.",
                                )),
                        )
                        .content(CardContent::new().child(
                            "The size variant still controls title typography independently.",
                        ))
                        .footer(CardFooter::new().child(
                            Button::new("custom-card-action").outline().label("Continue"),
                        )),
                ),
            )
            .child(
                section("Trailing Media").max_w_md().child(
                    Card::new()
                        .header(
                            CardHeader::new()
                                .title(CardTitle::new().child("Trailing Preview"))
                                .description(CardDescription::new().child(
                                    "A final media slot follows the footer while preserving normal bottom spacing.",
                                )),
                        )
                        .footer(CardFooter::new().child("Updated moments ago"))
                        .bottom_media(
                            CardMedia::new()
                                .h(px(96.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(cx.theme().muted)
                                .text_color(cx.theme().muted_foreground)
                                .child("Trailing media"),
                        ),
                ),
            )
    }
}
