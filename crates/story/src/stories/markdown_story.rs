use gpui::*;
use hearth_gpui::{
    ActiveTheme as _,
    button::Button,
    h_flex,
    text::{
        MarkdownBlockKind, MarkdownElementKind, MarkdownInlineKind, MarkdownNode,
        MarkdownParseContext, MarkdownPlugin, MarkdownResourcePolicy, MarkdownStyle,
        MarkdownTextStyle, StreamingTextPacer, TextView, TextViewState, markdown_ast,
    },
    v_flex,
};

const STREAM_EXAMPLE: &str = include_str!("../../examples/fixtures/test.md");
const STYLE_EXAMPLE: &str = r#"## Semantic styles

[This link has a custom background and underline. Hover it to remove both.](https://example.com)

![Inline image](icons/heart.svg) [This deliberately long link wraps across visual lines and keeps the same hover state in every fragment.](https://example.com/wrapped-link)

Inline plugin: `badge:atomic flow`

```rust
fn streamed_markdown() -> &'static str {
    "custom code block"
}
```
"#;

struct InlineBadgePlugin;

impl MarkdownPlugin for InlineBadgePlugin {
    fn name(&self) -> &str {
        "inline-badge"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let markdown_ast::Node::InlineCode(code) = node else {
            return None;
        };
        let label = code.value.strip_prefix("badge:")?.trim().to_string();
        Some(
            MarkdownNode::new("inline-badge", label.clone())
                .text(label)
                .markdown(cx.node_source(node).unwrap_or_default()),
        )
    }

    fn render(&self, node: &MarkdownNode, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .px_2()
            .py_0p5()
            .rounded(cx.theme().style.radii.sm)
            .bg(cx.theme().accent)
            .text_xs()
            .child(node.as_text().to_string())
    }
}

pub struct MarkdownStory {
    markdown_state: Entity<TextViewState>,
    replay_id: usize,
    _update_task: Task<()>,
}

impl super::Story for MarkdownStory {
    fn title() -> &'static str {
        "Markdown"
    }

    fn description() -> &'static str {
        "Render Markdown with semantic styles, custom block renderers, and progressive LLM streaming."
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
        let markdown_state =
            cx.new(|cx| TextViewState::markdown("# Streaming Markdown Parse\n\n", cx));

        Self {
            markdown_state,
            replay_id: 0,
            _update_task: Task::ready(()),
        }
    }

    /// Applies semantic inline styles and refines the built-in code-block renderer.
    fn styled_markdown(view: TextView, cx: &App) -> TextView {
        let style = MarkdownStyle::default()
            .inline(
                MarkdownInlineKind::Link,
                MarkdownTextStyle::default()
                    .color(cx.theme().link)
                    .underline(UnderlineStyle {
                        thickness: px(1.),
                        ..Default::default()
                    }),
            )
            .inline(
                MarkdownInlineKind::LinkHover,
                MarkdownTextStyle::default().color(cx.theme().primary),
            )
            .inline(
                MarkdownInlineKind::InlineCode,
                MarkdownTextStyle::default()
                    .color(cx.theme().primary)
                    .font_family(cx.theme().mono_font_family.clone())
                    .font_size(px(13.))
                    .line_height(px(18.))
                    .background(cx.theme().muted)
                    .padding_x(px(4.))
                    .padding_y(px(2.))
                    .corner_radius(cx.theme().style.radii.sm),
            )
            .element(
                MarkdownElementKind::CodeBlock,
                StyleRefinement::default()
                    .rounded(cx.theme().style.radii.md)
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted.opacity(0.55)),
            );

        let border = cx.theme().border;
        let muted_foreground = cx.theme().muted_foreground;
        let radius = cx.theme().style.radii.md;
        view.markdown_resource_policy(MarkdownResourcePolicy::llm_safe())
            .on_link_click(|url, _, cx| cx.open_url(url))
            .plugin(InlineBadgePlugin)
            .markdown_style(style)
            .markdown_builtin_renderer(
                MarkdownBlockKind::CodeBlock,
                move |context, _window, _cx| {
                    let language = context
                        .code_language()
                        .unwrap_or("plain text")
                        .to_uppercase();
                    v_flex()
                        .w_full()
                        .rounded(radius)
                        .border_1()
                        .border_color(border)
                        .overflow_hidden()
                        .child(
                            h_flex()
                                .px_3()
                                .py_1p5()
                                .border_b_1()
                                .border_color(border)
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(muted_foreground)
                                .child(format!("CUSTOM RENDERER · {language}")),
                        )
                        .child(context.into_default())
                },
            )
    }

    /// Replays Markdown in randomized character-safe chunks to simulate LLM deltas.
    fn replay(&mut self, cx: &mut Context<Self>) {
        self.replay_id = self.replay_id.wrapping_add(1);
        let replay_id = self.replay_id;
        self.markdown_state.update(cx, |state, cx| {
            state.set_text("", cx);
            state.begin_streaming(cx);
            state.set_follow_tail(true, cx);
        });

        self._update_task = cx.spawn(async move |weak_self, cx| {
            let mut pacer = StreamingTextPacer::new();
            pacer.push_str(
                "Streaming repairs **strong text**, `inline code`, and [pending links](https://example.com/path) before their closers arrive.\n\n",
            );
            pacer.push_str(STREAM_EXAMPLE);
            let frame_interval = pacer.frame_interval();

            while let Some(chunk) = pacer.take_chunk() {
                let should_continue = weak_self
                    .update(cx, |this, cx| {
                        if replay_id != this.replay_id {
                            return false;
                        }

                        this.markdown_state.update(cx, |state, cx| {
                            state.push_str(&chunk, cx);
                        });
                        true
                    })
                    .unwrap_or(false);
                if !should_continue {
                    return;
                }

                cx.background_executor()
                    .timer(frame_interval)
                    .await;
            }

            _ = weak_self.update(cx, |this, cx| {
                if replay_id != this.replay_id {
                    return;
                }

                this.markdown_state.update(cx, |state, cx| {
                    state.finish_streaming(cx);
                });
            });
        });
    }
}

impl Render for MarkdownStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                                    .child(
                                        "LLM deltas, semantic styles, hover state, and renderer overrides",
                                    ),
                            ),
                    )
                    .child(
                        Button::new("replay")
                            .outline()
                            .label("Replay")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.replay(cx);
                            })),
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
                                v_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("Style and interaction cases"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(
                                                "Hover both links. The second link wraps beside an inline image.",
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .id("style-cases-scroll")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .pr_2()
                                    .child(Self::styled_markdown(
                                        TextView::markdown("style-showcase", STYLE_EXAMPLE)
                                            .selectable(true),
                                        cx,
                                    )),
                            ),
                    )
                    .child(
                        v_flex()
                            .id("contents")
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
                                            .child(
                                                "Replay paces buffered text chunks, repairs incomplete inline tails, and settles canonically.",
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .id("stream-scroll")
                                    .flex_1()
                                    .min_h_0()
                                    .pr_2()
                                    .child(Self::styled_markdown(
                                        TextView::new(&self.markdown_state)
                                            .selectable(true)
                                            .scrollable(true)
                                            .follow_tail(true),
                                        cx,
                                    )),
                            ),
                    ),
            )
    }
}
