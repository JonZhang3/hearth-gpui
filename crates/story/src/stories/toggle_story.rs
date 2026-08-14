// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added examples for `trailing_icon`, `invalid`, `items_center`, `aria_label`, `mode`,
//   `selection` and 2 more.
// - Removed examples using `segmented`.
// - Reworked Toggle story around accessibility semantics and ARIA state, focus-visible and focus
//   restoration behavior, invalid and validation state handling.
use gpui::{
    App, AppContext as _, Axis, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement as _, Render, SharedString, Styled as _, Window, px,
};

use hearth_gpui::{
    Disableable, IconName, Sizable,
    button::{
        Toggle, ToggleGroup, ToggleGroupItem, ToggleGroupMode, ToggleGroupSelection, ToggleVariants,
    },
    h_flex, v_flex,
};

use crate::section;

pub struct ToggleStory {
    focus_handle: FocusHandle,
    bold: bool,
    outline: bool,
    selected: bool,
    invalid: bool,
    sizes: [bool; 4],
    single: Option<SharedString>,
    multiple: Vec<SharedString>,
    vertical: Vec<SharedString>,
}

impl ToggleStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            bold: false,
            outline: false,
            selected: true,
            invalid: false,
            sizes: [false; 4],
            single: Some("center".into()),
            multiple: vec!["bold".into()],
            vertical: vec!["left".into()],
        })
    }
}

impl super::Story for ToggleStory {
    fn title() -> &'static str {
        "Toggle"
    }

    fn description() -> &'static str {
        "A two-state button and composable single or multiple selection group."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ToggleStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ToggleStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_4()
            .child(
                section("States").child(
                    h_flex()
                        .gap_3()
                        .child(
                            Toggle::new("toggle-default")
                                .label("Bold")
                                .icon(IconName::Check)
                                .checked(self.bold)
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    this.bold = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Toggle::new("toggle-outline")
                                .outline()
                                .label("Outline")
                                .icon(IconName::Star)
                                .trailing_icon(IconName::ChevronDown)
                                .checked(self.outline)
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    this.outline = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Toggle::new("toggle-checked")
                                .label("Selected")
                                .checked(self.selected)
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    this.selected = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Toggle::new("toggle-invalid")
                                .label("Invalid")
                                .invalid(true)
                                .checked(self.invalid)
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    this.invalid = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Toggle::new("toggle-disabled")
                                .label("Disabled")
                                .disabled(true),
                        ),
                ),
            )
            .child(
                section("Sizes").child(
                    h_flex()
                        .items_center()
                        .gap_3()
                        .child(
                            Toggle::new("toggle-xs")
                                .xsmall()
                                .icon(IconName::Check)
                                .aria_label("Extra small bold")
                                .checked(self.sizes[0])
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    this.sizes[0] = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Toggle::new("toggle-sm")
                                .small()
                                .icon(IconName::Check)
                                .aria_label("Small bold")
                                .checked(self.sizes[1])
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    this.sizes[1] = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Toggle::new("toggle-md")
                                .icon(IconName::Check)
                                .aria_label("Default bold")
                                .checked(self.sizes[2])
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    this.sizes[2] = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Toggle::new("toggle-lg")
                                .large()
                                .icon(IconName::Check)
                                .aria_label("Large bold")
                                .checked(self.sizes[3])
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    this.sizes[3] = *checked;
                                    cx.notify();
                                })),
                        ),
                ),
            )
            .child(
                section("Single selection").child(
                    ToggleGroup::new("alignment-group")
                        .mode(ToggleGroupMode::Single)
                        .selection(ToggleGroupSelection::Single(self.single.clone()))
                        .aria_label("Text alignment")
                        .child(
                            ToggleGroupItem::new("left")
                                .icon(IconName::ArrowLeft)
                                .aria_label("Align left"),
                        )
                        .child(
                            ToggleGroupItem::new("center")
                                .icon(IconName::Minus)
                                .aria_label("Align center"),
                        )
                        .child(
                            ToggleGroupItem::new("right")
                                .icon(IconName::ArrowRight)
                                .aria_label("Align right"),
                        )
                        .on_change(cx.listener(|this, selection, _, cx| {
                            if let ToggleGroupSelection::Single(value) = selection {
                                this.single = value.clone();
                                cx.notify();
                            }
                        })),
                ),
            )
            .child(
                section("Multiple selection, outline, spacing 0").child(
                    ToggleGroup::new("formatting-group")
                        .mode(ToggleGroupMode::Multiple)
                        .selection(ToggleGroupSelection::Multiple(self.multiple.clone()))
                        .outline()
                        .spacing(px(0.))
                        .aria_label("Text formatting")
                        .child(
                            ToggleGroupItem::new("bold")
                                .icon(IconName::Check)
                                .aria_label("Bold"),
                        )
                        .child(
                            ToggleGroupItem::new("italic")
                                .icon(IconName::Eye)
                                .aria_label("Italic"),
                        )
                        .child(
                            ToggleGroupItem::new("underline")
                                .icon(IconName::Inbox)
                                .aria_label("Underline"),
                        )
                        .on_change(cx.listener(|this, selection, _, cx| {
                            if let ToggleGroupSelection::Multiple(values) = selection {
                                this.multiple = values.clone();
                                cx.notify();
                            }
                        })),
                ),
            )
            .child(
                section("Vertical orientation").child(
                    ToggleGroup::new("vertical-group")
                        .mode(ToggleGroupMode::Multiple)
                        .selection(ToggleGroupSelection::Multiple(self.vertical.clone()))
                        .orientation(Axis::Vertical)
                        .outline()
                        .spacing(px(0.))
                        .aria_label("Paragraph alignment")
                        .child(ToggleGroupItem::new("left").label("Left"))
                        .child(ToggleGroupItem::new("center").label("Center"))
                        .child(ToggleGroupItem::new("right").label("Right").disabled(true))
                        .on_change(cx.listener(|this, selection, _, cx| {
                            if let ToggleGroupSelection::Multiple(values) = selection {
                                this.vertical = values.clone();
                                cx.notify();
                            }
                        })),
                ),
            )
    }
}
