use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, Empty, Hsla, InteractiveElement, IntoElement, ParentElement as _,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _, px, rems,
};

use crate::{
    ActiveTheme as _, Icon, IconName, Sizable, Size, StyledExt,
    text::{Text, TextViewStyle},
    theme::Density,
    v_flex,
};

/// The variant of the [`Alert`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AlertVariant {
    #[default]
    Default,
    Info,
    Success,
    Warning,
    Error,
}

impl AlertVariant {
    fn fg(&self, cx: &App) -> Hsla {
        match self {
            Self::Default => cx.theme().foreground,
            Self::Info => cx.theme().info,
            Self::Success => cx.theme().success,
            Self::Warning => cx.theme().warning,
            Self::Error => cx.theme().danger,
        }
    }

    fn description_fg(&self, cx: &App) -> Hsla {
        match self {
            Self::Default => cx.theme().muted_foreground,
            Self::Info => cx.theme().info.opacity(0.9),
            Self::Success => cx.theme().success.opacity(0.9),
            Self::Warning => cx.theme().warning.opacity(0.9),
            Self::Error => cx.theme().danger.opacity(0.9),
        }
    }
}

/// Alert used to display a message to the user.
#[derive(IntoElement)]
pub struct Alert {
    id: ElementId,
    style: StyleRefinement,
    variant: AlertVariant,
    icon: Icon,
    title: Option<SharedString>,
    message: Text,
    size: Size,
    banner: bool,
    on_close: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    visible: bool,
}

impl Alert {
    /// Create a new alert with the given message.
    pub fn new(id: impl Into<ElementId>, message: impl Into<Text>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            variant: AlertVariant::default(),
            icon: Icon::new(IconName::Info),
            title: None,
            message: message.into(),
            size: Size::default(),
            banner: false,
            visible: true,
            on_close: None,
        }
    }

    /// Create a new info [`AlertVariant::Info`] with the given message.
    pub fn info(id: impl Into<ElementId>, message: impl Into<Text>) -> Self {
        Self::new(id, message)
            .with_variant(AlertVariant::Info)
            .icon(IconName::Info)
    }

    /// Create a new [`AlertVariant::Success`] alert with the given message.
    pub fn success(id: impl Into<ElementId>, message: impl Into<Text>) -> Self {
        Self::new(id, message)
            .with_variant(AlertVariant::Success)
            .icon(IconName::CircleCheck)
    }

    /// Create a new [`AlertVariant::Warning`] alert with the given message.
    pub fn warning(id: impl Into<ElementId>, message: impl Into<Text>) -> Self {
        Self::new(id, message)
            .with_variant(AlertVariant::Warning)
            .icon(IconName::TriangleAlert)
    }

    /// Create a new [`AlertVariant::Error`] alert with the given message.
    pub fn error(id: impl Into<ElementId>, message: impl Into<Text>) -> Self {
        Self::new(id, message)
            .with_variant(AlertVariant::Error)
            .icon(IconName::CircleX)
    }

    /// Sets the [`AlertVariant`] of the alert.
    pub fn with_variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the icon for the alert.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Set the title for the alert.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set alert as banner style.
    ///
    /// The `banner` style will make the alert take the full width of the container and not border and radius.
    /// This mode will not display `title`.
    pub fn banner(mut self) -> Self {
        self.banner = true;
        self
    }

    /// Set the visibility of the alert.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set alert as closable, true will show Close icon.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Rc::new(on_close));
        self
    }
}

impl Sizable for Alert {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Alert {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Alert {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.visible {
            return Empty.into_any_element();
        }

        let (padding_x, padding_y, gap) = match self.size {
            Size::XSmall => (px(10.), px(6.), px(6.)),
            Size::Small => (px(12.), px(8.), px(8.)),
            Size::Large => (px(20.), px(14.), px(12.)),
            _ => match cx.theme().style.density {
                Density::Compact => (px(10.), px(8.), px(8.)),
                Density::Standard | Density::Comfortable => (px(16.), px(12.), px(10.)),
            },
        };

        let emphasis_fg = self.variant.fg(cx);
        let description_fg = self.variant.description_fg(cx);
        let closable = self.on_close.is_some();

        v_flex()
            .id(self.id)
            .role(Role::Alert)
            .relative()
            .w_full()
            .text_color(cx.theme().foreground)
            .bg(cx.theme().tokens.background)
            .px(padding_x)
            .py(padding_y)
            .when(closable, |this| this.pr(px(72.)))
            .text_sm()
            .border_1()
            .border_color(cx.theme().border)
            .when(!self.banner, |this| this.rounded(cx.theme().style.radii.lg))
            .refine_style(&self.style)
            .child(
                div()
                    .flex()
                    .items_start()
                    .when(self.banner, |this| this.items_center())
                    .overflow_hidden()
                    .gap(gap)
                    .child(
                        div()
                            .when(!self.banner, |this| this.mt(px(2.)))
                            .child(self.icon.text_color(emphasis_fg)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .overflow_hidden()
                            .gap(px(2.))
                            .when(!self.banner, |this| {
                                this.when_some(self.title, |this, title| {
                                    this.child(
                                        div()
                                            .w_full()
                                            .truncate()
                                            .font_medium()
                                            .text_color(emphasis_fg)
                                            .child(title),
                                    )
                                })
                            })
                            .child(
                                div().text_color(description_fg).child(
                                    self.message
                                        .style(TextViewStyle::default().paragraph_gap(rems(0.2))),
                                ),
                            ),
                    ),
            )
            .when_some(self.on_close, |this, on_close| {
                this.child(
                    div()
                        .id("close")
                        .absolute()
                        .top(px(10.))
                        .right(px(12.))
                        .p_0p5()
                        .rounded(cx.theme().style.radii.md)
                        .hover(|this| this.bg(cx.theme().accent))
                        .active(|this| this.bg(cx.theme().accent.opacity(0.8)))
                        .on_click(move |ev, window, cx| {
                            on_close(ev, window, cx);
                        })
                        .child(
                            Icon::new(IconName::Close)
                                .with_size(self.size.max(Size::Medium))
                                .flex_shrink_0(),
                        ),
                )
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn alert_variants_keep_neutral_surface_and_semantic_text(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|_, cx| {
            assert_eq!(AlertVariant::Default.fg(cx), cx.theme().foreground);
            assert_eq!(
                AlertVariant::Default.description_fg(cx),
                cx.theme().muted_foreground
            );
            assert_eq!(AlertVariant::Error.fg(cx), cx.theme().danger);
            assert_eq!(
                AlertVariant::Error.description_fg(cx),
                cx.theme().danger.opacity(0.9)
            );
        });
    }
}
