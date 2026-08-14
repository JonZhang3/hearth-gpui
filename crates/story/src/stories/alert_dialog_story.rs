// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `media`, `size_8`, `destructive`, `variant`.
// - Removed examples using `p_0`, `on_ok`, `on_cancel`, `p_4`, `border_t_1`, `primary` and 15 more.
// - Reworked Alert Dialog story around accessibility semantics and ARIA state, focus-visible and
//   focus restoration behavior.
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, Window, div,
};

use hearth_gpui::{
    ActiveTheme as _, Icon, IconName, WindowExt as _,
    button::{Button, ButtonVariant},
    dialog::{AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogSize},
    v_flex,
};

use crate::section;

pub struct AlertDialogStory {
    focus_handle: FocusHandle,
}

impl super::Story for AlertDialogStory {
    fn title() -> &'static str {
        "AlertDialog"
    }

    fn description() -> &'static str {
        "A modal dialog that interrupts the user with important content"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl AlertDialogStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
        })
    }
}

impl Focusable for AlertDialogStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AlertDialogStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("alert-dialog-story")
            .track_focus(&self.focus_handle)
            .size_full()
            .child(
                v_flex()
                    .gap_6()
                    .child(section("Default").child(
                        AlertDialog::new(cx)
                            .trigger(Button::new("default-alert").outline().label("Show Dialog"))
                            .on_action(|_, window, cx| {
                                window.push_notification("Action confirmed", cx);
                                true
                            })
                            .content(|content, _, _| {
                                content
                                    .title("Are you absolutely sure?")
                                    .description(
                                        "This action cannot be undone. This will permanently delete your account from our servers.",
                                    )
                                    .cancel(AlertDialogCancel::new("default-cancel", "Cancel"))
                                    .action(AlertDialogAction::new("default-continue", "Continue"))
                            }),
                    ))
                    .child(section("Small").child(
                        AlertDialog::new(cx)
                            .trigger(Button::new("small-alert").outline().label("Request Access"))
                            .content(|content, _, _| {
                                content
                                    .size(AlertDialogSize::Small)
                                    .title("Allow accessory to connect?")
                                    .description(
                                        "Do you want to allow the USB accessory to connect to this device?",
                                    )
                                    .cancel(AlertDialogCancel::new("small-cancel", "Don't allow"))
                                    .action(AlertDialogAction::new("small-allow", "Allow"))
                            }),
                    ))
                    .child(section("With Media").child(
                        AlertDialog::new(cx)
                            .trigger(Button::new("media-alert").outline().label("Share Project"))
                            .content(|content, _, cx| {
                                content
                                    .media(
                                        Icon::new(IconName::Info)
                                            .size_8()
                                            .text_color(cx.theme().foreground),
                                    )
                                    .title("Share this project?")
                                    .description(
                                        "Anyone with the link will be able to view and edit this project.",
                                    )
                                    .cancel(AlertDialogCancel::new("media-cancel", "Cancel"))
                                    .action(AlertDialogAction::new("media-share", "Share"))
                            }),
                    ))
                    .child(section("Destructive").child(
                        AlertDialog::new(cx)
                            .trigger(
                                Button::new("delete-alert")
                                    .destructive()
                                    .label("Delete Chat"),
                            )
                            .on_action(|_, window, cx| {
                                window.push_notification("Chat deleted", cx);
                                true
                            })
                            .content(|content, _, cx| {
                                content
                                    .size(AlertDialogSize::Small)
                                    .media(
                                        Icon::new(IconName::TriangleAlert)
                                            .size_8()
                                            .text_color(cx.theme().danger),
                                    )
                                    .title("Delete chat?")
                                    .description(
                                        "This will permanently delete this chat conversation. Review Settings before continuing.",
                                    )
                                    .cancel(AlertDialogCancel::new("delete-cancel", "Cancel"))
                                    .action(
                                        AlertDialogAction::new("delete-confirm", "Delete")
                                            .variant(ButtonVariant::Destructive),
                                    )
                            }),
                    ))
                    .child(section("Imperative").child(
                        Button::new("imperative-alert")
                            .outline()
                            .label("Open Imperatively")
                            .on_click(|_, window, cx| {
                                window.open_alert_dialog(cx, |dialog, _, _| {
                                    dialog.content(|content, _, _| {
                                        content
                                            .title("Session expired")
                                            .description(
                                                "Your session expired due to inactivity. Sign in again to continue.",
                                            )
                                            .action(AlertDialogAction::new(
                                                "session-sign-in",
                                                "Sign in",
                                            ))
                                    })
                                });
                            }),
                    )),
            )
    }
}
