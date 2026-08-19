use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement as _,
    ParentElement as _, Render, ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window,
    div, px,
};

use hearth_gpui::{
    dock::PanelControl,
    scroll::ScrollableElement as _,
    text::{Markdown, MarkdownElement, MarkdownFont, MarkdownStyle},
};

use crate::Story;

pub struct WelcomeStory {
    focus_handle: FocusHandle,
    scroll_handle: ScrollHandle,
    markdown: Entity<Markdown>,
}

impl WelcomeStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            scroll_handle: ScrollHandle::new(),
            markdown: cx.new(|cx| Markdown::new(include_str!("../../../../README.md"), cx)),
        }
    }
}

impl Story for WelcomeStory {
    fn title() -> &'static str {
        "Introduction"
    }

    fn description() -> &'static str {
        "UI components for building fantastic desktop application by using GPUI."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn paddings() -> gpui::Pixels {
        px(0.)
    }
}

impl Focusable for WelcomeStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WelcomeStory {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .relative()
            .size_full()
            .child(
                div()
                    .id("welcome-markdown-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .child(
                        MarkdownElement::new(
                            self.markdown.clone(),
                            MarkdownStyle::themed(MarkdownFont::Preview, window, cx),
                        )
                        .scroll_handle(self.scroll_handle.clone())
                        .px_4(),
                    ),
            )
            .vertical_scrollbar(&self.scroll_handle)
    }
}
