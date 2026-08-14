// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public types: `BreadcrumbLink`, `BreadcrumbPage`, `BreadcrumbSeparator`,
//   `BreadcrumbEllipsis`.
// - Added public methods: `aria_label`, `label`, `href`.
// - Removed public methods: `children`.
// - Removed or replaced `id`, `is_last`, `children`.
// - Reworked Breadcrumb around accessibility semantics and ARIA state, semantic Style Preset
//   geometry and density, keyboard navigation and activation behavior, focus-visible and focus
//   restoration behavior.
use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement as _, IntoElement, KeyboardButton,
    ParentElement, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{
    ActiveTheme as _, FocusableExt as _, Icon, IconName, Sizable as _, StyledExt as _,
    accessibility::{accessibility_current_page, accessibility_state},
    theme::Density,
};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// Resolves shadcn Breadcrumb list spacing from semantic density and viewport width.
fn breadcrumb_list_gap(density: Density, wide: bool) -> Pixels {
    if density == Density::Compact || !wide {
        px(6.)
    } else {
        px(10.)
    }
}

/// Resolves shadcn Breadcrumb item spacing from semantic density.
fn breadcrumb_item_gap(density: Density) -> Pixels {
    if density == Density::Compact {
        px(4.)
    } else {
        px(6.)
    }
}

/// A breadcrumb navigation landmark containing an ordered list of locations.
#[derive(IntoElement)]
pub struct Breadcrumb {
    id: ElementId,
    style: StyleRefinement,
    aria_label: SharedString,
    children: Vec<AnyElement>,
}

impl Breadcrumb {
    /// Creates a breadcrumb navigation landmark with stable element identity.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            aria_label: "Breadcrumb".into(),
            children: Vec::new(),
        }
    }

    /// Sets the accessible name announced for the navigation landmark.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = label.into();
        self
    }
}

impl ParentElement for Breadcrumb {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Breadcrumb {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let list_gap = breadcrumb_list_gap(
            cx.theme().style.density,
            window.viewport_size().width >= px(640.),
        );

        div()
            .id(self.id)
            .role(Role::Navigation)
            .aria_label(self.aria_label)
            .refine_style(&self.style)
            .child(
                div()
                    .id("list")
                    .role(Role::List)
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(list_gap)
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .children(self.children),
            )
    }
}

/// A list item within a [`Breadcrumb`].
#[derive(IntoElement)]
pub struct BreadcrumbItem {
    id: ElementId,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl BreadcrumbItem {
    /// Creates a breadcrumb list item with stable element identity.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for BreadcrumbItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for BreadcrumbItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for BreadcrumbItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .role(Role::ListItem)
            .flex()
            .flex_row()
            .items_center()
            .gap(breadcrumb_item_gap(cx.theme().style.density))
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// An interactive link to an ancestor location in a [`Breadcrumb`].
#[derive(IntoElement)]
pub struct BreadcrumbLink {
    id: ElementId,
    style: StyleRefinement,
    label: Option<SharedString>,
    aria_label: Option<SharedString>,
    href: Option<SharedString>,
    on_click: Option<ClickHandler>,
    disabled: bool,
    children: Vec<AnyElement>,
}

impl BreadcrumbLink {
    /// Creates a breadcrumb link with stable element identity.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            label: None,
            aria_label: None,
            href: None,
            on_click: None,
            disabled: false,
            children: Vec::new(),
        }
    }

    /// Sets the visible link label and its default accessible name.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets an accessible name for custom link content.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Opens the URL when the link is activated.
    pub fn href(mut self, href: impl Into<SharedString>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Registers a callback without replacing URL navigation.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Sets whether the link is excluded from activation and keyboard focus.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for BreadcrumbLink {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for BreadcrumbLink {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for BreadcrumbLink {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let href = self.href.clone();
        let on_click = self.on_click.clone();
        let has_target = href.is_some() || on_click.is_some();
        let interactive = has_target && !self.disabled;
        let keyboard_href = href.clone();
        let keyboard_on_click = on_click.clone();
        let accessible_label = self.aria_label.or_else(|| self.label.clone());
        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let focus_visible =
            interactive && focus_handle.is_focused(window) && window.last_input_was_keyboard();

        let element = div()
            .id(self.id)
            .role(if has_target { Role::Link } else { Role::Label })
            .when_some(accessible_label, |this, label| this.aria_label(label))
            .flex()
            .items_center()
            .font_normal()
            .text_color(cx.theme().muted_foreground)
            .when(interactive, |this| {
                this.track_focus(&focus_handle.tab_stop(true))
                    .cursor_pointer()
                    .hover(|this| this.text_color(cx.theme().foreground))
                    .on_key_down(move |event, window, cx| {
                        if event.keystroke.modifiers.modified() {
                            return;
                        }
                        // A link follows native anchor behavior: only Enter activates it.
                        if event.keystroke.key == "enter" {
                            window.prevent_default();
                            cx.stop_propagation();
                            if let Some(href) = &keyboard_href {
                                cx.open_url(href);
                            }
                            if let Some(on_click) = &keyboard_on_click {
                                on_click(&ClickEvent::default(), window, cx);
                            }
                        }
                    })
            })
            .when(has_target && self.disabled, |this| this.opacity(0.5))
            .refine_style(&self.style)
            .when_some(self.label, |this, label| this.child(label))
            .children(self.children)
            .when(interactive, |this| {
                this.on_click(move |event, window, cx| {
                    // GPUI maps Space to click for generic focusable elements. Ignore that
                    // synthetic click without consuming the original keyboard event.
                    if matches!(
                        event,
                        ClickEvent::Keyboard(event) if event.button == KeyboardButton::Space
                    ) {
                        return;
                    }
                    if let Some(href) = &href {
                        cx.open_url(href);
                    }
                    if let Some(on_click) = &on_click {
                        on_click(event, window, cx);
                    }
                })
            })
            .focus_ring(focus_visible, px(0.), window, cx);

        accessibility_state(element, false, false, has_target && self.disabled)
    }
}

/// The non-interactive current location in a [`Breadcrumb`].
#[derive(IntoElement)]
pub struct BreadcrumbPage {
    id: ElementId,
    style: StyleRefinement,
    label: Option<SharedString>,
    aria_label: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl BreadcrumbPage {
    /// Creates a current-page element with stable element identity.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            label: None,
            aria_label: None,
            children: Vec::new(),
        }
    }

    /// Sets the visible current-page label and its default accessible name.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets an accessible name for custom current-page content.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }
}

impl ParentElement for BreadcrumbPage {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for BreadcrumbPage {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for BreadcrumbPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let accessible_label = self.aria_label.or_else(|| self.label.clone());
        let element = div()
            .id(self.id)
            .role(Role::Link)
            .when_some(accessible_label, |this, label| this.aria_label(label))
            .flex()
            .items_center()
            .font_normal()
            .text_color(cx.theme().foreground)
            .refine_style(&self.style)
            .when_some(self.label, |this, label| this.child(label))
            .children(self.children);

        accessibility_current_page(element)
    }
}

/// A presentational separator between breadcrumb items.
#[derive(IntoElement)]
pub struct BreadcrumbSeparator {
    style: StyleRefinement,
    child: Option<AnyElement>,
}

impl BreadcrumbSeparator {
    /// Creates a separator using a ChevronRight icon by default.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            child: None,
        }
    }

    /// Replaces the default ChevronRight icon with custom content.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl Default for BreadcrumbSeparator {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for BreadcrumbSeparator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for BreadcrumbSeparator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .child(self.child.unwrap_or_else(|| {
                Icon::new(IconName::ChevronRight)
                    .with_size(px(14.))
                    .into_any_element()
            }))
    }
}

/// A presentational ellipsis used when intermediate breadcrumb items collapse.
#[derive(IntoElement)]
pub struct BreadcrumbEllipsis {
    style: StyleRefinement,
}

impl BreadcrumbEllipsis {
    /// Creates a centered MoreHorizontal ellipsis.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
        }
    }
}

impl Default for BreadcrumbEllipsis {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for BreadcrumbEllipsis {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for BreadcrumbEllipsis {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .size(px(20.))
            .flex()
            .items_center()
            .justify_center()
            .refine_style(&self.style)
            .child(
                Icon::new(IconName::Ellipsis)
                    .with_size(px(16.))
                    .text_color(cx.theme().muted_foreground),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementExt as _;
    use gpui::{
        AppContext as _, Element as _, Render, TestAppContext, VisualTestContext, accesskit,
    };
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    #[gpui::test]
    fn test_breadcrumb_builder(_cx: &mut gpui::TestAppContext) {
        let breadcrumb = Breadcrumb::new("docs").aria_label("Documentation breadcrumb");
        let item = BreadcrumbItem::new("home-item");
        let link = BreadcrumbLink::new("home-link")
            .label("Home")
            .href("https://example.com")
            .disabled(false);
        let page = BreadcrumbPage::new("current-page").label("Breadcrumb");
        let separator = BreadcrumbSeparator::new();
        let ellipsis = BreadcrumbEllipsis::new();

        assert_eq!(breadcrumb.aria_label.as_ref(), "Documentation breadcrumb");
        assert!(item.children.is_empty());
        assert_eq!(link.label.as_deref(), Some("Home"));
        assert_eq!(link.href.as_deref(), Some("https://example.com"));
        assert!(!link.disabled);
        assert_eq!(page.label.as_deref(), Some("Breadcrumb"));
        assert!(separator.child.is_none());
        assert_eq!(ellipsis.style, StyleRefinement::default());
    }

    #[test]
    fn semantic_density_resolves_shadcn_breadcrumb_spacing() {
        assert_eq!(breadcrumb_list_gap(Density::Standard, false), px(6.));
        assert_eq!(breadcrumb_list_gap(Density::Standard, true), px(10.));
        assert_eq!(breadcrumb_item_gap(Density::Standard), px(6.));

        assert_eq!(breadcrumb_list_gap(Density::Compact, false), px(6.));
        assert_eq!(breadcrumb_list_gap(Density::Compact, true), px(6.));
        assert_eq!(breadcrumb_item_gap(Density::Compact), px(4.));

        assert_eq!(breadcrumb_list_gap(Density::Comfortable, false), px(6.));
        assert_eq!(breadcrumb_list_gap(Density::Comfortable, true), px(10.));
        assert_eq!(breadcrumb_item_gap(Density::Comfortable), px(6.));
    }

    struct AccessibilityProbe {
        metadata: Arc<Mutex<Vec<(Role, Option<String>, bool, Option<accesskit::AriaCurrent>)>>>,
    }

    impl Render for AccessibilityProbe {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let metadata = self.metadata.clone();
            div().on_prepaint(move |_, window, cx| {
                let mut link_node = accesskit::Node::new(Role::Link);
                let link = BreadcrumbLink::new("probe-link")
                    .label("Home")
                    .href("https://example.com")
                    .disabled(true)
                    .render(window, cx)
                    .into_element();
                let link_role = link.a11y_role().expect("targeted link must expose a role");
                link.write_a11y_info(&mut link_node);

                let mut inert_node = accesskit::Node::new(Role::Label);
                let inert = BreadcrumbLink::new("probe-inert")
                    .label("Section")
                    .render(window, cx)
                    .into_element();
                let inert_role = inert
                    .a11y_role()
                    .expect("label-only breadcrumb content must expose a role");
                inert.write_a11y_info(&mut inert_node);

                let mut page_node = accesskit::Node::new(Role::Link);
                let page = BreadcrumbPage::new("probe-page")
                    .label("Breadcrumb")
                    .render(window, cx)
                    .into_element();
                let page_role = page.a11y_role().expect("current page must expose a role");
                page.write_a11y_info(&mut page_node);

                *metadata.lock().unwrap() = vec![
                    (
                        link_role,
                        link_node.label().map(ToOwned::to_owned),
                        link_node.is_disabled(),
                        link_node.aria_current(),
                    ),
                    (
                        inert_role,
                        inert_node.label().map(ToOwned::to_owned),
                        inert_node.is_disabled(),
                        inert_node.aria_current(),
                    ),
                    (
                        page_role,
                        page_node.label().map(ToOwned::to_owned),
                        page_node.is_disabled(),
                        page_node.aria_current(),
                    ),
                ];
            })
        }
    }

    #[gpui::test]
    fn breadcrumb_text_supplies_accessibility_names_and_current_state(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let metadata = Arc::new(Mutex::new(Vec::new()));
        let captured = metadata.clone();
        let (_, cx) = cx.add_window_view(move |_, _| AccessibilityProbe { metadata });
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        assert_eq!(
            *captured.lock().unwrap(),
            vec![
                (Role::Link, Some("Home".into()), true, None),
                (Role::Label, Some("Section".into()), false, None),
                (
                    Role::Link,
                    Some("Breadcrumb".into()),
                    true,
                    Some(accesskit::AriaCurrent::Page),
                ),
            ]
        );
    }

    struct KeyboardFixture {
        calls: Arc<AtomicUsize>,
    }

    struct SpacePropagationFixture {
        calls: Arc<AtomicUsize>,
        propagated: Arc<AtomicUsize>,
    }

    impl Render for SpacePropagationFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            let propagated = self.propagated.clone();
            div()
                .on_key_down(move |event, _, _| {
                    if event.keystroke.key == "space" {
                        propagated.fetch_add(1, Ordering::SeqCst);
                    }
                })
                .child(
                    BreadcrumbLink::new("space-link")
                        .label("Documentation")
                        .on_click(move |_, _, _| {
                            calls.fetch_add(1, Ordering::SeqCst);
                        }),
                )
        }
    }

    struct FocusOrderFixture {
        calls: Arc<AtomicUsize>,
    }

    impl Render for FocusOrderFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            div()
                .child(BreadcrumbLink::new("inert-link").label("Inert"))
                .child(BreadcrumbLink::new("action-link").label("Action").on_click(
                    move |_, _, _| {
                        calls.fetch_add(1, Ordering::SeqCst);
                    },
                ))
        }
    }

    impl Render for KeyboardFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            BreadcrumbLink::new("keyboard-link")
                .label("Documentation")
                .href("https://example.com/docs")
                .on_click(move |_, _, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                })
        }
    }

    #[gpui::test]
    fn enter_activates_href_and_callback_once(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let captured = calls.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| KeyboardFixture { calls });
            crate::Root::new(fixture, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
            _ = window.draw(cx);
        });
        assert!(cx.update(|window, cx| window.focused(cx).is_some()));
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();

        assert_eq!(captured.load(Ordering::SeqCst), 1);
        assert_eq!(cx.opened_url().as_deref(), Some("https://example.com/docs"));
    }

    #[gpui::test]
    fn space_does_not_activate_a_breadcrumb_link(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let captured = calls.clone();
        let propagated = Arc::new(AtomicUsize::new(0));
        let captured_propagated = propagated.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| SpacePropagationFixture { calls, propagated });
            crate::Root::new(fixture, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
            _ = window.draw(cx);
        });
        cx.simulate_keystrokes("space");
        cx.run_until_parked();

        assert_eq!(captured.load(Ordering::SeqCst), 0);
        assert_eq!(captured_propagated.load(Ordering::SeqCst), 1);
        assert!(cx.opened_url().is_none());
    }

    #[gpui::test]
    fn label_only_content_is_skipped_by_keyboard_focus(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let captured = calls.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| FocusOrderFixture { calls });
            crate::Root::new(fixture, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
            _ = window.draw(cx);
        });
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();

        assert_eq!(captured.load(Ordering::SeqCst), 1);
    }
}
