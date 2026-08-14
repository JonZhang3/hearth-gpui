use gpui::{
    Div, Hsla, InteractiveElement as _, Pixels, Size, Styled as _, div, prelude::FluentBuilder as _,
};

/// Creates the shared full-window surface that owns modal occlusion and backdrop paint.
pub(crate) fn modal_overlay(size: Size<Pixels>, background: Option<Hsla>) -> Div {
    div()
        .occlude()
        .w(size.width)
        .h(size.height)
        .when_some(background, |this, background| this.bg(background))
}
