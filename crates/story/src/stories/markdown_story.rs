use std::time::Duration;

use gpui::{prelude::FluentBuilder as _, *};
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
    stream_list_state: ListState,
    stream_pending: String,
    stream_finished: bool,
    bytes_to_reveal_per_tick: usize,
    replay_id: usize,
    _stream_subscription: Subscription,
    _provider_task: Task<()>,
    _reveal_task: Task<()>,
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
        let stream_markdown =
            cx.new(|cx| Markdown::new_with_options("# Streaming Markdown\n\n", options, cx));
        let stream_list_state = ListState::new(1, ListAlignment::Top, px(512.));
        stream_list_state.set_follow_mode(FollowMode::Tail);
        let _stream_subscription = cx.observe(&stream_markdown, |this, _, cx| {
            this.stream_list_state.remeasure_items(0..1);
            cx.notify();
        });
        Self {
            style_markdown: cx.new(|cx| Markdown::new(STYLE_EXAMPLE, cx)),
            stream_markdown,
            style_scroll_handle: ScrollHandle::new(),
            stream_list_state,
            stream_pending: String::new(),
            stream_finished: true,
            bytes_to_reveal_per_tick: 1,
            replay_id: 0,
            _stream_subscription,
            _provider_task: Task::ready(()),
            _reveal_task: Task::ready(()),
        }
    }

    /// Replays character-safe chunks to model provider delta arrival independently of parsing.
    fn replay(&mut self, cx: &mut Context<Self>) {
        self.replay_id = self.replay_id.wrapping_add(1);
        let replay_id = self.replay_id;
        self.stream_pending.clear();
        self.stream_finished = false;
        self.bytes_to_reveal_per_tick = 1;
        self.stream_markdown
            .update(cx, |markdown, cx| markdown.replace("", cx));
        self.stream_list_state.remeasure_items(0..1);
        self.stream_list_state.set_follow_mode(FollowMode::Tail);

        self._provider_task = cx.spawn(async move |weak_self, cx| {
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
                    .update(cx, |this, _| {
                        if this.replay_id != replay_id {
                            return false;
                        }
                        this.stream_pending.push_str(&chunk);
                        this.bytes_to_reveal_per_tick = (this.stream_pending.len() as f32
                            / 200.0
                            * 16.0)
                            .ceil()
                            .max(1.0) as usize;
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
            _ = weak_self.update(cx, |this, _| {
                if this.replay_id == replay_id {
                    this.stream_finished = true;
                }
            });
        });

        self._reveal_task = cx.spawn(async move |weak_self, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let should_continue = weak_self
                    .update(cx, |this, cx| {
                        if this.replay_id != replay_id {
                            return false;
                        }
                        if this.stream_pending.is_empty() {
                            return !this.stream_finished;
                        }

                        let end = this
                            .stream_pending
                            .ceil_char_boundary(this.bytes_to_reveal_per_tick)
                            .min(this.stream_pending.len());
                        let chunk = this.stream_pending.drain(..end).collect::<String>();
                        this.stream_markdown
                            .update(cx, |markdown, cx| markdown.append(&chunk, cx));
                        true
                    })
                    .unwrap_or(false);
                if !should_continue {
                    break;
                }
            }
        });
    }

    fn markdown_element(
        markdown: &Entity<Markdown>,
        font: MarkdownFont,
        scroll_handle: Option<&ScrollHandle>,
        window: &Window,
        cx: &App,
    ) -> MarkdownElement {
        let mut style = MarkdownStyle::themed(font, window, cx);
        style.container_style.padding.right = Some(px(8.).into());
        MarkdownElement::new(markdown.clone(), style)
            .when_some(scroll_handle, |element, handle| {
                element.scroll_handle(handle.clone())
            })
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
                                                Some(&self.style_scroll_handle),
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
                                        list(
                                            self.stream_list_state.clone(),
                                            cx.processor(|this, _, window, cx| {
                                                Self::markdown_element(
                                                    &this.stream_markdown,
                                                    MarkdownFont::Agent,
                                                    None,
                                                    window,
                                                    cx,
                                                )
                                                .into_any_element()
                                            }),
                                        )
                                            .size_full(),
                                    )
                                    .vertical_scrollbar(&self.stream_list_state),
                            ),
                    ),
            )
    }
}
