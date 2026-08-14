// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `aria_label`, `flex_shrink_0`, `invalid`, `addon`, `p_0`, `w_full` and 11
//   more.
// - Removed examples using `is_horizontal`, `label_width`, `label_fn`, `gap_2`, `input_background`,
//   `pr_0` and 3 more.
// - Reworked Form story around accessibility semantics and ARIA state, invalid and validation state
//   handling.
use gpui::{
    App, AppContext, Axis, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement as _, Render, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use hearth_gpui::{
    ActiveTheme as _, AxisExt, Disableable as _, IndexPath, Selectable, Sizable, Size,
    button::{Button, ButtonGroup},
    checkbox::Checkbox,
    color_picker::{ColorPicker, ColorPickerState},
    date_picker::{DatePicker, DatePickerState},
    form::{
        FieldBody, FieldContent, FieldDescription, FieldError, FieldGroup, FieldLabel, FieldLegend,
        FieldSet, FieldTitle, field, v_form,
    },
    h_flex,
    input::{Input, InputGroup, InputGroupAddon, InputState},
    select::{Select, SelectState},
    separator::Separator,
    switch::Switch,
    v_flex,
};

pub struct FormStory {
    focus_handle: FocusHandle,
    name_prefix_state: Entity<SelectState<Vec<String>>>,
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
    bio_input: Entity<InputState>,
    color_state: Entity<ColorPickerState>,
    subscribe_email: bool,
    date: Entity<DatePickerState>,
    layout: Axis,
    size: Size,
    columns: usize,
}

impl super::Story for FormStory {
    fn title() -> &'static str {
        "Form"
    }

    fn description() -> &'static str {
        "Form to collect multiple inputs."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl FormStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_prefix_state = cx.new(|cx| {
            SelectState::new(
                vec![
                    "Mr.".to_string(),
                    "Mrs.".to_string(),
                    "Ms.".to_string(),
                    "Dr.".to_string(),
                ],
                Some(IndexPath::default()),
                window,
                cx,
            )
        });

        let name_input = cx.new(|cx| InputState::new(window, cx).default_value("Jason Lee"));
        let color_state = cx.new(|cx| ColorPickerState::new(window, cx));

        let email_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter text here..."));
        let bio_input = cx.new(|cx| {
            InputState::new(window, cx)
                .auto_grow(5, 20)
                .placeholder("Enter text here...")
                .default_value("Hello 世界，this is GPUI component.")
        });
        let date = cx.new(|cx| DatePickerState::new(window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            name_prefix_state,
            name_input,
            email_input,
            bio_input,
            date,
            color_state,
            subscribe_email: false,
            layout: Axis::Vertical,
            size: Size::default(),
            columns: 1,
        }
    }
}

impl Focusable for FormStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FormStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_multi_column = self.columns > 1;
        let label_width = px(if is_multi_column { 100. } else { 140. });
        let name_prefix_state = self.name_prefix_state.clone();
        let name_input = self.name_input.clone();
        let email_input = self.email_input.clone();
        let bio_input = self.bio_input.clone();
        let date = self.date.clone();
        let color_state = self.color_state.clone();
        let border = cx.theme().border;
        let subscribe_email = self.subscribe_email;
        let vertical_layout = self.layout.is_vertical();
        let on_subscribe = cx.listener(|this, checked: &bool, _, cx| {
            this.subscribe_email = *checked;
            cx.notify();
        });
        let on_vertical_layout = cx.listener(|this, checked: &bool, _, cx| {
            this.layout = if *checked {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
            cx.notify();
        });

        v_flex()
            .id("form-story")
            .size_full()
            .p_4()
            .justify_start()
            .gap_3()
            .child(
                h_flex()
                    .gap_3()
                    .flex_wrap()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_x_3()
                            .child(
                                Switch::new("layout")
                                    .checked(self.layout.is_horizontal())
                                    .label("Horizontal")
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        if *checked {
                                            this.layout = Axis::Horizontal;
                                        } else {
                                            this.layout = Axis::Vertical;
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Switch::new("column")
                                    .checked(self.columns > 1)
                                    .label("Multi Columns")
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        if *checked {
                                            this.columns = 2;
                                        } else {
                                            this.columns = 1;
                                        }
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        ButtonGroup::new("size")
                            .outline()
                            .small()
                            .child(
                                Button::new("large")
                                    .selected(self.size == Size::Large)
                                    .child("Large"),
                            )
                            .child(
                                Button::new("medium")
                                    .child("Medium")
                                    .selected(self.size == Size::Medium),
                            )
                            .child(
                                Button::new("small")
                                    .child("Small")
                                    .selected(self.size == Size::Small),
                            )
                            .on_click(cx.listener(|this, selecteds: &Vec<usize>, _, cx| {
                                if selecteds.contains(&0) {
                                    this.size = Size::Large;
                                } else if selecteds.contains(&1) {
                                    this.size = Size::Medium;
                                } else if selecteds.contains(&2) {
                                    this.size = Size::Small;
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(Separator::horizontal())
            .child(
                v_form()
                    .layout(self.layout)
                    .with_size(self.size)
                    .columns(self.columns)
                    .child(field("form-name").aria_label("Name").content(move |state| {
                        FieldBody::new()
                            .child(
                                FieldLabel::new("Name")
                                    .disabled(state.disabled())
                                    .w(label_width)
                                    .flex_shrink_0(),
                            )
                            .child(
                                FieldContent::new().child(
                                    InputGroup::new("form-name-input-group")
                                        .aria_label("Full name")
                                        .disabled(state.disabled())
                                        .invalid(state.invalid())
                                        .addon(
                                            InputGroupAddon::new().p_0().w(px(120.)).child(
                                                h_flex()
                                                    .w_full()
                                                    .items_center()
                                                    .on_mouse_down(
                                                        gpui::MouseButton::Left,
                                                        |_, _, cx| cx.stop_propagation(),
                                                    )
                                                    .child(
                                                        div().flex_1().min_w_0().child(
                                                            Select::new(&name_prefix_state)
                                                                .appearance(false)
                                                                .disabled(state.disabled())
                                                                .w_full(),
                                                        ),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(1.))
                                                            .h_5()
                                                            .flex_none()
                                                            .bg(border),
                                                    ),
                                            ),
                                        )
                                        .input(
                                            Input::new(&name_input)
                                                .aria_label("Full name")
                                                .disabled(state.disabled())
                                                .invalid(state.invalid()),
                                        ),
                                ),
                            )
                    }))
                    .child(
                        field("form-email")
                            .aria_label("Email")
                            .required(true)
                            .content(move |state| {
                                FieldBody::new()
                                    .child(
                                        FieldLabel::new("Email")
                                            .disabled(state.disabled())
                                            .required(state.required())
                                            .w(label_width)
                                            .flex_shrink_0(),
                                    )
                                    .child(
                                        FieldContent::new().child(
                                            Input::new(&email_input)
                                                .disabled(state.disabled())
                                                .invalid(state.invalid()),
                                        ),
                                    )
                            }),
                    )
                    .child(
                        field("form-bio")
                            .aria_label("Bio")
                            .aria_description("Use at most 100 words to describe yourself.")
                            .when(self.layout.is_vertical(), |this| this.items_start())
                            .content(move |state| {
                                FieldBody::new()
                                    .child(
                                        FieldLabel::new("Bio")
                                            .disabled(state.disabled())
                                            .w(label_width)
                                            .flex_shrink_0(),
                                    )
                                    .child(
                                        FieldContent::new()
                                            .child(
                                                Input::new(&bio_input)
                                                    .disabled(state.disabled())
                                                    .invalid(state.invalid()),
                                            )
                                            .child(FieldDescription::new(
                                                "Use at most 100 words to describe yourself.",
                                            )),
                                    )
                            }),
                    )
                    .child(
                        field("form-full-width")
                            .when(is_multi_column, |this| this.col_span(2))
                            .content(|_| {
                                FieldBody::new().child("This is a full width form field.")
                            }),
                    )
                    .child(
                        field("form-birthday")
                            .aria_label("Please select your birthday")
                            .aria_description("Select your birthday, we will send you a gift.")
                            .content(move |state| {
                                FieldBody::new()
                                    .child(
                                        FieldLabel::new("Please select your birthday")
                                            .disabled(state.disabled())
                                            .w(label_width)
                                            .flex_shrink_0(),
                                    )
                                    .child(
                                        FieldContent::new()
                                            .child(
                                                DatePicker::new(&date).disabled(state.disabled()),
                                            )
                                            .child(FieldDescription::new(
                                                "Select your birthday, we will send you a gift.",
                                            )),
                                    )
                            }),
                    )
                    .child(
                        field("form-newsletter")
                            .items_start()
                            .when(is_multi_column, |this| this.col_start(1))
                            .content(move |state| {
                                FieldBody::new().child(
                                    Switch::new("subscribe-newsletter")
                                        .label("Subscribe our newsletter")
                                        .checked(subscribe_email)
                                        .disabled(state.disabled())
                                        .invalid(state.invalid())
                                        .on_click(on_subscribe),
                                )
                            }),
                    )
                    .child(field("form-theme-color").items_start().content(move |_| {
                        FieldBody::new()
                            .child(ColorPicker::new(&color_state).small().label("Theme color"))
                    }))
                    .child(
                        field("form-layout-checkbox")
                            .items_start()
                            .content(move |state| {
                                FieldBody::new().child(
                                    Checkbox::new("use-vertical-layout")
                                        .label("Vertical layout")
                                        .checked(vertical_layout)
                                        .disabled(state.disabled())
                                        .invalid(state.invalid())
                                        .on_click(on_vertical_layout),
                                )
                            }),
                    )
                    .child(
                        field("form-validation-example")
                            .aria_label("Validation example")
                            .aria_description("Enter a valid email address.")
                            .invalid(true)
                            .when(is_multi_column, |this| this.col_span(2))
                            .content(move |state| {
                                FieldBody::new()
                                    .child(
                                        FieldLabel::new("Validation example")
                                            .disabled(state.disabled())
                                            .w(label_width)
                                            .flex_shrink_0(),
                                    )
                                    .child(
                                        FieldContent::new()
                                            .child(FieldTitle::new("Invalid field state"))
                                            .child(FieldError::new(
                                                "form-validation-error",
                                                "Enter a valid email address.",
                                            )),
                                    )
                            }),
                    ),
            )
            .child(
                FieldSet::new("form-preferences")
                    .aria_label("Preferences")
                    .content(|state| {
                        FieldBody::new()
                            .child(FieldLegend::new("Preferences"))
                            .child(
                                FieldGroup::new()
                                    .selection()
                                    .child(
                                        Checkbox::new("preference-email")
                                            .label("Email updates")
                                            .disabled(state.disabled()),
                                    )
                                    .child(
                                        Checkbox::new("preference-product")
                                            .label("Product updates")
                                            .disabled(state.disabled()),
                                    ),
                            )
                    }),
            )
    }
}
