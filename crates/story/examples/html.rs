use gpui::*;
use hearth_gpui::{
    ActiveTheme as _,
    highlighter::Language,
    input::{Input, InputState, TabSize},
    resizable::h_resizable,
    scroll::ScrollableElement as _,
    text::html,
};
use hearth_gpui_assets::Assets;

pub struct Example {
    input_state: Entity<InputState>,
    scroll_handle: ScrollHandle,
    _subscribe: Subscription,
}

const EXAMPLE: &str = include_str!("./fixtures/test.html");

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Language::Html)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .default_value(EXAMPLE)
                .placeholder("Enter your HTML here...")
        });

        let _subscribe = cx.subscribe(
            &input_state,
            |_, _, _: &hearth_gpui::input::InputEvent, cx| {
                cx.notify();
            },
        );

        Self {
            input_state,
            scroll_handle: ScrollHandle::new(),
            _subscribe,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_resizable("container")
            .child(
                div()
                    .id("source")
                    .size_full()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(cx.theme().mono_font_size)
                    .child(
                        Input::new(&self.input_state)
                            .h_full()
                            .appearance(false)
                            .focus_bordered(false),
                    )
                    .into_any(),
            )
            .child(
                div()
                    .relative()
                    .size_full()
                    .child(
                        div()
                            .id("html-preview-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                            .child(
                                html(self.input_state.read(cx).value().clone())
                                    .scroll_handle(self.scroll_handle.clone())
                                    .p_5()
                                    .selectable(true),
                            ),
                    )
                    .vertical_scrollbar(&self.scroll_handle)
                    .into_any(),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx| {
        hearth_gpui_story::init(cx);
        cx.activate(true);

        hearth_gpui_story::create_new_window("HTML Render (native)", Example::view, cx);
    });
}
