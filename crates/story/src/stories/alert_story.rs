// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Removed or replaced story helpers: `set_size`.
// - Added examples for `w_full`, `gap_4`, `destructive`, `description_element`, `aria_label`.
// - Removed examples using `outline`, `compact`, `set_size`, `gap_2`.
// - Reworked Alert story around accessibility semantics and ARIA state, keyboard navigation and
//   activation behavior, focus-visible and focus restoration behavior.
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    ActiveTheme as _, IconName, Sizable as _, alert::Alert, button::Button, dock::PanelControl,
    text::markdown, v_flex,
};

use crate::section;

pub struct AlertStory {
    banner_visible: bool,
    focus_handle: FocusHandle,
}

impl AlertStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            banner_visible: true,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for AlertStory {
    fn title() -> &'static str {
        "Alert"
    }

    fn description() -> &'static str {
        "Displays a callout for user attention."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for AlertStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AlertStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Basic").w_2_3().child(
                    v_flex()
                        .w_full()
                        .gap_4()
                        .child(
                            Alert::new("payment-success")
                                .icon(IconName::CircleCheck)
                                .title("Payment successful")
                                .description(
                                    "Your payment of $29.99 has been processed. A receipt has been sent to your email address.",
                                ),
                        )
                        .child(
                            Alert::new("feature-available")
                                .icon(IconName::Info)
                                .title("New feature available")
                                .description(
                                    "We've added dark mode support. You can enable it in your account settings.",
                                ),
                        ),
                ),
            )
            .child(
                section("Content combinations").w_2_3().child(
                    v_flex()
                        .w_full()
                        .gap_4()
                        .child(Alert::new("title-only").title("Title only alert"))
                        .child(
                            Alert::new("description-only")
                                .description("This alert has a description but no title or icon."),
                        )
                        .child(
                            Alert::new("without-icon")
                                .title("No icon")
                                .description("Title and description align to the leading edge."),
                        ),
                ),
            )
            .child(
                section("Destructive").w_2_3().child(
                    Alert::new("payment-failed")
                        .destructive()
                        .icon(IconName::TriangleAlert)
                        .title("Payment failed")
                        .description(
                            "Your payment could not be processed. Please check your payment method and try again.",
                        ),
                ),
            )
            .child(
                section("Action").w_2_3().child(
                    Alert::new("dark-mode-action")
                        .title("Dark mode is now available")
                        .description("Enable it under your profile settings to get started.")
                        .action(Button::new("enable-dark-mode").xsmall().label("Enable")),
                ),
            )
            .child(
                section("Custom colors").w_2_3().child(
                    Alert::new("subscription-warning")
                        .icon(IconName::TriangleAlert)
                        .title("Your subscription will expire in 3 days.")
                        .description(
                            "Renew now to avoid service interruption or upgrade to a paid plan to continue using the service.",
                        )
                        .bg(cx.theme().warning.opacity(0.08))
                        .border_color(cx.theme().warning.opacity(0.5))
                        .text_color(cx.theme().warning),
                ),
            )
            .child(
                section("Long content").w_2_3().child(
                    Alert::new("long-content")
                        .destructive()
                        .icon(IconName::TriangleAlert)
                        .title("Unable to process your payment")
                        .description_element(markdown(
                            "Please verify your **billing information** and try again.\n\
                            - Check your card details\n\
                            - Ensure sufficient funds\n\
                            - Verify billing address",
                        ))
                        .aria_label(
                            "Unable to process your payment. Please verify your billing information, card details, funds, and billing address.",
                        ),
                ),
            )
            .child(
                section("Banner and closable").w_2_3().child(
                    v_flex()
                        .w_full()
                        .gap_3()
                        .child(
                            Alert::new("closable-banner")
                                .banner()
                                .visible(self.banner_visible)
                                .icon(IconName::Info)
                                .title("Maintenance scheduled")
                                .description("The service will be unavailable tonight from 2:00 to 4:00.")
                                .on_close(cx.listener(|this, _, _, cx| {
                                    this.banner_visible = false;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Alert::new("closable-alert")
                                .icon(IconName::Info)
                                .title("Closable alert")
                                .description("The close button is keyboard accessible.")
                                .on_close(|_, _, _| {}),
                        ),
                ),
            )
    }
}
