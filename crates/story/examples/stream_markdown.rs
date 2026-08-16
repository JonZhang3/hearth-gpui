use gpui::*;
use hearth_gpui_assets::Assets;
use hearth_gpui_story::MarkdownStory;

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx| {
        hearth_gpui_story::init(cx);
        cx.activate(true);

        hearth_gpui_story::create_new_window_with_size(
            "Stream Markdown",
            Some(size(px(1040.), px(780.))),
            MarkdownStory::view,
            cx,
        );
    });
}
