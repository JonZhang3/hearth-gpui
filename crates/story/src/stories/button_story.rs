// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Removed or replaced story helpers: `on_click`, `on_hover`.
// - Added examples for `flex_wrap`, `destructive`, `aria_label`, `trailing_icon`, `rounded_full`,
//   `group` and 1 more.
// - Removed examples using `color`, `foreground`, `hover`, `refresh`, `max_w_lg`, `loading` and 15
//   more.
// - Reworked Button story around accessibility semantics and ARIA state, focus-visible and focus
//   restoration behavior.
use gpui::{
    App, AppContext as _, Entity, Focusable, IntoElement, ParentElement as _, Render, Styled as _,
    Window,
};
use gpui_component::{
    Disableable as _, IconName, Sizable as _,
    button::{Button, ButtonGroup, ButtonGroupSeparator, ButtonGroupText},
    h_flex,
    spinner::Spinner,
    v_flex,
};

use crate::section;

pub struct ButtonStory {
    focus_handle: gpui::FocusHandle,
}

impl ButtonStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
        })
    }
}

impl super::Story for ButtonStory {
    fn title() -> &'static str {
        "Button"
    }

    fn description() -> &'static str {
        "Displays an action using the shadcn Vega visual baseline."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ButtonStory {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ButtonStory {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Variants").child(
                    h_flex()
                        .gap_3()
                        .flex_wrap()
                        .child(Button::new("default").label("Default"))
                        .child(Button::new("outline").outline().label("Outline"))
                        .child(Button::new("secondary").secondary().label("Secondary"))
                        .child(Button::new("ghost").ghost().label("Ghost"))
                        .child(
                            Button::new("destructive")
                                .destructive()
                                .label("Destructive"),
                        )
                        .child(Button::new("link").link().label("Link")),
                ),
            )
            .child(
                section("Sizes").child(
                    h_flex()
                        .items_center()
                        .gap_4()
                        .flex_wrap()
                        .child(Button::new("xs").xsmall().label("Extra Small"))
                        .child(
                            Button::new("xs-icon")
                                .xsmall()
                                .outline()
                                .icon(IconName::ArrowUp)
                                .aria_label("Extra small"),
                        )
                        .child(Button::new("sm").small().label("Small"))
                        .child(
                            Button::new("sm-icon")
                                .small()
                                .outline()
                                .icon(IconName::ArrowUp)
                                .aria_label("Small"),
                        )
                        .child(Button::new("md").label("Default"))
                        .child(
                            Button::new("md-icon")
                                .outline()
                                .icon(IconName::ArrowUp)
                                .aria_label("Default"),
                        )
                        .child(Button::new("lg").large().label("Large"))
                        .child(
                            Button::new("lg-icon")
                                .large()
                                .outline()
                                .icon(IconName::ArrowUp)
                                .aria_label("Large"),
                        ),
                ),
            )
            .child(
                section("With Icon").child(
                    h_flex()
                        .gap_3()
                        .child(
                            Button::new("leading-icon")
                                .outline()
                                .icon(IconName::Github)
                                .label("New Branch"),
                        )
                        .child(
                            Button::new("trailing-icon")
                                .outline()
                                .label("Continue")
                                .trailing_icon(IconName::ArrowUp),
                        ),
                ),
            )
            .child(
                section("Rounded").child(
                    Button::new("rounded")
                        .outline()
                        .rounded_full()
                        .icon(IconName::ArrowUp)
                        .aria_label("Move up"),
                ),
            )
            .child(
                section("Spinner").child(
                    h_flex()
                        .gap_3()
                        .child(
                            Button::new("generating")
                                .outline()
                                .icon(Spinner::new())
                                .label("Generating")
                                .disabled(true),
                        )
                        .child(
                            Button::new("downloading")
                                .secondary()
                                .label("Downloading")
                                .trailing_icon(Spinner::new())
                                .disabled(true),
                        ),
                ),
            )
            .child(
                section("Button Group").child(
                    ButtonGroup::new("message-actions")
                        .aria_label("Message actions")
                        .child(
                            Button::new("back")
                                .outline()
                                .icon(IconName::ArrowLeft)
                                .aria_label("Back"),
                        )
                        .group(
                            ButtonGroup::new("archive-report")
                                .child(Button::new("archive").outline().label("Archive"))
                                .child(Button::new("report").outline().label("Report")),
                        )
                        .separator(ButtonGroupSeparator::new())
                        .text(ButtonGroupText::new("More"))
                        .group(
                            ButtonGroup::new("snooze-more")
                                .child(Button::new("snooze").outline().label("Snooze"))
                                .child(
                                    Button::new("more")
                                        .outline()
                                        .icon(IconName::Ellipsis)
                                        .aria_label("More actions"),
                                ),
                        ),
                ),
            )
    }
}
