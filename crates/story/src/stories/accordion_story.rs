// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added story helpers for `set_open_values`.
// - Removed or replaced story helpers: `toggle_accordion`, `set_size`.
// - Added examples for `framed`, `open_values`, `on_open_change`, `set_open_values`, `max_w_md`,
//   `default_open_values`.
// - Removed examples using `outline`, `compact`, `set_size`, `bordered`, `multiple`, `open` and 2
//   more.
// - Reworked Accordion story around accessibility semantics and ARIA state.
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Window,
};
use gpui_component::{
    IconName, accordion::Accordion, checkbox::Checkbox, h_flex, switch::Switch, v_flex,
};

use crate::section;

pub struct AccordionStory {
    open_values: Vec<SharedString>,
    framed: bool,
    disabled: bool,
    multiple: bool,
    show_icon: bool,
    focus_handle: FocusHandle,
}

impl super::Story for AccordionStory {
    fn title() -> &'static str {
        "Accordion"
    }

    fn description() -> &'static str {
        "The accordion uses collapse internally to make it collapsible."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl AccordionStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            framed: false,
            open_values: vec!["accessibility".into(), "composition".into(), "third".into()],
            disabled: false,
            multiple: true,
            show_icon: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn set_open_values(
        &mut self,
        open_values: Vec<SharedString>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_values = open_values;
        cx.notify();
    }
}

impl Focusable for AccordionStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AccordionStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_5()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .flex_wrap()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Checkbox::new("multiple")
                                    .label("Multiple")
                                    .checked(self.multiple)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.multiple = *checked;
                                        if !checked {
                                            this.open_values.truncate(1);
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("show_icon")
                                    .label("Icon")
                                    .checked(self.show_icon)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.show_icon = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("disabled")
                                    .label("Disabled")
                                    .checked(self.disabled)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.disabled = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("framed")
                                    .label("Framed")
                                    .checked(self.framed)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.framed = *checked;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                section("Normal").max_w_md().child(
                    (if self.multiple {
                        Accordion::multiple("test")
                    } else {
                        Accordion::single("test")
                    })
                    .framed(self.framed)
                    .disabled(self.disabled)
                    .open_values(self.open_values.clone())
                    .item("accessibility", |this| {
                        let item = if self.show_icon {
                            this.icon(IconName::Info)
                        } else {
                            this
                        };
                        item.title("Is it accessible?")
                            .child("Yes. It adheres to the WAI-ARIA design pattern.")
                    })
                    .item("composition", |this| {
                        let item = if self.show_icon {
                            this.icon(IconName::Inbox)
                        } else {
                            this
                        };
                        item.title("Is it styled with complex elements?").child(
                            v_flex()
                                .gap_4()
                                .child("We can put any view here, like a v_flex with a text view")
                                .child(
                                    h_flex()
                                        .gap_4()
                                        .child(Switch::new("switch1").label("Switch"))
                                        .child(Checkbox::new("checkbox1").label("Or a Checkbox")),
                                ),
                        )
                    })
                    .item("third", |this| {
                        let item = if self.show_icon {
                            this.icon(IconName::Moon)
                        } else {
                            this
                        };
                        item.title("This is third accordion").child(
                            "This is the third accordion content. \
                                It can be any view, like a text view or a button.",
                        )
                    })
                    .on_open_change(cx.listener(
                        |this, values: &[SharedString], window, cx| {
                            this.set_open_values(values.to_vec(), window, cx);
                        },
                    )),
                ),
            )
            .child(
                section("Single").max_w_md().child(
                    Accordion::single("single-example")
                        .framed(self.framed)
                        .disabled(self.disabled)
                        .default_open_values(["shipping"])
                        .item("shipping", |item| {
                            item.title("What are your shipping options?")
                                .child("We offer standard, express, and overnight shipping.")
                        })
                        .item("returns", |item| {
                            item.title("What is your return policy?")
                                .child("Returns are accepted within 30 days of delivery.")
                        })
                        .item("support", |item| {
                            item.title("How can I contact customer support?")
                                .child("Contact support by email or live chat.")
                        }),
                ),
            )
    }
}
