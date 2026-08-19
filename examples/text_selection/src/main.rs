//! Window-level text selection demo: drag across multiple chat bubbles and
//! press cmd-c / ctrl-c to copy the merged text.
//!
//! - Drag from anywhere inside the window (even the blank space between
//!   bubbles) to start a selection that spans multiple `TextView`s.
//! - The copied text keeps the top-to-bottom order, joined by newlines.
//! - Clicking the `Button` does NOT start a selection.
//! - Dragging inside the `Input` only drives the `Input`'s own selection.
//!
//! Run: `cargo run -p text_selection`

use gpui::{prelude::FluentBuilder as _, *};
use hearth_gpui::{
    button::Button,
    input::{Input, InputState},
    text::{Markdown, MarkdownElement, MarkdownFont, MarkdownStyle},
    *,
};
use hearth_gpui_assets::Assets;

struct ChatExample {
    input: Entity<InputState>,
    messages: Vec<Entity<Markdown>>,
}

impl ChatExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Type here (selection must NOT start from here)")
            }),
            messages: [
                "**Hello!** How can I help you today?",
                "I want to select text *across* multiple bubbles.",
                "Sure — selection is local to each Markdown entity, then press `cmd-c` to copy it.",
                "Nice, it keeps source-mapped selection stable.",
            ]
            .into_iter()
            .map(|source| cx.new(|cx| Markdown::new(source, cx)))
            .collect(),
        }
    }

    fn bubble(&self, ix: usize, mine: bool, window: &Window, cx: &App) -> impl IntoElement {
        div().flex().when(mine, |this| this.justify_end()).child(
            div()
                .max_w(px(420.))
                .p_3()
                .rounded_lg()
                .bg(if mine {
                    cx.theme().primary.opacity(0.1)
                } else {
                    cx.theme().muted
                })
                .child(MarkdownElement::new(
                    self.messages[ix].clone(),
                    MarkdownStyle::themed(MarkdownFont::Agent, window, cx),
                )),
        )
    }
}

impl Render for ChatExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(self.bubble(0, false, window, cx))
            .child(self.bubble(1, true, window, cx))
            .child(self.bubble(2, false, window, cx))
            .child(self.bubble(3, true, window, cx))
            .child(div().flex_1())
            .child(Button::new("noop").label("Clicking me must not start selection"))
            .child(Input::new(&self.input))
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx| {
        // This must be called before using any Hearth GPUI features.
        hearth_gpui::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(800.), px(600.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| ChatExample::new(window, cx));
                // The first level view on the window should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
