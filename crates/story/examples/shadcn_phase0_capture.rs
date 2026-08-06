use std::{env, path::PathBuf, sync::Arc};

use anyhow::{Context as _, Result};
use gpui::{
    AppContext as _, Context, Entity, HeadlessAppContext, IntoElement, ParentElement as _, Render,
    Styled as _, Window, div, px, size,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IndexPath, StyledExt as _, Theme, ThemeMode,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    popover::Popover,
    radio::Radio,
    searchable_list::SearchableVec,
    select::{Select, SelectState},
    switch::Switch,
    v_flex,
};
use gpui_component_assets::Assets;

const CAPTURE_WIDTH: f32 = 1440.0;
const CAPTURE_HEIGHT: f32 = 1000.0;

/// Common-API state matrix that can be compiled in both the frozen baseline
/// and the current checkout without importing the new Style Preset API.
struct Phase0Capture {
    input: Entity<InputState>,
    disabled_input: Entity<InputState>,
    select: Entity<SelectState<SearchableVec<&'static str>>>,
    revision: String,
}

impl Render for Phase0Capture {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = if cx.theme().mode.is_dark() {
            "Default Dark"
        } else {
            "Default Light"
        };

        v_flex()
            .size_full()
            .p_8()
            .gap_6()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_2xl().font_semibold().child("Phase 0 baseline"))
                    .child(format!(
                        "GPUI Component: {} · Color Theme: {} · viewport: 1440 x 1000",
                        self.revision, mode
                    )),
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(div().text_lg().font_semibold().child("Button states"))
                    .child(
                        h_flex()
                            .gap_3()
                            .child(Button::new("phase0-default").label("Default"))
                            .child(Button::new("phase0-primary").primary().label("Primary"))
                            .child(Button::new("phase0-outline").outline().label("Outline"))
                            .child(
                                Button::new("phase0-disabled")
                                    .disabled(true)
                                    .label("Disabled"),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(div().text_lg().font_semibold().child("Selection states"))
                    .child(
                        h_flex()
                            .gap_5()
                            .child(Checkbox::new("phase0-checkbox-off").label("Unchecked"))
                            .child(
                                Checkbox::new("phase0-checkbox-on")
                                    .checked(true)
                                    .label("Checked"),
                            )
                            .child(Radio::new("phase0-radio-off").label("Radio"))
                            .child(
                                Radio::new("phase0-radio-on")
                                    .checked(true)
                                    .label("Selected"),
                            )
                            .child(Switch::new("phase0-switch").checked(true).label("Enabled")),
                    ),
            )
            .child(
                v_flex()
                    .w(px(560.))
                    .gap_3()
                    .child(div().text_lg().font_semibold().child("Input states"))
                    .child(Input::new(&self.input))
                    .child(Input::new(&self.disabled_input).disabled(true))
                    .child(div().w_full().h(px(44.)).child(Select::new(&self.select))),
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(div().text_lg().font_semibold().child("Open overlay"))
                    .child(
                        Popover::new("phase0-popover")
                            .default_open(true)
                            .trigger(Button::new("phase0-popover-trigger").label("Popover"))
                            .content(|_, _, _| {
                                div().w(px(280.)).child("Frozen default Popover surface")
                            }),
                    ),
            )
    }
}

/// Draws twice so keyed state and deferred overlay geometry settle before the
/// screenshot is captured.
fn draw_window(cx: &mut HeadlessAppContext, window: gpui::AnyWindowHandle) -> Result<()> {
    for _ in 0..2 {
        cx.update_window(window, |_, window, cx| {
            let _ = window.draw(cx);
        })?;
        cx.run_until_parked();
    }
    Ok(())
}

fn main() -> Result<()> {
    let revision = env::var("SHADCN_CAPTURE_REVISION").unwrap_or_else(|_| "working-tree".into());
    let output_dir = env::var_os("SHADCN_CAPTURE_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("docs/shadcn/screenshots/phase0"));
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(Assets),
        gpui_platform::current_headless_renderer,
    );
    cx.update(gpui_component::init);

    let capture_revision = revision.clone();
    let window = cx.open_window(
        size(px(CAPTURE_WIDTH), px(CAPTURE_HEIGHT)),
        move |window, cx| {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Input")
                    .default_value("Baseline input value")
            });
            let disabled_input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Disabled input")
                    .default_value("Unavailable")
            });
            let select = cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(vec!["Alpha", "Beta", "Gamma"]),
                    Some(IndexPath::new(0)),
                    window,
                    cx,
                )
            });
            cx.new(|_| Phase0Capture {
                input,
                disabled_input,
                select,
                revision: capture_revision,
            })
        },
    )?;
    let window = window.into();

    for (mode, name) in [
        (ThemeMode::Light, "phase0-light.png"),
        (ThemeMode::Dark, "phase0-dark.png"),
    ] {
        cx.update(|cx| Theme::change(mode, None, cx));
        draw_window(&mut cx, window)?;
        cx.capture_screenshot(window)?
            .save(output_dir.join(name))
            .with_context(|| format!("failed to save {name}"))?;
    }

    println!(
        "Captured Phase 0 {} references in {}",
        revision,
        output_dir.display()
    );
    Ok(())
}
