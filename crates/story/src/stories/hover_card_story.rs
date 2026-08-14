// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added story helpers for `render_controlled_example`, `render_safe_transfer_example`.
// - Added examples for `render_safe_transfer_example`, `render_controlled_example`, `w_full`,
//   `justify_between`, `fallback`, `min_w_0` and 5 more.
// - Removed examples using `gap_3`, `src`, `anchor`, `text_sm`.
// - Reworked Hover Card story around keyboard navigation and activation behavior.
use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Window, div, px, relative,
};
use hearth_gpui::{
    ActiveTheme, StyledExt,
    avatar::{Avatar, AvatarFallback, AvatarImage},
    button::Button,
    h_flex,
    hover_card::{HoverCard, HoverCardAlign, HoverCardSide},
    v_flex,
};
use std::time::Duration;

use crate::{Story, section};

pub struct HoverCardStory {
    controlled_open: bool,
}

impl HoverCardStory {
    fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self {
            controlled_open: false,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for HoverCardStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(self.render_basic_example(cx))
            .child(self.render_user_profile_example(cx))
            .child(self.render_custom_timing_example(cx))
            .child(self.render_positioning_examples(cx))
            .child(self.render_safe_transfer_example())
            .child(self.render_controlled_example(cx))
    }
}

impl HoverCardStory {
    /// Basic hover card example
    fn render_basic_example(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("Basic").child(
            HoverCard::new("basic")
                .trigger(
                    div()
                        .child("Hover over me")
                        .text_color(cx.theme().primary)
                        .cursor_pointer()
                        .text_sm(),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .child("This is a hover card")
                                .font_semibold()
                                .text_sm(),
                        )
                        .child(
                            div()
                                .child("You can display rich content when hovering over a trigger element.")
                                .text_color(cx.theme().muted_foreground)
                                .text_sm(),
                        ),
                ),
        )
    }

    fn render_user_profile_example(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("User Profile Preview").child(
            h_flex()
                .child("Hover over ")
                .child(
                    HoverCard::new("user-profile")
                        // shadcn's canonical profile example overrides HoverCardContent to w-80.
                        // Apply the width to the surface, not to its child layout.
                        .w(px(320.))
                        .trigger(
                            div()
                                .child("@huacnlee")
                                .cursor_pointer()
                                .text_color(cx.theme().link),
                        )
                        .content(|_, _, cx| {
                            h_flex()
                                .w_full()
                                .justify_between()
                                .gap_4()
                                .items_start()
                                .child(
                                    Avatar::new("hover-card-jason", "Jason Lee")
                                        .image(AvatarImage::new(
                                            "https://avatars.githubusercontent.com/u/5518?s=64",
                                        ))
                                        .fallback(AvatarFallback::text("JL")),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap_1()
                                        .line_height(relative(1.))
                                        .child(div().child("Jason Lee").font_semibold())
                                        .child(
                                            div()
                                                .child("@huacnlee")
                                                .text_color(cx.theme().link)
                                                .text_sm(),
                                        )
                                        .child(div().mt_1().child("The author of Hearth GPUI.")),
                                )
                        }),
                )
                .child(" to see their profile"),
        )
    }

    /// Custom timing configuration example
    fn render_custom_timing_example(&self, _: &mut Context<Self>) -> impl IntoElement {
        section("Custom Timing").child(
            h_flex()
                .gap_4()
                .child(
                    HoverCard::new("fast-open")
                        .open_delay(Duration::from_millis(200))
                        .close_delay(Duration::from_millis(100))
                        .trigger(Button::new("fast").label("Fast Open (200ms)").outline())
                        .child(div().child("This hover card opens after 200ms").text_sm()),
                )
                .child(
                    HoverCard::new("slow-open")
                        .open_delay(Duration::from_secs(1))
                        .close_delay(Duration::from_secs_f32(0.5))
                        .trigger(Button::new("slow").label("Slow Open (1000ms)").outline())
                        .child(div().child("This hover card opens after 1000ms").text_sm()),
                ),
        )
    }

    /// Displays the four physical placement sides supported by shadcn.
    fn render_positioning_examples(&self, _: &mut Context<Self>) -> impl IntoElement {
        section("Positioning").child(
            h_flex()
                .gap_4()
                .items_center()
                .justify_center()
                .child(
                    HoverCard::new("side-top")
                        .side(HoverCardSide::Top)
                        .trigger(Button::new("top").label("Top").outline())
                        .child("Positioned above the trigger"),
                )
                .child(
                    HoverCard::new("side-right")
                        .side(HoverCardSide::Right)
                        .trigger(Button::new("right").label("Right").outline())
                        .child("Positioned to the right"),
                )
                .child(
                    HoverCard::new("side-bottom")
                        .side(HoverCardSide::Bottom)
                        .trigger(Button::new("bottom").label("Bottom").outline())
                        .child("Positioned below the trigger"),
                )
                .child(
                    HoverCard::new("side-left")
                        .side(HoverCardSide::Left)
                        .align(HoverCardAlign::End)
                        .trigger(Button::new("left").label("Left / End").outline())
                        .child("Positioned to the left and end-aligned"),
                ),
        )
    }

    /// Demonstrates controlled state and the same focus/hover interaction contract.
    fn render_controlled_example(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("Controlled").child(
            HoverCard::new("controlled-hover-card")
                .open(self.controlled_open)
                .on_open_change(cx.listener(|this, open, _, cx| {
                    this.controlled_open = *open;
                    cx.notify();
                }))
                .trigger(
                    Button::new("controlled-trigger")
                        .label("Hover or focus me")
                        .outline(),
                )
                .child("This preview is controlled by the Story state."),
        )
    }

    /// Provides a visible target for checking diagonal trigger-to-content movement.
    fn render_safe_transfer_example(&self) -> impl IntoElement {
        section("Safe Pointer Transfer").child(
            HoverCard::new("safe-transfer")
                .trigger(
                    Button::new("safe-transfer-trigger")
                        .label("Move diagonally into the card")
                        .outline(),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(div().font_semibold().child("Safe corridor"))
                        .child("The preview remains open while the pointer crosses the gap."),
                ),
        )
    }
}

impl Story for HoverCardStory {
    fn title() -> &'static str {
        "HoverCard"
    }

    fn description() -> &'static str {
        "A non-modal preview opened by pointer hover or keyboard focus, with safe transfer and configurable delays."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}
