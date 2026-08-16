---
title: TextView
description: Renders Markdown and HTML text with optional custom Markdown plugins.
---

# TextView

`TextView` renders formatted text in GPUI. It supports Markdown and simple HTML, text selection, code block actions, and custom Markdown plugins for project-specific syntax.

## Import

```rust
use hearth_gpui::text::{markdown, TextView};
```

## Usage

### Markdown

Use the `markdown` helper when you only need to render Markdown text:

```rust
use hearth_gpui::text::markdown;

markdown("# Hello\n\nThis is **Markdown**.")
    .selectable(true)
    .scrollable(true)
```

You can also construct a `TextView` directly when you need a stable id:

```rust
use hearth_gpui::text::TextView;

TextView::markdown("preview", markdown_source)
    .selectable(true)
```

### HTML

```rust
TextView::html("html-preview", "<strong>Hello</strong>")
```

## Semantic Markdown Styles

`MarkdownStyle` provides per-view styling for Markdown semantics without changing the parser. Inline styles can replace inherited values or explicitly clear them, while element styles accept the complete GPUI `StyleRefinement` API.

```rust
use gpui::{hsla, px, Styled as _};
use hearth_gpui::text::{
    markdown, MarkdownElementKind, MarkdownInlineKind, MarkdownStyle,
    MarkdownTextStyle,
};

let style = MarkdownStyle::default()
    .inline(
        MarkdownInlineKind::Link,
        MarkdownTextStyle::default()
            .color(hsla(0.58, 0.75, 0.55, 1.0))
            .no_underline(),
    )
    .inline(
        MarkdownInlineKind::LinkHover,
        MarkdownTextStyle::default().background(hsla(0.58, 0.4, 0.5, 0.14)),
    )
    .element(
        MarkdownElementKind::CodeBlock,
        gpui::StyleRefinement::default()
            .p_3()
            .rounded(px(8.0))
            .bg(hsla(0.62, 0.12, 0.16, 1.0)),
    );

markdown(source).markdown_style(style)
```

Element selectors cover the document, paragraphs, each heading level, blockquotes, lists and markers, task checkboxes, code blocks and actions, tables and cells, images, and horizontal rules. Inline selectors cover plain text, emphasis variants, inline and block code text, links and link hover, marks, and footnote references. `syntax_theme(...)` overrides code syntax highlighting for only this view.

Style precedence is: active `Theme`, `TextViewStyle`, `MarkdownStyle`, transient interaction state such as link hover, then an optional block renderer.

## Built-in Block Renderers

Use `.markdown_builtin_renderer(...)` when styling is not enough and a built-in block needs custom composition. Wrapping `context.into_default()` retains selection, links, code actions, and other framework-managed behavior.

```rust
use hearth_gpui::text::MarkdownBlockKind;

markdown(source).markdown_builtin_renderer(
    MarkdownBlockKind::CodeBlock,
    |context, _window, _cx| {
        let language = context.code_language().unwrap_or("text").to_string();
        gpui::div()
            .child(gpui::div().child(language))
            .child(context.into_default())
    },
)
```

Replacing the default element completely transfers interaction and accessibility responsibility to the custom renderer. Inline content supports complete semantic text styling, but not arbitrary inline element replacement.

## Streaming Markdown

Keep one `TextViewState`, append each LLM delta with `push_str`, and call `finish_streaming` when the stream completes:

```rust
let state = cx.new(|cx| TextViewState::markdown("", cx));

state.update(cx, |state, cx| state.push_str(delta, cx));
state.update(cx, |state, cx| state.finish_streaming(cx));

TextView::new(&state).selectable(true)
```

Append parsing runs in the background, coalesces queued deltas, preserves the last valid rendered document on a transient parse failure, and performs a canonical full parse after 100 ms of inactivity. `finish_streaming` requests that canonical parse immediately. Reference-style links remain correct when their definitions arrive in later chunks.

## Markdown Plugins

Use `.plugin(...)` to support custom Markdown formats. A plugin owns both parsing and rendering, so callers only need to attach it to the `TextView`:

```rust
markdown(source)
    .plugin(TickerPlugin::new())
```

A Markdown plugin implements `MarkdownPlugin`:

```rust
use gpui::{App, IntoElement, ParentElement as _, Window};
use hearth_gpui::text::{
    markdown_ast, MarkdownNode, MarkdownParseContext, MarkdownPlugin,
};

struct TickerNode {
    symbol: String,
}

struct TickerPlugin;

impl TickerPlugin {
    fn new() -> Self {
        Self
    }
}

impl MarkdownPlugin for TickerPlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "ticker"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let markdown_ast::Node::Paragraph(paragraph) = node else {
            return None;
        };
        let [markdown_ast::Node::Text(text)] = paragraph.children.as_slice() else {
            return None;
        };
        let symbol = text.value.strip_prefix('$')?;

        Some(
            MarkdownNode::new(
                "ticker",
                TickerNode {
                    symbol: symbol.to_string(),
                },
            )
            .text(format!("${symbol}"))
            .markdown(cx.node_source(node).unwrap_or(text.value.as_str())),
        )
    }

    fn render(
        &self,
        node: &MarkdownNode,
        _window: &mut Window,
        _cx: &mut App,
    ) -> impl IntoElement {
        let ticker = node.data::<TickerNode>().expect("ticker node data");

        gpui::div().child(format!("${}", ticker.symbol))
    }
}
```

Then attach it to a Markdown `TextView`:

```rust
markdown("$AAPL.US")
    .plugin(TickerPlugin::new())
```

## MarkdownNode

`MarkdownNode` is the neutral data passed between `parse` and `render`.

```rust
MarkdownNode::new("ticker", TickerNode { symbol })
    .text("$AAPL.US")
    .markdown("$AAPL.US")
```

- `name` is the stable node name used to match the renderer.
- `data` is typed parser output read with `node.data::<T>()`.
- `text` is the plain text representation used by selection and fallback rendering.
- `markdown` is the Markdown representation used when the document is serialized back to Markdown.

## Block Plugins

Custom Markdown rendering currently supports block plugins. Return `true` from `is_block()` for plugins that should be registered today:

```rust
fn is_block(&self) -> bool {
    true
}
```

Inline plugins are reserved for future `TextView` support.

## Code Block Actions

You can render controls for Markdown code blocks:

```rust
markdown(source)
    .code_block_actions(|code_block, _window, _cx| {
        gpui::div().child(format!("Run {}", code_block.lang().unwrap_or_default()))
    })
```
