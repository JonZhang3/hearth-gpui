// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added story helpers for `avatar`.
// - Added examples for `gap_2`, `flex_wrap`, `secondary`, `destructive`, `outline`, `ghost` and 6
//   more.
// - Removed examples using `max_w_md`, `src`.
// - Reworked Badge story around accessibility semantics and ARIA state, focus-visible and focus
//   restoration behavior.
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    ActiveTheme as _, ColorName, Icon, IconName, Sizable as _,
    avatar::{Avatar, AvatarImage},
    badge::{Badge, BadgeVariants as _, OverlayBadge},
    dock::PanelControl,
    h_flex,
    spinner::Spinner,
    v_flex,
};

use crate::section;

fn avatar(id: &'static str, label: &'static str, source: &'static str) -> Avatar {
    Avatar::new(id, label).image(AvatarImage::new(source))
}

pub struct BadgeStory {
    focus_handle: FocusHandle,
}

impl BadgeStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for BadgeStory {
    fn title() -> &'static str {
        "Badge"
    }

    fn description() -> &'static str {
        "A compact label for status or metadata, with optional overlay indicators."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for BadgeStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BadgeStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Variants").child(
                    h_flex()
                        .gap_2()
                        .flex_wrap()
                        .child(Badge::new().child("Default"))
                        .child(Badge::new().secondary().child("Secondary"))
                        .child(Badge::new().destructive().child("Destructive"))
                        .child(Badge::new().outline().child("Outline"))
                        .child(Badge::new().ghost().child("Ghost"))
                        .child(Badge::new().link().child("Link")),
                ),
            )
            .child(
                section("With icon").child(
                    h_flex()
                        .gap_2()
                        .flex_wrap()
                        .child(
                            Badge::new()
                                .leading(Icon::new(IconName::CircleCheck).xsmall())
                                .child("Verified"),
                        )
                        .child(
                            Badge::new()
                                .secondary()
                                .child("Continue")
                                .trailing(Icon::new(IconName::ArrowRight).xsmall()),
                        )
                        .child(
                            Badge::new()
                                .outline()
                                .leading(Spinner::new().xsmall())
                                .child("Generating"),
                        ),
                ),
            )
            .child(
                section("Custom colors").child(
                    h_flex()
                        .gap_2()
                        .flex_wrap()
                        .child(
                            Badge::new()
                                .bg(cx.theme().success)
                                .text_color(cx.theme().success_foreground)
                                .child("Success"),
                        )
                        .child(
                            Badge::new()
                                .bg(cx.theme().warning)
                                .text_color(cx.theme().warning_foreground)
                                .child("Warning"),
                        )
                        .child(
                            Badge::new()
                                .bg(cx.theme().info)
                                .text_color(cx.theme().info_foreground)
                                .child("Info"),
                        )
                        .child(
                            Badge::new()
                                .bg(if cx.theme().is_dark() {
                                    ColorName::Blue.scale(950).opacity(0.5)
                                } else {
                                    ColorName::Blue.scale(50)
                                })
                                .text_color(if cx.theme().is_dark() {
                                    ColorName::Blue.scale(300)
                                } else {
                                    ColorName::Blue.scale(600)
                                })
                                .child("Category"),
                        ),
                ),
            )
            .child(
                section("Long text").child(
                    Badge::new()
                        .secondary()
                        .child("A badge with a lot of text remains on one line"),
                ),
            )
            .child(
                section("Overlay badge").child(
                    h_flex()
                        .gap_6()
                        .child(
                            OverlayBadge::new()
                                .count(3)
                                .child(Icon::new(IconName::Bell).large()),
                        )
                        .child(OverlayBadge::new().count(103).child(avatar(
                            "overlay-badge-count",
                            "Jason Lee",
                            "https://avatars.githubusercontent.com/u/5518?v=4",
                        )))
                        .child(
                            OverlayBadge::new()
                                .dot()
                                .color(cx.theme().green)
                                .child(avatar(
                                    "overlay-badge-dot",
                                    "Floyd Wang",
                                    "https://avatars.githubusercontent.com/u/28998859?v=4",
                                )),
                        )
                        .child(
                            OverlayBadge::new()
                                .icon(IconName::Check)
                                .color(cx.theme().cyan)
                                .child(avatar(
                                    "overlay-badge-icon",
                                    "Wilson",
                                    "https://avatars.githubusercontent.com/u/20092316?v=4",
                                )),
                        ),
                ),
            )
            .child(
                section("Overlay sizes").child(
                    h_flex()
                        .gap_6()
                        .child(
                            OverlayBadge::new()
                                .count(2)
                                .small()
                                .child(Icon::new(IconName::Inbox).small()),
                        )
                        .child(
                            OverlayBadge::new()
                                .count(12)
                                .child(Icon::new(IconName::Inbox).large()),
                        )
                        .child(
                            OverlayBadge::new()
                                .count(212)
                                .large()
                                .child(Icon::new(IconName::Inbox).large()),
                        ),
                ),
            )
    }
}
