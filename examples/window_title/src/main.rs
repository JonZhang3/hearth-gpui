// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Removed the legacy `primary` button variant and unused variant import.
// - Enabled application-owned title-bar dragging for the custom window.
use gpui::*;
use hearth_gpui::{Root, TitleBar, button::Button, h_flex, v_flex};

pub struct Example;
impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                // Render custom title bar on top of Root view.
                TitleBar::new().child(
                    h_flex()
                        .w_full()
                        .pr_2()
                        .justify_between()
                        .child("App with Custom title bar")
                        .child("Right Item"),
                ),
            )
            .child(
                div()
                    .id("window-body")
                    .p_5()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .child("Hello, World!")
                    .child(
                        Button::new("ok")
                            .label("Let's Go!")
                            .on_click(|_, _, _| println!("Clicked!")),
                    ),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(hearth_gpui_assets::Assets);

    app.run(move |cx| {
        hearth_gpui::init(cx);

        cx.spawn(async move |cx| {
            let window_options = WindowOptions {
                // Setup GPUI to use custom title bar
                titlebar: Some(TitleBar::title_bar_options()),
                app_owns_titlebar_drag: true,
                ..Default::default()
            };

            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|_| Example);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
