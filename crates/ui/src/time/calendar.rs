// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added range selection, configurable week starts, outside-day visibility, animation, and ARIA
//   labeling APIs.
// - Reworked month and year paging, active-date focus, and keyboard navigation around a
//   visible-month state.
// - Added semantic calendar metrics, reduced-motion transitions, and accessible grid metadata.
// - Removed legacy single-mode and direct previous/next month rendering helpers.
use std::{borrow::Cow, rc::Rc};

use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use gpui::{
    AnyElement, App, ClickEvent, Context, Div, ElementId, Empty, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement, Pixels,
    Render, RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _, px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme, Disableable as _, Icon, IconName, Sizable, Size, StyledExt as _,
    actions::{
        Cancel, Confirm, SelectDown, SelectFirst, SelectLast, SelectLeft, SelectPageDown,
        SelectPageUp, SelectRight, SelectUp,
    },
    animation::{Transition, effective_motion_duration},
    button::Button,
    h_flex,
    theme::{Density, StylePreset},
    v_flex,
};

use super::utils::days_in_month;

const CONTEXT: &str = "Calendar";
const MAX_DISABLED_DATE_SCAN_DAYS: usize = 366;

/// Registers native keyboard navigation for Calendar composite widgets.
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
        KeyBinding::new("pageup", SelectPageUp, Some(CONTEXT)),
        KeyBinding::new("pagedown", SelectPageDown, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("space", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ]);
}

/// Events emitted by the calendar.
pub enum CalendarEvent {
    /// The user selected a date.
    Selected(Date),
}

/// The date of the calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Date {
    Single(Option<NaiveDate>),
    Range(Option<NaiveDate>, Option<NaiveDate>),
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(Some(date)) => write!(f, "{}", date),
            Self::Single(None) => write!(f, "nil"),
            Self::Range(Some(start), Some(end)) => write!(f, "{} - {}", start, end),
            Self::Range(None, None) => write!(f, "nil"),
            Self::Range(Some(start), None) => write!(f, "{} - nil", start),
            Self::Range(None, Some(end)) => write!(f, "nil - {}", end),
        }
    }
}

impl From<NaiveDate> for Date {
    fn from(date: NaiveDate) -> Self {
        Self::Single(Some(date))
    }
}

impl From<(NaiveDate, NaiveDate)> for Date {
    fn from((start, end): (NaiveDate, NaiveDate)) -> Self {
        Self::Range(Some(start), Some(end))
    }
}

impl Date {
    /// Check if the date is set.
    pub fn is_some(&self) -> bool {
        match self {
            Self::Single(Some(_)) | Self::Range(Some(_), _) => true,
            _ => false,
        }
    }

    /// Check if the date is complete.
    pub fn is_complete(&self) -> bool {
        match self {
            Self::Range(Some(_), Some(_)) => true,
            Self::Single(Some(_)) => true,
            _ => false,
        }
    }

    /// Get the start date.
    pub fn start(&self) -> Option<NaiveDate> {
        match self {
            Self::Single(Some(date)) => Some(*date),
            Self::Range(Some(start), _) => Some(*start),
            _ => None,
        }
    }

    /// Get the end date.
    pub fn end(&self) -> Option<NaiveDate> {
        match self {
            Self::Range(_, Some(end)) => Some(*end),
            _ => None,
        }
    }

    /// Return formatted date string.
    pub fn format(&self, format: &str) -> Option<SharedString> {
        match self {
            Self::Single(Some(date)) => Some(date.format(format).to_string().into()),
            Self::Range(Some(start), Some(end)) => {
                Some(format!("{} - {}", start.format(format), end.format(format)).into())
            }
            _ => None,
        }
    }

    fn is_active(&self, v: &NaiveDate) -> bool {
        let v = *v;
        match self {
            Self::Single(d) => Some(v) == *d,
            Self::Range(start, end) => Some(v) == *start || Some(v) == *end,
        }
    }

    fn is_in_range(&self, v: &NaiveDate) -> bool {
        let v = *v;
        match self {
            Self::Range(start, end) => {
                if let Some(start) = start {
                    if let Some(end) = end {
                        v >= *start && v <= *end
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Advance a range selection while keeping completed endpoints ordered.
    fn select_range_date(self, date: NaiveDate) -> Self {
        match self {
            Self::Range(Some(start), None) if date < start => Self::Range(Some(date), Some(start)),
            Self::Range(Some(start), None) => Self::Range(Some(start), Some(date)),
            Self::Range(_, _) => Self::Range(Some(date), None),
            Self::Single(_) => self,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Day,
    Month,
    Year,
}

impl ViewMode {
    fn is_day(&self) -> bool {
        matches!(self, Self::Day)
    }

    fn is_month(&self) -> bool {
        matches!(self, Self::Month)
    }

    fn is_year(&self) -> bool {
        matches!(self, Self::Year)
    }
}

/// Matcher to match dates before and after the interval.
pub struct IntervalMatcher {
    before: Option<NaiveDate>,
    after: Option<NaiveDate>,
}

/// Matcher to match dates within the range.
pub struct RangeMatcher {
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

/// Matcher to match dates.
pub enum Matcher {
    /// Match declare days of the week.
    ///
    /// Matcher::DayOfWeek(vec![0, 6])
    /// Will match the days of the week that are Sunday and Saturday.
    DayOfWeek(Vec<u32>),
    /// Match the included days, except for those before and after the interval.
    ///
    /// Matcher::Interval(IntervalMatcher {
    ///   before: Some(NaiveDate::from_ymd(2020, 1, 2)),
    ///   after: Some(NaiveDate::from_ymd(2020, 1, 3)),
    /// })
    /// Will match the days that are not between 2020-01-02 and 2020-01-03.
    Interval(IntervalMatcher),
    /// Match the days within the range.
    ///
    /// Matcher::Range(RangeMatcher {
    ///   from: Some(NaiveDate::from_ymd(2020, 1, 1)),
    ///   to: Some(NaiveDate::from_ymd(2020, 1, 3)),
    /// })
    /// Will match the days that are between 2020-01-01 and 2020-01-03.
    Range(RangeMatcher),
    /// Match dates using a custom function.
    ///
    /// let matcher = Matcher::Custom(Box::new(|date: &NaiveDate| {
    ///     date.day0() < 5
    /// }));
    /// Will match first 5 days of each month
    Custom(Box<dyn Fn(&NaiveDate) -> bool + Send + Sync>),
}

impl From<Vec<u32>> for Matcher {
    fn from(days: Vec<u32>) -> Self {
        Matcher::DayOfWeek(days)
    }
}

impl<F> From<F> for Matcher
where
    F: Fn(&NaiveDate) -> bool + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Matcher::Custom(Box::new(f))
    }
}

impl Matcher {
    /// Create a new interval matcher.
    pub fn interval(before: Option<NaiveDate>, after: Option<NaiveDate>) -> Self {
        Matcher::Interval(IntervalMatcher { before, after })
    }

    /// Create a new range matcher.
    pub fn range(from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        Matcher::Range(RangeMatcher { from, to })
    }

    /// Create a new custom matcher.
    pub fn custom<F>(f: F) -> Self
    where
        F: Fn(&NaiveDate) -> bool + Send + Sync + 'static,
    {
        Matcher::Custom(Box::new(f))
    }

    /// Check if the date matches the matcher.
    pub fn is_match(&self, date: &Date) -> bool {
        match date {
            Date::Single(Some(date)) => self.matched(date),
            Date::Range(Some(start), Some(end)) => self.matched(start) || self.matched(end),
            _ => false,
        }
    }

    fn matched(&self, date: &NaiveDate) -> bool {
        match self {
            Matcher::DayOfWeek(days) => days.contains(&date.weekday().num_days_from_sunday()),
            Matcher::Interval(interval) => {
                let before_check = interval.before.map_or(false, |before| date < &before);
                let after_check = interval.after.map_or(false, |after| date > &after);
                before_check || after_check
            }
            Matcher::Range(range) => {
                let from_check = range.from.map_or(false, |from| date < &from);
                let to_check = range.to.map_or(false, |to| date > &to);
                !from_check && !to_check
            }
            Matcher::Custom(f) => f(date),
        }
    }
}

#[derive(IntoElement)]
pub struct Calendar {
    id: ElementId,
    size: Size,
    state: Entity<CalendarState>,
    style: StyleRefinement,
    number_of_months: Option<usize>,
    week_starts_on: Weekday,
    show_outside_days: bool,
    animated: bool,
    aria_label: SharedString,
}

/// Use to store the state of the calendar.
pub struct CalendarState {
    focus_handle: FocusHandle,
    view_mode: ViewMode,
    date: Date,
    current_year: i32,
    current_month: u8,
    years: Vec<Vec<i32>>,
    year_page: i32,
    today: NaiveDate,
    number_of_months: usize,
    active_date: NaiveDate,
    previous_month: Option<(i32, u8)>,
    transition_direction: i8,
    transition_generation: u64,
    pub(crate) disabled_matcher: Option<Rc<Matcher>>,
}

impl CalendarState {
    /// Create a new calendar state.
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let today = Local::now().naive_local().date();
        Self {
            focus_handle: cx.focus_handle(),
            view_mode: ViewMode::Day,
            date: Date::Single(None),
            current_month: today.month() as u8,
            current_year: today.year(),
            years: vec![],
            year_page: 0,
            today,
            number_of_months: 1,
            active_date: today,
            previous_month: None,
            transition_direction: 0,
            transition_generation: 0,
            disabled_matcher: None,
        }
        .year_range((today.year() - 50, today.year() + 50))
    }

    /// Create Calendar state configured for range selection.
    pub fn range(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut state = Self::new(window, cx);
        state.date = Date::Range(None, None);
        state
    }

    /// Set the disabled matcher of the calendar state.
    pub fn disabled_matcher(mut self, matcher: impl Into<Matcher>) -> Self {
        self.disabled_matcher = Some(Rc::new(matcher.into()));
        self
    }

    /// Set the disabled matcher of the calendar.
    ///
    /// The disabled matcher will be used to disable the days that match the matcher.
    pub fn set_disabled_matcher(
        &mut self,
        disabled: impl Into<Matcher>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.disabled_matcher = Some(Rc::new(disabled.into()));
        cx.notify();
    }

    /// Set the date of the calendar.
    ///
    /// When you set a range date, the mode will be automatically set to `Mode::Range`.
    pub fn set_date(&mut self, date: impl Into<Date>, _: &mut Window, cx: &mut Context<Self>) {
        let date = date.into();

        let invalid = self
            .disabled_matcher
            .as_ref()
            .map_or(false, |matcher| matcher.is_match(&date));

        if invalid {
            return;
        }

        self.date = match date {
            Date::Range(Some(start), Some(end)) if end < start => {
                Date::Range(Some(end), Some(start))
            }
            Date::Range(None, Some(end)) => Date::Range(Some(end), None),
            date => date,
        };
        match self.date {
            Date::Single(Some(date)) => {
                self.current_month = date.month() as u8;
                self.current_year = date.year();
                self.active_date = date;
            }
            Date::Range(Some(start), end) => {
                let active = end.unwrap_or(start);
                self.current_month = active.month() as u8;
                self.current_year = active.year();
                self.active_date = active;
            }
            _ => {}
        }

        cx.notify()
    }

    /// Get the date of the calendar.
    pub fn date(&self) -> Date {
        self.date
    }

    /// Resets the visible page and keyboard cursor to a DatePicker reopen anchor.
    ///
    /// This intentionally avoids a month transition because the enclosing Popover already owns
    /// the opening motion. It also invalidates pending Calendar transition completion work.
    pub(crate) fn reset_view_to_date(&mut self, date: NaiveDate, cx: &mut Context<Self>) {
        self.current_year = date.year();
        self.current_month = date.month() as u8;
        self.active_date = date;
        self.previous_month = None;
        self.transition_direction = 0;
        self.transition_generation = self.transition_generation.wrapping_add(1);
        cx.notify();
    }

    /// Returns the first visible month for internal composite-state verification.
    #[cfg(test)]
    pub(crate) fn visible_month(&self) -> (i32, u8) {
        (self.current_year, self.current_month)
    }

    /// Returns the keyboard cursor date for internal composite-state verification.
    #[cfg(test)]
    pub(crate) fn active_date(&self) -> NaiveDate {
        self.active_date
    }

    /// Set number of months to show.
    pub fn set_number_of_months(
        &mut self,
        number_of_months: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.number_of_months = number_of_months.max(1);
        cx.notify();
    }

    /// Set the year range of the calendar, default is 50 years before and after the current year.
    ///
    /// Each year page contains 20 years, so the range will be divided into chunks of 20 years is better.
    pub fn year_range(mut self, range: (i32, i32)) -> Self {
        self.apply_year_range(range);
        self
    }

    /// Set the year range of the calendar.
    pub fn set_year_range(&mut self, range: (i32, i32), cx: &mut Context<Self>) {
        self.apply_year_range(range);
        cx.notify();
    }

    fn apply_year_range(&mut self, range: (i32, i32)) {
        let lower = range.0.min(range.1);
        let upper = range.0.max(range.1);
        let (start, end) = if lower == upper {
            if upper == i32::MAX {
                (upper.saturating_sub(1), upper)
            } else {
                (lower, lower.saturating_add(1))
            }
        } else {
            (lower, upper)
        };
        self.years = (start..end)
            .collect::<Vec<_>>()
            .chunks(20)
            .map(|chunk| chunk.to_vec())
            .collect::<Vec<_>>();
        self.year_page = self
            .years
            .iter()
            .position(|years| years.contains(&self.current_year))
            .unwrap_or(0) as i32;
    }

    /// Get year and month by offset month.
    fn offset_year_month(&self, offset_month: usize) -> (i32, u32) {
        let mut month = self.current_month as i32 + offset_month as i32;
        let mut year = self.current_year;
        while month < 1 {
            month += 12;
            year -= 1;
        }
        while month > 12 {
            month -= 12;
            year += 1;
        }

        (year, month as u32)
    }

    fn has_prev_year_page(&self) -> bool {
        self.year_page > 0
    }

    fn has_next_year_page(&self) -> bool {
        self.year_page < self.years.len() as i32 - 1
    }

    fn prev_year_page(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.move_year_page(-1, cx);
        self.focus_handle.focus(window, cx);
    }

    fn move_year_page(&mut self, delta: i32, cx: &mut Context<Self>) {
        let next = self.year_page + delta;
        if next < 0 || next >= self.years.len() as i32 {
            return;
        }
        let active_index = self
            .years
            .get(self.year_page as usize)
            .and_then(|years| years.iter().position(|year| *year == self.current_year))
            .unwrap_or(0);
        let Some(next_year) = self
            .years
            .get(next as usize)
            .and_then(|years| years.get(active_index).or_else(|| years.last()))
            .copied()
        else {
            return;
        };
        self.year_page = next;
        self.current_year = next_year;
        cx.notify();
    }

    fn next_year_page(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.move_year_page(1, cx);
        self.focus_handle.focus(window, cx);
    }

    fn move_month(&mut self, delta: i32, animated: bool, cx: &mut Context<Self>) {
        let (year, month) =
            normalized_year_month(self.current_year, self.current_month as i32 + delta);
        self.set_visible_month(year, month, animated, cx);
    }

    /// Updates the visible month and records the previous page for an optional transition.
    fn set_visible_month(&mut self, year: i32, month: u8, animated: bool, cx: &mut Context<Self>) {
        let previous = (self.current_year, self.current_month);
        let previous_index = previous.0 as i64 * 12 + previous.1 as i64 - 1;
        let next_index = year as i64 * 12 + month as i64 - 1;
        let direction = (next_index - previous_index).signum() as i8;
        self.current_year = year;
        self.current_month = month;
        self.active_date = clamp_day(self.active_date.day(), year, month);
        self.transition_direction = if animated { direction } else { 0 };
        self.previous_month = (animated && direction != 0).then_some(previous);
        self.transition_generation = self.transition_generation.wrapping_add(1);
        let generation = self.transition_generation;
        if self.previous_month.is_some() {
            let duration = effective_motion_duration(cx.theme().style.motion.normal(), cx);
            cx.spawn(async move |state, cx| {
                cx.background_executor().timer(duration).await;
                let _ = state.update(cx, |state, cx| {
                    if state.transition_generation == generation {
                        state.previous_month = None;
                        state.transition_direction = 0;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }

    fn month_name(&self, offset_month: usize) -> SharedString {
        let (_, month) = self.offset_year_month(offset_month);
        match month {
            1 => t!("Calendar.month.January"),
            2 => t!("Calendar.month.February"),
            3 => t!("Calendar.month.March"),
            4 => t!("Calendar.month.April"),
            5 => t!("Calendar.month.May"),
            6 => t!("Calendar.month.June"),
            7 => t!("Calendar.month.July"),
            8 => t!("Calendar.month.August"),
            9 => t!("Calendar.month.September"),
            10 => t!("Calendar.month.October"),
            11 => t!("Calendar.month.November"),
            12 => t!("Calendar.month.December"),
            _ => Cow::Borrowed(""),
        }
        .into()
    }

    fn month_short_name(&self, offset_month: usize) -> SharedString {
        let (_, month) = self.offset_year_month(offset_month);
        match month {
            1 => t!("Calendar.month_short.January"),
            2 => t!("Calendar.month_short.February"),
            3 => t!("Calendar.month_short.March"),
            4 => t!("Calendar.month_short.April"),
            5 => t!("Calendar.month_short.May"),
            6 => t!("Calendar.month_short.June"),
            7 => t!("Calendar.month_short.July"),
            8 => t!("Calendar.month_short.August"),
            9 => t!("Calendar.month_short.September"),
            10 => t!("Calendar.month_short.October"),
            11 => t!("Calendar.month_short.November"),
            12 => t!("Calendar.month_short.December"),
            _ => Cow::Borrowed(""),
        }
        .into()
    }

    fn year_name(&self, offset_month: usize) -> SharedString {
        let (year, _) = self.offset_year_month(offset_month);
        year.to_string().into()
    }

    fn set_view_mode(&mut self, mode: ViewMode, window: &mut Window, cx: &mut Context<Self>) {
        self.view_mode = mode;
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    fn months(&self) -> Vec<SharedString> {
        [
            t!("Calendar.month.January"),
            t!("Calendar.month.February"),
            t!("Calendar.month.March"),
            t!("Calendar.month.April"),
            t!("Calendar.month.May"),
            t!("Calendar.month.June"),
            t!("Calendar.month.July"),
            t!("Calendar.month.August"),
            t!("Calendar.month.September"),
            t!("Calendar.month.October"),
            t!("Calendar.month.November"),
            t!("Calendar.month.December"),
        ]
        .iter()
        .map(|s| s.clone().into())
        .collect()
    }

    /// Move the active date while keeping it inside the visible month window.
    fn move_active_date(&mut self, days: i64, cx: &mut Context<Self>) {
        let Some(date) =
            next_enabled_date(self.active_date, days, self.disabled_matcher.as_deref())
        else {
            return;
        };
        self.active_date = date;
        self.keep_active_date_visible(date);
        cx.notify();
    }

    fn keep_active_date_visible(&mut self, date: NaiveDate) {
        let first_month = self.current_year as i64 * 12 + self.current_month as i64 - 1;
        let active_month = date.year() as i64 * 12 + date.month() as i64 - 1;
        let last_month = first_month + self.number_of_months.max(1) as i64 - 1;
        if active_month < first_month {
            self.current_year = date.year();
            self.current_month = date.month() as u8;
        } else if active_month > last_month {
            let next_first = active_month - self.number_of_months.max(1) as i64 + 1;
            self.current_year = next_first.div_euclid(12) as i32;
            self.current_month = (next_first.rem_euclid(12) + 1) as u8;
        }
    }

    fn select_active_date(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let date = self.active_date;
        self.select_date(date, window, cx);
    }

    fn select_date(&mut self, date: NaiveDate, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .disabled_matcher
            .as_ref()
            .is_some_and(|matcher| matcher.matched(&date))
        {
            return;
        }
        match self.date {
            Date::Single(_) => {
                self.set_date(date, window, cx);
                cx.emit(CalendarEvent::Selected(self.date));
            }
            Date::Range(Some(_), None) => {
                self.set_date(self.date.select_range_date(date), window, cx);
                cx.emit(CalendarEvent::Selected(self.date));
            }
            Date::Range(_, _) => self.set_date(Date::Range(Some(date), None), window, cx),
        }
        self.active_date = date;
        self.keep_active_date_visible(date);
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    fn move_active_year(&mut self, delta: i32, cx: &mut Context<Self>) {
        let Some(year) = self.current_year.checked_add(delta) else {
            return;
        };
        let Some(page) = self.years.iter().position(|years| years.contains(&year)) else {
            return;
        };
        self.current_year = year;
        self.year_page = page as i32;
        cx.notify();
    }

    fn on_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Day => self.move_active_date(-1, cx),
            ViewMode::Month => {
                self.current_month = ((self.current_month as i32 - 2).rem_euclid(12) + 1) as u8;
                cx.notify();
            }
            ViewMode::Year => self.move_active_year(-1, cx),
        }
    }

    fn on_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Day => self.move_active_date(1, cx),
            ViewMode::Month => {
                self.current_month = (self.current_month % 12) + 1;
                cx.notify();
            }
            ViewMode::Year => self.move_active_year(1, cx),
        }
    }

    fn on_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Day => self.move_active_date(-7, cx),
            ViewMode::Month => {
                self.current_month = ((self.current_month as i32 - 4).rem_euclid(12) + 1) as u8;
                cx.notify();
            }
            ViewMode::Year => self.move_active_year(-5, cx),
        }
    }

    fn on_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Day => self.move_active_date(7, cx),
            ViewMode::Month => {
                self.current_month = ((self.current_month as i32 + 2).rem_euclid(12) + 1) as u8;
                cx.notify();
            }
            ViewMode::Year => self.move_active_year(5, cx),
        }
    }

    fn move_page(&mut self, delta: i32, animated: bool, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Day => self.move_month(delta, animated, cx),
            ViewMode::Month => {
                self.current_year += delta;
                cx.notify();
            }
            ViewMode::Year => self.move_year_page(delta, cx),
        }
    }

    fn move_to_edge(&mut self, week_starts_on: Weekday, last: bool, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Day => {
                let current = self.active_date.weekday().num_days_from_monday() as i64;
                let start = week_starts_on.num_days_from_monday() as i64;
                let from_start = (current - start).rem_euclid(7);
                self.move_active_date(if last { 6 - from_start } else { -from_start }, cx);
            }
            ViewMode::Month => {
                self.current_month = if last { 12 } else { 1 };
                cx.notify();
            }
            ViewMode::Year => {
                let year = self
                    .years
                    .get(self.year_page as usize)
                    .and_then(|years| if last { years.last() } else { years.first() })
                    .copied();
                if let Some(year) = year {
                    self.current_year = year;
                    cx.notify();
                }
            }
        }
    }

    fn on_confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Day => self.select_active_date(window, cx),
            ViewMode::Month | ViewMode::Year => self.set_view_mode(ViewMode::Day, window, cx),
        }
    }

    fn on_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if self.view_mode.is_day() {
            cx.propagate();
        } else {
            self.set_view_mode(ViewMode::Day, window, cx);
        }
    }
}

impl Render for CalendarState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl Focusable for CalendarState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Normalize an arbitrary one-based month into a valid year/month pair.
fn normalized_year_month(year: i32, month: i32) -> (i32, u8) {
    let index = year * 12 + month - 1;
    (index.div_euclid(12), (index.rem_euclid(12) + 1) as u8)
}

/// Find the next enabled date without allowing an unbounded matcher scan.
fn next_enabled_date(active: NaiveDate, days: i64, matcher: Option<&Matcher>) -> Option<NaiveDate> {
    if days == 0 {
        return Some(active);
    }

    let step = Duration::days(days.signum());
    let mut date = active.checked_add_signed(Duration::days(days))?;
    for _ in 0..MAX_DISABLED_DATE_SCAN_DAYS {
        if !matcher.is_some_and(|matcher| matcher.matched(&date)) {
            return Some(date);
        }
        date = date.checked_add_signed(step)?;
    }
    None
}

/// Keep a day valid when navigating between months with different lengths.
fn clamp_day(day: u32, year: i32, month: u8) -> NaiveDate {
    let next = normalized_year_month(year, month as i32 + 1);
    let last = NaiveDate::from_ymd_opt(next.0, next.1 as u32, 1)
        .expect("normalized next month must be valid")
        - Duration::days(1);
    NaiveDate::from_ymd_opt(year, month as u32, day.min(last.day()))
        .expect("clamped calendar date must be valid")
}

#[derive(Clone, Copy)]
struct CalendarMetrics {
    cell: Pixels,
    padding: Pixels,
    radius: Pixels,
    month_gap: Pixels,
    caption_height: Pixels,
    caption_padding_left: Pixels,
    caption_padding_right: Pixels,
    caption_gap: Pixels,
}

impl CalendarMetrics {
    /// Resolve Calendar geometry from semantic Style Preset values without preset ID checks.
    fn resolve(size: Size, cx: &App) -> Self {
        Self::from_style(size, &cx.theme().style)
    }

    fn from_style(size: Size, style: &StylePreset) -> Self {
        let baseline = style.controls.sm.height;
        let cell = match size {
            Size::XSmall => style.controls.xs.height,
            Size::Small => (baseline - px(4.)).max(style.controls.xs.height),
            Size::Medium => baseline,
            Size::Large => style.controls.lg.height,
            Size::Size(cell) => cell,
        };
        Self {
            cell,
            padding: match style.density {
                Density::Compact => px(8.),
                Density::Standard | Density::Comfortable => px(12.),
            },
            radius: match style.density {
                Density::Comfortable => style.radii.xl,
                Density::Compact | Density::Standard => style.radii.md,
            },
            month_gap: px(16.),
            caption_height: match style.density {
                Density::Compact => style.controls.xs.height,
                Density::Standard | Density::Comfortable => style.controls.sm.height,
            },
            caption_padding_left: match style.density {
                Density::Compact => px(6.),
                Density::Standard => px(8.),
                Density::Comfortable => px(12.),
            },
            caption_padding_right: match style.density {
                Density::Compact | Density::Standard => px(4.),
                Density::Comfortable => px(8.),
            },
            caption_gap: px(6.),
        }
    }

    fn grid_width(self) -> Pixels {
        self.cell * 7.
    }

    fn month_picker_item_width(self) -> Pixels {
        self.cell * 2.5
    }

    fn month_picker_grid_width(self) -> Pixels {
        self.month_picker_item_width() * 3. + px(16.)
    }

    fn year_picker_item_width(self) -> Pixels {
        (self.grid_width() - px(16.)) / 5.
    }
}

impl Calendar {
    /// Create a new Calendar backed by the supplied state entity.
    pub fn new(state: &Entity<CalendarState>) -> Self {
        Self {
            id: ("calendar", state.entity_id()).into(),
            size: Size::default(),
            state: state.clone(),
            style: StyleRefinement::default(),
            number_of_months: None,
            week_starts_on: Weekday::Sun,
            show_outside_days: true,
            animated: false,
            aria_label: "Calendar".into(),
        }
    }

    /// Override the number of visible months. Values below one resolve to one.
    pub fn number_of_months(mut self, number_of_months: usize) -> Self {
        self.number_of_months = Some(number_of_months.max(1));
        self
    }

    /// Set the first day of the week. Sunday is used by default.
    pub fn week_starts_on(mut self, week_starts_on: Weekday) -> Self {
        self.week_starts_on = week_starts_on;
        self
    }

    /// Show or hide dates outside their owning month.
    pub fn show_outside_days(mut self, show: bool) -> Self {
        self.show_outside_days = show;
        self
    }

    /// Enable direction-aware month transitions. Motion is disabled by default.
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Set the accessible name exposed by the Calendar grid.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = label.into();
        self
    }

    fn resolved_month_count(&self, cx: &App) -> usize {
        self.number_of_months
            .unwrap_or_else(|| self.state.read(cx).number_of_months)
            .max(1)
    }

    fn render_header(
        &self,
        number_of_months: usize,
        metrics: CalendarMetrics,
        window: &mut Window,
        cx: &mut App,
    ) -> Div {
        let state = self.state.read(cx);
        let view_mode = state.view_mode;
        let entity_id = self.state.entity_id();
        let month_label = state.month_short_name(0);
        let month_aria_label = state.month_name(0);
        let year_label = state.current_year.to_string();

        h_flex()
            .h(metrics.cell)
            .w_full()
            .items_center()
            .justify_between()
            .child(
                Button::new(("calendar-prev", entity_id))
                    .icon(IconName::ChevronLeft)
                    .aria_label(t!("Calendar.previous"))
                    .ghost()
                    .tab_stop(false)
                    .with_size(Size::Size(metrics.cell))
                    .disabled(view_mode.is_month())
                    .when(view_mode.is_day(), |this| {
                        let animated = self.animated;
                        this.on_click(window.listener_for(
                            &self.state,
                            move |state, _, window, cx| {
                                state.move_month(-1, animated, cx);
                                state.focus_handle.focus(window, cx);
                            },
                        ))
                    })
                    .when(view_mode.is_year(), |this| {
                        this.disabled(!state.has_prev_year_page()).on_click(
                            window.listener_for(&self.state, CalendarState::prev_year_page),
                        )
                    }),
            )
            .when(number_of_months == 1, |this| {
                this.child(
                    h_flex()
                        .items_center()
                        .justify_center()
                        .gap_1()
                        .child(
                            Button::new(("calendar-month", entity_id))
                                .aria_label(month_aria_label)
                                .ghost()
                                .tab_stop(false)
                                .with_size(Size::Size(metrics.caption_height))
                                .pl(metrics.caption_padding_left)
                                .pr(metrics.caption_padding_right)
                                .child(
                                    h_flex()
                                        .gap(metrics.caption_gap)
                                        .text_sm()
                                        .font_medium()
                                        .child(month_label)
                                        .child(
                                            Icon::new(IconName::ChevronDown)
                                                .small()
                                                .text_color(cx.theme().muted_foreground),
                                        ),
                                )
                                .on_click(window.listener_for(
                                    &self.state,
                                    move |state, _, window, cx| {
                                        state.set_view_mode(
                                            if view_mode.is_month() {
                                                ViewMode::Day
                                            } else {
                                                ViewMode::Month
                                            },
                                            window,
                                            cx,
                                        );
                                    },
                                )),
                        )
                        .child(
                            Button::new(("calendar-year", entity_id))
                                .aria_label(year_label.clone())
                                .ghost()
                                .tab_stop(false)
                                .with_size(Size::Size(metrics.caption_height))
                                .pl(metrics.caption_padding_left)
                                .pr(metrics.caption_padding_right)
                                .child(
                                    h_flex()
                                        .gap(metrics.caption_gap)
                                        .text_sm()
                                        .font_medium()
                                        .child(year_label)
                                        .child(
                                            Icon::new(IconName::ChevronDown)
                                                .small()
                                                .text_color(cx.theme().muted_foreground),
                                        ),
                                )
                                .on_click(window.listener_for(
                                    &self.state,
                                    move |state, _, window, cx| {
                                        state.set_view_mode(
                                            if view_mode.is_year() {
                                                ViewMode::Day
                                            } else {
                                                ViewMode::Year
                                            },
                                            window,
                                            cx,
                                        );
                                    },
                                )),
                        ),
                )
            })
            .when(number_of_months > 1, |this| {
                this.child(
                    h_flex()
                        .flex_1()
                        .justify_around()
                        .children((0..number_of_months).map(|offset| {
                            h_flex()
                                .gap_1()
                                .font_medium()
                                .text_sm()
                                .child(state.month_name(offset))
                                .child(state.year_name(offset))
                        })),
                )
            })
            .child(
                Button::new(("calendar-next", entity_id))
                    .icon(IconName::ChevronRight)
                    .aria_label(t!("Calendar.next"))
                    .ghost()
                    .tab_stop(false)
                    .with_size(Size::Size(metrics.cell))
                    .disabled(view_mode.is_month())
                    .when(view_mode.is_day(), |this| {
                        let animated = self.animated;
                        this.on_click(window.listener_for(
                            &self.state,
                            move |state, _, window, cx| {
                                state.move_month(1, animated, cx);
                                state.focus_handle.focus(window, cx);
                            },
                        ))
                    })
                    .when(view_mode.is_year(), |this| {
                        this.disabled(!state.has_next_year_page()).on_click(
                            window.listener_for(&self.state, CalendarState::next_year_page),
                        )
                    }),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_day(
        &self,
        date: NaiveDate,
        owning_year: i32,
        owning_month: u8,
        panel: usize,
        row: usize,
        column: usize,
        visible_start: (i32, u8),
        interactive: bool,
        metrics: CalendarMetrics,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let state = self.state.read(cx);
        let outside = date.year() != owning_year || date.month() != owning_month as u32;
        let duplicate = outside && self.resolved_month_count(cx) > 1 && {
            let first = visible_start.0 * 12 + visible_start.1 as i32 - 1;
            let value = date.year() * 12 + date.month() as i32 - 1;
            value >= first && value < first + self.resolved_month_count(cx) as i32
        };
        if (outside && !self.show_outside_days) || duplicate {
            return div().size(metrics.cell).into_any_element();
        }

        let selected = state.date.is_active(&date);
        let in_range = state.date.is_in_range(&date);
        let range_complete = matches!(state.date, Date::Range(Some(_), Some(_)));
        let (range_start, range_end) = match state.date {
            Date::Range(start, end) => (start == Some(date), end == Some(date)),
            Date::Single(_) => (false, false),
        };
        let today = date == state.today;
        let disabled = state
            .disabled_matcher
            .as_ref()
            .is_some_and(|matcher| matcher.matched(&date));
        let active_descendant = state.active_date == date;
        let focus_visible = interactive
            && active_descendant
            && state.focus_handle.is_focused(window)
            && window.last_input_was_keyboard();
        let range_track = range_complete && in_range;
        let id: ElementId = SharedString::from(format!(
            "calendar-day-{:?}-{panel}-{}",
            self.state.entity_id(),
            date.num_days_from_ce()
        ))
        .into();
        let button_id: ElementId = SharedString::from(format!(
            "calendar-day-button-{:?}-{panel}-{}",
            self.state.entity_id(),
            date.num_days_from_ce()
        ))
        .into();

        let day_button = h_flex()
            .id(button_id)
            .relative()
            .size(metrics.cell)
            .justify_center()
            .rounded(metrics.radius)
            .text_sm()
            .font_normal()
            .when(today && !selected && !in_range, |this| {
                this.bg(cx.theme().muted).text_color(cx.theme().foreground)
            })
            .when(in_range && !selected, |this| {
                this.rounded_none()
                    .bg(cx.theme().muted)
                    .text_color(cx.theme().foreground)
            })
            .when(selected, |this| {
                this.bg(cx.theme().primary)
                    .text_color(cx.theme().primary_foreground)
            })
            .when(outside || disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .when(interactive && !selected && !in_range && !disabled, |this| {
                this.hover(|this| this.bg(cx.theme().muted).text_color(cx.theme().foreground))
            })
            .when(interactive && !disabled, |this| {
                this.active(|this| this.relative().top(px(1.)))
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        crate::global_state::GlobalState::suppress_text_selection(cx);
                    })
            })
            .when(focus_visible, |this| {
                let ring_width = cx.theme().style.focus.ring_width;
                let ring_offset = cx.theme().style.focus.ring_offset;
                let inset = ring_width + ring_offset;
                this.child(
                    div()
                        .absolute()
                        .top(-inset)
                        .right(-inset)
                        .bottom(-inset)
                        .left(-inset)
                        .border(ring_width)
                        .border_color(cx.theme().ring.opacity(0.5))
                        .rounded(metrics.radius + inset),
                )
            })
            .child(date.day().to_string());

        let element = h_flex()
            .id(id)
            .when(interactive, |this| {
                this.role(Role::GridCell)
                    .aria_selected(selected || in_range)
                    .aria_row_index(row + 2)
                    .aria_column_index(column + 1)
            })
            .when(active_descendant && interactive, |this| {
                this.aria_active_descendant()
            })
            .size(metrics.cell)
            .justify_center()
            .rounded(metrics.radius)
            .when(range_track, |this| {
                this.rounded_none()
                    .bg(cx.theme().muted)
                    .when(range_start || column == 0, |this| {
                        this.rounded_tl(metrics.radius).rounded_bl(metrics.radius)
                    })
                    .when(range_end || column == 6, |this| {
                        this.rounded_tr(metrics.radius).rounded_br(metrics.radius)
                    })
            })
            .when(disabled, |this| this.opacity(0.5))
            .when(interactive && !disabled, |this| {
                this.on_click(
                    window.listener_for(&self.state, move |state, _, window, cx| {
                        state.select_date(date, window, cx);
                    }),
                )
            })
            .child(day_button);

        crate::accessibility::accessibility_state_with_current(
            element,
            false,
            false,
            disabled,
            today.then_some(gpui::accesskit::AriaCurrent::Date),
        )
        .into_any_element()
    }

    fn render_days(
        &self,
        number_of_months: usize,
        visible_start: Option<(i32, u8)>,
        interactive: bool,
        metrics: CalendarMetrics,
        window: &mut Window,
        cx: &mut App,
    ) -> Div {
        let (current_year, current_month) = visible_start.unwrap_or_else(|| {
            let state = self.state.read(cx);
            (state.current_year, state.current_month)
        });
        let mut weekdays = [
            t!("Calendar.week.0"),
            t!("Calendar.week.1"),
            t!("Calendar.week.2"),
            t!("Calendar.week.3"),
            t!("Calendar.week.4"),
            t!("Calendar.week.5"),
            t!("Calendar.week.6"),
        ];
        weekdays.rotate_left(self.week_starts_on.num_days_from_sunday() as usize);

        h_flex()
            .gap(metrics.month_gap)
            .items_start()
            .children((0..number_of_months).map(|panel| {
                let (year, month) =
                    normalized_year_month(current_year, current_month as i32 + panel as i32);
                let weeks = days_in_month(year, month as i32, self.week_starts_on);
                v_flex()
                    .gap_2()
                    .child(
                        h_flex().children(weekdays.iter().enumerate().map(|(column, label)| {
                            h_flex()
                                .id(SharedString::from(format!(
                                    "calendar-weekday-{:?}-{panel}-{column}",
                                    self.state.entity_id()
                                )))
                                .role(Role::ColumnHeader)
                                .aria_column_index(column + 1)
                                .size(metrics.cell)
                                .justify_center()
                                .text_xs()
                                .font_normal()
                                .text_color(cx.theme().muted_foreground)
                                .child(label.clone())
                        })),
                    )
                    .children(weeks.into_iter().enumerate().map(|(row, week)| {
                        h_flex().children(week.into_iter().enumerate().map(|(column, date)| {
                            self.render_day(
                                date,
                                year,
                                month,
                                panel,
                                row,
                                column,
                                (current_year, current_month),
                                interactive,
                                metrics,
                                window,
                                cx,
                            )
                        }))
                    }))
            }))
    }

    fn picker_item(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        active: bool,
        metrics: CalendarMetrics,
        window: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        let focus_visible = active
            && self.state.read(cx).focus_handle.is_focused(window)
            && window.last_input_was_keyboard();
        h_flex()
            .id(id)
            .role(Role::GridCell)
            .aria_selected(active)
            .when(active, |this| this.aria_active_descendant())
            .relative()
            .h(metrics.cell)
            .justify_center()
            .rounded(metrics.radius)
            .text_sm()
            .font_normal()
            .when(active, |this| {
                this.bg(cx.theme().primary)
                    .text_color(cx.theme().primary_foreground)
            })
            .when(!active, |this| {
                this.hover(|this| this.bg(cx.theme().muted).text_color(cx.theme().foreground))
            })
            .active(|this| this.relative().top(px(1.)))
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                crate::global_state::GlobalState::suppress_text_selection(cx);
            })
            .when(focus_visible, |this| {
                let ring_width = cx.theme().style.focus.ring_width;
                let ring_offset = cx.theme().style.focus.ring_offset;
                let inset = ring_width + ring_offset;
                this.child(
                    div()
                        .absolute()
                        .top(-inset)
                        .right(-inset)
                        .bottom(-inset)
                        .left(-inset)
                        .border(ring_width)
                        .border_color(cx.theme().ring.opacity(0.5))
                        .rounded(metrics.radius + inset),
                )
            })
            .child(label.into())
    }

    fn render_months(
        &self,
        metrics: CalendarMetrics,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let (current_month, months) = {
            let state = self.state.read(cx);
            (state.current_month, state.months())
        };
        let animated = self.animated;
        let mut rows = Vec::with_capacity(4);
        for row in 0..4 {
            let mut items = Vec::with_capacity(3);
            for column in 0..3 {
                let index = row * 3 + column;
                let month = months[index].clone();
                items.push(
                    self.picker_item(
                        SharedString::from(format!(
                            "calendar-month-option-{:?}-{index}",
                            self.state.entity_id()
                        )),
                        month,
                        current_month == index as u8 + 1,
                        metrics,
                        window,
                        cx,
                    )
                    .w(metrics.month_picker_item_width())
                    .px_2()
                    .aria_row_index(row + 1)
                    .aria_column_index(column + 1)
                    .on_click(window.listener_for(
                        &self.state,
                        move |state, _, window, cx| {
                            state.set_visible_month(
                                state.current_year,
                                index as u8 + 1,
                                animated,
                                cx,
                            );
                            state.set_view_mode(ViewMode::Day, window, cx);
                        },
                    )),
                );
            }
            rows.push(h_flex().gap_2().children(items));
        }

        v_flex()
            .id(("calendar-months", self.state.entity_id()))
            .role(Role::Grid)
            .mt_3()
            .w(metrics.month_picker_grid_width())
            .gap_2()
            .children(rows)
    }

    fn render_years(
        &self,
        metrics: CalendarMetrics,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let (current_year, years) = {
            let state = self.state.read(cx);
            (
                state.current_year,
                state
                    .years
                    .get(state.year_page as usize)
                    .cloned()
                    .unwrap_or_default(),
            )
        };
        let animated = self.animated;
        let mut rows = Vec::with_capacity(years.len().div_ceil(5));
        for (row_index, years) in years.chunks(5).enumerate() {
            let mut items = Vec::with_capacity(years.len());
            for (column, year) in years.iter().copied().enumerate() {
                let index = row_index * 5 + column;
                items.push(
                    self.picker_item(
                        SharedString::from(format!(
                            "calendar-year-option-{:?}-{index}",
                            self.state.entity_id()
                        )),
                        year.to_string(),
                        current_year == year,
                        metrics,
                        window,
                        cx,
                    )
                    .w(metrics.year_picker_item_width())
                    .aria_row_index(row_index + 1)
                    .aria_column_index(column + 1)
                    .on_click(window.listener_for(
                        &self.state,
                        move |state, _, window, cx| {
                            state.set_visible_month(year, state.current_month, animated, cx);
                            state.set_view_mode(ViewMode::Day, window, cx);
                        },
                    )),
                );
            }
            rows.push(h_flex().gap_1().children(items));
        }

        v_flex()
            .id(("calendar-years", self.state.entity_id()))
            .role(Role::Grid)
            .mt_3()
            .w(metrics.grid_width())
            .gap_1()
            .children(rows)
    }
}

impl Sizable for Calendar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Calendar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl EventEmitter<CalendarEvent> for CalendarState {}
impl RenderOnce for Calendar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let view_mode = self.state.read(cx).view_mode;
        let number_of_months = self.resolved_month_count(cx);
        self.state.update(cx, |state, _| {
            state.number_of_months = number_of_months;
        });
        let metrics = CalendarMetrics::resolve(self.size, cx);
        let header = self.render_header(number_of_months, metrics, window, cx);
        let (previous_month, direction, generation) = {
            let state = self.state.read(cx);
            (
                state.previous_month,
                state.transition_direction as f32,
                state.transition_generation,
            )
        };
        let body = match view_mode {
            ViewMode::Day if self.animated && previous_month.is_some() => {
                let duration = effective_motion_duration(cx.theme().style.motion.normal(), cx);
                let easing = cx.theme().style.motion.move_easing;
                let previous =
                    self.render_days(number_of_months, previous_month, false, metrics, window, cx);
                let current = self.render_days(number_of_months, None, true, metrics, window, cx);
                div()
                    .relative()
                    .overflow_hidden()
                    .child(
                        Transition::new(duration)
                            .ease_token(easing)
                            .slide_x(px(0.), metrics.cell * -direction)
                            .fade(1., 0.)
                            .apply(
                                div().absolute().inset_0().child(previous),
                                SharedString::from(format!(
                                    "calendar-month-exit-{:?}-{generation}",
                                    self.state.entity_id()
                                )),
                            ),
                    )
                    .child(
                        Transition::new(duration)
                            .ease_token(easing)
                            .slide_x(metrics.cell * direction, px(0.))
                            .fade(0., 1.)
                            .apply(
                                div().child(current),
                                SharedString::from(format!(
                                    "calendar-month-enter-{:?}-{generation}",
                                    self.state.entity_id()
                                )),
                            ),
                    )
                    .into_any_element()
            }
            ViewMode::Day => self
                .render_days(number_of_months, None, true, metrics, window, cx)
                .into_any_element(),
            ViewMode::Month => self.render_months(metrics, window, cx).into_any_element(),
            ViewMode::Year => self.render_years(metrics, window, cx).into_any_element(),
        };
        let focus_handle = self.state.read(cx).focus_handle.clone().tab_stop(true);
        let week_starts_on = self.week_starts_on;

        v_flex()
            .id(self.id.clone())
            .role(Role::Grid)
            .aria_label(self.aria_label)
            .key_context(CONTEXT)
            .track_focus(&focus_handle)
            .on_action(window.listener_for(&self.state, CalendarState::on_left))
            .on_action(window.listener_for(&self.state, CalendarState::on_right))
            .on_action(window.listener_for(&self.state, CalendarState::on_up))
            .on_action(window.listener_for(&self.state, CalendarState::on_down))
            .on_action(window.listener_for(&self.state, {
                let animated = self.animated;
                move |state, _: &SelectPageUp, _, cx| state.move_page(-1, animated, cx)
            }))
            .on_action(window.listener_for(&self.state, {
                let animated = self.animated;
                move |state, _: &SelectPageDown, _, cx| state.move_page(1, animated, cx)
            }))
            .on_action(
                window.listener_for(&self.state, move |state, _: &SelectFirst, _, cx| {
                    state.move_to_edge(week_starts_on, false, cx);
                }),
            )
            .on_action(
                window.listener_for(&self.state, move |state, _: &SelectLast, _, cx| {
                    state.move_to_edge(week_starts_on, true, cx);
                }),
            )
            .on_action(window.listener_for(&self.state, CalendarState::on_confirm))
            .on_action(window.listener_for(&self.state, CalendarState::on_cancel))
            .bg(cx.theme().background)
            .p(metrics.padding)
            .gap_4()
            .refine_style(&self.style)
            .child(header)
            .child(body)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use gpui::px;

    use super::{
        CalendarMetrics, Date, Matcher, Size, StylePreset, next_enabled_date, normalized_year_month,
    };

    #[test]
    fn test_date_to_string() {
        let date = Date::Single(Some(NaiveDate::from_ymd_opt(2024, 8, 3).unwrap()));
        assert_eq!(date.to_string(), "2024-08-03");

        let date = Date::Single(None);
        assert_eq!(date.to_string(), "nil");

        let date = Date::Range(
            Some(NaiveDate::from_ymd_opt(2024, 8, 3).unwrap()),
            Some(NaiveDate::from_ymd_opt(2024, 8, 5).unwrap()),
        );
        assert_eq!(date.to_string(), "2024-08-03 - 2024-08-05");

        let date = Date::Range(Some(NaiveDate::from_ymd_opt(2024, 8, 3).unwrap()), None);
        assert_eq!(date.to_string(), "2024-08-03 - nil");

        let date = Date::Range(None, Some(NaiveDate::from_ymd_opt(2024, 8, 5).unwrap()));
        assert_eq!(date.to_string(), "nil - 2024-08-05");

        let date = Date::Range(None, None);
        assert_eq!(date.to_string(), "nil");
    }

    #[test]
    fn range_selection_orders_endpoints_and_restarts_after_completion() {
        let later = NaiveDate::from_ymd_opt(2026, 8, 20).unwrap();
        let earlier = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

        assert_eq!(
            Date::Range(Some(later), None).select_range_date(earlier),
            Date::Range(Some(earlier), Some(later))
        );
        assert_eq!(
            Date::Range(Some(earlier), None).select_range_date(earlier),
            Date::Range(Some(earlier), Some(earlier))
        );
        assert_eq!(
            Date::Range(Some(earlier), Some(later)).select_range_date(later),
            Date::Range(Some(later), None)
        );
    }

    #[test]
    fn month_normalization_handles_large_offsets() {
        assert_eq!(normalized_year_month(2024, 14), (2025, 2));
        assert_eq!(normalized_year_month(2024, -1), (2023, 11));
    }

    #[test]
    fn disabled_date_navigation_skips_finite_runs() {
        let friday = NaiveDate::from_ymd_opt(2026, 8, 7).unwrap();
        let weekends = Matcher::from(vec![0, 6]);

        assert_eq!(
            next_enabled_date(friday, 1, Some(&weekends)),
            NaiveDate::from_ymd_opt(2026, 8, 10)
        );
    }

    #[test]
    fn disabled_date_navigation_stops_at_safe_boundaries() {
        let date = NaiveDate::from_ymd_opt(2026, 8, 7).unwrap();
        let all_disabled = Matcher::custom(|_| true);
        let disabled_before_date = Matcher::interval(Some(date), None);

        assert_eq!(next_enabled_date(date, 1, Some(&all_disabled)), None);
        assert_eq!(
            next_enabled_date(date, -1, Some(&disabled_before_date)),
            None
        );
        assert_eq!(next_enabled_date(NaiveDate::MIN, -1, None), None);
    }

    #[test]
    fn built_in_presets_resolve_shadcn_calendar_geometry() {
        let vega = CalendarMetrics::from_style(Size::Medium, &StylePreset::vega());
        let nova = CalendarMetrics::from_style(Size::Medium, &StylePreset::nova());
        let maia = CalendarMetrics::from_style(Size::Medium, &StylePreset::maia());

        assert_eq!(
            (vega.cell, vega.padding, vega.radius),
            (px(32.), px(12.), px(8.))
        );
        assert_eq!(
            (nova.cell, nova.padding, nova.radius),
            (px(28.), px(8.), px(6.))
        );
        assert_eq!(
            (maia.cell, maia.padding, maia.radius),
            (px(32.), px(12.), px(18.))
        );
        assert_eq!(
            (
                vega.caption_height,
                vega.caption_padding_left,
                vega.caption_padding_right,
            ),
            (px(32.), px(8.), px(4.))
        );
        assert_eq!(
            (
                nova.caption_height,
                nova.caption_padding_left,
                nova.caption_padding_right,
            ),
            (px(24.), px(6.), px(4.))
        );
        assert_eq!(
            (
                maia.caption_height,
                maia.caption_padding_left,
                maia.caption_padding_right,
            ),
            (px(32.), px(12.), px(8.))
        );
        assert_eq!(
            vega.month_picker_item_width() * 3. + px(16.),
            vega.month_picker_grid_width()
        );
        assert!(vega.month_picker_grid_width() > vega.grid_width());
        assert_eq!(
            vega.year_picker_item_width() * 5. + px(16.),
            vega.grid_width()
        );
    }
}
