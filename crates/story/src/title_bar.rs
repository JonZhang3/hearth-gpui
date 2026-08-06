use std::rc::Rc;

use gpui::{
    Anchor, AnyElement, App, AppContext, Context, Entity, FocusHandle, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement as _, Render, SharedString, Styled as _, Subscription,
    Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, IconName, Side, Sizable as _, StyleRegistry, Theme, TitleBar, WindowExt as _,
    badge::Badge,
    button::Button,
    menu::{AppMenuBar, DropdownMenu as _},
    scroll::ScrollbarShow,
};

use crate::{SelectFont, SelectScrollbarShow, SelectStyle, ToggleListActiveHighlight, app_menus};

pub struct AppTitleBar {
    app_menu_bar: Entity<AppMenuBar>,
    font_size_selector: Entity<FontSizeSelector>,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    _subscriptions: Vec<Subscription>,
}

impl AppTitleBar {
    pub fn new(
        title: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let app_menu_bar = app_menus::init(title, cx);
        let font_size_selector = cx.new(|cx| FontSizeSelector::new(window, cx));

        Self {
            app_menu_bar,
            font_size_selector,
            child: Rc::new(|_, _| div().into_any_element()),
            _subscriptions: vec![],
        }
    }

    pub fn child<F, E>(mut self, f: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.child = Rc::new(move |window, cx| f(window, cx).into_any_element());
        self
    }
}

impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notifications_count = window.notifications(cx).len();

        TitleBar::new()
            // left side
            .child(div().flex().items_center().child(self.app_menu_bar.clone()))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child((self.child.clone())(window, cx))
                    .child(self.font_size_selector.clone())
                    .child(
                        Button::new("github")
                            .aria_label("GPUI Component GitHub repository")
                            .icon(IconName::Github)
                            .small()
                            .ghost()
                            .on_click(|_, _, cx| {
                                cx.open_url("https://github.com/longbridge/gpui-component")
                            }),
                    )
                    .child(
                        div().relative().child(
                            Badge::new().count(notifications_count).max(99).child(
                                Button::new("bell")
                                    .aria_label("Notifications")
                                    .small()
                                    .ghost()
                                    .icon(IconName::Bell),
                            ),
                        ),
                    ),
            )
    }
}

struct FontSizeSelector {
    focus_handle: FocusHandle,
}

impl FontSizeSelector {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_select_font(
        &mut self,
        font_size: &SelectFont,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).font_size = px(font_size.0 as f32);
        window.refresh();
    }

    fn on_select_style(
        &mut self,
        style: &SelectStyle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = Theme::set_style(&style.0, cx) {
            tracing::error!("Failed to select Style Preset: {error}");
        }
        window.refresh();
    }

    fn on_select_scrollbar_show(
        &mut self,
        show: &SelectScrollbarShow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).scrollbar_show = show.0;
        window.refresh();
    }

    fn on_toggle_list_active_highlight(
        &mut self,
        _: &ToggleListActiveHighlight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme = Theme::global_mut(cx);
        theme.list.active_highlight = !theme.list.active_highlight;
        window.refresh();
    }
}

impl Render for FontSizeSelector {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();
        let font_size = cx.theme().font_size.as_f32() as i32;
        let style_id = cx.theme().style.id.clone();
        let styles = StyleRegistry::sorted_styles(cx);
        let scroll_show = cx.theme().scrollbar_show;

        div()
            .id("font-size-selector")
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::on_select_font))
            .on_action(cx.listener(Self::on_select_style))
            .on_action(cx.listener(Self::on_select_scrollbar_show))
            .on_action(cx.listener(Self::on_toggle_list_active_highlight))
            .child(
                Button::new("btn")
                    .aria_label("Gallery settings")
                    .small()
                    .ghost()
                    .icon(IconName::Settings2)
                    .dropdown_menu(move |this, _, cx| {
                        let menu = this
                            .scrollable(true)
                            .check_side(Side::Right)
                            .max_h(px(480.))
                            .label("Font Size")
                            .menu_with_check("Large", font_size == 18, Box::new(SelectFont(18)))
                            .menu_with_check(
                                "Medium (default)",
                                font_size == 16,
                                Box::new(SelectFont(16)),
                            )
                            .menu_with_check("Small", font_size == 14, Box::new(SelectFont(14)))
                            .separator()
                            .label("Style Preset");
                        let menu = styles.iter().fold(menu, |menu, preset| {
                            menu.menu_with_check(
                                preset.name.clone(),
                                style_id == preset.id,
                                Box::new(SelectStyle(preset.id.clone())),
                            )
                        });
                        menu.separator()
                            .label("Scrollbar")
                            .menu_with_check(
                                "Scrolling to show",
                                scroll_show == ScrollbarShow::Scrolling,
                                Box::new(SelectScrollbarShow(ScrollbarShow::Scrolling)),
                            )
                            .menu_with_check(
                                "Hover to show",
                                scroll_show == ScrollbarShow::Hover,
                                Box::new(SelectScrollbarShow(ScrollbarShow::Hover)),
                            )
                            .menu_with_check(
                                "Always show",
                                scroll_show == ScrollbarShow::Always,
                                Box::new(SelectScrollbarShow(ScrollbarShow::Always)),
                            )
                            .separator()
                            .menu_with_check(
                                "List Active Highlight",
                                cx.theme().list.active_highlight,
                                Box::new(ToggleListActiveHighlight),
                            )
                    })
                    .anchor(Anchor::TopRight),
            )
    }
}
