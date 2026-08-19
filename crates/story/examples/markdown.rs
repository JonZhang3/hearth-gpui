//! Production Markdown renderer showcase.

use gpui::*;
use hearth_gpui::{
    ActiveTheme as _,
    button::Button,
    h_flex,
    scroll::ScrollableElement as _,
    text::{
        CodeBlockRenderer, CopyButtonVisibility, Markdown, MarkdownElement, MarkdownFont,
        MarkdownOptions, MarkdownStyle, WrapButtonVisibility,
    },
    v_flex,
};
use hearth_gpui_assets::Assets;

const SOURCE: &str = r#"# Markdown renderer

This preview uses one source-mapped `MarkdownElement` backed by a persistent `Entity<Markdown>`.

> [!TIP]
> Selection, links, search highlights, navigation, and streaming updates share the same canonical source.

## Syntax

- **Strong**, *emphasis*, ~~strike~~, and [links](https://gpui.rs)
- Ordered and unordered lists
- Tables, footnotes[^1], task markers, and fenced code

| Interface | Owner |
| --- | --- |
| Parsing | Markdown entity |
| Vertical scroll | Host container |
| Selection | Markdown entity |

```rust
fn render_markdown(markdown: Entity<Markdown>, style: MarkdownStyle) {
    MarkdownElement::new(markdown, style);
}
```

[^1]: Footnotes remain navigable by source position.
"#;

struct Example {
    markdown: Entity<Markdown>,
    scroll_handle: ScrollHandle,
    appended: usize,
}

impl Example {
    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let options = MarkdownOptions {
            parse_html: true,
            parse_heading_slugs: true,
            render_metadata_blocks: true,
            ..Default::default()
        };
        Self {
            markdown: cx.new(|cx| Markdown::new_with_options(SOURCE, options, cx)),
            scroll_handle: ScrollHandle::new(),
            appended: 0,
        }
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = MarkdownStyle::themed(MarkdownFont::Preview, window, cx);

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .h_12()
                    .px_4()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("append-delta")
                            .label("Append delta")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.appended += 1;
                                let delta = format!(
                                    "\n\nStreaming delta {} keeps **selection** and source mappings stable.",
                                    this.appended
                                );
                                this.markdown
                                    .update(cx, |markdown, cx| markdown.append(&delta, cx));
                            })),
                    )
                    .child(
                        Button::new("search-source")
                            .outline()
                            .label("Find source")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.markdown.update(cx, |markdown, cx| {
                                    markdown.search("source", false, cx);
                                });
                            })),
                    )
                    .child(
                        Button::new("reset")
                            .outline()
                            .label("Reset")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.appended = 0;
                                this.markdown
                                    .update(cx, |markdown, cx| markdown.replace(SOURCE, cx));
                                this.scroll_handle.set_offset(point(px(0.), px(0.)));
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
                            .id("markdown-preview-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                            .child(
                                MarkdownElement::new(self.markdown.clone(), style)
                                    .scroll_handle(self.scroll_handle.clone())
                                    .p_5()
                                    .on_url_click(|url, _, cx| cx.open_url(&url))
                                    .code_block_renderer(CodeBlockRenderer::Default {
                                        copy_button_visibility:
                                            CopyButtonVisibility::VisibleOnHover,
                                        wrap_button_visibility:
                                            WrapButtonVisibility::VisibleOnHover,
                                        border: true,
                                    }),
                            ),
                    )
                    .vertical_scrollbar(&self.scroll_handle),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);
    app.run(move |cx| {
        hearth_gpui_story::init(cx);
        cx.activate(true);
        hearth_gpui_story::create_new_window("Markdown Renderer", Example::view, cx);
    });
}
