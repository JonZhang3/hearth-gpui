use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, SharedString,
    Styled, Subscription, Window, div,
};

use gpui_component::{
    Disableable as _, Icon, IconName, Sizable as _, StyledExt,
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    label::{HighlightsMatch, Label},
    v_flex,
};

use crate::section;

pub struct LabelStory {
    focus_handle: gpui::FocusHandle,
    username_input: Entity<InputState>,
    disabled_input: Entity<InputState>,
    highlights_input: Entity<InputState>,
    highlights_text: SharedString,
    prefix: bool,
    masked: bool,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for LabelStory {
    fn title() -> &'static str {
        "Label"
    }

    fn description() -> &'static str {
        "A label for form controls and composed inline content."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl LabelStory {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let username_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter your username"));
        let disabled_input = cx.new(|cx| InputState::new(window, cx).placeholder("Unavailable"));
        let highlights_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Text to highlight")
                .clean_on_escape()
        });
        let _subscriptions =
            vec![
                cx.subscribe(&highlights_input, |this, state, event: &InputEvent, cx| {
                    if let InputEvent::Change = event {
                        this.highlights_text = state.read(cx).value();
                        cx.notify();
                    }
                }),
            ];

        Self {
            focus_handle: cx.focus_handle(),
            username_input,
            disabled_input,
            highlights_input,
            highlights_text: Default::default(),
            prefix: false,
            masked: false,
            _subscriptions,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn highlights_text(&self) -> HighlightsMatch {
        if self.prefix {
            HighlightsMatch::Prefix(self.highlights_text.clone())
        } else {
            HighlightsMatch::Full(self.highlights_text.clone())
        }
    }
}

impl Focusable for LabelStory {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LabelStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let username_focus = self.username_input.read(cx).focus_handle(cx);
        let disabled_focus = self.disabled_input.read(cx).focus_handle(cx);
        let highlights = self.highlights_text();

        v_flex()
            .gap_6()
            .child(
                section("Basic")
                    .max_w_md()
                    .items_start()
                    .child(Label::new("Username")),
            )
            .child(
                section("With Input").max_w_md().items_start().child(
                    v_flex()
                        .w_full()
                        .gap_2()
                        .child(Label::new("Username").for_focus(&username_focus))
                        .child(Input::new(&self.username_input).aria_label("Username")),
                ),
            )
            .child(
                section("Disabled").max_w_md().items_start().child(
                    v_flex()
                        .w_full()
                        .gap_2()
                        .child(
                            Label::new("Username")
                                .for_focus(&disabled_focus)
                                .disabled(true),
                        )
                        .child(
                            Input::new(&self.disabled_input)
                                .aria_label("Username")
                                .disabled(true),
                        ),
                ),
            )
            .child(
                section("With Checkbox").max_w_md().items_start().child(
                    Checkbox::new("terms")
                        .label("Accept terms and conditions")
                        .checked(true),
                ),
            )
            .child(
                section("Composed Content").max_w_md().items_start().child(
                    Label::empty()
                        .child(Icon::new(IconName::Info).xsmall())
                        .child("Additional information"),
                ),
            )
            .child(
                section("Project Extensions")
                    .max_w_md()
                    .items_start()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(Input::new(&self.highlights_input).w_1_2())
                                    .child(
                                        Checkbox::new("prefix")
                                            .label("Prefix only")
                                            .checked(self.prefix)
                                            .on_click(cx.listener(|view, _, _, cx| {
                                                view.prefix = !view.prefix;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                Label::new("Company Address")
                                    .secondary("(optional)")
                                    .highlights(highlights.clone()),
                            )
                            .child(Label::new("AAA中文BB").highlights(highlights.clone()))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Label::new("9,182.1 USD")
                                            .masked(self.masked)
                                            .highlights(highlights),
                                    )
                                    .child(
                                        Button::new("toggle-mask")
                                            .with_variant(ButtonVariant::Ghost)
                                            .xsmall()
                                            .icon(if self.masked {
                                                IconName::EyeOff
                                            } else {
                                                IconName::Eye
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.masked = !this.masked;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .child(Label::new("Styled override").text_lg().font_semibold()),
                            ),
                    ),
            )
    }
}
