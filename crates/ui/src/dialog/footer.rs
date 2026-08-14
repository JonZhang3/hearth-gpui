use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window,
    prelude::FluentBuilder as _, relative,
};

use crate::{
    ActiveTheme as _, StyledExt as _,
    button::Button,
    dialog::{CancelDialog, ConfirmDialog},
    h_flex,
};

/// Footer section of a dialog, typically contains action buttons.
///
/// # Examples
///
/// ```ignore
/// DialogFooter::new()
///     .child(DialogClose::new(Button::new("cancel").label("Cancel")))
///     .child(DialogAction::new(Button::new("confirm").label("Confirm")))
/// ```
#[derive(IntoElement)]
pub struct DialogFooter {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl DialogFooter {
    /// Creates an end-aligned action row using the active modal metrics.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for DialogFooter {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for DialogFooter {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for DialogFooter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = cx.theme().style.modals;

        h_flex()
            .gap_2()
            .justify_end()
            .line_height(relative(1.))
            .when(metrics.footer_separated, |this| {
                this.ml(-metrics.padding)
                    .mr(-metrics.padding)
                    .mb(-metrics.padding)
                    .p(metrics.footer_padding)
                    .border_t_1()
                    .border_color(cx.theme().border)
            })
            .when(metrics.footer_tinted, |this| {
                this.bg(cx.theme().muted.opacity(0.5))
                    .rounded_b(cx.theme().style.radii.xl)
            })
            .refine_style(&self.style)
            .children(self.children)
    }
}

#[derive(IntoElement)]
pub struct DialogClose {
    button: Button,
}

impl DialogClose {
    /// Creates a cancellation control without adding a layout wrapper.
    pub fn new(button: Button) -> Self {
        Self { button }
    }
}

impl RenderOnce for DialogClose {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.button.append_on_click(move |_, window, cx| {
            window.dispatch_action(Box::new(CancelDialog), cx)
        })
    }
}

#[derive(IntoElement)]
pub struct DialogAction {
    button: Button,
}

impl DialogAction {
    /// Creates a confirmation control without adding a layout wrapper.
    pub fn new(button: Button) -> Self {
        Self { button }
    }
}

impl RenderOnce for DialogAction {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.button.append_on_click(move |_, window, cx| {
            window.dispatch_action(Box::new(ConfirmDialog), cx)
        })
    }
}
