use chrono::NaiveDate;
use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IconName, IndexPath, Placement, Sizable as _,
    StyleRegistry, StyledExt as _, Theme, ThemeMode, WindowExt as _,
    accordion::Accordion,
    alert::Alert,
    avatar::{Avatar, AvatarFallback},
    badge::{Badge, BadgeVariants as _},
    button::Button,
    button::Toggle,
    calendar::{Calendar, CalendarState, Date},
    card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardMedia, CardTitle},
    checkbox::Checkbox,
    collapsible::Collapsible,
    combobox::{Combobox, ComboboxState},
    date_picker::{DatePicker, DatePickerState},
    dialog::{AlertDialogAction, AlertDialogCancel},
    form::field,
    group_box::GroupBox,
    h_flex,
    hover_card::HoverCard,
    input::{
        Input, InputState, NumberInput, OtpInput, OtpInputGroup, OtpInputSeparator, OtpInputSlot,
        OtpState,
    },
    menu::{DropdownMenu as _, PopupMenuItem},
    popover::Popover,
    progress::Progress,
    radio::Radio,
    searchable_list::SearchableVec,
    select::{Select, SelectState},
    skeleton::Skeleton,
    slider::{Slider, SliderState},
    spinner::Spinner,
    switch::Switch,
    table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow},
    v_flex,
};

use crate::section;

fn alignment_otp(state: &Entity<OtpState>) -> OtpInput {
    OtpInput::new(state)
        .child((0..3).fold(OtpInputGroup::new(), |group, index| {
            group.child(OtpInputSlot::new(index))
        }))
        .child(OtpInputSeparator::new())
        .child((3..6).fold(OtpInputGroup::new(), |group, index| {
            group.child(OtpInputSlot::new(index))
        }))
        .aria_label("One-time code")
}

/// Deterministic state matrix for Color Theme and Style Preset verification.
pub struct ShadcnAlignmentStory {
    input: Entity<InputState>,
    text_area: Entity<InputState>,
    number_input: Entity<InputState>,
    calendar: Entity<CalendarState>,
    range_calendar: Entity<CalendarState>,
    date_picker: Entity<DatePickerState>,
    slider: Entity<SliderState>,
    range_slider: Entity<SliderState>,
    disabled_input: Entity<InputState>,
    invalid_input: Entity<InputState>,
    read_only_input: Entity<InputState>,
    otp: Entity<OtpState>,
    invalid_otp: Entity<OtpState>,
    form_input: Entity<InputState>,
    select: Entity<SelectState<SearchableVec<&'static str>>>,
    combobox: Entity<ComboboxState<SearchableVec<&'static str>>>,
    checked: bool,
    switched: bool,
}

impl super::Story for ShadcnAlignmentStory {
    fn title() -> &'static str {
        "Shadcn Alignment"
    }

    fn description() -> &'static str {
        "Compare geometry and interaction states across independent Color Themes and Style Presets."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        cx.new(|cx| Self {
            input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Input")
                    .default_value("Same content, different geometry")
            }),
            text_area: cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .default_value("Multiline text\n第二行内容")
            }),
            number_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value("42")
                    .step(1.)
                    .min(0.)
            }),
            calendar: cx.new(|cx| {
                let mut calendar = CalendarState::new(window, cx).disabled_matcher(vec![0, 6]);
                calendar.set_date(NaiveDate::from_ymd_opt(2026, 8, 6).unwrap(), window, cx);
                calendar
            }),
            range_calendar: cx.new(|cx| {
                let mut calendar = CalendarState::range(window, cx);
                calendar.set_date(
                    Date::Range(
                        Some(NaiveDate::from_ymd_opt(2026, 8, 6).unwrap()),
                        Some(NaiveDate::from_ymd_opt(2026, 8, 14).unwrap()),
                    ),
                    window,
                    cx,
                );
                calendar
            }),
            date_picker: cx.new(|cx| {
                let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
                picker.set_date(NaiveDate::from_ymd_opt(2026, 8, 6).unwrap(), window, cx);
                picker
            }),
            slider: cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(100.)
                    .step(1.)
                    .default_value(64.)
            }),
            range_slider: cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(100.)
                    .step(1.)
                    .default_value(20.0..72.0)
            }),
            disabled_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Disabled input")
                    .default_value("Unavailable")
            }),
            invalid_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Invalid input")
                    .default_value("Invalid value")
            }),
            read_only_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Read-only input")
                    .default_value("Selectable, not editable")
            }),
            otp: cx.new(|cx| OtpState::new(6, window, cx).default_value("123")),
            invalid_otp: cx.new(|cx| OtpState::new(6, window, cx).default_value("12")),
            form_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("name@example.com")
                    .default_value("invalid")
            }),
            select: cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(vec!["Alpha", "Beta", "Gamma"]),
                    Some(IndexPath::new(0)),
                    window,
                    cx,
                )
            }),
            combobox: cx.new(|cx| {
                ComboboxState::new(
                    SearchableVec::new(vec!["Rust", "TypeScript", "Swift"]),
                    vec![IndexPath::new(0)],
                    window,
                    cx,
                )
                .searchable(true)
            }),
            checked: true,
            switched: true,
        })
    }
}

impl Render for ShadcnAlignmentStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.theme().style.clone();
        let active_style_id = style.id.clone();
        let active_mode = cx.theme().mode;
        let registered_styles = StyleRegistry::sorted_styles(cx);

        v_flex()
            .w_full()
            .gap_4()
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(div().font_semibold().child(style.name.clone()))
                            .child(div().text_sm().text_color(cx.theme().muted_foreground).child(
                                format!("id: {} · density: {:?}", style.id, style.density),
                            )),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "Color Theme: {} · shadcn/ui: 607e8a9717fe6ff0d374ba74c651012f9c052534",
                                cx.theme().theme_name()
                            )),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(div().text_sm().font_semibold().child("Color Theme"))
                    .child(
                        Button::new("align-theme-light")
                            .small()
                            .outline()
                            .label("Light")
                            .pressed(active_mode == ThemeMode::Light)
                            .on_click(|_, window, cx| {
                                Theme::change(ThemeMode::Light, Some(window), cx);
                            }),
                    )
                    .child(
                        Button::new("align-theme-dark")
                            .small()
                            .outline()
                            .label("Dark")
                            .pressed(active_mode == ThemeMode::Dark)
                            .on_click(|_, window, cx| {
                                Theme::change(ThemeMode::Dark, Some(window), cx);
                            }),
                    )
                    .child(div().ml_2().text_sm().font_semibold().child("Style Preset"))
                    .children(registered_styles.into_iter().map(move |preset| {
                        let preset_id = preset.id.clone();
                        let is_selected = active_style_id == preset_id;
                        Button::new(SharedString::from(format!(
                            "align-style-{}",
                            preset.id
                        )))
                        .small()
                        .outline()
                        .label(preset.name.clone())
                        .pressed(is_selected)
                        .on_click(move |_, _, cx| {
                            if let Err(error) = Theme::set_style(&preset_id, cx) {
                                tracing::error!("Failed to select Style Preset: {error}");
                            }
                        })
                    })),
            )
            .child(
                section("Button sizes")
                    .child(Button::new("align-xs").xsmall().label("XSmall"))
                    .child(Button::new("align-sm").small().label("Small"))
                    .child(Button::new("align-md").label("Medium"))
                    .child(Button::new("align-lg").large().label("Large")),
            )
            .child(
                section("Button states")
                    .child(Button::new("align-default").label("Default"))
                    .child(Button::new("align-outline").outline().label("Outline"))
                    .child(Button::new("align-secondary").secondary().label("Secondary"))
                    .child(Button::new("align-destructive").destructive().label("Destructive"))
                    .child(Button::new("align-ghost").ghost().label("Ghost"))
                    .child(Button::new("align-link").link().label("Link"))
                    .child(
                        Button::new("align-icon")
                            .aria_label("Search")
                            .icon(IconName::Search),
                    )
                    .child(
                        Button::new("align-icon-text")
                            .icon(IconName::Search)
                            .label("Search"),
                    )
                    .child(Button::new("align-cjk").label("中文操作"))
                    .child(Button::new("align-long").label("Long desktop action"))
                    .child(
                        Button::new("align-loading")
                            .icon(Spinner::new())
                            .label("Loading")
                            .disabled(true),
                    )
                    .child(Button::new("align-disabled").disabled(true).label("Disabled")),
            )
            .child(
                section("Toggle and selection states")
                    .child(Toggle::new("align-toggle-off").label("Off"))
                    .child(Toggle::new("align-toggle-on").label("Selected").checked(true))
                    .child(Toggle::new("align-toggle-disabled").label("Disabled").disabled(true))
                    .child(Radio::new("align-radio-off").label("Unchecked"))
                    .child(Radio::new("align-radio-on").label("Checked").checked(true))
                    .child(Radio::new("align-radio-invalid").label("Invalid").invalid(true))
                    .child(Radio::new("align-radio-disabled").label("Disabled").disabled(true))
                    .child(
                        Checkbox::new("align-checkbox-off")
                            .checked(false)
                            .label("Unchecked"),
                    )
                    .child(
                        Checkbox::new("align-checkbox-indeterminate")
                            .indeterminate(true)
                            .label("Indeterminate"),
                    )
                    .child(
                        Checkbox::new("align-checkbox-invalid")
                            .invalid(true)
                            .label("Invalid"),
                    )
                    .child(
                        Checkbox::new("align-checkbox")
                            .checked(self.checked)
                            .label("Checked")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.checked = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("align-checkbox-disabled")
                            .checked(true)
                            .label("Disabled")
                            .disabled(true),
                    )
                    .child(
                        Switch::new("align-switch")
                            .checked(self.switched)
                            .label("Enabled")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.switched = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Switch::new("align-switch-disabled")
                            .checked(true)
                            .label("Disabled")
                            .disabled(true),
                    ),
            )
            .child(
                section("Inputs and composite controls")
                    .max_w(px(520.))
                    .child(Input::new(&self.input))
                    .child(Input::new(&self.disabled_input).disabled(true))
                    .child(Input::new(&self.invalid_input).invalid(true))
                    .child(Input::new(&self.read_only_input).read_only(true))
                    .child(Input::new(&self.text_area).h(px(96.)))
                    .child(NumberInput::new(&self.number_input))
                    .child(alignment_otp(&self.otp))
                    .child(alignment_otp(&self.invalid_otp).invalid(true))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .child(div().text_sm().child("Select · Alpha"))
                            .child(
                                div()
                                    .w_full()
                                    .h(px(44.))
                                    .flex_none()
                                    .child(
                                        Select::new(&self.select)
                                            .aria_label("Example selection")
                                            .w_full(),
                                    ),
                            ),
                    )
                    .child(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .child(div().text_sm().child("Combobox · Rust"))
                            .child(
                                div()
                                    .w_full()
                                    .h(px(44.))
                                    .flex_none()
                                    .child(
                                        Combobox::new(&self.combobox)
                                            .aria_label("Example combobox")
                                            .w_full(),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Form field contract").max_w(px(520.)).child(
                    field()
                        .label("Email")
                        .description("Used for account notifications.")
                        .required(true)
                        .error("Enter a valid email address.")
                        .child(Input::new(&self.form_input).invalid(true)),
                ),
            )
            .child(
                section("Disclosure and dynamic content")
                    .max_w(px(640.))
                    .child(
                        Accordion::single("align-accordion")
                            .open_values(["expanded"])
                            .item("expanded", |item| {
                                item.title("Expanded item")
                                    .aria_label("Expanded item")
                                    .child(
                                        "Measured content remains mounted through its exit motion.",
                                    )
                            })
                            .item("collapsed", |item| {
                                item.title("Collapsed item")
                                    .aria_label("Collapsed item")
                                    .child("This content is excluded while collapsed.")
                            }),
                    ),
            )
            .child(
                section("Collapsible dynamic content")
                    .max_w(px(640.))
                    .child(
                    Collapsible::new()
                        .id("align-collapsible")
                        .open(true)
                        .child(div().font_medium().child("Always-visible summary"))
                        .content(
                            div()
                                .pt_2()
                                .child("Measured content uses the shared motion contract."),
                        ),
                ),
            )
            .child(
                section("Supporting and native surfaces")
                    .max_w(px(640.))
                    .child(
                        Alert::new("align-default-alert")
                            .title("Heads up!")
                            .description("You can add components to your app."),
                    )
                    .child(
                        Alert::new("align-info-alert")
                            .description("Semantic colors remain owned by the Color Theme."),
                    )
                    .child(
                        Alert::new("align-error-alert")
                            .destructive()
                            .title("Unable to continue")
                            .description("Your session has expired."),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Avatar::new("alignment-avatar", "GPUI Component")
                                    .fallback(AvatarFallback::text("GC")),
                            )
                            .child(Badge::new().child("Primary"))
                            .child(
                                Badge::new()
                                    .bg(cx.theme().success)
                                    .text_color(cx.theme().success_foreground)
                                    .child("Success"),
                            )
                            .child(Badge::new().destructive().child("Danger"))
                            .child(Spinner::new()),
                    )
                    .child(Progress::new("align-progress").value(64.))
                    .child(Progress::new("align-progress-loading").loading(true))
                    .child(
                        v_flex()
                            .gap_2()
                            .child(Skeleton::new().w(px(280.)).h_4())
                            .child(Skeleton::new().w(px(220.)).h_4()),
                    )
                    .child(
                        GroupBox::new()
                            .title("GPUI-native container")
                            .child("Shared radii and semantic surfaces; native behavior retained."),
                    ),
            )
            .child(
                section("Card surfaces").max_w(px(760.)).child(
                    h_flex()
                        .w_full()
                        .items_start()
                        .gap_4()
                        .child(
                            Card::new()
                                .w(px(360.))
                                .header(
                                    CardHeader::new()
                                        .title(CardTitle::new().child("Default Card"))
                                        .description(CardDescription::new().child(
                                            "Semantic spacing, radius, color, and elevation.",
                                        )),
                                )
                                .content(CardContent::new().child(
                                    "The active Style Preset controls geometry.",
                                ))
                                .footer(CardFooter::new().child(
                                    Button::new("align-card-action").outline().label("Action"),
                                )),
                        )
                        .child(
                            Card::new()
                                .small()
                                .w(px(360.))
                                .media(
                                    CardMedia::new()
                                        .h(px(72.))
                                        .bg(cx.theme().muted)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("Media"),
                                )
                                .header(
                                    CardHeader::new()
                                        .title(CardTitle::new().child("Small Card"))
                                        .description(CardDescription::new().child(
                                            "Compact spacing with preset-specific typography.",
                                        )),
                                )
                                .content(CardContent::new().child("Deterministic capture surface.")),
                        ),
                ),
            )
            .child(
                section("Range Calendar")
                    .max_w(px(760.))
                    .child(Calendar::new(&self.range_calendar).number_of_months(2)),
            )
            .child(
                section("Calendar, DatePicker, and Slider states")
                    .max_w(px(760.))
                    .child(
                        h_flex()
                            .w_full()
                            .items_start()
                            .gap_4()
                            .child(Calendar::new(&self.calendar))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_3()
                                    .child(
                                        DatePicker::new(&self.date_picker)
                                            .cleanable(true)
                                            .w_full(),
                                    )
                                    .child(
                                        DatePicker::new(&self.date_picker)
                                            .disabled(true)
                                            .w_full(),
                                    )
                                    .child(div().text_sm().child("Single value · 64"))
                                    .child(Slider::new(&self.slider))
                                    .child(div().text_sm().child("Range · 20–72"))
                                    .child(Slider::new(&self.range_slider))
                                    .child(div().text_sm().child("Disabled"))
                                    .child(Slider::new(&self.slider).disabled(true)),
                            ),
                    ),
            )
            .child(
                section("Data surface states")
                    .max_w(px(760.))
                    .child(
                        Table::new()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().style.radii.md)
                            .child(
                                TableHeader::new().child(
                                    TableRow::new()
                                        .child(TableHead::new().child("State"))
                                        .child(TableHead::new().child("Content")),
                                ),
                            )
                            .child(
                                TableBody::new()
                                    .child(
                                        TableRow::new()
                                            .child(TableCell::new().child("Default"))
                                            .child(TableCell::new().child("Stable row geometry")),
                                    )
                                    .child(
                                        TableRow::new()
                                            .bg(cx.theme().tokens.table_hover)
                                            .child(TableCell::new().child("Hover reference"))
                                            .child(TableCell::new().child("Pointer feedback")),
                                    )
                                    .child(
                                        TableRow::new()
                                            .bg(cx.theme().tokens.table_active)
                                            .border_l_2()
                                            .border_color(cx.theme().table_active_border)
                                            .child(TableCell::new().child("Selected"))
                                            .child(TableCell::new().child("Persistent selection")),
                                    )
                                    .child(
                                        TableRow::new()
                                            .bg(cx.theme().tokens.secondary_active)
                                            .child(TableCell::new().child("Active"))
                                            .child(TableCell::new().child("Pressed feedback")),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .gap_3()
                            .child(
                                Table::new()
                                    .flex_1()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .rounded(cx.theme().style.radii.md)
                                    .child(
                                        TableBody::new().child(
                                            TableRow::new().child(
                                                TableCell::new()
                                                    .child("No results. Empty height is stable."),
                                            ),
                                        ),
                                    ),
                            )
                            .child(
                                Table::new()
                                    .flex_1()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .rounded(cx.theme().style.radii.md)
                                    .child(
                                        TableBody::new().children((0..2).map(|_| {
                                            TableRow::new().child(
                                                TableCell::new().child(
                                                    Skeleton::new().w(px(240.)).h_3(),
                                                ),
                                            )
                                        })),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Anchored overlays")
                    .child(Button::new("align-tooltip").outline().label("Tooltip").tooltip(
                        "Compact action hint",
                    ))
                    .child(
                        Popover::new("align-popover")
                            .trigger(Button::new("align-popover-trigger").outline().label("Popover"))
                            .child(div().w(px(260.)).p_3().child("Popover content")),
                    )
                    .child(
                        HoverCard::new("align-hover-card")
                            .trigger(
                                Button::new("align-hover-card-trigger")
                                    .outline()
                                    .label("HoverCard"),
                            )
                            .child(div().child("Hover card content")),
                    )
                    .child(
                        Button::new("align-menu")
                            .outline()
                            .label("Menu")
                            .dropdown_menu(|menu, _, _| {
                                menu.item(PopupMenuItem::new("Open"))
                                    .item(PopupMenuItem::new("Disabled").disabled(true))
                            }),
                    ),
            )
            .child(
                section("Modal and feedback overlays")
                    .child(
                        Button::new("align-dialog")
                            .outline()
                            .label("Dialog")
                            .on_click(|_, window, cx| {
                                window.open_dialog(cx, |dialog, _, _| {
                                    dialog
                                        .aria_label("Alignment Dialog")
                                        .title("Alignment Dialog")
                                        .child(
                                            "Focus trap, Escape, outside click, and exit motion.",
                                        )
                                });
                            }),
                    )
                    .child(
                        Button::new("align-alert-dialog")
                            .outline()
                            .label("AlertDialog")
                            .on_click(|_, window, cx| {
                                window.open_alert_dialog(cx, |dialog, _, _| {
                                    dialog.content(|content, _, _| {
                                        content
                                            .title("Confirm action")
                                            .description(
                                                "This action requires an explicit decision.",
                                            )
                                            .cancel(AlertDialogCancel::new(
                                                "alignment-alert-cancel",
                                                "Cancel",
                                            ))
                                            .action(AlertDialogAction::new(
                                                "alignment-alert-confirm",
                                                "Continue",
                                            ))
                                    })
                                });
                            }),
                    )
                    .child(
                        Button::new("align-sheet")
                            .outline()
                            .label("Sheet")
                            .on_click(|_, window, cx| {
                                window.open_sheet_at(Placement::Right, cx, |sheet, _, _| {
                                    sheet
                                        .aria_label("Alignment Sheet")
                                        .title("Alignment Sheet")
                                        .child("Sheet content")
                                });
                            }),
                    )
                    .child(
                        Button::new("align-notification")
                            .outline()
                            .label("Notification")
                            .on_click(|_, window, cx| {
                                window.push_notification("Alignment notification", cx);
                            }),
                    ),
            )
    }
}
