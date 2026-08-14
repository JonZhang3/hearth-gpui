---
title: Calendar
description: 用于展示月份、浏览日期和选择单日或区间的灵活日历组件。
---

# Calendar

Calendar 是一个与 shadcn 对齐的独立日历组件，支持单日与日期区间选择、多月视图、禁用日期规则、可配置的周起始日、可选月份动画和完整的键盘导航能力。

- [CalendarState] 负责状态与选择管理
- [Calendar] 负责渲染日历界面

## 导入

```rust
use hearth_gpui::{
    calendar::{Calendar, CalendarState, CalendarEvent, Date, Matcher},
};
```

## 用法

### 基础日历

```rust
let state = cx.new(|cx| CalendarState::new(window, cx));
Calendar::new(&state)
```

### 带边框的日历

shadcn Calendar 本身不包含边框。Calendar 作为独立卡片使用时，通过 `Styled` 添加外层表面：

```rust
Calendar::new(&state)
    .border_1()
    .border_color(cx.theme().border)
    .rounded(cx.theme().style.radii.md)
    .shadow_xs()
```

Calendar 使用主题的标准文字等级：日期和标题使用 `text-sm`，星期标题使用 `text-xs`。

### 初始日期

```rust
use chrono::Local;

let state = cx.new(|cx| {
    let mut state = CalendarState::range(window, cx);
    state.set_date(Local::now().naive_local().date(), window, cx);
    state
});

Calendar::new(&state)
    .number_of_months(2)
```

第一次选择后区间保持待完成状态，第二次选择完成有序区间；即使第二次选择的日期早于第一次，也会自动调整起止顺序。

### 日期区间

```rust
use chrono::{Local, Days};

let state = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    let now = Local::now().naive_local().date();
    state.set_date(
        Date::Range(Some(now), now.checked_add_days(Days::new(7))),
        window,
        cx
    );
    state
});

Calendar::new(&state)
```

### 多月显示

```rust
Calendar::new(&state)
    .number_of_months(2)

Calendar::new(&state)
    .number_of_months(3)
```

### 尺寸

```rust
Calendar::new(&state).large()
Calendar::new(&state)
Calendar::new(&state).small()
```

默认日期单元尺寸由当前 Style Preset 决定：Vega 和 Maia 使用 32px，Nova 使用 28px。

### 周起始日与跨月日期

```rust
use chrono::Weekday;

Calendar::new(&state)
    .week_starts_on(Weekday::Mon)
    .show_outside_days(false)
```

默认从星期日开始，并始终渲染六行日期以保持布局稳定。

### 月份切换动画

```rust
Calendar::new(&state).animated(true)
```

动画默认关闭。启用后使用当前 Style Preset 的 motion token，并在 reduced motion 开启时退化为静态切换。

## 日期限制

### 禁用周末

```rust
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(vec![0, 6])
});
```

### 禁用日期区间

```rust
use chrono::{Local, Days};

let now = Local::now().naive_local().date();

let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::range(
            Some(now),
            now.checked_add_days(Days::new(7)),
        ))
});
```

### 自定义禁用规则

```rust
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            date.weekday() == chrono::Weekday::Mon
        }))
});
```

## 月份与年份导航

Calendar 自带这些导航能力：

- 上一月 / 下一月按钮
- 点击月份切换月视图
- 点击年份切换年视图
- 在年视图中按页浏览年份

月份和年份选择网格是保留的 GPUI 桌面交互。方向键移动当前日期或网格选项，Home/End 移动到边界，Page Up/Page Down 切换月份或分页，Enter/Space 确认，Escape 返回日期视图。

### 自定义年份范围

```rust
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .year_range((2020, 2030))
});
```

## 监听选择事件

```rust
let state = cx.new(|cx| CalendarState::new(window, cx));

cx.subscribe(&state, |view, _, event, _| {
    match event {
        CalendarEvent::Selected(date) => {
            match date {
                Date::Single(Some(selected_date)) => {
                    println!("Date selected: {}", selected_date);
                }
                Date::Range(Some(start), Some(end)) => {
                    println!("Range selected: {} to {}", start, end);
                }
                _ => {}
            }
        }
    }
});
```

## 示例

### 仅工作日

```rust
use chrono::Weekday;

let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
        }))
});
```

### 假期禁用

```rust
use chrono::NaiveDate;
use std::collections::HashSet;

let holidays: HashSet<NaiveDate> = [
    NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    NaiveDate::from_ymd_opt(2024, 7, 4).unwrap(),
    NaiveDate::from_ymd_opt(2024, 12, 25).unwrap(),
].into_iter().collect();
```

### 多月区间选择

```rust
let state = cx.new(|cx| {
    CalendarState::range(window, cx)
});

Calendar::new(&state)
    .number_of_months(3)
```

[Calendar]: https://docs.rs/hearth-gpui/latest/hearth_gpui/calendar/struct.Calendar.html
[CalendarState]: https://docs.rs/hearth-gpui/latest/hearth_gpui/calendar/struct.CalendarState.html
[RangeMatcher]: https://docs.rs/hearth-gpui/latest/hearth_gpui/calendar/struct.RangeMatcher.html
