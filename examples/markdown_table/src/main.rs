//! A minimal markdown viewer for debugging table rendering.
//!
//! Cycle between the three table layouts:
//! - wrap (default): cells wrap, columns shrink to fit the frame.
//! - adaptive (`style.table` overflow-x: scroll): columns fit their content,
//!   wrap down to a floor as the frame narrows, then scroll horizontally.
//! - nowrap (adaptive + `style.table_cell` white-space: nowrap): cells stay
//!   on a single line, the table scrolls as soon as the content overflows.
//!
//! Edit `src/report.md` to change the markdown source.
//!
//! Run: `cargo run -p markdown_table`

use gpui::*;
use hearth_gpui::{
    button::Button,
    scroll::ScrollableElement as _,
    text::{Markdown, MarkdownElement, MarkdownFont, MarkdownStyle},
    *,
};
use hearth_gpui_assets::Assets;

const SOURCE: &str = include_str!("report.md");

/// Markdown source: `MD_FILE=<path>` overrides the bundled `report.md`, so you
/// can iterate on a repro without recompiling.
fn source() -> SharedString {
    match std::env::var("MD_FILE") {
        Ok(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|err| format!("Failed to read `{path}`: {err}"))
            .into(),
        Err(_) => SOURCE.into(),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum TableMode {
    Wrap,
    Adaptive,
    Nowrap,
}

impl TableMode {
    /// `TABLE_MODE=wrap|adaptive|nowrap` picks the initial mode.
    fn initial() -> Self {
        match std::env::var("TABLE_MODE").as_deref() {
            Ok("wrap") => Self::Wrap,
            Ok("nowrap") => Self::Nowrap,
            _ => Self::Adaptive,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Wrap => Self::Adaptive,
            Self::Adaptive => Self::Nowrap,
            Self::Nowrap => Self::Wrap,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Wrap => "Table: wrap",
            Self::Adaptive => "Table: scroll (adaptive)",
            Self::Nowrap => "Table: scroll (nowrap)",
        }
    }
}

struct Example {
    mode: TableMode,
    scroll_handle: ScrollHandle,
    markdown: Entity<Markdown>,
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut markdown_style = MarkdownStyle::themed(MarkdownFont::Preview, window, cx);
        markdown_style.table_columns_min_size = self.mode != TableMode::Wrap;
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .p_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("toggle")
                            .label(self.mode.label())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.mode = this.mode.next();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("markdown-table-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                            .child(
                                MarkdownElement::new(self.markdown.clone(), markdown_style)
                                    .scroll_handle(self.scroll_handle.clone())
                                    .p_4(),
                            ),
                    )
                    .vertical_scrollbar(&self.scroll_handle),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx| {
        // This must be called before using any Hearth GPUI features.
        hearth_gpui::init(cx);

        // `WIN_W=<px>` overrides the window width, to check how the table
        // adapts at different frame widths.
        let width = std::env::var("WIN_W")
            .ok()
            .and_then(|w| w.parse().ok())
            .unwrap_or(900.);
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(width), px(700.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let markdown = cx.new(|cx| Markdown::new(source(), cx));
                let view = cx.new(|_| Example {
                    mode: TableMode::initial(),
                    scroll_handle: ScrollHandle::new(),
                    markdown,
                });
                // The first level view on the window should be a Root.
                cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
