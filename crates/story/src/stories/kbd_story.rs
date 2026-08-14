// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `gap_2`, `appearance`.
// - Reworked Kbd story around keyboard navigation and activation behavior.
use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, Keystroke, ParentElement, Render,
    Styled, Window,
};

use gpui_component::{
    Icon, IconName, Sizable as _, h_flex,
    kbd::{Kbd, KbdGroup},
    v_flex,
};

use crate::section;

pub struct KbdStory {
    focus_handle: gpui::FocusHandle,
}

impl super::Story for KbdStory {
    fn title() -> &'static str {
        "Kbd"
    }

    fn description() -> &'static str {
        "Displays textual keyboard input and grouped shortcuts."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl KbdStory {
    pub(crate) fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}
impl Focusable for KbdStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for KbdStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Basic").child(
                    h_flex()
                        .gap_2()
                        .child(Kbd::new().child("Ctrl"))
                        .child(Kbd::new().child("⌘K"))
                        .child(Kbd::new().child("Ctrl + B")),
                ),
            )
            .child(
                section("Modifier Keys").child(
                    h_flex()
                        .gap_2()
                        .child(Kbd::new().child("⌘"))
                        .child(Kbd::new().child("C")),
                ),
            )
            .child(
                section("KbdGroup").child(
                    KbdGroup::new()
                        .child(Kbd::new().child("Ctrl"))
                        .child(Kbd::new().child("Shift"))
                        .child(Kbd::new().child("P")),
                ),
            )
            .child(
                section("Arrow Keys").child(
                    KbdGroup::new()
                        .child(Kbd::new().child("↑"))
                        .child(Kbd::new().child("↓"))
                        .child(Kbd::new().child("←"))
                        .child(Kbd::new().child("→")),
                ),
            )
            .child(
                section("Icons and Text").child(
                    KbdGroup::new()
                        .child(
                            Kbd::new()
                                .child(Icon::new(IconName::ArrowLeft).xsmall())
                                .child("Left"),
                        )
                        .child(
                            Kbd::new()
                                .child(Icon::new(IconName::LoaderCircle).xsmall())
                                .child("Loading"),
                        ),
                ),
            )
            .child(
                section("Platform Keystrokes").child(
                    h_flex()
                        .gap_2()
                        .child(Kbd::from_keystroke(
                            Keystroke::parse("cmd-shift-p").unwrap(),
                        ))
                        .child(Kbd::from_keystroke(Keystroke::parse("cmd-ctrl-t").unwrap()))
                        .child(Kbd::from_keystroke(Keystroke::parse("escape").unwrap()))
                        .child(Kbd::from_keystroke(Keystroke::parse("enter").unwrap())),
                ),
            )
            .child(
                section("Project Extensions").child(
                    h_flex()
                        .gap_2()
                        .child(Kbd::new().child("Outline").outline())
                        .child(Kbd::new().child("Unstyled").appearance(false)),
                ),
            )
    }
}
