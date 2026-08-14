use std::rc::Rc;

use gpui::{
    App, Axis, ElementId, InteractiveElement as _, IntoElement, ParentElement, RenderOnce, Role,
    SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{AxisExt, Sizable, Size, StyledExt as _, stepper::StepperItem};

/// A step-by-step progress for users to navigate through a series of steps or stages.
#[derive(IntoElement)]
pub struct Stepper {
    id: ElementId,
    style: StyleRefinement,
    items: Vec<StepperItem>,
    step: usize,
    layout: Axis,
    disabled: bool,
    size: Size,
    text_center: bool,
    aria_label: Option<SharedString>,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
}

impl Stepper {
    /// Creates a new stepper with the given ID.
    ///
    /// Default use is horizontal layout with step 0 selected.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            items: Vec::new(),
            step: 0,
            layout: Axis::Horizontal,
            disabled: false,
            size: Size::default(),
            text_center: false,
            aria_label: None,
            on_click: None,
        }
    }

    /// Sets the accessible name announced for the step list.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set whether to center the text within each stepper item.
    pub fn text_center(mut self, center: bool) -> Self {
        self.text_center = center;
        self
    }

    /// Set the layout of the stepper, default is horizontal.
    pub fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    /// Sets the layout of the stepper to Vertical.
    pub fn vertical(mut self) -> Self {
        self.layout = Axis::Vertical;
        self
    }

    /// Sets the selected index of the stepper, default is 0.
    pub fn selected_index(mut self, index: usize) -> Self {
        self.step = index;
        self
    }

    /// Adds a stepper item to the stepper.
    pub fn item(mut self, item: StepperItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple stepper items to the stepper.
    pub fn items(mut self, items: impl IntoIterator<Item = StepperItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Set the disabled state of the stepper, default is false.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add an on_click handler for when a step is clicked.
    ///
    /// The first parameter is the `step` of currently clicked item.
    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: Fn(&usize, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl Sizable for Stepper {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Stepper {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Stepper {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let total_items = self.items.len();
        let selected_step = normalize_selected_step(self.step, total_items);
        let interactive = self.on_click.is_some();
        let aria_label = self
            .aria_label
            .unwrap_or_else(|| t!("Stepper.label").into());

        div()
            .id(self.id.clone())
            .role(Role::List)
            .aria_label(aria_label)
            .w_full()
            .when(self.layout.is_horizontal(), |this| this.h_flex())
            .when(self.layout.is_vertical(), |this| this.v_flex())
            .refine_style(&self.style)
            .children(self.items.into_iter().enumerate().map(|(step, item)| {
                let is_last = step + 1 == total_items;
                item.owner_id(self.id.clone())
                    .step(step)
                    .with_size(self.size)
                    .checked_step(selected_step)
                    .size_of_set(total_items)
                    .layout(self.layout)
                    .text_center(self.text_center)
                    .interactive(interactive)
                    .when(self.disabled, |this| this.disabled(true))
                    .is_last(is_last)
                    .when_some(self.on_click.clone(), |this, on_click| {
                        this.on_click(move |window, cx| {
                            on_click(&step, window, cx);
                        })
                    })
            }))
    }
}

/// Keeps the controlled current step inside the rendered item collection.
fn normalize_selected_step(selected: usize, item_count: usize) -> Option<usize> {
    item_count.checked_sub(1).map(|last| selected.min(last))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        AppContext as _, KeyDownEvent, KeyUpEvent, Keystroke, Render, TestAppContext,
        VisualTestContext,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn selected_step_is_normalized_to_available_items() {
        assert_eq!(normalize_selected_step(0, 0), None);
        assert_eq!(normalize_selected_step(1, 3), Some(1));
        assert_eq!(normalize_selected_step(8, 3), Some(2));
    }

    #[gpui::test]
    fn stepper_builder_preserves_configuration(_cx: &mut gpui::TestAppContext) {
        let stepper = Stepper::new("builder")
            .aria_label("Checkout progress")
            .vertical()
            .text_center(true)
            .selected_index(2)
            .disabled(true)
            .large()
            .item(StepperItem::new().label("Details"))
            .on_click(|_, _, _| {});

        assert_eq!(stepper.aria_label.as_deref(), Some("Checkout progress"));
        assert_eq!(stepper.layout, Axis::Vertical);
        assert!(stepper.text_center);
        assert_eq!(stepper.step, 2);
        assert!(stepper.disabled);
        assert_eq!(stepper.size, Size::Large);
        assert_eq!(stepper.items.len(), 1);
        assert!(stepper.on_click.is_some());
    }

    struct KeyboardFixture {
        calls: Arc<AtomicUsize>,
        selected: Arc<AtomicUsize>,
    }

    impl Render for KeyboardFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            let selected = self.selected.clone();
            div()
                .child(Stepper::new("read-only").item(StepperItem::new().label("Read only")))
                .child(
                    Stepper::new("interactive")
                        .items([
                            StepperItem::new().label("Disabled").disabled(true),
                            StepperItem::new().label("Enabled"),
                        ])
                        .on_click(move |step, _, _| {
                            calls.fetch_add(1, Ordering::SeqCst);
                            selected.store(*step, Ordering::SeqCst);
                        }),
                )
        }
    }

    #[gpui::test]
    fn keyboard_activation_skips_read_only_and_disabled_steps(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let selected = Arc::new(AtomicUsize::new(usize::MAX));
        let captured_calls = calls.clone();
        let captured_selected = selected.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| KeyboardFixture { calls, selected });
            crate::Root::new(fixture, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
            window.focus_next(cx);
        });
        let space = Keystroke::parse("space").expect("space must be a valid keystroke");
        cx.simulate_event(KeyDownEvent {
            keystroke: space.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyDownEvent {
            keystroke: space.clone(),
            is_held: true,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke: space });
        cx.run_until_parked();

        assert_eq!(captured_calls.load(Ordering::SeqCst), 1);
        assert_eq!(captured_selected.load(Ordering::SeqCst), 1);
    }
}
