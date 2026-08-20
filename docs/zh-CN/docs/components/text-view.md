---
title: Markdown 与 Text View
description: 渲染持久化 Markdown 文档和 HTML 文本。
---

# Markdown 与 Text View

Markdown 与 HTML 使用独立渲染路径。`MarkdownElement` 是由 `Entity<Markdown>` 驱动、带源码映射的 GPUI element；`TextView` 继续负责 HTML。

## Markdown

Markdown entity 应创建一次，并由所属 view 持久持有：

```rust
use gpui::{AppContext as _, Entity};
use hearth_gpui::text::Markdown;

let markdown: Entity<Markdown> =
    cx.new(|cx| Markdown::new("# Hello\n\nThis is **Markdown**.", cx));
```

render 时解析主题样式，并与 entity 一起传给 `MarkdownElement`：

```rust
use hearth_gpui::text::{MarkdownElement, MarkdownFont, MarkdownStyle};

let style = MarkdownStyle::themed(MarkdownFont::Preview, window, cx);
MarkdownElement::new(markdown.clone(), style)
```

`MarkdownFont::Agent`、`Editor`、`Preview` 分别提供 agent、editor、preview 三种场景的排版 profile。`MarkdownStyle` 可配置标题、链接、inline code、引用、code block、分隔线、表格、语法高亮、选择区域和软换行。

### Inline code

内置样式将 inline code 渲染为圆角胶囊：

- 使用 `Theme.mono_font_family` 等宽字体，字号为正文的 87.5%；
- 背景由前景色推导，保证暗色主题下的可读性；
- 水平 padding 4px，圆角取自当前 Style Preset 的 `radii.sm`；
- 在 `MarkdownFont::Preview` 中保持完整前景色，因为此时正文是弱化色。

`MarkdownStyle.inline_code_box` 持有胶囊指标，默认全零，因此自定义样式仍可使用普通文本背景：

```rust
use hearth_gpui::text::{InlineCodeBoxStyle, MarkdownStyle};

let mut style = MarkdownStyle::themed(MarkdownFont::Editor, window, cx);
style.inline_code_box = InlineCodeBoxStyle {
    padding_x: px(4.),
    corner_radius: px(6.),
};
```

Code block 不受影响：它们保持 `Theme.mono_font_size`，inline code 则随正文字号缩放。

### 外层容器拥有滚动

`MarkdownElement` 不创建纵向滚动区域。overflow、scrollbar 和 follow-tail 策略全部属于外层容器：

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

传入 handle 只用于标题、脚注、源码位置和选择边缘的导航，不会把滚动所有权交给 Markdown。

### Streaming

Provider delta 直接更新 canonical source。解析在 UI 线程外执行；同一时间只允许一个解析任务，任务运行期间到达的 delta 会合并处理，过期结果不会发布：

```rust
markdown.update(cx, |markdown, cx| markdown.append(provider_delta, cx));

// 开始新的回复。
markdown.update(cx, |markdown, cx| markdown.replace("", cx));
```

调用方无需增加第二层 pacing。若外层容器仍处于 follow-tail 状态，可在 append 后调用 `scroll_handle.scroll_to_bottom()`。

### Options 与交互

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

Element callback 支持 URL click/hover、inline-code link、source click、task checkbox toggle，以及由调用方控制的图片解析。默认 code renderer 支持复制、换行切换、边框、完整 fenced info、source path、语法高亮，以及可选 Mermaid SVG 渲染。

搜索与导航统一使用 canonical source 的 UTF-8 byte range：

```rust
markdown.update(cx, |markdown, cx| {
    markdown.search("query", false, cx);
    markdown.scroll_to_heading("streaming", cx);
});
```

普通 Copy 输出渲染后的纯文本；`CopyAsMarkdown` 输出再平衡后的合法 Markdown：选择边界会从定界符语法中收敛，被选择截断的定界符会重新补回，例如在 `**bold**` 中选择 `old` 会复制 `**old**`。完全落在 inline code 内的选择复制纯文本。

鼠标选择：词选择基于渲染文本计算（双击 inline code 只选中内容、不含反引号），shift-click 从选择尾部扩展，词/行拖拽越过锚点后反向扩展。点击链接不会创建选择，激活在释放时判定。

## HTML

HTML 继续通过 `TextView` 使用：

```rust
use hearth_gpui::text::{TextView, html};

html("<p>Hello <strong>HTML</strong>.</p>")

TextView::html("stable-id", "<p>Persistent HTML</p>")
    .selectable(true)
```

旧 `markdown(source)`、`TextView::markdown`、`MarkdownState`、streaming buffer/configuration 和 Markdown plugin Interface 已移除。Markdown 调用方必须迁移到持久化 `Entity<Markdown>`。
