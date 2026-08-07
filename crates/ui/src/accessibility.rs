use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Window, accesskit,
};

/// Wraps an element with accessibility states that GPUI does not yet expose
/// through `InteractiveElement` fluent methods.
pub(crate) struct AccessibilityStateElement<E> {
    element: E,
    invalid: bool,
    read_only: bool,
    disabled: bool,
    current: Option<accesskit::AriaCurrent>,
}

/// Applies form-control accessibility states while preserving the wrapped
/// element's layout, painting, interaction, role, and synthetic children.
pub(crate) fn accessibility_state(
    element: impl IntoElement,
    invalid: bool,
    read_only: bool,
    disabled: bool,
) -> AccessibilityStateElement<impl Element> {
    AccessibilityStateElement {
        element: element.into_element(),
        invalid,
        read_only,
        disabled,
        current: None,
    }
}

/// Marks an element as the current page while preserving its GPUI element behavior.
pub(crate) fn accessibility_current_page(
    element: impl IntoElement,
) -> AccessibilityStateElement<impl Element> {
    AccessibilityStateElement {
        element: element.into_element(),
        invalid: false,
        read_only: false,
        disabled: true,
        current: Some(accesskit::AriaCurrent::Page),
    }
}

impl<E: Element> IntoElement for AccessibilityStateElement<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E: Element> Element for AccessibilityStateElement<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.element.id()
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        self.element.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.element.request_layout(id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.element
            .prepaint(id, inspector_id, bounds, request_layout, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.paint(
            id,
            inspector_id,
            bounds,
            request_layout,
            prepaint,
            window,
            cx,
        );
    }

    fn a11y_role(&self) -> Option<accesskit::Role> {
        self.element.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.element.write_a11y_info(node);
        if self.invalid {
            node.set_invalid(accesskit::Invalid::True);
        }
        if self.read_only {
            node.set_read_only();
        }
        if self.disabled {
            node.set_disabled();
        }
        if let Some(current) = self.current {
            node.set_aria_current(current);
        }
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.element.a11y_synthetic_children(prepaint, builder);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        InteractiveElement as _, Render, RenderOnce as _, Role, StatefulInteractiveElement as _,
        div,
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn writes_extended_accessibility_states() {
        let element = div().id("field").role(Role::TextInput);
        let wrapped = accessibility_state(element, true, true, true);
        let mut node = accesskit::Node::new(Role::TextInput);

        wrapped.write_a11y_info(&mut node);

        assert_eq!(node.invalid(), Some(accesskit::Invalid::True));
        assert!(node.is_read_only());
        assert!(node.is_disabled());
    }

    #[test]
    fn writes_current_page_accessibility_state() {
        let element = div().id("current-page").role(Role::Link);
        let wrapped = accessibility_current_page(element);
        let mut node = accesskit::Node::new(Role::Link);

        wrapped.write_a11y_info(&mut node);

        assert!(node.is_disabled());
        assert_eq!(node.aria_current(), Some(accesskit::AriaCurrent::Page));
    }

    #[gpui::test]
    fn disabled_toggle_and_switch_write_accessibility_state(cx: &mut gpui::TestAppContext) {
        use crate::{Disableable as _, ElementExt as _, button::Toggle, switch::Switch};

        struct DisabledControlProbe {
            states: Arc<Mutex<Vec<(Option<String>, bool)>>>,
        }

        impl Render for DisabledControlProbe {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                let states = self.states.clone();
                div().on_prepaint(move |_, window, cx| {
                    let mut toggle_node = accesskit::Node::new(Role::Button);
                    Toggle::new("disabled-toggle")
                        .label("Disabled")
                        .disabled(true)
                        .render(window, cx)
                        .into_element()
                        .write_a11y_info(&mut toggle_node);

                    let mut switch_node = accesskit::Node::new(Role::Switch);
                    Switch::new("disabled-switch")
                        .label("Disabled")
                        .disabled(true)
                        .render(window, cx)
                        .into_element()
                        .write_a11y_info(&mut switch_node);

                    *states.lock().unwrap() = vec![
                        (
                            toggle_node.label().map(ToOwned::to_owned),
                            toggle_node.is_disabled(),
                        ),
                        (
                            switch_node.label().map(ToOwned::to_owned),
                            switch_node.is_disabled(),
                        ),
                    ];
                })
            }
        }

        cx.update(crate::init);
        let states = Arc::new(Mutex::new(Vec::new()));
        let captured = states.clone();
        let (_, cx) = cx.add_window_view(move |_, _| DisabledControlProbe { states });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        assert_eq!(
            *captured.lock().unwrap(),
            vec![
                (Some("Disabled".into()), true),
                (Some("Disabled".into()), true)
            ]
        );
    }
}
