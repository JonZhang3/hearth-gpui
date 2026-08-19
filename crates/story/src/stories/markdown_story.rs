use std::time::Duration;

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

const STREAM_EXAMPLE: &str = include_str!("../../examples/fixtures/test.md");
const STYLE_EXAMPLE: &str = r#"## Zed-style renderer

[Links keep source-mapped hit testing across wrapped visual lines.](https://example.com)

Inline code such as `MarkdownElement` uses the configured monospace style.

> [!TIP]
> Selection, link hit testing, copy, and scrolling are owned by one Markdown entity.

```rust
fn streamed_markdown() -> &'static str {
    "one custom Element"
}
```
"#;

pub struct MarkdownStory {
    style_markdown: Entity<Markdown>,
    stream_markdown: Entity<Markdown>,
    style_scroll_handle: ScrollHandle,
    stream_scroll_handle: ScrollHandle,
    following_tail: bool,
    replay_id: usize,
    _update_task: Task<()>,
}

impl super::Story for MarkdownStory {
    fn title() -> &'static str {
        "Markdown"
    }

    fn description() -> &'static str {
        "Render source-mapped Markdown with entity-local selection and progressive LLM updates."
    }

    fn paddings() -> Pixels {
        px(0.)
    }

    fn container_scrollable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl MarkdownStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let options = MarkdownOptions {
            parse_html: true,
            render_mermaid_diagrams: true,
            parse_heading_slugs: true,
            render_metadata_blocks: true,
            ..Default::default()
        };
        Self {
            style_markdown: cx.new(|cx| Markdown::new(STYLE_EXAMPLE, cx)),
            stream_markdown: cx
                .new(|cx| Markdown::new_with_options("# Streaming Markdown\n\n", options, cx)),
            style_scroll_handle: ScrollHandle::new(),
            stream_scroll_handle: ScrollHandle::new(),
            following_tail: true,
            replay_id: 0,
            _update_task: Task::ready(()),
        }
    }

    /// Replays character-safe chunks to model provider delta arrival independently of parsing.
    fn replay(&mut self, cx: &mut Context<Self>) {
        self.replay_id = self.replay_id.wrapping_add(1);
        let replay_id = self.replay_id;
        self.following_tail = true;
        self.stream_markdown
            .update(cx, |markdown, cx| markdown.replace("", cx));
        self.stream_scroll_handle.scroll_to_bottom();

        self._update_task = cx.spawn(async move |weak_self, cx| {
            let source = format!(
                "Streaming repairs **strong text**, `inline code`, and [pending links](https://example.com/path) without replacing the last valid frame.\n\n{STREAM_EXAMPLE}"
            );
            let mut cursor = 0;
            while cursor < source.len() {
                let mut end = (cursor + 24).min(source.len());
                while !source.is_char_boundary(end) {
                    end -= 1;
                }
                let chunk = source[cursor..end].to_string();
                cursor = end;
                let should_continue = weak_self
                    .update(cx, |this, cx| {
                        if this.replay_id != replay_id {
                            return false;
                        }
                        this.stream_markdown
                            .update(cx, |markdown, cx| markdown.append(&chunk, cx));
                        if this.following_tail {
                            this.stream_scroll_handle.scroll_to_bottom();
                        }
                        true
                    })
                    .unwrap_or(false);
                if !should_continue {
                    return;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(24))
                    .await;
            }
        });
    }

    /// Pauses follow-tail as soon as a wheel gesture leaves the bottom edge.
    fn update_follow_tail(&mut self, event: &ScrollWheelEvent, window: &Window) {
        let max_offset = self.stream_scroll_handle.max_offset().y.max(px(0.));
        let delta = event.delta.pixel_delta(window.line_height()).y;
        let next_offset = (self.stream_scroll_handle.offset().y + delta).clamp(-max_offset, px(0.));
        self.following_tail = (next_offset + max_offset).abs() <= px(1.);
    }

    fn markdown_element(
        markdown: &Entity<Markdown>,
        font: MarkdownFont,
        scroll_handle: &ScrollHandle,
        window: &Window,
        cx: &App,
    ) -> MarkdownElement {
        let mut style = MarkdownStyle::themed(font, window, cx);
        style.container_style.padding.right = Some(px(8.).into());
        MarkdownElement::new(markdown.clone(), style)
            .scroll_handle(scroll_handle.clone())
            .on_url_click(|url, _, cx| cx.open_url(&url))
            .code_block_renderer(CodeBlockRenderer::Default {
                copy_button_visibility: CopyButtonVisibility::VisibleOnHover,
                wrap_button_visibility: WrapButtonVisibility::VisibleOnHover,
                border: true,
            })
    }
}

impl Render for MarkdownStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.following_tail {
            let max_offset = self.stream_scroll_handle.max_offset().y.max(px(0.));
            self.following_tail =
                (self.stream_scroll_handle.offset().y + max_offset).abs() <= px(1.);
        }

        v_flex()
            .id("markdown-story")
            .size_full()
            .p_5()
            .gap_4()
            .child(
                h_flex()
                    .w_full()
                    .items_end()
                    .justify_between()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Streaming Markdown"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Single-element rendering, source mapping, and host-owned scrolling"),
                            ),
                    )
                    .child(
                        Button::new("replay")
                            .outline()
                            .label("Replay")
                            .on_click(cx.listener(|this, _, _, cx| this.replay(cx))),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .items_start()
                    .gap_5()
                    .child(
                        v_flex()
                            .w(px(380.))
                            .h_full()
                            .flex_none()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Style and interaction cases"),
                            )
                            .child(
                                div()
                                    .id("style-cases-scroll")
                                    .relative()
                                    .flex_1()
                                    .min_h_0()
                                    .child(
                                        div()
                                            .id("style-cases-scroll-area")
                                            .size_full()
                                            .overflow_y_scroll()
                                            .track_scroll(&self.style_scroll_handle)
                                            .child(Self::markdown_element(
                                                &self.style_markdown,
                                                MarkdownFont::Preview,
                                                &self.style_scroll_handle,
                                                window,
                                                cx,
                                            )),
                                    )
                                    .vertical_scrollbar(&self.style_scroll_handle),
                            ),
                    )
                    .child(
                        v_flex()
                            .h_full()
                            .flex_1()
                            .min_w_0()
                            .gap_3()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("Live LLM-style stream"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Provider deltas coalesce behind one full-source background parser."),
                                    ),
                            )
                            .child(
                                div()
                                    .id("stream-scroll")
                                    .relative()
                                    .flex_1()
                                    .min_h_0()
                                    .child(
                                        div()
                                            .id("stream-scroll-area")
                                            .size_full()
                                            .overflow_y_scroll()
                                            .track_scroll(&self.stream_scroll_handle)
                                            .on_scroll_wheel(cx.listener(|this, event, window, _| {
                                                this.update_follow_tail(event, window);
                                            }))
                                            .child(Self::markdown_element(
                                                &self.stream_markdown,
                                                MarkdownFont::Agent,
                                                &self.stream_scroll_handle,
                                                window,
                                                cx,
                                            )),
                                    )
                                    .vertical_scrollbar(&self.stream_scroll_handle),
                            ),
                    ),
            )
    }
}
