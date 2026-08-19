use gpui::{
    App, AppContext as _, BenchAppContext, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window,
    div, point, px,
};
use hearth_gpui::text::{Markdown, MarkdownElement, MarkdownFont, MarkdownStyle};

/// Representative Markdown workload rendered inside a host-owned scroll area.
struct MarkdownScrollBench {
    markdown: Entity<Markdown>,
    scroll_handle: ScrollHandle,
    selectable: bool,
    stream: bool,
    appended_chunks: usize,
    source: String,
}

impl MarkdownScrollBench {
    /// Creates a stable Markdown document with the requested approximate size.
    fn new(target_bytes: usize, selectable: bool, stream: bool, cx: &mut App) -> Self {
        let source = markdown_fixture(target_bytes);
        Self {
            markdown: cx.new(|cx| Markdown::new(source.clone(), cx)),
            scroll_handle: ScrollHandle::new(),
            selectable,
            stream,
            appended_chunks: 0,
            source,
        }
    }

    /// Moves between the top and bottom and optionally publishes one provider delta.
    fn advance_frame(&mut self, cx: &mut Context<Self>) {
        let at_top = self.scroll_handle.offset().y == px(0.);
        let target = if at_top {
            -self.scroll_handle.max_offset().y
        } else {
            px(0.)
        };
        self.scroll_handle.set_offset(point(px(0.), target));

        if self.stream {
            if self.appended_chunks == 32 {
                self.markdown
                    .update(cx, |markdown, cx| markdown.replace(&self.source, cx));
                self.appended_chunks = 0;
            } else {
                self.markdown.update(cx, |markdown, cx| {
                    markdown.append("\nA streamed **delta** with `inline code`.", cx)
                });
                self.appended_chunks += 1;
            }
        }
        cx.notify();
    }
}

impl Render for MarkdownScrollBench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut style = MarkdownStyle::themed(MarkdownFont::Agent, window, cx);
        style.prevent_mouse_interaction = !self.selectable;
        div()
            .id("markdown-scroll-bench")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .child(
                MarkdownElement::new(self.markdown.clone(), style)
                    .scroll_handle(self.scroll_handle.clone()),
            )
    }
}

/// Builds mixed Markdown so every benchmark covers representative layout work.
fn markdown_fixture(target_bytes: usize) -> String {
    const SECTION: &str = r#"
## Renderer section

A paragraph with **strong**, *emphasis*, `inline code`, and a [link](https://example.com).

> A block quote used to exercise nested block layout and semantic colors.

- one list item
- another list item with a longer line that wraps at normal Story widths

| Name | Value | Status |
| --- | ---: | :---: |
| Alpha | 42 | Ready |
| Beta | 108 | Running |

```rust
fn render(value: usize) -> usize {
    value + 1
}
```
"#;

    let mut source = String::with_capacity(target_bytes + SECTION.len());
    source.push_str("# Markdown scroll benchmark\n");
    while source.len() < target_bytes {
        source.push_str(SECTION);
    }
    source
}

/// Installs Hearth globals and mounts one benchmark view in a headless window.
fn mount_markdown(
    cx: &mut BenchAppContext,
    bytes: usize,
    selectable: bool,
    stream: bool,
) -> Entity<MarkdownScrollBench> {
    cx.update(hearth_gpui::init);
    let mut window = cx.add_empty_window();
    window.update(|window, cx| {
        window.replace_root(cx, |_, cx| {
            MarkdownScrollBench::new(bytes, selectable, stream, cx)
        })
    })
}

/// Runs a renderer benchmark after the initial parse and layout caches settle.
fn bench_scroll(cx: &mut BenchAppContext, bytes: usize, selectable: bool, stream: bool) {
    let view = mount_markdown(cx, bytes, selectable, stream);
    cx.run_until_idle();
    cx.bench_renderer(view, |view, _, cx| view.advance_frame(cx));
}

#[gpui::bench]
fn markdown_scroll_8k(cx: &mut BenchAppContext) {
    bench_scroll(cx, 8 * 1024, false, false);
}

#[gpui::bench]
fn markdown_scroll_64k(cx: &mut BenchAppContext) {
    bench_scroll(cx, 64 * 1024, false, false);
}

#[gpui::bench]
fn markdown_scroll_64k_selectable(cx: &mut BenchAppContext) {
    bench_scroll(cx, 64 * 1024, true, false);
}

#[gpui::bench]
fn markdown_scroll_64k_streaming(cx: &mut BenchAppContext) {
    bench_scroll(cx, 64 * 1024, true, true);
}

gpui::bench_group!(
    benches,
    markdown_scroll_8k,
    markdown_scroll_64k,
    markdown_scroll_64k_selectable,
    markdown_scroll_64k_streaming,
);
gpui::bench_main!(benches);
