---
title: TextView
description: 渲染 Markdown 与 HTML 文本，并支持自定义 Markdown 插件。
---

# TextView

`TextView` 用于在 GPUI 中渲染格式化文本。它支持 Markdown、简单 HTML、文本选择、代码块操作，以及通过 Markdown 插件解析和渲染项目自定义语法。

## 导入

```rust
use hearth_gpui::text::{markdown, TextView};
```

## 用法

### Markdown

只需要渲染 Markdown 时，可以使用 `markdown` helper：

```rust
use hearth_gpui::text::markdown;

markdown("# Hello\n\nThis is **Markdown**.")
    .selectable(true)
    .scrollable(true)
```

如果需要稳定 id，也可以直接构造 `TextView`：

```rust
use hearth_gpui::text::TextView;

TextView::markdown("preview", markdown_source)
    .selectable(true)
```

### HTML

```rust
TextView::html("html-preview", "<strong>Hello</strong>")
```

## Markdown 语义样式

`MarkdownStyle` 可以按 Markdown 语义为单个视图设置样式，不改变 parser。Inline 样式既能覆盖继承值，也能显式清除继承值；element 样式支持完整的 GPUI `StyleRefinement` API。

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

Element selector 覆盖 document、paragraph、各级 heading、blockquote、list 与 marker、task checkbox、code block 与 actions、table 与 cell、image 和 horizontal rule。Inline selector 覆盖 plain text、各类 emphasis、inline 与 block code text、link 与 link hover、mark 和 footnote reference。`syntax_theme(...)` 可以只覆盖当前视图的代码语法高亮。

样式优先级为：当前 `Theme`、`TextViewStyle`、`MarkdownStyle`、link hover 等临时交互状态，最后是可选的 block renderer。

## 内置 Block Renderer

样式不足以表达自定义组合时，使用 `.markdown_builtin_renderer(...)` 覆盖内置 block。包装 `context.into_default()` 可以保留 selection、link、code actions 等框架管理的行为。

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

如果完全替换默认 element，自定义 renderer 需要自行负责交互与无障碍行为。Inline 内容支持完整的语义文本样式，但不支持任意 inline element 替换。

## 流式 Markdown

复用同一个 `TextViewState`，通过 `push_str` 追加每个 LLM delta，并在流结束时调用 `finish_streaming`：

```rust
let state = cx.new(|cx| TextViewState::markdown("", cx));

state.update(cx, |state, cx| state.push_str(delta, cx));
state.update(cx, |state, cx| state.finish_streaming(cx));

TextView::new(&state).selectable(true)
```

追加解析在后台执行，会合并排队的 delta；临时解析失败时保留最后一份有效文档；空闲 100 ms 后执行 canonical full parse。`finish_streaming` 会立即请求该 canonical parse。引用式链接的 definition 即使在后续 chunk 才到达，也能得到正确结果。

## Markdown 插件

使用 `.plugin(...)` 支持自定义 Markdown 格式。插件同时拥有解析和渲染逻辑，调用方只需要把它挂到 `TextView` 上：

```rust
markdown(source)
    .plugin(TickerPlugin::new())
```

Markdown 插件实现 `MarkdownPlugin`：

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

然后挂到 Markdown `TextView`：

```rust
markdown("$AAPL.US")
    .plugin(TickerPlugin::new())
```

## MarkdownNode

`MarkdownNode` 是 `parse` 和 `render` 之间传递的中性数据结构。

```rust
MarkdownNode::new("ticker", TickerNode { symbol })
    .text("$AAPL.US")
    .markdown("$AAPL.US")
```

- `name` 是稳定的节点名称，用于匹配 renderer。
- `data` 是 parser 产生的类型化数据，通过 `node.data::<T>()` 读取。
- `text` 是纯文本表示，用于选择和未注册 renderer 时的回退渲染。
- `markdown` 是 Markdown 表示，用于将文档重新序列化为 Markdown。

## Block 插件

当前自定义 Markdown 渲染支持 block 插件。现在可注册的插件需要在 `is_block()` 中返回 `true`：

```rust
fn is_block(&self) -> bool {
    true
}
```

Inline 插件保留给未来的 `TextView` 支持。

## 代码块操作

可以为 Markdown 代码块渲染操作控件：

```rust
markdown(source)
    .code_block_actions(|code_block, _window, _cx| {
        gpui::div().child(format!("Run {}", code_block.lang().unwrap_or_default()))
    })
```
