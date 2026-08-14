use std::{path::PathBuf, rc::Rc, sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use gpui::{
    Anchor, AppContext as _, Context, Entity, HeadlessAppContext, IntoElement, ParentElement as _,
    Render, Styled as _, Window, div, px, size,
};
use hearth_gpui::{
    ActiveTheme as _, StyledExt as _, Theme, ThemeMode, button::Button, popover::Popover,
};
use hearth_gpui_assets::Assets;

const CAPTURE_WIDTH: f32 = 960.;
const CAPTURE_HEIGHT: f32 = 640.;
const SHADCN_REVISION: &str = "607e8a9717fe6ff0d374ba74c651012f9c052534";

/// Deterministic Popover scene used for placement, edge, and closing references.
struct OverlayCaptureRoot {
    anchor: Anchor,
    label: &'static str,
    trigger_x: f32,
    trigger_y: f32,
    content_width: f32,
    open: bool,
}

#[derive(Clone, Copy)]
struct CaptureScene {
    mode: ThemeMode,
    anchor: Anchor,
    label: &'static str,
    trigger: (f32, f32),
    content_width: f32,
    capture_closing: bool,
}

impl Render for OverlayCaptureRoot {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let metadata = format!(
            "{} · {} · Vega · shadcn/ui {}",
            self.label,
            theme.theme_name(),
            SHADCN_REVISION
        );

        div()
            .size_full()
            .relative()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(64.))
                    .px_6()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(div().font_semibold().child("Overlay placement reference"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(metadata),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left(px(self.trigger_x))
                    .top(px(self.trigger_y))
                    .child(
                        Popover::new(format!("overlay-capture-{}", self.label))
                            .anchor(self.anchor)
                            .open(self.open)
                            .on_open_change(|_, _, _| {})
                            .trigger(
                                Button::new(format!("overlay-trigger-{}", self.label))
                                    .outline()
                                    .label(self.label),
                            )
                            .child(
                                div()
                                    .w(px(self.content_width))
                                    .child(format!("{} Popover content", self.label)),
                            ),
                    ),
            )
    }
}

/// Draws enough frames to resolve trigger bounds and reach the open resting state.
fn settle_window(cx: &mut HeadlessAppContext, window: gpui::AnyWindowHandle) -> Result<()> {
    for frame in 0..3 {
        cx.update_window(window, |_, window, cx| {
            let _ = window.draw(cx);
        })?;
        cx.run_until_parked();
        if frame == 0 {
            std::thread::sleep(Duration::from_millis(250));
        }
    }
    Ok(())
}

/// Captures one open scene and optionally an immediate closing frame.
fn capture_scene(
    cx: &mut HeadlessAppContext,
    output_dir: &PathBuf,
    scene: CaptureScene,
) -> Result<()> {
    cx.update(|cx| {
        Theme::change(scene.mode, None, cx);
        Theme::set_style("vega", cx)
    })?;

    let root_slot = Rc::new(std::cell::RefCell::new(None::<Entity<OverlayCaptureRoot>>));
    let root_slot_for_window = Rc::clone(&root_slot);
    let window = cx.open_window(size(px(CAPTURE_WIDTH), px(CAPTURE_HEIGHT)), move |_, cx| {
        let root = cx.new(|_| OverlayCaptureRoot {
            anchor: scene.anchor,
            label: scene.label,
            trigger_x: scene.trigger.0,
            trigger_y: scene.trigger.1,
            content_width: scene.content_width,
            open: true,
        });
        *root_slot_for_window.borrow_mut() = Some(root.clone());
        root
    })?;
    let window = window.into();
    settle_window(cx, window)?;

    let mode_id = if scene.mode == ThemeMode::Dark {
        "dark"
    } else {
        "light"
    };
    let open_path = output_dir.join(format!("popover-{}-{mode_id}-open.png", scene.label));
    cx.capture_screenshot(window)?
        .save(&open_path)
        .with_context(|| format!("failed to save {}", open_path.display()))?;

    if scene.capture_closing {
        let root = root_slot
            .borrow()
            .clone()
            .context("overlay capture root was not initialized")?;
        cx.update(|cx| {
            root.update(cx, |root, cx| {
                root.open = false;
                cx.notify();
            })
        });
        cx.update_window(window, |_, window, cx| {
            let _ = window.draw(cx);
        })?;

        let closing_path =
            output_dir.join(format!("popover-{}-{mode_id}-closing.png", scene.label));
        cx.capture_screenshot(window)?
            .save(&closing_path)
            .with_context(|| format!("failed to save {}", closing_path.display()))?;
    }

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
        hearth_gpui::init(cx);
        cx.set_reduce_motion(false);
    });

    let output_dir = PathBuf::from("docs/shadcn/screenshots/overlays");
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    for (anchor, label, trigger) in [
        (Anchor::BottomCenter, "top", (420., 250.)),
        (Anchor::TopCenter, "bottom", (420., 350.)),
        (Anchor::RightCenter, "left", (380., 290.)),
        (Anchor::LeftCenter, "right", (510., 290.)),
    ] {
        capture_scene(
            &mut cx,
            &output_dir,
            CaptureScene {
                mode: ThemeMode::Light,
                anchor,
                label,
                trigger,
                content_width: 220.,
                capture_closing: label == "right",
            },
        )?;
    }

    capture_scene(
        &mut cx,
        &output_dir,
        CaptureScene {
            mode: ThemeMode::Light,
            anchor: Anchor::TopLeft,
            label: "constrained-edge",
            trigger: (6., 70.),
            content_width: 360.,
            capture_closing: false,
        },
    )?;
    capture_scene(
        &mut cx,
        &output_dir,
        CaptureScene {
            mode: ThemeMode::Dark,
            anchor: Anchor::LeftCenter,
            label: "right",
            trigger: (510., 290.),
            content_width: 220.,
            capture_closing: true,
        },
    )?;

    println!("Captured overlay references in {}", output_dir.display());
    Ok(())
}
