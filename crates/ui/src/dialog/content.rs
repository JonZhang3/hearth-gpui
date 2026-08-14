use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window,
};

use crate::{ActiveTheme as _, StyledExt as _, v_flex};

/// Content container for a dialog.
#[derive(IntoElement)]
pub struct DialogContent {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl DialogContent {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for DialogContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for DialogContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for DialogContent {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // Direct children are the Dialog's major content sections and share the preset modal gap.
        v_flex()
            .w_full()
            .flex_1()
            .gap(cx.theme().style.modals.gap)
            .refine_style(&self.style)
            .children(self.children)
    }
}
