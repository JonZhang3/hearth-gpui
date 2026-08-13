---
title: Command
description: 支持分组操作、搜索和键盘导航的命令菜单。
---

# Command

`Command` 是 GPUI 原生命令面板。它复用虚拟化 List 的键盘与可访问性契约，并将筛选、活动项、勾选状态和执行操作相互分离。

## 导入

```rust
use gpui_component::command::{
    Command, CommandDialog, CommandEmpty, CommandGroup, CommandInput,
    CommandItem, CommandList, CommandSeparator, CommandShortcut, CommandState,
};
```

## 创建状态

```rust
let commands = CommandList::new()
    .empty(CommandEmpty::new("没有匹配的命令。"))
    .group(
        CommandGroup::new("suggestions")
            .heading("建议")
            .item(
                CommandItem::new("calendar", "日历")
                    .icon(IconName::Calendar)
                    .keyword("日期"),
            ),
    )
    .separator(CommandSeparator::new())
    .group(
        CommandGroup::new("settings")
            .heading("设置")
            .item(
                CommandItem::new("profile", "个人资料")
                    .shortcut(CommandShortcut::new("⌘P")),
            ),
    );

let command = cx.new(|cx| CommandState::new(commands, window, cx));
```

## 内联 Command

```rust
Command::new(&command)
    .input(CommandInput::new().placeholder("输入命令或搜索……"))
    .w(px(420.))
```

`CommandList` 会根据筛选后可见的分组与条目自然增长，并以与 shadcn 对齐的
`288px` 为最大高度。因此搜索结果减少时，Command 表面高度也会同步收缩。仅在
确实需要固定高度时，才调用 `.h(...)`。

## CommandDialog

```rust
CommandDialog::new(&command)
    .trigger(Button::new("open-command").label("打开命令面板"))
```

程序化显示使用 `CommandDialog::open`。语义标题与说明不会参与视觉布局，默认不显示关闭按钮，模态生命周期动效复用现有 Dialog。

## 交互与筛选

- 对 label 与 keywords 执行不区分大小写的子串匹配，保持原始分组和项目顺序，不引入模糊排序依赖。
- Up/Down、Home/End、Enter 和 Escape 沿用 GPUI List 键盘契约，禁用项会被跳过。
- `checked` 与当前键盘活动项相互独立；存在快捷键时不显示尾部勾选图标，与 shadcn 组合一致。
- 固定版本的 shadcn Command 没有声明进入、退出或颜色 transition，因此 Command 本身不增加动效。
- Vega 是默认视觉基线；Nova 与 Maia 通过语义 Style Preset 解析密度、间距和圆角，不按 preset ID 分支。

## 事件

订阅 `CommandEvent::Select(id)` 可接收确认执行的命令，`CommandEvent::Cancel` 对应 Escape。也可以为单个项目设置 `on_select`。
