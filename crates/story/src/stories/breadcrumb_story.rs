// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `ghost`, `aria_label`, `dropdown_menu`, `item`.
// - Removed examples using `gap_4`, `items_center`.
// - Reworked Breadcrumb story around accessibility semantics and ARIA state, focus-visible and
//   focus restoration behavior.
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, prelude::FluentBuilder as _, px,
};

use gpui_component::{
    IconName, Sizable as _,
    breadcrumb::{
        Breadcrumb, BreadcrumbEllipsis, BreadcrumbItem, BreadcrumbLink, BreadcrumbPage,
        BreadcrumbSeparator,
    },
    button::Button,
    menu::{DropdownMenu as _, PopupMenuItem},
    v_flex,
};

use crate::section;

pub struct BreadcrumbStory {
    focus_handle: FocusHandle,
    clicked_item: Option<String>,
}

impl BreadcrumbStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            clicked_item: None,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for BreadcrumbStory {
    fn title() -> &'static str {
        "Breadcrumb"
    }

    fn description() -> &'static str {
        "A breadcrumb navigation element that shows the current location in a hierarchy."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for BreadcrumbStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BreadcrumbStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Basic").max_w_md().child(
                    Breadcrumb::new("basic-breadcrumb")
                        .child(
                            BreadcrumbItem::new("basic-home-item").child(
                                BreadcrumbLink::new("basic-home-link")
                                    .label("Home")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clicked_item = Some("Home".into());
                                        cx.notify();
                                    })),
                            ),
                        )
                        .child(BreadcrumbSeparator::new())
                        .child(
                            BreadcrumbItem::new("basic-components-item").child(
                                BreadcrumbLink::new("basic-components-link")
                                    .label("Components")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clicked_item = Some("Components".into());
                                        cx.notify();
                                    })),
                            ),
                        )
                        .child(BreadcrumbSeparator::new())
                        .child(
                            BreadcrumbItem::new("basic-page-item")
                                .child(BreadcrumbPage::new("basic-page").label("Breadcrumb")),
                        ),
                ),
            )
            .child(
                section("Custom Separator").max_w_md().child(
                    Breadcrumb::new("custom-separator-breadcrumb")
                        .child(
                            BreadcrumbItem::new("custom-home-item").child(
                                BreadcrumbLink::new("custom-home-link")
                                    .label("Home")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clicked_item = Some("Home".into());
                                        cx.notify();
                                    })),
                            ),
                        )
                        .child(BreadcrumbSeparator::new().child("/"))
                        .child(
                            BreadcrumbItem::new("custom-page-item")
                                .child(BreadcrumbPage::new("custom-page").label("Components")),
                        ),
                ),
            )
            .child(
                section("Collapsed").max_w_md().child(
                    Breadcrumb::new("ellipsis-breadcrumb")
                        .child(
                            BreadcrumbItem::new("ellipsis-home-item").child(
                                BreadcrumbLink::new("ellipsis-home-link")
                                    .label("Home")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clicked_item = Some("Home".into());
                                        cx.notify();
                                    })),
                            ),
                        )
                        .child(BreadcrumbSeparator::new())
                        .child(
                            BreadcrumbItem::new("ellipsis-item").child(BreadcrumbEllipsis::new()),
                        )
                        .child(BreadcrumbSeparator::new())
                        .child(
                            BreadcrumbItem::new("ellipsis-page-item")
                                .child(BreadcrumbPage::new("ellipsis-page").label("Breadcrumb")),
                        ),
                ),
            )
            .child(
                section("Dropdown").max_w_md().child(
                    Breadcrumb::new("collapsed-breadcrumb")
                        .child(
                            BreadcrumbItem::new("collapsed-home-item").child(
                                BreadcrumbLink::new("collapsed-home-link")
                                    .label("Home")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clicked_item = Some("Home".into());
                                        cx.notify();
                                    })),
                            ),
                        )
                        .child(BreadcrumbSeparator::new())
                        .child(
                            BreadcrumbItem::new("collapsed-menu-item").child(
                                Button::new("collapsed-menu")
                                    .ghost()
                                    .small()
                                    .icon(IconName::Ellipsis)
                                    .aria_label("Show collapsed breadcrumb items")
                                    .dropdown_menu(|menu, _, _| {
                                        menu.item(PopupMenuItem::new("Documentation"))
                                            .item(PopupMenuItem::new("Themes"))
                                            .item(PopupMenuItem::new("GitHub"))
                                    }),
                            ),
                        )
                        .child(BreadcrumbSeparator::new())
                        .child(
                            BreadcrumbItem::new("collapsed-components-item").child(
                                BreadcrumbLink::new("collapsed-components-link")
                                    .label("Components")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clicked_item = Some("Components".into());
                                        cx.notify();
                                    })),
                            ),
                        )
                        .child(BreadcrumbSeparator::new())
                        .child(
                            BreadcrumbItem::new("collapsed-page-item")
                                .child(BreadcrumbPage::new("collapsed-page").label("Breadcrumb")),
                        ),
                ),
            )
            .child(
                section("Disabled and Wrapping").child(
                    Breadcrumb::new("wrapping-breadcrumb")
                        .w(px(240.))
                        .child(
                            BreadcrumbItem::new("wrapping-home-item").child(
                                BreadcrumbLink::new("wrapping-home-link")
                                    .label("Home")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clicked_item = Some("Home".into());
                                        cx.notify();
                                    }))
                                    .disabled(true),
                            ),
                        )
                        .child(BreadcrumbSeparator::new())
                        .child(
                            BreadcrumbItem::new("wrapping-section-item").child(
                                BreadcrumbLink::new("wrapping-section-link")
                                    .label("Long documentation section")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clicked_item =
                                            Some("Long documentation section".into());
                                        cx.notify();
                                    })),
                            ),
                        )
                        .child(BreadcrumbSeparator::new())
                        .child(
                            BreadcrumbItem::new("wrapping-page-item")
                                .child(BreadcrumbPage::new("wrapping-page").label("Current page")),
                        ),
                ),
            )
            .when_some(self.clicked_item.clone(), |this, item| {
                this.child(format!("Activated: {item}"))
            })
    }
}
