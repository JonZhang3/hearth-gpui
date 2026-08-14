// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public methods: `title_element`, `description`, `description_element`, `aria_label`,
//   `aria_description`, `show_close_button`, `initial_focus`.
// - Removed public methods: `resizable`.
// - Added or exposed behavior through `for_density`, `title_element`, `description`,
//   `description_element`, `aria_label`, `aria_description`, `show_close_button`, `initial_focus`
//   and 5 more.
// - Removed or replaced `resizable`.
// - Reworked Sheet around accessibility semantics and ARIA state, interruptible and
//   reduced-motion-aware transitions, semantic Style Preset geometry and density, focus-visible and
//   focus restoration behavior.
use std::rc::Rc;

use gpui::{
    AbsoluteLength, AnyElement, App, ClickEvent, DefiniteLength, DismissEvent, ElementId,
    EventEmitter, FocusHandle, InteractiveElement as _, IntoElement, KeyBinding, MouseButton,
    ParentElement, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, WindowControlArea, anchored, div, hsla, point,
    prelude::FluentBuilder as _, px,
};
use rust_i18n::t;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    ActiveTheme, Density, Disableable as _, ElementExt as _, FocusTrapElement as _, IconName,
    Placement, Sizable, StyledExt as _, WindowExt as _,
    actions::Cancel,
    animation::{OverlayPhase, Transition},
    button::Button,
    dialog::modal_overlay,
    geometry::LengthExt as _,
    scroll::ScrollableElement as _,
    text::{SelectionScope, SelectionScopeElement as _},
    title_bar::TITLE_BAR_HEIGHT,
    v_flex,
};

const CONTEXT: &str = "Sheet";
const DEFAULT_SIDE_MAX_SIZE: Pixels = px(384.);
const DEFAULT_SIDE_FRACTION: f32 = 0.75;

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))])
}

/// The settings for sheets.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SheetSettings {
    /// The native title-bar safe area reserved above the sheet.
    pub margin_top: Pixels,
}

impl Default for SheetSettings {
    fn default() -> Self {
        Self {
            margin_top: TITLE_BAR_HEIGHT,
        }
    }
}

/// Component-local geometry derived from semantic Style Preset density.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SheetMetrics {
    content_gap: Pixels,
    section_padding: Pixels,
    header_gap: Pixels,
    footer_gap: Pixels,
    close_inset: Pixels,
    title_is_base: bool,
}

impl SheetMetrics {
    /// Resolves the pinned Vega, Nova, and Maia Sheet geometry without branching on preset IDs.
    fn for_density(density: Density) -> Self {
        match density {
            Density::Standard => Self {
                content_gap: px(16.),
                section_padding: px(16.),
                header_gap: px(6.),
                footer_gap: px(8.),
                close_inset: px(16.),
                title_is_base: false,
            },
            Density::Compact => Self {
                content_gap: px(16.),
                section_padding: px(16.),
                header_gap: px(2.),
                footer_gap: px(8.),
                close_inset: px(12.),
                title_is_base: true,
            },
            Density::Comfortable => Self {
                content_gap: px(0.),
                section_padding: px(24.),
                header_gap: px(6.),
                footer_gap: px(8.),
                close_inset: px(16.),
                title_is_base: true,
            },
        }
    }
}

type SizeObserver = Rc<dyn Fn(Pixels, &mut App)>;

/// A modal surface that slides in from one edge of the window.
#[derive(IntoElement)]
pub struct Sheet {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) placement: Placement,
    pub(crate) instance_id: u64,
    size: Option<DefiniteLength>,
    on_close: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
    title: Option<AnyElement>,
    title_text: Option<SharedString>,
    description: Option<AnyElement>,
    description_text: Option<SharedString>,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
    footer: Option<AnyElement>,
    initial_focus: Option<FocusHandle>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    overlay: bool,
    overlay_closable: bool,
    show_close_button: bool,
    pub(crate) lifecycle_phase: OverlayPhase,
    pub(crate) measured_size: Option<Pixels>,
    pub(crate) observe_size: Option<SizeObserver>,
}

impl Sheet {
    /// Creates a Sheet with responsive shadcn-compatible default sizing.
    pub fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            placement: Placement::Right,
            instance_id: 0,
            size: None,
            on_close: Rc::new(|_, _, _| {}),
            title: None,
            title_text: None,
            description: None,
            description_text: None,
            aria_label: None,
            aria_description: None,
            footer: None,
            initial_focus: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
            overlay: true,
            overlay_closable: true,
            show_close_button: true,
            lifecycle_phase: OverlayPhase::Open,
            measured_size: None,
            observe_size: None,
        }
    }

    /// Sets a semantic text title and uses it as the default accessible name.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        let title = title.into();
        self.title = Some(title.clone().into_any_element());
        self.title_text = Some(title);
        self
    }

    /// Sets a custom title element. Use [`Self::aria_label`] to name the dialog surface.
    pub fn title_element(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self.title_text = None;
        self
    }

    /// Sets a semantic text description and uses it as the default accessible description.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        let description = description.into();
        self.description = Some(description.clone().into_any_element());
        self.description_text = Some(description);
        self
    }

    /// Sets a custom description element. Use [`Self::aria_description`] for assistive technology.
    pub fn description_element(mut self, description: impl IntoElement) -> Self {
        self.description = Some(description.into_any_element());
        self.description_text = None;
        self
    }

    /// Sets the accessible name announced for the Sheet dialog surface.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets the accessible description announced for the Sheet dialog surface.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Sets the footer content.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Overrides the responsive width for left/right sheets or height for top/bottom sheets.
    pub fn size(mut self, size: impl Into<DefiniteLength>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Sets whether the visual backdrop is painted.
    pub fn overlay(mut self, overlay: bool) -> Self {
        self.overlay = overlay;
        self
    }

    /// Sets whether a primary click on the visible backdrop dismisses the Sheet.
    pub fn overlay_closable(mut self, overlay_closable: bool) -> Self {
        self.overlay_closable = overlay_closable;
        self
    }

    /// Sets whether the icon-only close button is rendered.
    pub fn show_close_button(mut self, show: bool) -> Self {
        self.show_close_button = show;
        self
    }

    /// Sets the preferred initial focus target inside the Sheet.
    pub fn initial_focus(mut self, focus_handle: FocusHandle) -> Self {
        self.initial_focus = Some(focus_handle);
        self
    }

    /// Listens to user-initiated dismissal from Escape, backdrop, or the close button.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Rc::new(on_close);
        self
    }

    /// Resolves the along-axis size while preserving explicit `.size(...)` overrides.
    fn resolved_size(&self, available: Pixels, rem_size: Pixels) -> Option<Pixels> {
        self.size
            .map(|size| size.to_pixels(AbsoluteLength::Pixels(available), rem_size))
            .or_else(|| {
                self.placement
                    .is_horizontal()
                    .then_some((available * DEFAULT_SIDE_FRACTION).min(DEFAULT_SIDE_MAX_SIZE))
            })
    }

    /// Returns the mirrored translation endpoints for the current lifecycle phase.
    fn motion_translation(
        placement: Placement,
        distance: Pixels,
        closing: bool,
    ) -> (gpui::Point<Pixels>, gpui::Point<Pixels>) {
        let offset = match placement {
            Placement::Top => point(px(0.), -distance),
            Placement::Right => point(distance, px(0.)),
            Placement::Bottom => point(px(0.), distance),
            Placement::Left => point(-distance, px(0.)),
        };
        if closing {
            (point(px(0.), px(0.)), offset)
        } else {
            (offset, point(px(0.), px(0.)))
        }
    }
}

impl EventEmitter<DismissEvent> for Sheet {}

impl ParentElement for Sheet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Sheet {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Sheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let placement = self.placement;
        let window_paddings = crate::window_border::window_paddings(window);
        let view_size = window.viewport_size()
            - gpui::size(
                window_paddings.left + window_paddings.right,
                window_paddings.top + window_paddings.bottom,
            );
        let top = cx.theme().sheet.margin_top;
        let available_axis = if placement.is_horizontal() {
            view_size.width
        } else {
            view_size.height - top
        };
        let style_axis_size = if placement.is_horizontal() {
            self.style.size.width
        } else {
            self.style.size.height
        };
        let explicit_size = self
            .size
            .map(|size| size.to_pixels(AbsoluteLength::Pixels(available_axis), window.rem_size()));
        let style_axis_pixels = style_axis_size.and_then(|size| {
            size.to_pixels(AbsoluteLength::Pixels(available_axis), window.rem_size())
        });
        let resolved_size = explicit_size.or_else(|| {
            style_axis_size
                .is_none()
                .then(|| self.resolved_size(available_axis, window.rem_size()))?
        });
        // A configured axis size is known before layout. Auto-sized top and bottom
        // sheets use one fully offscreen measuring frame, then restart with the
        // measured height so no partially visible first frame can be painted.
        let configured_motion_size = explicit_size.or(style_axis_pixels).or(resolved_size);
        let measured_motion_size = self.measured_size.filter(|size| *size > px(0.));
        let motion_ready = configured_motion_size.is_some() || measured_motion_size.is_some();
        let motion_distance = measured_motion_size
            .or(configured_motion_size)
            .unwrap_or(available_axis.max(px(0.)));
        let has_explicit_size = self.size.is_some();
        let has_axis_size = has_explicit_size || style_axis_size.is_some();
        let closing = self.lifecycle_phase == OverlayPhase::Closing;
        let motion = cx.theme().style.motion;
        let metrics = SheetMetrics::for_density(cx.theme().style.density);
        let overlay_opacity = cx.theme().style.modals.overlay_opacity;
        let elevation_enabled = cx.theme().style.elevation.enabled;
        let aria_label = self.aria_label.clone().or(self.title_text.clone());
        let aria_description = self
            .aria_description
            .clone()
            .or(self.description_text.clone());
        let on_close = self.on_close.clone();
        let focus_handle = self.focus_handle.clone();

        if !closing {
            let initial_focus = self.initial_focus.clone();
            window.defer(cx, move |window, cx| {
                if !focus_handle.is_focused(window) {
                    return;
                }
                if let Some(initial_focus) = initial_focus {
                    initial_focus.focus(window, cx);
                    if focus_handle.contains_focused(window, cx) {
                        return;
                    }
                    focus_handle.focus(window, cx);
                }

                let starting_focus = window.focused(cx);
                const MAX_INITIAL_FOCUS_ATTEMPTS: usize = 100;
                for _ in 0..MAX_INITIAL_FOCUS_ATTEMPTS {
                    window.focus_next(cx);
                    if focus_handle.contains_focused(window, cx) {
                        return;
                    }
                    if window.focused(cx) == starting_focus {
                        break;
                    }
                }
                focus_handle.focus(window, cx);
            });
        }

        let backdrop = modal_overlay(
            view_size,
            Some(if self.overlay {
                hsla(0., 0., 0., overlay_opacity)
            } else {
                hsla(0., 0., 0., 0.)
            }),
        )
        .id(("sheet-backdrop", self.instance_id))
        .absolute()
        .top_0()
        .left_0()
        .when(self.overlay && !closing, |this| {
            this.window_control_area(WindowControlArea::Drag)
                .on_any_mouse_down({
                    let on_close = self.on_close.clone();
                    move |event, window, cx| {
                        if event.position.y < top {
                            return;
                        }
                        cx.stop_propagation();
                        if self.overlay_closable && event.button == MouseButton::Left {
                            window.close_sheet(cx);
                            on_close(&ClickEvent::default(), window, cx);
                        }
                    }
                })
        });
        let header = (self.title.is_some() || self.description.is_some()).then(|| {
            v_flex()
                .gap(metrics.header_gap)
                .p(metrics.section_padding)
                .when_some(self.title, |this, title| {
                    this.child(
                        div()
                            .font_medium()
                            .when(metrics.title_is_base, |this| this.text_base())
                            .when(!metrics.title_is_base, |this| this.text_sm())
                            .text_color(cx.theme().foreground)
                            .child(title),
                    )
                })
                .when_some(self.description, |this, description| {
                    this.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(description),
                    )
                })
        });

        let content = v_flex()
            .id(("sheet-content", self.instance_id))
            .role(Role::Dialog)
            .when_some(aria_label, |this, label| this.aria_label(label))
            .when_some(aria_description, |this, description| {
                this.aria_description(description)
            })
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .focus_trap(
                ElementId::NamedInteger("sheet-focus-trap".into(), self.instance_id),
                &self.focus_handle,
            )
            .when(!closing, |this| {
                this.on_action({
                    let on_close = self.on_close.clone();
                    move |_: &Cancel, window, cx| {
                        window.close_sheet(cx);
                        on_close(&ClickEvent::default(), window, cx);
                    }
                })
            })
            .relative()
            .occlude()
            .flex_col()
            .w_full()
            .when(placement.is_horizontal() || has_axis_size, |this| {
                this.h_full()
            })
            .when(placement.is_vertical() && !has_axis_size, |this| {
                this.h_auto()
            })
            .gap(metrics.content_gap)
            .text_sm()
            .bg(cx.theme().tokens.popover)
            .text_color(cx.theme().popover_foreground)
            .border_color(cx.theme().border)
            .when(elevation_enabled, |this| this.shadow_lg())
            .map(|this| match placement {
                Placement::Top => this.border_b_1(),
                Placement::Right => this.border_l_1(),
                Placement::Bottom => this.border_t_1(),
                Placement::Left => this.border_r_1(),
            })
            .children(header)
            .child(
                div().flex_1().overflow_hidden().child(
                    v_flex()
                        .size_full()
                        .overflow_y_scrollbar()
                        .px(metrics.section_padding)
                        .children(self.children),
                ),
            )
            .when_some(self.footer, |this, footer| {
                this.child(
                    v_flex()
                        .mt_auto()
                        .gap(metrics.footer_gap)
                        .p(metrics.section_padding)
                        .w_full()
                        .child(footer),
                )
            })
            .when(self.show_close_button, |this| {
                this.child(
                    div()
                        .absolute()
                        .top(metrics.close_inset)
                        .right(metrics.close_inset)
                        .child(
                            Button::new("close")
                                .aria_label(t!("Common.Close"))
                                .small()
                                .ghost()
                                .icon(IconName::Close)
                                .disabled(closing)
                                .on_click(move |_, window, cx| {
                                    window.close_sheet(cx);
                                    on_close(&ClickEvent::default(), window, cx);
                                }),
                        ),
                )
            })
            .when(closing, |this| {
                this.child(div().absolute().top_0().left_0().size_full().occlude())
            })
            .refine_style(&self.style);

        let (from, to) = Self::motion_translation(placement, motion_distance, closing);
        let motion_key = if motion_ready {
            "sheet-content-motion"
        } else {
            "sheet-content-motion-measuring"
        };
        let content = Transition::new(motion.slow())
            .ease_token(if closing {
                motion.exit_easing
            } else {
                motion.enter_easing
            })
            .slide_x(from.x, to.x)
            .slide_y(from.y, to.y)
            .apply(content, (motion_key, self.instance_id))
            .selection_scope(SelectionScope::Sheet);

        // The absolute shell owns edge anchoring and size. Motion is applied to the
        // relative surface inside it so directional offsets never overwrite `top`,
        // `right`, `bottom`, or `left` placement constraints.
        let content_shell = div()
            .absolute()
            .map(|this| match placement {
                Placement::Top => this.top(top).left_0().right_0(),
                Placement::Right => this.top(top).right_0().bottom_0(),
                Placement::Bottom => this.bottom_0().left_0().right_0(),
                Placement::Left => this.top(top).left_0().bottom_0(),
            })
            .map(
                |this| match (placement.is_horizontal(), explicit_size, style_axis_size) {
                    (true, Some(size), _) => this.w(size).max_w(view_size.width),
                    (false, Some(size), _) => this.h(size).max_h(available_axis),
                    (true, None, Some(size)) => this.w(size).max_w(view_size.width),
                    (false, None, Some(size)) => this.h(size).max_h(available_axis),
                    (true, None, None) => this
                        .w(resolved_size.expect("horizontal Sheet default size must resolve"))
                        .max_w(view_size.width),
                    (false, None, None) => this.h_auto().max_h(available_axis),
                },
            )
            .when_some(self.observe_size, |this, observe_size| {
                this.on_prepaint(move |bounds, _, cx| {
                    let measured = if placement.is_horizontal() {
                        bounds.size.width
                    } else {
                        bounds.size.height
                    };
                    observe_size(measured, cx);
                })
            })
            .child(content);

        anchored()
            .position(point(window_paddings.left, window_paddings.top))
            .snap_to_window()
            .child(
                div()
                    .relative()
                    .w(view_size.width)
                    .h(view_size.height)
                    .child(backdrop)
                    .child(content_shell),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn text_slots_and_builders_preserve_semantic_state(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let initial_focus = cx.focus_handle();
            let sheet = Sheet::new(window, cx)
                .title("Connection settings")
                .description("Configure the active connection.")
                .size(px(420.))
                .show_close_button(false)
                .overlay(false)
                .overlay_closable(false)
                .initial_focus(initial_focus.clone());

            assert_eq!(sheet.title_text.as_deref(), Some("Connection settings"));
            assert_eq!(
                sheet.description_text.as_deref(),
                Some("Configure the active connection.")
            );
            assert_eq!(sheet.size, Some(DefiniteLength::from(px(420.))));
            assert!(!sheet.show_close_button);
            assert!(!sheet.overlay);
            assert!(!sheet.overlay_closable);
            assert_eq!(sheet.initial_focus, Some(initial_focus));
        });
    }

    #[test]
    fn metrics_match_pinned_sheet_presets() {
        let vega = SheetMetrics::for_density(Density::Standard);
        let nova = SheetMetrics::for_density(Density::Compact);
        let maia = SheetMetrics::for_density(Density::Comfortable);

        assert_eq!(vega.section_padding, px(16.));
        assert_eq!(vega.header_gap, px(6.));
        assert_eq!(vega.close_inset, px(16.));
        assert!(!vega.title_is_base);

        assert_eq!(nova.section_padding, px(16.));
        assert_eq!(nova.header_gap, px(2.));
        assert_eq!(nova.close_inset, px(12.));
        assert!(nova.title_is_base);

        assert_eq!(maia.section_padding, px(24.));
        assert_eq!(maia.content_gap, px(0.));
        assert_eq!(maia.close_inset, px(16.));
        assert!(maia.title_is_base);
    }

    #[test]
    fn content_motion_is_mirrored_for_every_placement() {
        let distance = px(320.);
        for (placement, expected) in [
            (Placement::Top, point(px(0.), -distance)),
            (Placement::Right, point(distance, px(0.))),
            (Placement::Bottom, point(px(0.), distance)),
            (Placement::Left, point(-distance, px(0.))),
        ] {
            let (enter_from, enter_to) = Sheet::motion_translation(placement, distance, false);
            let (exit_from, exit_to) = Sheet::motion_translation(placement, distance, true);
            assert_eq!(enter_from, expected);
            assert_eq!(enter_to, point(px(0.), px(0.)));
            assert_eq!(exit_from, point(px(0.), px(0.)));
            assert_eq!(exit_to, expected);
        }
    }
}
