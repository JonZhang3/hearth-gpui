use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Window,
};
use gpui_component::{
    Disableable as _, Sizable as _,
    native_select::{NativeSelect, NativeSelectOptGroup, NativeSelectOption},
    v_flex,
};

use crate::section;

pub struct NativeSelectStory {
    focus_handle: FocusHandle,
    fruit: SharedString,
    food: SharedString,
    size: SharedString,
}

impl NativeSelectStory {
    /// Creates the interactive values used by the NativeSelect examples.
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            fruit: "".into(),
            food: "".into(),
            size: "default".into(),
        }
    }

    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(Self::new)
    }

    fn fruit_options(select: NativeSelect) -> NativeSelect {
        select
            .child(NativeSelectOption::new("", "Select a fruit"))
            .child(NativeSelectOption::new("apple", "Apple"))
            .child(NativeSelectOption::new("banana", "Banana"))
            .child(NativeSelectOption::new("blueberry", "Blueberry"))
            .child(NativeSelectOption::new("grapes", "Grapes").disabled(true))
            .child(NativeSelectOption::new("pineapple", "Pineapple"))
    }
}

impl super::Story for NativeSelectStory {
    fn title() -> &'static str {
        "Native Select"
    }

    fn description() -> &'static str {
        "Displays a compact select trigger backed by the operating system option menu."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for NativeSelectStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NativeSelectStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Basic").max_w_md().items_start().child(
                    Self::fruit_options(NativeSelect::new("native-fruit"))
                        .value(self.fruit.clone())
                        .aria_label("Fruit")
                        .on_change(cx.listener(|this, value: &SharedString, _, cx| {
                            this.fruit = value.clone();
                            cx.notify();
                        })),
                ),
            )
            .child(
                section("With Groups").max_w_md().items_start().child(
                    NativeSelect::new("native-food")
                        .value(self.food.clone())
                        .aria_label("Food")
                        .child(NativeSelectOption::new("", "Select a food"))
                        .child(
                            NativeSelectOptGroup::new("Fruits")
                                .child(NativeSelectOption::new("apple", "Apple"))
                                .child(NativeSelectOption::new("banana", "Banana"))
                                .child(NativeSelectOption::new("blueberry", "Blueberry")),
                        )
                        .child(
                            NativeSelectOptGroup::new("Vegetables")
                                .child(NativeSelectOption::new("carrot", "Carrot"))
                                .child(NativeSelectOption::new("broccoli", "Broccoli"))
                                .child(NativeSelectOption::new("spinach", "Spinach")),
                        )
                        .on_change(cx.listener(|this, value: &SharedString, _, cx| {
                            this.food = value.clone();
                            cx.notify();
                        })),
                ),
            )
            .child(
                section("Sizes").max_w_md().items_start().child(
                    v_flex()
                        .gap_4()
                        .child(
                            NativeSelect::new("native-small")
                                .small()
                                .default_value("small")
                                .child(NativeSelectOption::new("small", "Small"))
                                .child(NativeSelectOption::new("compact", "Compact")),
                        )
                        .child(
                            NativeSelect::new("native-default")
                                .value(self.size.clone())
                                .child(NativeSelectOption::new("default", "Default"))
                                .child(NativeSelectOption::new("comfortable", "Comfortable"))
                                .on_change(cx.listener(|this, value: &SharedString, _, cx| {
                                    this.size = value.clone();
                                    cx.notify();
                                })),
                        ),
                ),
            )
            .child(
                section("States").max_w_md().items_start().child(
                    v_flex()
                        .gap_4()
                        .child(
                            Self::fruit_options(NativeSelect::new("native-disabled"))
                                .disabled(true),
                        )
                        .child(
                            Self::fruit_options(NativeSelect::new("native-invalid"))
                                .aria_label("Invalid fruit")
                                .invalid(true),
                        ),
                ),
            )
    }
}
