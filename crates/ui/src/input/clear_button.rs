use gpui::App;
use rust_i18n::t;

use crate::{Icon, IconName, Sizable as _, button::Button};

#[inline]
pub(crate) fn clear_button(_: &App) -> Button {
    Button::new("clean")
        .aria_label(t!("Common.Clear"))
        .icon(Icon::new(IconName::Close))
        .ghost()
        .xsmall()
        .tab_stop(false)
}
