// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `week_starts_on`, `calendar_animated`.
// - Added or exposed behavior through `translation_locale`, `locale_kind`, `month_translation_key`,
//   `localized_month`, `english_ordinal`, `format_localized_endpoint`, `format_custom_date`,
//   `format_date_display` and 19 more.
// - Removed or replaced `on_enter`, `focus_back_if_need`, `toggle_calendar`.
// - Reworked Date Picker around accessibility semantics and ARIA state, interruptible and
//   reduced-motion-aware transitions, semantic Style Preset geometry and density, keyboard
//   navigation and activation behavior, focus-visible and focus restoration behavior.
// - Replaced legacy radius access with `Theme.style.radii.md`.
use std::rc::Rc;

use chrono::{Datelike, NaiveDate, Weekday};
use gpui::{
    App, AppContext, ClickEvent, Context, ElementId, Empty, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Subscription, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme, Disableable, IconName, Sizable, Size, StyledExt as _,
    actions::{Cancel, Confirm},
    button::Button,
    h_flex,
    input::{Delete, clear_button},
    popover::{Popover, PopoverAlign},
    v_flex,
};

use super::calendar::{Calendar, CalendarEvent, CalendarState, Date, Matcher};

const CONTEXT: &'static str = "DatePicker";

#[derive(Clone, Copy)]
enum LocalizedDateStyle {
    Single,
    Range,
}

#[derive(Clone, Copy)]
enum LocaleKind {
    English,
    Chinese,
    French,
    Italian,
}

/// Maps locale variants to the translation locale provided by the component catalog.
fn translation_locale(locale: &str) -> &'static str {
    let locale = locale.replace('_', "-").to_ascii_lowercase();
    if locale == "zh-hk" {
        "zh-HK"
    } else if locale == "zh-tw" || locale.starts_with("zh-hant") {
        "zh-TW"
    } else if locale == "zh" || locale.starts_with("zh-") {
        "zh-CN"
    } else if locale == "fr" || locale.starts_with("fr-") {
        "fr"
    } else if locale == "it" || locale.starts_with("it-") {
        "it"
    } else {
        "en"
    }
}

/// Resolves supported locale families while keeping English as the stable fallback.
fn locale_kind(locale: &str) -> LocaleKind {
    let locale = locale.replace('_', "-").to_ascii_lowercase();
    if locale == "zh" || locale.starts_with("zh-") {
        LocaleKind::Chinese
    } else if locale == "fr" || locale.starts_with("fr-") {
        LocaleKind::French
    } else if locale == "it" || locale.starts_with("it-") {
        LocaleKind::Italian
    } else {
        LocaleKind::English
    }
}

/// Returns the Calendar translation key for a month.
fn month_translation_key(month: u32, short: bool) -> &'static str {
    match (short, month) {
        (false, 1) => "Calendar.month.January",
        (false, 2) => "Calendar.month.February",
        (false, 3) => "Calendar.month.March",
        (false, 4) => "Calendar.month.April",
        (false, 5) => "Calendar.month.May",
        (false, 6) => "Calendar.month.June",
        (false, 7) => "Calendar.month.July",
        (false, 8) => "Calendar.month.August",
        (false, 9) => "Calendar.month.September",
        (false, 10) => "Calendar.month.October",
        (false, 11) => "Calendar.month.November",
        (false, 12) => "Calendar.month.December",
        (true, 1) => "Calendar.month_short.January",
        (true, 2) => "Calendar.month_short.February",
        (true, 3) => "Calendar.month_short.March",
        (true, 4) => "Calendar.month_short.April",
        (true, 5) => "Calendar.month_short.May",
        (true, 6) => "Calendar.month_short.June",
        (true, 7) => "Calendar.month_short.July",
        (true, 8) => "Calendar.month_short.August",
        (true, 9) => "Calendar.month_short.September",
        (true, 10) => "Calendar.month_short.October",
        (true, 11) => "Calendar.month_short.November",
        (true, 12) => "Calendar.month_short.December",
        _ => unreachable!("chrono months are always in 1..=12"),
    }
}

/// Resolves a month name from the shared Calendar translations.
fn localized_month(month: u32, short: bool, locale: &str) -> String {
    let key = month_translation_key(month, short);
    crate::_rust_i18n_try_translate(translation_locale(locale), key)
        .or_else(|| crate::_rust_i18n_try_translate("en", key))
        .map(|value| value.into_owned())
        .unwrap_or_else(|| month.to_string())
}

/// Returns the English ordinal suffix used by the shadcn single-date format.
fn english_ordinal(day: u32) -> &'static str {
    if (11..=13).contains(&(day % 100)) {
        "th"
    } else {
        match day % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    }
}

/// Formats one endpoint using the locale order and shadcn's long/short month intent.
fn format_localized_endpoint(date: NaiveDate, locale: &str, style: LocalizedDateStyle) -> String {
    match locale_kind(locale) {
        LocaleKind::Chinese => format!("{}年{}月{}日", date.year(), date.month(), date.day()),
        LocaleKind::French | LocaleKind::Italian => {
            let month = localized_month(
                date.month(),
                matches!(style, LocalizedDateStyle::Range),
                locale,
            );
            format!("{} {} {}", date.day(), month, date.year())
        }
        LocaleKind::English => {
            let short = matches!(style, LocalizedDateStyle::Range);
            let month = localized_month(date.month(), short, "en");
            if short {
                format!("{} {:02}, {}", month, date.day(), date.year())
            } else {
                format!(
                    "{} {}{}, {}",
                    month,
                    date.day(),
                    english_ordinal(date.day()),
                    date.year()
                )
            }
        }
    }
}

/// Formats a Date with an explicit Chrono format, including an incomplete range start.
fn format_custom_date(date: Date, format: &str) -> Option<SharedString> {
    match date {
        Date::Single(Some(date)) => Some(date.format(format).to_string().into()),
        Date::Range(Some(start), Some(end)) => {
            Some(format!("{} - {}", start.format(format), end.format(format)).into())
        }
        Date::Range(Some(start), None) => Some(start.format(format).to_string().into()),
        _ => None,
    }
}

/// Formats the current value from the active locale unless the caller overrides the format.
fn format_date_display(
    date: Date,
    date_format: Option<&str>,
    locale: &str,
) -> Option<SharedString> {
    if let Some(format) = date_format {
        return format_custom_date(date, format);
    }

    match date {
        Date::Single(Some(date)) => {
            Some(format_localized_endpoint(date, locale, LocalizedDateStyle::Single).into())
        }
        Date::Range(Some(start), Some(end)) => Some(
            format!(
                "{} - {}",
                format_localized_endpoint(start, locale, LocalizedDateStyle::Range),
                format_localized_endpoint(end, locale, LocalizedDateStyle::Range)
            )
            .into(),
        ),
        Date::Range(Some(start), None) => {
            Some(format_localized_endpoint(start, locale, LocalizedDateStyle::Range).into())
        }
        _ => None,
    }
}

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("space", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("delete", Delete, Some(CONTEXT)),
        KeyBinding::new("backspace", Delete, Some(CONTEXT)),
    ])
}

/// Events emitted by the DatePicker.
#[derive(Clone)]
pub enum DatePickerEvent {
    Change(Date),
}

/// Preset value for DateRangePreset.
#[derive(Clone)]
pub enum DateRangePresetValue {
    Single(NaiveDate),
    Range(NaiveDate, NaiveDate),
}

/// Preset for date range selection.
#[derive(Clone)]
pub struct DateRangePreset {
    label: SharedString,
    value: DateRangePresetValue,
}

impl DateRangePreset {
    /// Creates a new DateRangePreset with a date.
    pub fn single(label: impl Into<SharedString>, date: NaiveDate) -> Self {
        DateRangePreset {
            label: label.into(),
            value: DateRangePresetValue::Single(date),
        }
    }
    /// Creates a new DateRangePreset with a range of dates.
    pub fn range(label: impl Into<SharedString>, start: NaiveDate, end: NaiveDate) -> Self {
        DateRangePreset {
            label: label.into(),
            value: DateRangePresetValue::Range(start, end),
        }
    }
}

/// Use to store the state of the date picker.
pub struct DatePickerState {
    focus_handle: FocusHandle,
    date: Date,
    open: bool,
    calendar: Entity<CalendarState>,
    date_format: Option<SharedString>,
    number_of_months: usize,
    disabled_matcher: Option<Rc<Matcher>>,
    _subscriptions: Vec<Subscription>,
}

impl Focusable for DatePickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<DatePickerEvent> for DatePickerState {}

impl DatePickerState {
    /// Create a date state.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_range(false, window, cx)
    }

    /// Create a date state with range mode.
    pub fn range(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_range(true, window, cx)
    }

    fn new_with_range(is_range: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date = if is_range {
            Date::Range(None, None)
        } else {
            Date::Single(None)
        };

        let calendar = cx.new(|cx| {
            let mut this = CalendarState::new(window, cx);
            this.set_date(date, window, cx);
            this
        });

        let _subscriptions = vec![cx.subscribe_in(
            &calendar,
            window,
            |this, _, ev: &CalendarEvent, window, cx| match ev {
                CalendarEvent::Selected(date) => {
                    this.update_date(*date, true, window, cx);
                }
            },
        )];

        Self {
            focus_handle: cx.focus_handle(),
            date,
            calendar,
            open: false,
            date_format: None,
            number_of_months: 1,
            disabled_matcher: None,
            _subscriptions,
        }
    }

    /// Sets a Chrono format override for the displayed date.
    ///
    /// Without an override, DatePicker follows the active application locale.
    pub fn date_format(mut self, format: impl Into<SharedString>) -> Self {
        self.date_format = Some(format.into());
        self
    }

    /// Set the number of months calendar view to display, default is 1.
    pub fn number_of_months(mut self, number_of_months: usize) -> Self {
        self.number_of_months = number_of_months;
        self
    }

    /// Get the date of the date picker.
    pub fn date(&self) -> Date {
        self.date
    }

    /// Set the date of the date picker.
    pub fn set_date(&mut self, date: impl Into<Date>, window: &mut Window, cx: &mut Context<Self>) {
        self.update_date(date.into(), false, window, cx);
    }

    /// Set the disabled match for the calendar.
    pub fn disabled_matcher(mut self, disabled: impl Into<Matcher>) -> Self {
        self.disabled_matcher = Some(Rc::new(disabled.into()));
        self
    }

    /// Set the year range for the internal calendar.
    ///
    /// Default is 50 years before and after the current year.
    /// `range` uses a half-open interval `(start, end)` where `end` is exclusive.
    pub fn set_year_range(&mut self, range: (i32, i32), cx: &mut Context<Self>) {
        self.calendar.update(cx, |state, cx| {
            state.set_year_range(range, cx);
        });
    }

    fn update_date(&mut self, date: Date, emit: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.date = date;
        self.calendar.update(cx, |view, cx| {
            view.set_date(date, window, cx);
        });
        if emit {
            cx.emit(DatePickerEvent::Change(date));
        }
        cx.notify();
    }

    /// Set the disabled matcher of the date picker.
    fn set_canlendar_disabled_matcher(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let matcher = self.disabled_matcher.clone();
        self.calendar.update(cx, |state, _| {
            state.disabled_matcher = matcher;
        });
    }

    fn on_escape(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            cx.propagate();
            return;
        }
        self.open = false;
        cx.notify();
    }

    fn on_delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        self.clean(&ClickEvent::default(), window, cx);
    }

    /// Opens the calendar from the focused Trigger without toggling on key repeat.
    fn on_confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.set_open(true, cx);
        }
    }

    fn clean(&mut self, _: &gpui::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        match self.date {
            Date::Single(_) => {
                self.update_date(Date::Single(None), true, window, cx);
            }
            Date::Range(_, _) => {
                self.update_date(Date::Range(None, None), true, window, cx);
            }
        }
    }

    /// Synchronizes the controlled Popover state with the DatePicker state.
    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.open == open {
            return;
        }
        if open && let Date::Range(Some(start), _) = self.date {
            self.calendar.update(cx, |calendar, cx| {
                calendar.reset_view_to_date(start, cx);
            });
        }
        self.open = open;
        cx.notify();
    }

    fn select_preset(
        &mut self,
        preset: &DateRangePreset,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match preset.value {
            DateRangePresetValue::Single(single) => {
                self.update_date(Date::Single(Some(single)), true, window, cx)
            }
            DateRangePresetValue::Range(start, end) => {
                self.update_date(Date::Range(Some(start), Some(end)), true, window, cx)
            }
        }
    }
}

/// A DatePicker element.
#[derive(IntoElement)]
pub struct DatePicker {
    id: ElementId,
    style: StyleRefinement,
    state: Entity<DatePickerState>,
    cleanable: bool,
    placeholder: Option<SharedString>,
    size: Size,
    number_of_months: Option<usize>,
    presets: Option<Vec<DateRangePreset>>,
    appearance: bool,
    disabled: bool,
    week_starts_on: Weekday,
    calendar_animated: bool,
}

impl Sizable for DatePicker {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Focusable for DatePicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for DatePicker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for DatePicker {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Render for DatePickerState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui::IntoElement {
        Empty
    }
}

impl DatePicker {
    /// Create a new DatePicker with the given [`DatePickerState`].
    pub fn new(state: &Entity<DatePickerState>) -> Self {
        Self {
            id: ("date-picker", state.entity_id()).into(),
            state: state.clone(),
            cleanable: false,
            placeholder: None,
            size: Size::default(),
            style: StyleRefinement::default(),
            number_of_months: None,
            presets: None,
            appearance: true,
            disabled: false,
            week_starts_on: Weekday::Sun,
            calendar_animated: false,
        }
    }

    /// Set the placeholder of the date picker, default: "".
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set whether to show the clear button when the input field is not empty, default is false.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }

    /// Set preset ranges for the date picker.
    pub fn presets(mut self, presets: Vec<DateRangePreset>) -> Self {
        self.presets = Some(presets);
        self
    }

    /// Set number of months to display in the calendar.
    ///
    /// When omitted, the value configured on [`DatePickerState`] is used (default: 1).
    pub fn number_of_months(mut self, number_of_months: usize) -> Self {
        self.number_of_months = Some(number_of_months.max(1));
        self
    }

    /// Set the first weekday used by the embedded Calendar.
    pub fn week_starts_on(mut self, week_starts_on: Weekday) -> Self {
        self.week_starts_on = week_starts_on;
        self
    }

    /// Enable month transition motion for the embedded Calendar.
    pub fn calendar_animated(mut self, animated: bool) -> Self {
        self.calendar_animated = animated;
        self
    }

    /// Set appearance of the date picker, if false, the date picker will be in a minimal style.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }
}

impl RenderOnce for DatePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state.update(cx, |state, cx| {
            state.set_canlendar_disabled_matcher(window, cx);
            if self.disabled {
                state.set_open(false, cx);
            }
        });

        let state = self.state.read(cx);
        let show_clean = self.cleanable && state.date.is_some();
        let placeholder = self
            .placeholder
            .clone()
            .unwrap_or_else(|| t!("DatePicker.placeholder").into());
        let locale = crate::locale();
        let display_title =
            format_date_display(state.date, state.date_format.as_deref(), locale.as_ref())
                .unwrap_or(placeholder.clone());
        let open = state.open && !self.disabled;
        let has_date = state.date.is_some();
        let number_of_months = self
            .number_of_months
            .unwrap_or(state.number_of_months)
            .max(1);
        let calendar = state.calendar.clone();
        let calendar_focus = calendar.focus_handle(cx);

        let mut trigger_style_element = div().w_full();
        trigger_style_element = trigger_style_element.refine_style(&self.style);
        let trigger_style = trigger_style_element.style().clone();
        let button_style = self.style.clone();
        let trigger_state = self.state.clone();
        let trigger_focus = self.focus_handle(cx);
        let trigger_id = self.id.clone();
        let disabled = self.disabled;
        let appearance = self.appearance;
        let size = self.size;

        let content_state = self.state.clone();
        let presets = self.presets.clone();
        let week_starts_on = self.week_starts_on;
        let calendar_animated = self.calendar_animated;

        Popover::new(self.id.clone())
            .align(PopoverAlign::Start)
            .open(open)
            .appearance(false)
            .trigger_style(trigger_style)
            .track_focus(&calendar_focus)
            .aria_label(t!("DatePicker.placeholder"))
            .on_open_change({
                let state = self.state.clone();
                move |open, _, cx| {
                    state.update(cx, |state, cx| state.set_open(*open && !disabled, cx));
                }
            })
            .trigger_builder(move |expanded, window, cx| {
                let trigger =
                    Button::new(("date-picker-trigger", trigger_state.entity_id()))
                        .outline()
                        .with_size(size)
                        .focus_handle(trigger_focus)
                        .aria_label(display_title.clone())
                        .aria_expanded(expanded)
                        .disabled(disabled)
                        .font_normal()
                        .w_full()
                        .refine_style(&button_style)
                        .icon(IconName::Calendar)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_left()
                                .when(!has_date, |this| {
                                    this.text_color(cx.theme().muted_foreground)
                                })
                                .child(display_title.clone()),
                        )
                        .when(!disabled && show_clean, |this| {
                            this.child(clear_button(cx).on_click(
                                window.listener_for(&trigger_state, DatePickerState::clean),
                            ))
                        })
                        .when(!appearance, |this| this.ghost().shadow_none());

                div()
                    .id((trigger_id.clone(), "control"))
                    .key_context(CONTEXT)
                    .when(disabled, |this| {
                        this.on_click(|_, _, cx| cx.stop_propagation())
                    })
                    .when(!disabled, |this| {
                        this.on_action(
                            window.listener_for(&trigger_state, DatePickerState::on_confirm),
                        )
                        .on_action(window.listener_for(&trigger_state, DatePickerState::on_delete))
                    })
                    .on_action(window.listener_for(&trigger_state, DatePickerState::on_escape))
                    .child(trigger)
                    .into_any_element()
            })
            .content(move |_, window, cx| {
                let radius = cx.theme().style.radii.md;
                let elevation = cx.theme().style.elevation.enabled;
                let overlay_padding = cx.theme().style.overlays.padding;
                let overlay_gap = cx.theme().style.overlays.gap;
                let surface = Popover::render_popover_content(false, window, cx)
                    .id("date-picker-content")
                    .role(Role::Dialog)
                    .aria_label(t!("DatePicker.placeholder"))
                    .w_auto()
                    .p_0()
                    .gap_0()
                    .rounded(radius)
                    .bg(cx.theme().tokens.popover)
                    .text_color(cx.theme().popover_foreground)
                    .border_1()
                    .border_color(cx.theme().foreground.opacity(0.1))
                    .when(elevation, |this| this.shadow_md())
                    .child(
                        h_flex()
                            .gap_0()
                            .h_full()
                            .items_stretch()
                            .when_some(presets.clone(), |this, presets| {
                                this.child(
                                    v_flex()
                                        .self_stretch()
                                        .p(overlay_padding)
                                        .gap(overlay_gap)
                                        .border_r_1()
                                        .border_color(cx.theme().border)
                                        .children(presets.into_iter().enumerate().map(
                                            |(index, preset)| {
                                                Button::new(("date-picker-preset", index))
                                                    .small()
                                                    .ghost()
                                                    .tab_stop(false)
                                                    .label(preset.label.clone())
                                                    .on_click(window.listener_for(
                                                        &content_state,
                                                        move |state, _, window, cx| {
                                                            state
                                                                .select_preset(&preset, window, cx);
                                                        },
                                                    ))
                                            },
                                        )),
                                )
                            })
                            .child(
                                Calendar::new(&calendar)
                                    .number_of_months(number_of_months)
                                    .week_starts_on(week_starts_on)
                                    .animated(calendar_animated)
                                    .border_0()
                                    .rounded_none()
                                    .with_size(size),
                            ),
                    );

                surface
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    fn display(date: Date, date_format: Option<&str>, locale: &str) -> String {
        format_date_display(date, date_format, locale)
            .expect("the test date should produce display text")
            .to_string()
    }

    #[test]
    fn formats_english_single_date_like_shadcn() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 16).unwrap();

        assert_eq!(
            display(Date::Single(Some(date)), None, "en"),
            "September 16th, 2026"
        );
    }

    #[test]
    fn formats_english_range_with_compact_endpoints() {
        let start = NaiveDate::from_ymd_opt(2025, 9, 28).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 10, 15).unwrap();

        assert_eq!(
            display(Date::Range(Some(start), Some(end)), None, "en-US"),
            "Sep 28, 2025 - Oct 15, 2025"
        );
    }

    #[test]
    fn formats_english_ordinal_boundaries() {
        let expectations = [
            (1, "1st"),
            (2, "2nd"),
            (3, "3rd"),
            (4, "4th"),
            (11, "11th"),
            (12, "12th"),
            (13, "13th"),
            (21, "21st"),
            (22, "22nd"),
            (23, "23rd"),
        ];

        for (day, expected) in expectations {
            assert_eq!(format!("{day}{}", english_ordinal(day)), expected);
        }
    }

    #[test]
    fn formats_chinese_dates_in_local_order() {
        let start = NaiveDate::from_ymd_opt(2025, 9, 28).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 10, 15).unwrap();

        assert_eq!(
            display(Date::Range(Some(start), Some(end)), None, "zh-CN"),
            "2025年9月28日 - 2025年10月15日"
        );
    }

    #[test]
    fn formats_locale_variants_with_catalog_month_names() {
        let date = Date::Single(Some(NaiveDate::from_ymd_opt(2026, 9, 16).unwrap()));

        assert_eq!(display(date, None, "it-IT"), "16 Settembre 2026");
        assert_eq!(display(date, None, "zh-Hant-TW"), "2026年9月16日");
    }

    #[test]
    fn displays_incomplete_range_start() {
        let start = NaiveDate::from_ymd_opt(2025, 9, 28).unwrap();

        assert_eq!(
            display(Date::Range(Some(start), None), None, "en"),
            "Sep 28, 2025"
        );
    }

    #[test]
    fn unknown_locale_falls_back_to_english() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 16).unwrap();

        assert_eq!(
            display(Date::Single(Some(date)), None, "unknown"),
            "September 16th, 2026"
        );
    }

    #[test]
    fn explicit_date_format_overrides_locale() {
        let start = NaiveDate::from_ymd_opt(2025, 9, 28).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 10, 15).unwrap();

        assert_eq!(
            display(
                Date::Range(Some(start), Some(end)),
                Some("%Y/%m/%d"),
                "zh-CN"
            ),
            "2025/09/28 - 2025/10/15"
        );
    }

    #[test]
    fn locale_is_resolved_for_each_display_render() {
        let date = Date::Single(Some(NaiveDate::from_ymd_opt(2026, 9, 16).unwrap()));

        assert_eq!(display(date, None, "en"), "September 16th, 2026");
        assert_eq!(display(date, None, "zh-TW"), "2026年9月16日");
    }

    #[gpui::test]
    fn date_picker_uses_single_month_state_default(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let state = cx.new(|cx| DatePickerState::new(window, cx));
            let picker = DatePicker::new(&state);

            assert_eq!(state.read(cx).number_of_months, 1);
            assert_eq!(picker.number_of_months, None);
            assert_eq!(
                DatePicker::new(&state).number_of_months(2).number_of_months,
                Some(2)
            );
        });
    }

    #[gpui::test]
    fn keyboard_open_does_not_toggle_on_repeat(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let state = cx.new(|cx| DatePickerState::new(window, cx));
            state.update(cx, |state, cx| {
                state.on_confirm(&Confirm { secondary: false }, window, cx)
            });
            assert!(state.read(cx).open);

            state.update(cx, |state, cx| {
                state.on_confirm(&Confirm { secondary: false }, window, cx)
            });
            assert!(state.read(cx).open);

            state.update(cx, |state, cx| state.set_open(false, cx));
            assert!(!state.read(cx).open);
        });
    }

    #[gpui::test]
    fn range_reopens_from_its_start_month(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let start = NaiveDate::from_ymd_opt(2026, 7, 26).unwrap();
            let end = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
            let state = cx.new(|cx| DatePickerState::range(window, cx));

            state.update(cx, |state, cx| {
                state.set_date(Date::Range(Some(start), Some(end)), window, cx);
            });
            let calendar = state.read(cx).calendar.clone();
            assert_eq!(calendar.read(cx).visible_month(), (2026, 9));
            assert_eq!(calendar.read(cx).active_date(), end);

            state.update(cx, |state, cx| state.set_open(true, cx));
            assert_eq!(calendar.read(cx).visible_month(), (2026, 7));
            assert_eq!(calendar.read(cx).active_date(), start);
        });
    }

    #[gpui::test]
    fn incomplete_range_reopens_from_its_start_month(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let start = NaiveDate::from_ymd_opt(2026, 7, 26).unwrap();
            let state = cx.new(|cx| DatePickerState::range(window, cx));

            state.update(cx, |state, cx| {
                state.set_date(Date::Range(Some(start), None), window, cx);
                state.set_open(true, cx);
            });
            let calendar = state.read(cx).calendar.clone();
            assert_eq!(calendar.read(cx).visible_month(), (2026, 7));
            assert_eq!(calendar.read(cx).active_date(), start);
        });
    }

    #[gpui::test]
    fn selecting_dates_and_presets_keeps_the_popover_open(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let selected = NaiveDate::from_ymd_opt(2026, 8, 12).unwrap();
            let preset = DateRangePreset::single("Today", selected);
            let state = cx.new(|cx| DatePickerState::new(window, cx));

            state.update(cx, |state, cx| {
                state.set_open(true, cx);
                state.update_date(Date::Single(Some(selected)), true, window, cx);
            });
            assert!(state.read(cx).open);

            state.update(cx, |state, cx| {
                state.select_preset(&preset, window, cx);
            });
            assert!(state.read(cx).open);
        });
    }
}
