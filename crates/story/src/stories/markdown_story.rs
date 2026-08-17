use gpui::*;
use hearth_gpui::{
    ActiveTheme as _, ElementExt as _,
    button::Button,
    h_flex,
    text::{
        MarkdownBlockKind, MarkdownElementKind, MarkdownInlineKind, MarkdownStyle,
        MarkdownTextStyle, StreamingTextPacer, TextView, TextViewState,
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
    following: bool,
    pending_follow: bool,
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
            this.mark_stream_content_changed(cx);
        })];

        Self {
            markdown_state,
            scroll_handle: ScrollHandle::new(),
            replay_id: 0,
            following: true,
            pending_follow: true,
            _subscriptions,
            _update_task: Task::ready(()),
        }
    }

    /// Mark a follow request without reading the previous frame's content height.
    fn mark_stream_content_changed(&mut self, cx: &mut Context<Self>) {
        self.pending_follow |= self.following;
        cx.notify();
    }

    /// Apply a pending follow after the scroll container has measured its new content.
    fn apply_pending_follow(&mut self, cx: &mut Context<Self>) {
        if !self.following || !self.pending_follow {
            return;
        }
        let max_offset = self.scroll_handle.max_offset();
        let offset = self.scroll_handle.offset();
        self.scroll_handle
            .set_offset(point(offset.x, -max_offset.y));
        self.pending_follow = false;
        cx.notify();
    }

    /// Handle scroll intent before GPUI applies the wheel delta to the scroll handle.
    fn on_stream_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let delta_y = event.delta.pixel_delta(window.line_height()).y;
        if delta_y > px(0.) {
            // Any upward intent must win over a pending stream-follow request.
            self.following = false;
            self.pending_follow = false;
            cx.notify();
        } else if delta_y < px(0.) {
            cx.on_next_frame(window, |this, _, cx| {
                this.sync_follow_mode(cx);
            });
        }
    }

    /// Re-evaluate whether downward scrolling has returned the viewport near the bottom.
    fn sync_follow_mode(&mut self, cx: &mut Context<Self>) {
        self.following = is_near_bottom(
            self.scroll_handle.offset().y,
            self.scroll_handle.max_offset().y,
        );
        if self.following {
            self.pending_follow = true;
        }
        cx.notify();
    }

    /// Restores bottom-follow mode when the user explicitly starts a new replay.
    fn reset_replay_scroll(&mut self) {
        self.following = true;
        self.pending_follow = true;
        let offset = self.scroll_handle.offset();
        let max_offset = self.scroll_handle.max_offset();
        self.scroll_handle
            .set_offset(point(offset.x, -max_offset.y));
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
                    .background(cx.theme().muted)
                    .padding_x(px(4.))
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
                                                "Replay paces buffered text chunks, repairs incomplete inline tails, and settles canonically.",
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
                                    .on_scroll_wheel(cx.listener(|this, event, window, cx| {
                                        this.on_stream_scroll_wheel(event, window, cx);
                                    }))
                                    .on_prepaint({
                                        let story = cx.entity();
                                        move |_, _, cx| {
                                            story.update(cx, |this, cx| {
                                                this.apply_pending_follow(cx);
                                            });
                                        }
                                    })
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
        ScrollDelta, ScrollWheelEvent, StatefulInteractiveElement as _, Styled as _,
        TestAppContext, VisualTestContext, Window, div, point,
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
        let mut checked_pause_and_resume = false;
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

                if !checked_pause_and_resume && max_offset > px(50.) {
                    cx.simulate_event(ScrollWheelEvent {
                        position: scroll_handle.bounds().center(),
                        delta: ScrollDelta::Pixels(point(px(0.), px(8.))),
                        ..Default::default()
                    });
                    cx.run_until_parked();
                    cx.update(|window, cx| {
                        _ = window.draw(cx);
                    });
                    assert!(!cx.update(|_, cx| story.read(cx).following));
                    assert_eq!(
                        scroll_handle.max_offset().y + scroll_handle.offset().y,
                        px(8.),
                        "a small upward wheel gesture should move away from the bottom"
                    );

                    cx.executor().advance_clock(Duration::from_millis(50));
                    cx.run_until_parked();
                    cx.update(|window, cx| {
                        _ = window.draw(cx);
                    });
                    let paused_max = scroll_handle.max_offset().y;
                    assert_ne!(
                        -scroll_handle.offset().y,
                        paused_max,
                        "user scrolling upward should pause automatic following"
                    );

                    scroll_handle.set_offset(point(px(0.), -paused_max));
                    story.update(cx, |story, cx| story.sync_follow_mode(cx));
                    cx.update(|window, cx| {
                        _ = window.draw(cx);
                    });
                    assert!(cx.update(|_, cx| story.read(cx).following));
                    assert_eq!(-scroll_handle.offset().y, scroll_handle.max_offset().y);
                    checked_pause_and_resume = true;
                }
            }
            if replay_finished {
                assert!(saw_overflow);
                assert!(checked_pause_and_resume);

                // Completed output must release bottom-follow on the first small upward gesture too.
                cx.simulate_event(ScrollWheelEvent {
                    position: scroll_handle.bounds().center(),
                    delta: ScrollDelta::Pixels(point(px(0.), px(8.))),
                    ..Default::default()
                });
                cx.run_until_parked();
                cx.update(|window, cx| {
                    _ = window.draw(cx);
                });
                assert!(!cx.update(|_, cx| story.read(cx).following));
                assert_eq!(
                    scroll_handle.max_offset().y + scroll_handle.offset().y,
                    px(8.),
                    "completed output should not snap a small upward gesture back to the bottom"
                );
                return;
            }
        }

        panic!("streaming replay did not finish within the chunk budget");
    }
}
