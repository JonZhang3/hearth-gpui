use std::time::Duration;

use gpui::*;
use hearth_gpui::{
    ActiveTheme as _,
    button::Button,
    h_flex,
    text::{
        MarkdownBlockKind, MarkdownElementKind, MarkdownInlineKind, MarkdownStyle,
        MarkdownTextStyle, TextView, TextViewState,
    },
    v_flex,
};

const STREAM_EXAMPLE: &str = include_str!("../../examples/fixtures/test.md");
const AUTO_FOLLOW_THRESHOLD_PX: f32 = 24.;
const STYLE_EXAMPLE: &str = r#"## Semantic styles

[This link has a custom background and underline. Hover it to remove both.](https://example.com)

![Inline image](icons/heart.svg) [This deliberately long link wraps across visual lines and keeps the same hover state in every fragment.](https://example.com/wrapped-link)

```rust
fn streamed_markdown() -> &'static str {
    "custom code block"
}
```
"#;

pub struct MarkdownStory {
    markdown_state: Entity<TextViewState>,
    scroll_handle: ScrollHandle,
    replay_id: usize,
    _subscriptions: Vec<Subscription>,
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
        let _subscriptions = vec![cx.observe(&markdown_state, |this, _, cx| {
            this.follow_stream_if_near_bottom(cx);
        })];

        Self {
            markdown_state,
            scroll_handle: ScrollHandle::new(),
            replay_id: 0,
            _subscriptions,
            _update_task: Task::ready(()),
        }
    }

    /// Requests bottom alignment after parsed Markdown has updated the content layout.
    fn follow_stream_if_near_bottom(&mut self, cx: &mut Context<Self>) {
        let offset = self.scroll_handle.offset();
        let max_offset = self.scroll_handle.max_offset();
        if !is_near_bottom(offset.y, max_offset.y) {
            return;
        }

        self.scroll_handle.scroll_to_bottom();
        cx.notify();
    }

    /// Restores bottom-follow mode when the user explicitly starts a new replay.
    fn reset_replay_scroll(&self) {
        let offset = self.scroll_handle.offset();
        let max_offset = self.scroll_handle.max_offset();
        self.scroll_handle
            .set_offset(point(offset.x, -max_offset.y));
        self.scroll_handle.scroll_to_bottom();
    }

    /// Applies semantic inline styles and refines the built-in code-block renderer.
    fn styled_markdown(view: TextView, cx: &App) -> TextView {
        let style = MarkdownStyle::default()
            .inline(
                MarkdownInlineKind::Link,
                MarkdownTextStyle::default()
                    .color(cx.theme().link)
                    .background(cx.theme().accent)
                    .underline(UnderlineStyle {
                        thickness: px(1.),
                        ..Default::default()
                    }),
            )
            .inline(
                MarkdownInlineKind::LinkHover,
                MarkdownTextStyle::default()
                    .color(cx.theme().primary)
                    .no_background()
                    .no_underline(),
            )
            .inline(
                MarkdownInlineKind::InlineCode,
                MarkdownTextStyle::default()
                    .color(cx.theme().primary)
                    .background(cx.theme().muted),
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
        view.markdown_style(style).markdown_builtin_renderer(
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
        self.reset_replay_scroll();
        self.markdown_state.update(cx, |state, cx| {
            state.set_text("", cx);
        });

        self._update_task = cx.spawn(async move |weak_self, cx| {
            let chars: Vec<char> = STREAM_EXAMPLE.chars().collect();
            let mut current = 0;

            while current < chars.len() {
                let chunk_size = (5 + rand::random::<usize>() % 15).min(chars.len() - current);
                let chunk: String = chars[current..current + chunk_size].iter().collect();
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

                current += chunk_size;
                cx.background_executor()
                    .timer(Duration::from_millis(50))
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

/// Returns whether the viewport is close enough to the bottom to keep following output.
fn is_near_bottom(scroll_offset_y: Pixels, max_offset_y: Pixels) -> bool {
    -scroll_offset_y >= max_offset_y - px(AUTO_FOLLOW_THRESHOLD_PX)
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
                                                "Replay appends randomized chunks and calls finish_streaming at completion.",
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .id("stream-scroll")
                                    .flex_1()
                                    .min_h_0()
                                    .track_scroll(&self.scroll_handle)
                                    .overflow_y_scroll()
                                    .pr_2()
                                    .child(Self::styled_markdown(
                                        TextView::new(&self.markdown_state).selectable(true),
                                        cx,
                                    )),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::is_near_bottom;
    use gpui::px;

    #[cfg(feature = "visual-test")]
    use super::MarkdownStory;
    #[cfg(feature = "visual-test")]
    use gpui::{
        Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Render,
        StatefulInteractiveElement as _, Styled as _, TestAppContext, VisualTestContext, Window,
        div,
    };
    #[cfg(feature = "visual-test")]
    use std::time::Duration;

    #[test]
    fn follows_when_viewport_is_at_or_near_bottom() {
        assert!(is_near_bottom(px(-100.), px(100.)));
        assert!(is_near_bottom(px(-76.), px(100.)));
    }

    #[test]
    fn pauses_when_viewport_moves_beyond_bottom_threshold() {
        assert!(!is_near_bottom(px(-75.), px(100.)));
    }

    #[test]
    fn follows_before_content_becomes_scrollable() {
        assert!(is_near_bottom(px(0.), px(0.)));
    }

    #[cfg(feature = "visual-test")]
    #[gpui::test]
    fn streaming_content_scrolls_to_the_bottom(cx: &mut TestAppContext) {
        struct GalleryScrollWrapper {
            story: Entity<MarkdownStory>,
            container: Entity<crate::StoryContainer>,
        }

        impl Render for GalleryScrollWrapper {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                hearth_gpui::v_flex().size_full().child(
                    div()
                        .id("gallery-story-scroll")
                        .flex_1()
                        .overflow_y_scroll()
                        .child(self.container.clone()),
                )
            }
        }

        cx.update(hearth_gpui::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let container = crate::StoryContainer::panel::<MarkdownStory>(window, cx);
            let story = container
                .read(cx)
                .story
                .clone()
                .expect("Markdown Story should be registered")
                .downcast::<MarkdownStory>()
                .expect("registered Story should retain its concrete type");
            GalleryScrollWrapper { story, container }
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        let story = cx.update(|_, cx| root.read(cx).story.clone());
        story.update(cx, |story, cx| {
            story.replay(cx);
        });
        let mut saw_overflow = false;
        for chunk_ix in 0..2_000 {
            cx.executor().advance_clock(Duration::from_millis(50));
            cx.run_until_parked();
            cx.update(|window, cx| {
                _ = window.draw(cx);
            });

            let (scroll_handle, replay_finished) = cx.update(|_, cx| {
                let story = story.read(cx);
                (story.scroll_handle.clone(), story._update_task.is_ready())
            });
            let max_offset = scroll_handle.max_offset().y;
            if max_offset > px(0.) {
                saw_overflow = true;
                assert_eq!(
                    -scroll_handle.offset().y,
                    max_offset,
                    "streaming pane stopped following after chunk {chunk_ix}"
                );
            }
            if replay_finished {
                assert!(saw_overflow);
                return;
            }
        }

        panic!("streaming replay did not finish within the chunk budget");
    }
}
