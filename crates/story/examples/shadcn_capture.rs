use std::{path::PathBuf, sync::Arc};

use anyhow::{Context as _, Result};
use gpui::{
    AnyView, AppContext as _, Context, HeadlessAppContext, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window,
    div, point, px, size,
};
use hearth_gpui::{ActiveTheme as _, StyledExt as _, Theme, ThemeMode};
use hearth_gpui_assets::Assets;
use hearth_gpui_story::{ShadcnAlignmentStory, Story};

const CAPTURE_WIDTH: f32 = 1440.0;
const CAPTURE_HEIGHT: f32 = 1000.0;
const PAGE_STEP: f32 = 820.0;
const SHADCN_REVISION: &str = "607e8a9717fe6ff0d374ba74c651012f9c052534";

/// Fixed chrome keeps capture identity visible while the component matrix scrolls.
struct CaptureRoot {
    story: AnyView,
    scroll_handle: ScrollHandle,
}

impl Render for CaptureRoot {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let metadata = format!(
            "Color Theme: {}  ·  Style Preset: {} ({})  ·  shadcn/ui: {}",
            theme.theme_name(),
            theme.style.name,
            theme.style.id,
            SHADCN_REVISION
        );

        div()
            .size_full()
            .relative()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                div()
                    .id("shadcn-capture-scroll")
                    .absolute()
                    .top(px(72.0))
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h(px(CAPTURE_HEIGHT - 72.0))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .child(div().p_8().child(self.story.clone())),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(72.0))
                    .px_8()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .bg(theme.background)
                    .border_b_1()
                    .border_color(theme.border)
                    .child(div().font_semibold().child("Shadcn Alignment"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(metadata),
                    ),
            )
    }
}

/// Draws the current window state before querying layout or capturing pixels.
fn draw_window(cx: &mut HeadlessAppContext, window: gpui::AnyWindowHandle) -> Result<()> {
    cx.update_window(window, |_, window, cx| {
        let _ = window.draw(cx);
    })?;
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        let _ = window.draw(cx);
    })?;
    Ok(())
}

/// Captures every scroll page for one independent Color Theme and Style pair.
fn capture_variant(
    cx: &mut HeadlessAppContext,
    window: gpui::AnyWindowHandle,
    scroll_handle: &ScrollHandle,
    output_dir: &PathBuf,
    mode: ThemeMode,
    style_id: &str,
) -> Result<usize> {
    cx.update(|cx| {
        Theme::change(mode, None, cx);
        Theme::set_style(style_id, cx)
    })?;

    scroll_handle.set_offset(point(px(0.0), px(0.0)));
    draw_window(cx, window)?;

    let max_scroll = scroll_handle.max_offset().y.max(px(0.0)).as_f32();
    let page_count = (max_scroll / PAGE_STEP).ceil() as usize + 1;
    let theme_id = match mode {
        ThemeMode::Light => "default-light",
        ThemeMode::Dark => "default-dark",
    };

    for page in 0..page_count {
        let scroll_y = (page as f32 * PAGE_STEP).min(max_scroll);
        scroll_handle.set_offset(point(px(0.0), px(-scroll_y)));
        draw_window(cx, window)?;

        let image = cx.capture_screenshot(window)?;
        let path = output_dir.join(format!("{theme_id}-{style_id}-page-{:02}.png", page + 1));
        image
            .save(&path)
            .with_context(|| format!("failed to save {}", path.display()))?;
    }

    Ok(page_count)
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
        cx.set_reduce_motion(true);
    });

    let scroll_handle = ScrollHandle::new();
    let root_scroll_handle = scroll_handle.clone();
    let window = cx.open_window(size(px(CAPTURE_WIDTH), px(CAPTURE_HEIGHT)), |window, cx| {
        let story = <ShadcnAlignmentStory as Story>::new_view(window, cx);
        cx.new(|_| CaptureRoot {
            story: story.into(),
            scroll_handle: root_scroll_handle,
        })
    })?;
    let window = window.into();

    let output_dir = PathBuf::from("docs/shadcn/screenshots");
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let mut total = 0;
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for style_id in ["vega", "nova", "maia"] {
            total += capture_variant(&mut cx, window, &scroll_handle, &output_dir, mode, style_id)?;
        }
    }

    println!(
        "Captured {total} fixed reference images in {}",
        output_dir.display()
    );
    Ok(())
}
