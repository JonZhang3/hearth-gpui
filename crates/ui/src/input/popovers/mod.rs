// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Updated input popover module exports after removing legacy hover styling.
mod code_action_menu;
mod completion_menu;
mod diagnostic_popover;
mod hover_popover;

pub(crate) use code_action_menu::*;
pub(crate) use completion_menu::*;
pub(crate) use diagnostic_popover::*;
pub(crate) use hover_popover::*;

use gpui::{
    App, Div, ElementId, Entity, InteractiveElement as _, IntoElement, SharedString, Stateful,
    StyleRefinement, Styled as _, Window, div, px, rems,
};

use crate::{
    ActiveTheme, StyledExt as _,
    text::{Markdown, MarkdownElement, MarkdownFont, MarkdownStyle},
};

pub(crate) enum ContextMenu {
    Completion(Entity<CompletionMenu>),
    CodeAction(Entity<CodeActionMenu>),
}

impl ContextMenu {
    pub(crate) fn is_open(&self, cx: &App) -> bool {
        match self {
            ContextMenu::Completion(menu) => menu.read(cx).is_open(),
            ContextMenu::CodeAction(menu) => menu.read(cx).is_open(),
        }
    }

    pub(crate) fn render(&self) -> impl IntoElement {
        match self {
            ContextMenu::Completion(menu) => menu.clone().into_any_element(),
            ContextMenu::CodeAction(menu) => menu.clone().into_any_element(),
        }
    }
}

pub(super) fn render_markdown(
    id: impl Into<ElementId>,
    markdown: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut App,
) -> MarkdownElement {
    let id = id.into();
    let source = markdown.into();
    let initial_source = source.clone();
    let state = window.use_keyed_state(id, cx, move |_, cx| Markdown::new(initial_source, cx));
    if state.read(cx).source() != source {
        state.update(cx, |markdown, cx| markdown.replace(source, cx));
    }

    let mut style = MarkdownStyle::themed(MarkdownFont::Editor, window, cx);
    style.base_text_style.font_size = px(12.).into();
    style.base_text_style.line_height = rems(1.35).into();
    style.code_block = StyleRefinement::default()
        .bg(cx.theme().transparent)
        .p_0()
        .text_size(px(11.));
    MarkdownElement::new(state, style)
}

pub(super) fn editor_popover(id: impl Into<ElementId>, cx: &App) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .occlude()
        .popover_style(cx)
        .text_xs()
        .p_1()
}
