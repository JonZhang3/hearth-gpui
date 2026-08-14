// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added story helpers for `item_label`.
// - Added examples for `aria_label`, `gap_3`, `side`, `side_offset`.
// - Removed examples using `primary`, `max_w`, `gap_2`, `text_sm`, `min_h`, `top_0` and 7 more.
// - Reworked Popover story around accessibility semantics and ARIA state, semantic Style Preset
//   geometry and density.
// - Replaced legacy radius access with `Theme.style.radii.md`.
use gpui::{
    Action, App, AppContext, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    Half, InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement as _, Render,
    Styled as _, WeakEntity, Window, actions, px,
};
use gpui_component::{
    ActiveTheme, WindowExt,
    button::Button,
    h_flex,
    input::{Input, InputState},
    list::{List, ListDelegate, ListItem, ListState},
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
    popover::{
        Popover, PopoverAlign, PopoverDescription, PopoverHeader, PopoverSide, PopoverTitle,
    },
    separator::Separator,
    v_flex,
};
use serde::Deserialize;
use std::time::Duration;

use crate::section;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = popover_story, no_json)]
struct Info(usize);

actions!(popover_story, [Copy, Paste, Cut, SearchAll, ToggleCheck]);
const CONTEXT: &str = "popover-story";
pub fn init(cx: &mut App) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-f", SearchAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-f", SearchAll, Some(CONTEXT)),
    ])
}

struct Form {
    parent: WeakEntity<PopoverStory>,
    input1: Entity<InputState>,
}

impl Form {
    fn new(parent: WeakEntity<PopoverStory>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            parent: parent,
            input1: cx.new(|cx| InputState::new(window, cx)),
        })
    }
}

impl Focusable for Form {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input1.focus_handle(cx)
    }
}

struct DropdownListDelegate {
    parent: WeakEntity<PopoverStory>,
}

impl ListDelegate for DropdownListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        10
    }

    fn item_label(&self, ix: gpui_component::IndexPath, _: &App) -> gpui::SharedString {
        format!("Item {}", ix.row).into()
    }

    fn render_item(
        &mut self,
        ix: gpui_component::IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Self::Item {
        ListItem::new(ix).child(format!("Item {}", ix.row))
    }

    fn set_selected_index(
        &mut self,
        _: Option<gpui_component::IndexPath>,
        _: &mut Window,
        _: &mut Context<gpui_component::list::ListState<Self>>,
    ) {
    }

    fn confirm(&mut self, _: bool, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        let _ = self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;
            cx.notify();
        });
    }

    fn cancel(&mut self, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        let _ = self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;
            cx.notify();
        });
    }
}

impl EventEmitter<DismissEvent> for Form {}

impl Render for Form {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let parent = self.parent.clone();
        v_flex()
            .gap_2()
            .p_3()
            .size_full()
            .child("This is a form container.")
            .child("Click submit to dismiss the popover.")
            .child(Input::new(&self.input1))
            .child(Button::new("submit").label("Submit").on_click(cx.listener(
                move |_, _, _, cx| {
                    let _ = parent.update(cx, |this, cx| {
                        this.form_popover_open = false;
                        cx.notify();
                    });
                },
            )))
    }
}

pub struct PopoverStory {
    focus_handle: FocusHandle,
    form: Entity<Form>,
    list: Entity<ListState<DropdownListDelegate>>,
    form_popover_open: bool,
    list_popover_open: bool,
    checked: bool,
    message: String,
}

impl super::Story for PopoverStory {
    fn title() -> &'static str {
        "Popover"
    }

    fn description() -> &'static str {
        "A popup displays content on top of the main page."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl PopoverStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let form = Form::new(cx.weak_entity(), window, cx);
        let parent = cx.weak_entity();
        let list = cx.new(|cx| {
            ListState::new(DropdownListDelegate { parent: parent }, window, cx).searchable(true)
        });

        cx.focus_self(window);

        Self {
            form,
            list,
            checked: true,
            form_popover_open: false,
            list_popover_open: false,
            focus_handle: cx.focus_handle(),
            message: "".to_string(),
        }
    }

    fn on_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked copy".to_string();
        cx.notify()
    }

    fn on_cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked cut".to_string();
        cx.notify()
    }

    fn on_paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked paste".to_string();
        cx.notify()
    }

    fn on_search_all(&mut self, _: &SearchAll, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked search all".to_string();
        cx.notify()
    }

    fn on_action_info(&mut self, info: &Info, _: &mut Window, cx: &mut Context<Self>) {
        self.message = format!("You have clicked info: {}", info.0);
        cx.notify()
    }

    fn on_action_toggle_check(&mut self, _: &ToggleCheck, _: &mut Window, cx: &mut Context<Self>) {
        self.checked = !self.checked;
        self.message = format!("You have clicked toggle check: {}", self.checked);
        cx.notify()
    }
}

impl Focusable for PopoverStory {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopoverStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.form.clone();

        v_flex()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_search_all))
            .on_action(cx.listener(Self::on_action_info))
            .on_action(cx.listener(Self::on_action_toggle_check))
            .size_full()
            .gap_6()
            .child(
                section("Basic Popover").child(
                    Popover::new("popover-0")
                        .trigger(Button::new("btn").outline().label("Popover"))
                        .aria_label("Popover overview")
                        .child(
                            PopoverHeader::new()
                                .child(PopoverTitle::new().child("About Popover"))
                                .child(
                                    PopoverDescription::new().child(
                                        "Display rich content next to an interactive trigger.",
                                    ),
                                ),
                        ),
                ),
            )
            .child(
                section("Popover with Form").child(
                    Popover::new("popover-form")
                        .p_0()
                        .text_sm()
                        .trigger(Button::new("pop").outline().label("Popup Form"))
                        .track_focus(&form.focus_handle(cx))
                        .open(self.form_popover_open)
                        .on_open_change(cx.listener(move |this, open, _, cx| {
                            println!("Popover form open changed: {}", open);
                            this.form_popover_open = *open;
                            cx.notify();
                        }))
                        .child(form.clone()),
                ),
            )
            .child(
                section("Popover with List").child(
                    Popover::new("popover-list")
                        .p_0()
                        .text_sm()
                        .open(self.list_popover_open)
                        .on_open_change(cx.listener(move |this, open, _, cx| {
                            this.list_popover_open = *open;
                            cx.notify();
                        }))
                        .trigger(Button::new("pop").outline().label("Popup List"))
                        .track_focus(&self.list.focus_handle(cx))
                        .child(List::new(&self.list))
                        .w_64()
                        .h(px(200.)),
                ),
            )
            .child(
                section("Right click to open Popover").child(
                    Popover::new("popover-right-click")
                        .mouse_button(MouseButton::Right)
                        .trigger(Button::new("btn").outline().label("Right Click Popover"))
                        .max_w(px(600.))
                        .content(|_, _, cx| {
                            v_flex()
                                .gap_2()
                                .child("Hello, this is a Popover on the Bottom Right.")
                                .child(Separator::horizontal())
                                .child(Button::new("info1").label("Dismiss").w(px(80.)).on_click(
                                    cx.listener(|_, _, window, cx| {
                                        window.push_notification(
                                            "You have clicked dismiss via DismissEvent.",
                                            cx,
                                        );
                                        cx.emit(DismissEvent);
                                    }),
                                ))
                        }),
                ),
            )
            .child(
                section("Styling Popover").child(
                    Popover::new("popover-1")
                        .trigger(Button::new("btn").outline().label("Style Popover"))
                        .appearance(false)
                        .py_1()
                        .px_2()
                        .bg(cx.theme().primary)
                        .text_color(cx.theme().primary_foreground)
                        .max_w(px(600.))
                        .rounded(cx.theme().style.radii.md.half())
                        .text_sm()
                        .shadow_2xl()
                        .child("A styled Popover with custom background and text color."),
                ),
            )
            .child(
                section("Default Open").child(
                    Popover::new("default-open-popover")
                        .default_open(true)
                        .trigger(
                            Button::new("default-open-btn")
                                .label("Default Open")
                                .outline(),
                        )
                        .child("This popover is open by default when first rendered."),
                ),
            )
            .child(
                section("Async Submenu")
                    .child(
                        Button::new("async-menu")
                            .outline()
                            .label("Async Menu")
                            .dropdown_menu(|menu, window, cx| {
                                // The submenu is attached as a plain menu value, its
                                // content is loaded asynchronously via `rebuild`.
                                let submenu = PopupMenu::build(window, cx, |menu, _, _| {
                                    menu.label("Loading...")
                                });

                                cx.spawn_in(window, {
                                    let submenu = submenu.clone();
                                    async move |_, cx| {
                                        cx.background_executor()
                                            .timer(Duration::from_secs(1))
                                            .await;
                                        _ = submenu.update_in(cx, |menu, window, cx| {
                                            menu.rebuild(window, cx, |menu, _, _| {
                                                (1..=3).fold(menu, |menu, ix| {
                                                    menu.menu(
                                                        format!("Loaded Item {}", ix),
                                                        Box::new(Info(ix)),
                                                    )
                                                })
                                            });
                                        });
                                    }
                                })
                                .detach();

                                menu.menu("Copy", Box::new(Copy))
                                    .separator()
                                    .item(PopupMenuItem::submenu("Async Submenu", submenu))
                            }),
                    )
                    .child(self.message.clone()),
            )
            .child(
                section("Popover Placement").child(
                    h_flex()
                        .gap_3()
                        .child(
                            Popover::new("side-top")
                                .side(PopoverSide::Top)
                                .align(PopoverAlign::Start)
                                .trigger(Button::new("top").outline().label("Top / Start"))
                                .child("Placed above and start-aligned."),
                        )
                        .child(
                            Popover::new("side-right")
                                .side(PopoverSide::Right)
                                .trigger(Button::new("right").outline().label("Right"))
                                .child("Placed on the right."),
                        )
                        .child(
                            Popover::new("side-bottom")
                                .side(PopoverSide::Bottom)
                                .align(PopoverAlign::End)
                                .trigger(Button::new("bottom").outline().label("Bottom / End"))
                                .child("Placed below and end-aligned."),
                        )
                        .child(
                            Popover::new("side-left")
                                .side(PopoverSide::Left)
                                .side_offset(px(8.))
                                .trigger(Button::new("left").outline().label("Left + offset"))
                                .child("Placed on the left with an 8px offset."),
                        ),
                ),
            )
    }
}
