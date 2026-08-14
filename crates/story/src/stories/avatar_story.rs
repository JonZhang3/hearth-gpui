// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added story helpers for `image_avatar`.
// - Added examples for `fallback`, `badge`, `avatar`.
// - Removed examples using `src`, `limit`, `ellipsis`, `border_3`, `shadow_sm`.
// - Reworked Avatar story around invalid and validation state handling.
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable as _,
    avatar::{Avatar, AvatarBadge, AvatarFallback, AvatarGroup, AvatarGroupCount, AvatarImage},
    dock::PanelControl,
    v_flex,
};

use crate::section;

const SHADCN_AVATAR: &str = "https://github.com/shadcn.png";
const MAX_AVATAR: &str = "https://github.com/maxleiter.png";
const EVIL_RABBIT_AVATAR: &str = "https://github.com/evilrabbit.png";

pub struct AvatarStory {
    focus_handle: gpui::FocusHandle,
}

impl AvatarStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for AvatarStory {
    fn title() -> &'static str {
        "Avatar"
    }

    fn description() -> &'static str {
        "Avatar displays a user image with fallback, badge, group, and count composition."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for AvatarStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Creates one image Avatar with a deterministic text fallback.
fn image_avatar(
    id: &'static str,
    label: &'static str,
    initials: &'static str,
    source: &'static str,
) -> Avatar {
    Avatar::new(id, label)
        .image(AvatarImage::new(source))
        .fallback(AvatarFallback::text(initials))
}

impl Render for AvatarStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Overview")
                    .max_w_md()
                    .child(image_avatar(
                        "avatar-overview-basic",
                        "Shadcn",
                        "CN",
                        SHADCN_AVATAR,
                    ))
                    .child(
                        image_avatar(
                            "avatar-overview-status",
                            "Evil Rabbit",
                            "ER",
                            EVIL_RABBIT_AVATAR,
                        )
                        .badge(AvatarBadge::new().bg(cx.theme().green)),
                    )
                    .child(
                        AvatarGroup::new()
                            .avatar(image_avatar(
                                "avatar-overview-group-shadcn",
                                "Shadcn",
                                "CN",
                                SHADCN_AVATAR,
                            ))
                            .avatar(image_avatar(
                                "avatar-overview-group-max",
                                "Max Leiter",
                                "ML",
                                MAX_AVATAR,
                            ))
                            .avatar(image_avatar(
                                "avatar-overview-group-rabbit",
                                "Evil Rabbit",
                                "ER",
                                EVIL_RABBIT_AVATAR,
                            ))
                            .count(AvatarGroupCount::text("+3")),
                    ),
            )
            .child(
                section("Badge with Icon").max_w_md().child(
                    image_avatar("avatar-badge-icon", "Max Leiter", "ML", MAX_AVATAR)
                        .badge(AvatarBadge::new().child(IconName::Plus)),
                ),
            )
            .child(
                section("Avatar Group")
                    .max_w_md()
                    .child(
                        AvatarGroup::new()
                            .avatar(image_avatar(
                                "avatar-group-shadcn",
                                "Shadcn",
                                "CN",
                                SHADCN_AVATAR,
                            ))
                            .avatar(image_avatar(
                                "avatar-group-max",
                                "Max Leiter",
                                "ML",
                                MAX_AVATAR,
                            ))
                            .avatar(image_avatar(
                                "avatar-group-rabbit",
                                "Evil Rabbit",
                                "ER",
                                EVIL_RABBIT_AVATAR,
                            )),
                    )
                    .child(
                        AvatarGroup::new()
                            .avatar(image_avatar(
                                "avatar-group-icon-shadcn",
                                "Shadcn",
                                "CN",
                                SHADCN_AVATAR,
                            ))
                            .avatar(image_avatar(
                                "avatar-group-icon-max",
                                "Max Leiter",
                                "ML",
                                MAX_AVATAR,
                            ))
                            .avatar(image_avatar(
                                "avatar-group-icon-rabbit",
                                "Evil Rabbit",
                                "ER",
                                EVIL_RABBIT_AVATAR,
                            ))
                            .count(AvatarGroupCount::icon(IconName::Plus)),
                    ),
            )
            .child(
                section("Sizes")
                    .max_w_md()
                    .child(
                        image_avatar("avatar-size-small", "Small Shadcn", "CN", SHADCN_AVATAR)
                            .small(),
                    )
                    .child(image_avatar(
                        "avatar-size-default",
                        "Default Shadcn",
                        "CN",
                        SHADCN_AVATAR,
                    ))
                    .child(
                        image_avatar("avatar-size-large", "Large Shadcn", "CN", SHADCN_AVATAR)
                            .large(),
                    ),
            )
            .child(
                section("Fallback")
                    .max_w_md()
                    .child(
                        Avatar::new("avatar-fallback-text", "Casey Newton")
                            .fallback(AvatarFallback::text("CN")),
                    )
                    .child(
                        Avatar::new("avatar-fallback-icon", "Organization")
                            .fallback(AvatarFallback::icon(IconName::Building2)),
                    )
                    .child(
                        Avatar::new("avatar-fallback-error", "Unavailable image")
                            .image(AvatarImage::new("invalid://avatar"))
                            .fallback(AvatarFallback::text("NA")),
                    ),
            )
    }
}
