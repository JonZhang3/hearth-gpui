use std::{path::PathBuf, sync::Arc};

use anyhow::{Context as _, Result};
use chrono::NaiveDate;
use gpui::{
    AppContext as _, Context, Entity, HeadlessAppContext, IntoElement, ParentElement as _, Render,
    Styled as _, Window, div, px, size,
};
use gpui_component::{
    ActiveTheme as _, StyledExt as _, Theme, ThemeMode,
    calendar::{Calendar, CalendarState},
    date_picker::{DatePicker, DatePickerState},
    form::field,
    h_flex,
    input::{Input, InputState},
    v_flex,
};
use gpui_component_assets::Assets;

const CAPTURE_WIDTH: f32 = 1000.;
const CAPTURE_HEIGHT: f32 = 700.;
const SHADCN_REVISION: &str = "607e8a9717fe6ff0d374ba74c651012f9c052534";

#[derive(Clone, Copy)]
struct LocaleCopy {
    locale: &'static str,
    title: &'static str,
    field_label: &'static str,
    description: &'static str,
    error: &'static str,
}

/// Fixed locale-sensitive Calendar and Form layout.
struct LocaleCaptureRoot {
    copy: LocaleCopy,
    calendar: Entity<CalendarState>,
    date_picker: Entity<DatePickerState>,
    input: Entity<InputState>,
}

impl LocaleCaptureRoot {
    /// Creates deterministic component state at a fixed date.
    fn new(copy: LocaleCopy, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date = NaiveDate::from_ymd_opt(2026, 8, 6).unwrap();
        let calendar = cx.new(|cx| {
            let mut state = CalendarState::new(window, cx).disabled_matcher(vec![0, 6]);
            state.set_date(date, window, cx);
            state
        });
        let date_picker = cx.new(|cx| {
            let mut state = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            state.set_date(date, window, cx);
            state
        });
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("invalid@example")
                .placeholder("name@example.com")
        });

        Self {
            copy,
            calendar,
            date_picker,
            input,
        }
    }
}

impl Render for LocaleCaptureRoot {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .size_full()
            .gap_5()
            .p_6()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(div().text_2xl().font_bold().child(self.copy.title))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(format!(
                        "locale: {} · Vega · shadcn/ui {} · 1000 x 700",
                        self.copy.locale, SHADCN_REVISION
                    )),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_start()
                    .gap_6()
                    .child(Calendar::new(&self.calendar))
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_4()
                            .child(DatePicker::new(&self.date_picker).cleanable(true).w_full())
                            .child(
                                field()
                                    .label(self.copy.field_label)
                                    .description(self.copy.description)
                                    .required(true)
                                    .error(self.copy.error)
                                    .child(Input::new(&self.input).invalid(true)),
                            ),
                    ),
            )
    }
}

/// Captures one locale after resolving all component layout and text runs.
fn capture_locale(
    cx: &mut HeadlessAppContext,
    output_dir: &PathBuf,
    copy: LocaleCopy,
) -> Result<()> {
    gpui_component::set_locale(copy.locale);
    cx.update(|cx| {
        Theme::change(ThemeMode::Light, None, cx);
        Theme::set_style("vega", cx)
    })?;

    let window = cx.open_window(
        size(px(CAPTURE_WIDTH), px(CAPTURE_HEIGHT)),
        move |window, cx| cx.new(|cx| LocaleCaptureRoot::new(copy, window, cx)),
    )?;
    let window = window.into();
    for _ in 0..3 {
        cx.update_window(window, |_, window, cx| {
            let _ = window.draw(cx);
        })?;
        cx.run_until_parked();
    }

    let path = output_dir.join(format!("locale-{}.png", copy.locale));
    cx.capture_screenshot(window)?
        .save(&path)
        .with_context(|| format!("failed to save {}", path.display()))?;
    Ok(())
}

fn main() -> Result<()> {
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(Assets),
        gpui_platform::current_headless_renderer,
    );
    cx.update(|cx| {
        gpui_component::init(cx);
        cx.set_reduce_motion(true);
    });

    let output_dir = PathBuf::from("docs/shadcn/screenshots/locales");
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    for copy in [
        LocaleCopy {
            locale: "en",
            title: "Locale layout reference",
            field_label: "Account email address",
            description: "Used for security notices and account recovery.",
            error: "Enter a complete and valid email address.",
        },
        LocaleCopy {
            locale: "zh-CN",
            title: "简体中文布局参考",
            field_label: "账户电子邮箱地址",
            description: "用于接收安全通知以及恢复账户访问权限。",
            error: "请输入完整且有效的电子邮箱地址。",
        },
        LocaleCopy {
            locale: "zh-TW",
            title: "繁體中文佈局參考",
            field_label: "帳戶電子郵件地址",
            description: "用於接收安全通知以及恢復帳戶存取權限。",
            error: "請輸入完整且有效的電子郵件地址。",
        },
    ] {
        capture_locale(&mut cx, &output_dir, copy)?;
    }

    println!("Captured locale references in {}", output_dir.display());
    Ok(())
}
