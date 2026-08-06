use gpui::App;
use rust_i18n::t;

use crate::{
    Icon, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
};

#[inline]
pub(crate) fn clear_button(_: &App) -> Button {
    Button::new("clean")
        .aria_label(t!("Common.Clear"))
        .icon(Icon::new(IconName::Close))
        .text()
        .xsmall()
        .tab_stop(false)
}
