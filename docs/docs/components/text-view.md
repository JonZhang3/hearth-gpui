---
title: Markdown and Text View
description: Renders persistent Markdown documents and HTML text.
---

# Markdown and Text View

Markdown and HTML use separate rendering paths. `MarkdownElement` is a source-mapped GPUI element backed by `Entity<Markdown>`. `TextView` remains the HTML renderer.

## Markdown

Create the entity once and retain it with the owning view:

```rust
use gpui::{AppContext as _, Entity};
use hearth_gpui::text::Markdown;

let markdown: Entity<Markdown> =
    cx.new(|cx| Markdown::new("# Hello\n\nThis is **Markdown**.", cx));
```

Resolve the style during render and pass both values to `MarkdownElement`:

```rust
use hearth_gpui::text::{MarkdownElement, MarkdownFont, MarkdownStyle};

let style = MarkdownStyle::themed(MarkdownFont::Preview, window, cx);
MarkdownElement::new(markdown.clone(), style)
```

`MarkdownFont::Agent`, `Editor`, and `Preview` select Zed-compatible typography profiles. `MarkdownStyle` exposes semantic refinements for headings, links, inline code, block quotes, code blocks, rules, tables, syntax highlighting, selection, and soft breaks.

### Host-owned scrolling

`MarkdownElement` does not create a vertical scroll area. The host owns overflow, scrollbar, and follow-tail policy:

```rust
use gpui::{div, ParentElement as _, ScrollHandle, StatefulInteractiveElement as _, Styled as _};
use hearth_gpui::scroll::ScrollableElement as _;

let scroll_handle = ScrollHandle::new();

div()
    .relative()
    .size_full()
    .child(
        div()
            .id("markdown-scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&scroll_handle)
            .child(
                MarkdownElement::new(markdown.clone(), style)
                    .scroll_handle(scroll_handle.clone()),
            ),
    )
    .vertical_scrollbar(&scroll_handle)
```

Passing the handle enables heading, footnote, source-index, and selection-edge navigation. It does not transfer scroll ownership to Markdown.

### Streaming

Provider deltas update the canonical source directly. Parsing runs off the UI thread, permits only one parse task at a time, coalesces arrivals while that task is running, and rejects stale results:

```rust
markdown.update(cx, |markdown, cx| markdown.append(provider_delta, cx));

// Start a new response.
markdown.update(cx, |markdown, cx| markdown.replace("", cx));
```

No additional pacing layer is required. If the host follows the tail, call `scroll_handle.scroll_to_bottom()` after an append while its follow policy remains active.

### Options and interaction

```rust
use hearth_gpui::text::MarkdownOptions;

let options = MarkdownOptions {
    parse_html: true,
    render_mermaid_diagrams: true,
    parse_heading_slugs: true,
    render_metadata_blocks: true,
    ..Default::default()
};

let markdown = cx.new(|cx| Markdown::new_with_options(source, options, cx));
```

Optional element callbacks support URL click/hover, inline-code links, source clicks, task checkbox toggles, and caller-owned image resolution. The default code renderer supports copy, wrap, border, fenced info strings, source paths, syntax highlighting, and optional Mermaid SVG rendering.

Search and navigation use canonical UTF-8 byte ranges:

```rust
markdown.update(cx, |markdown, cx| {
    markdown.search("query", false, cx);
    markdown.scroll_to_heading("streaming", cx);
});
```

Normal copy writes rendered plain text. `CopyAsMarkdown` writes the selected canonical source.

## HTML

HTML remains available through `TextView`:

```rust
use hearth_gpui::text::{TextView, html};

html("<p>Hello <strong>HTML</strong>.</p>")

TextView::html("stable-id", "<p>Persistent HTML</p>")
    .selectable(true)
```

The legacy `markdown(source)`, `TextView::markdown`, `MarkdownState`, streaming buffer/configuration, and Markdown plugin Interfaces were removed. Migrate Markdown owners to a persistent `Entity<Markdown>`.
