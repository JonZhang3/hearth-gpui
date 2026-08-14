---
title: Select
description: 通过按钮触发的选项选择组件。
---

# Select

:::info
在 `<= 0.3.x` 中，这个组件的名字是 `Dropdown`。

现在已经改名为 `Select`，以便更准确地表达它的用途。
:::

Select 允许用户从一组选项中选择一个值。

它支持搜索、分组、自定义渲染和多种状态，并内建键盘导航和可访问性支持。

:::tip
如需自定义触发器渲染或多选功能，请参阅 [Combobox](combobox)。
:::

## 导入

```rust
use hearth_gpui::select::{
    Select, SelectState, SelectItem, SelectDelegate,
    SelectEvent, SelectPosition, SearchableVec, SelectGroup
};
```

## 用法

### 基础用法

`SelectState` 的第一个类型参数表示状态中的选项集合，这些选项需要实现 [SelectItem] trait。

框架已经为 `String`、`SharedString` 和 `&'static str` 提供了默认实现。

```rust
let state = cx.new(|cx| {
    SelectState::new(
        vec!["Apple", "Orange", "Banana"],
        Some(IndexPath::default()),
        window,
        cx,
    )
});

Select::new(&state)
```

### Placeholder

```rust
let state = cx.new(|cx| {
    SelectState::new(
        vec!["Rust", "Go", "JavaScript"],
        None,
        window,
        cx,
    )
});

Select::new(&state)
    .placeholder("Select a language...")
```

### 可搜索

启用 `searchable(true)` 后，下拉菜单中会出现搜索能力：

```rust
let fruits = SearchableVec::new(vec![
    "Apple", "Orange", "Banana", "Grape", "Pineapple",
]);

let state = cx.new(|cx| {
    SelectState::new(fruits, None, window, cx).searchable(true)
});

Select::new(&state)
    .icon(IconName::Search)
```

### Content 定位

不可搜索的 Select 默认使用 `SelectPosition::ItemAligned`。菜单宽度为自动值时，Content 使用 Trigger 宽度，但不会小于 shadcn 的 9rem 最小宽度。Content 会覆盖 Trigger：存在已选值时对齐选中项，否则对齐第一个 enabled item。后者只作为临时键盘 cursor，不会提交选中值。弹层会限制在距离窗口边缘 8 px 的范围内。

可搜索 Select 始终使用 `SelectPosition::Popper`，在 Trigger 下方保留 4 px 间距，避免让搜索栏参与选中项对齐。显式设置的 `menu_width` 在两种模式下都继续优先于自动等宽规则。

当默认行为不适合当前布局时，可以显式指定定位模式：

```rust
Select::new(&state)
    .position(SelectPosition::Popper)
```

### 自定义 SelectItem

如果你希望选项携带更复杂的数据结构，或者希望 `selected_value` 返回自定义类型，可以自己实现 `SelectItem`。

同时也可以通过重写 `matches` 定制搜索逻辑。

```rust
#[derive(Debug, Clone)]
struct Country {
    name: SharedString,
    code: SharedString,
}

impl SelectItem for Country {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn display_title(&self) -> Option<gpui::AnyElement> {
        Some(format!("{} ({})", self.name, self.code).into_any_element())
    }

    fn value(&self) -> &Self::Value {
        &self.code
    }

    fn matches(&self, query: &str) -> bool {
        self.name.to_lowercase().contains(&query.to_lowercase()) ||
        self.code.to_lowercase().contains(&query.to_lowercase())
    }
}
```

### 分组

```rust
let mut grouped_items = SearchableVec::new(vec![]);

grouped_items.push(
    SelectGroup::new("A")
        .items(vec![
            Country { name: "Australia".into(), code: "AU".into() },
            Country { name: "Austria".into(), code: "AT".into() },
        ])
);
grouped_items.push(
    SelectGroup::new("B")
        .items(vec![
            Country { name: "Brazil".into(), code: "BR".into() },
            Country { name: "Belgium".into(), code: "BE".into() },
        ])
);

let state = cx.new(|cx| {
    SelectState::new(grouped_items, None, window, cx)
});

Select::new(&state)
    .group_separators(true)
```

### 尺寸

```rust
Select::new(&state).small()
Select::new(&state)
```

`small` 和默认尺寸与 shadcn 的 canonical size 对齐；`xsmall` 和 `large` 是用于桌面密集布局的 Hearth GPUI 扩展。

### 禁用态

```rust
Select::new(&state).disabled(true)
```

单个选项可以通过 `SelectItem::disabled` 返回 `true` 设置为禁用。

### 无效状态

```rust
Select::new(&state)
    .aria_label("联系方式")
    .aria_description("请选择一种联系方式")
    .invalid(true)
```

### 可清空

```rust
Select::new(&state)
    .cleanable(true)
```

### 自定义外观

```rust
Select::new(&state)
    .w(px(320.))
    .menu_width(px(400.))
    .menu_max_h(rems(10.))
    .appearance(false)
    .title_prefix("Country: ")
```

### 空状态

```rust
let state = cx.new(|cx| {
    SelectState::new(Vec::<SharedString>::new(), None, window, cx)
});

Select::new(&state)
    .empty(|_, cx| {
        h_flex()
            .h_24()
            .justify_center()
            .text_color(cx.theme().muted_foreground)
            .child("No options available")
    })
```

### 事件

```rust
cx.subscribe_in(&state, window, |view, state, event, window, cx| {
    match event {
        SelectEvent::Confirm(value) => {
            if let Some(selected_value) = value {
                println!("Selected: {:?}", selected_value);
            } else {
                println!("Selection cleared");
            }
        }
    }
});
```

### 更新选中项和数据

```rust
state.update(cx, |state, cx| {
    state.set_selected_index(Some(IndexPath::default().row(2)), window, cx);
});

state.update(cx, |state, cx| {
    state.set_selected_value(&"US".into(), window, cx);
});

let current_value = state.read(cx).selected_value();
```

更新选项列表：

```rust
state.update(cx, |state, cx| {
    let new_items = vec!["New Option 1".into(), "New Option 2".into()];
    state.set_items(new_items, window, cx);
});
```

## 示例

### 语言选择器

```rust
let languages = SearchableVec::new(vec![
    "Rust".into(),
    "TypeScript".into(),
    "Go".into(),
    "Python".into(),
    "JavaScript".into(),
]);

let state = cx.new(|cx| {
    SelectState::new(languages, None, window, cx)
});

Select::new(&state)
    .placeholder("Select language...")
    .title_prefix("Language: ")
```

### 与 Input 组合

```rust
h_flex()
    .border_1()
    .border_color(cx.theme().input)
    .rounded(cx.theme().style.radii.lg)
    .w_full()
    .gap_1()
    .child(
        div().w(px(140.)).child(
            Select::new(&country_state)
                .appearance(false)
                .py_2()
                .pl_3()
        )
    )
    .child(Separator::vertical())
    .child(
        div().flex_1().child(
            Input::new(&phone_input)
                .appearance(false)
                .placeholder("Phone number")
                .pr_3()
                .py_2()
        )
    )
```

## 可访问性

使用 `aria_label` 为触发器提供可访问名称，使用 `aria_description` 提供补充说明。当前选中项标题会作为 AccessKit value，展开态、无效态和禁用态会自动暴露。

```rust
Select::new(&state)
    .aria_label("国家或地区")
    .aria_description("请选择国家或地区")
```

## 键盘快捷键

| 按键 | 行为 |
| --- | --- |
| `Tab` | 聚焦到 Select |
| `Enter` | 打开菜单或确认当前项 |
| `Up/Down` | 在选项间移动 |
| `Home/End` | 移动到第一个或最后一个可用选项 |
| `Escape` | 关闭菜单 |
| `Space` | 打开菜单 |
| 可打印字符 | 在不可搜索 Select 中选择下一个匹配项 |

## Style Preset

Select 使用语义化的 Control、Focus、Radius、Elevation 和 Motion metrics。Vega 是默认基准；Nova 和 Maia 只改变密度、圆角、间距及弹出层阴影，不改变独立选择的 Color Theme。搜索、清空操作、自定义选项渲染和虚拟化桌面滚动属于 Hearth GPUI 扩展。

## Color Theme

Select 会使用当前主题中的这些 token：

- `background` - 输入区域背景
- `input` - 边框颜色
- `foreground` - 文本颜色
- `muted_foreground` - placeholder 与禁用态文字
- `accent` - 当前项背景
- `accent_foreground` - 当前项文字
- `border` - 菜单边框
- `radius` - 圆角

[SelectItem]: https://docs.rs/hearth-gpui/latest/hearth_gpui/select/trait.SelectItem.html
