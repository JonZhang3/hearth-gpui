// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `rotate`, `sub_title`, `text_lg`, `large`, `size_8`.
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, radians,
};
use hearth_gpui::{
    ActiveTheme as _, Icon, IconName, Sizable, Size,
    button::{Button, ButtonVariant, ButtonVariants},
    dock::PanelControl,
    h_flex, neutral_500, v_flex,
};

use crate::section;

pub struct IconStory {
    focus_handle: gpui::FocusHandle,
    entity_icon: Entity<Icon>,
}

impl IconStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            entity_icon: cx
                .new(|_| Icon::new(IconName::ArrowUp).rotate(radians(std::f32::consts::FRAC_PI_4))),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for IconStory {
    fn title() -> &'static str {
        "Icon"
    }

    fn description() -> &'static str {
        "SVG Icons based on Lucide.dev"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for IconStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for IconStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Icon")
                    .text_lg()
                    .child(IconName::Info)
                    .child(IconName::Map)
                    .child(IconName::Bot)
                    .child(IconName::Github)
                    .child(IconName::Calendar)
                    .child(IconName::Globe)
                    .child(IconName::Heart),
            )
            .child(
                section("Sizes")
                    .sub_title("Inherited, extra small, small, medium, large, and custom")
                    .text_lg()
                    .child(Icon::new(IconName::CircleCheck))
                    .child(Icon::new(IconName::CircleCheck).xsmall())
                    .child(Icon::new(IconName::CircleCheck).small())
                    .child(Icon::new(IconName::CircleCheck).with_size(Size::Medium))
                    .child(Icon::new(IconName::CircleCheck).large())
                    .child(Icon::new(IconName::CircleCheck).size_8()),
            )
            .child(
                section("Color and Transform")
                    .sub_title("Direct and Entity Icons inherit the surrounding color")
                    .text_lg()
                    .text_color(cx.theme().primary)
                    .child(Icon::new(IconName::ArrowUp))
                    .child(
                        Icon::new(IconName::ArrowUp).rotate(radians(std::f32::consts::FRAC_PI_2)),
                    )
                    .child(self.entity_icon.clone()),
            )
            .child(
                section("Explicit Color")
                    .child(
                        Icon::new(IconName::Maximize)
                            .size_6()
                            .text_color(cx.theme().green),
                    )
                    .child(
                        Icon::new(IconName::Minimize)
                            .size_6()
                            .text_color(cx.theme().red),
                    ),
            )
            .child(
                section("Informative Icon").child(
                    Icon::informative("icon-story-ready", IconName::CircleCheck, "Ready")
                        .large()
                        .text_color(cx.theme().green),
                ),
            )
            .child(
                section("Icon Button").child(
                    h_flex()
                        .gap_4()
                        .child(
                            Button::new("like1")
                                .icon(
                                    Icon::new(IconName::Heart)
                                        .text_color(neutral_500())
                                        .size_6(),
                                )
                                .with_variant(ButtonVariant::Ghost),
                        )
                        .child(
                            Button::new("like2")
                                .icon(
                                    Icon::new(IconName::HeartOff)
                                        .text_color(cx.theme().red)
                                        .size_6(),
                                )
                                .with_variant(ButtonVariant::Ghost),
                        )
                        .child(
                            Button::new("like3")
                                .icon(
                                    Icon::new(IconName::Heart)
                                        .text_color(cx.theme().green)
                                        .size_6(),
                                )
                                .with_variant(ButtonVariant::Ghost),
                        ),
                ),
            )
            .child(
                section("Button with size").child(
                    Button::new("button-with-size")
                        .outline()
                        .size_5()
                        .small()
                        .px_0()
                        .label("10"),
                ),
            )
    }
}
