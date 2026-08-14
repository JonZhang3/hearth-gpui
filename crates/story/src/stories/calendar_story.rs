// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `set_date`, `shadow_xs`, `max_w_2xl`, `number_of_months`, `animated`.
// - Reworked Calendar story around semantic Style Preset geometry and density.
use chrono::NaiveDate;
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window, prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme,
    calendar::{Calendar, CalendarState},
    v_flex,
};

use crate::section;

pub struct CalendarStory {
    focus_handle: FocusHandle,
    calendar: Entity<CalendarState>,
    calendar_wide: Entity<CalendarState>,
    calendar_with_disabled_matcher: Entity<CalendarState>,
    range_calendar: Entity<CalendarState>,
    animated_calendar: Entity<CalendarState>,
}

impl super::Story for CalendarStory {
    fn title() -> &'static str {
        "Calendar"
    }

    fn description() -> &'static str {
        "A calendar to select a date or date range."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl CalendarStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let calendar = cx.new(|cx| {
            let mut state = CalendarState::new(window, cx);
            state.set_date(NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(), window, cx);
            state
        });
        let calendar_wide = cx.new(|cx| CalendarState::new(window, cx));
        let calendar_with_disabled_matcher =
            cx.new(|cx| CalendarState::new(window, cx).disabled_matcher(vec![0, 3, 6]));
        let range_calendar = cx.new(|cx| CalendarState::range(window, cx));
        let animated_calendar = cx.new(|cx| CalendarState::new(window, cx));

        Self {
            calendar,
            calendar_wide,
            calendar_with_disabled_matcher,
            range_calendar,
            animated_calendar,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for CalendarStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CalendarStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                section("Normal").max_w_md().child(
                    Calendar::new(&self.calendar)
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded(cx.theme().style.radii.md)
                        .when(cx.theme().style.elevation.enabled, |this| this.shadow_xs()),
                ),
            )
            .child(
                section("With 3 Months")
                    .max_w_md()
                    .child(Calendar::new(&self.calendar_wide).number_of_months(3)),
            )
            .child(
                section("Range Calendar")
                    .max_w_2xl()
                    .child(Calendar::new(&self.range_calendar).number_of_months(2)),
            )
            .child(
                section("Animated")
                    .max_w_md()
                    .child(Calendar::new(&self.animated_calendar).animated(true)),
            )
            .child(
                section("With Disabled matcher (Sundays, Wednesdays, Saturdays)")
                    .max_w_md()
                    .child(Calendar::new(&self.calendar_with_disabled_matcher)),
            )
    }
}
