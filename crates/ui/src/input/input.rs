use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AccessibleAction, Animation, AnimationExt as _, AnyElement, App, Corners, DefiniteLength,
    Edges, EdgesRefinement, ElementId, Entity, Hsla, InteractiveElement as _, IntoElement,
    MouseButton, MouseDownEvent, ParentElement as _, Pixels, Rems, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, TextAlign, Window, div, px, relative,
};
use rust_i18n::t;

use crate::animation::Lerp;
use crate::button::Button;
use crate::input::clear_button;
use crate::native_menu::NativeMenu;
use crate::spinner::Spinner;
use crate::{ActiveTheme, Colorize, Density, MotionEasing, StylePreset, v_flex};
use crate::{IconName, Size};
use crate::{Sizable, StyleSized};
use crate::{StyledExt, h_flex};

use super::{
    InputContentType, InputState, content_type::sync_native_content_type, element::EditorScrollbar,
};

/// Returns `(background, foreground)` colors for input-like components.
pub(crate) fn input_style(disabled: bool, cx: &App) -> (Hsla, Hsla) {
    if disabled {
        (
            cx.theme().input.mix_oklab(cx.theme().transparent, 0.8),
            cx.theme().muted_foreground,
        )
    } else {
        (cx.theme().input_background(), cx.theme().foreground)
    }
}

/// The properties animated by a Style Preset's Input transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputMotionKind {
    Colors,
    Shadow,
    ColorsAndShadow,
}

/// Input-specific presentation derived from semantic Style Preset values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct InputMetrics {
    pub(super) radius: Pixels,
    pub(super) shadow: bool,
    pub(super) motion_kind: InputMotionKind,
    pub(super) light_background_alpha: f32,
    pub(super) dark_background_alpha: f32,
    pub(super) disabled_light_background_alpha: f32,
    pub(super) disabled_dark_background_alpha: f32,
}

/// Resolves Vega, Nova, and Maia Input presentation without branching on preset IDs.
pub(super) fn input_metrics(style: &StylePreset) -> InputMetrics {
    match style.density {
        Density::Standard => InputMetrics {
            radius: style.radii.md,
            shadow: style.elevation.enabled,
            motion_kind: InputMotionKind::Shadow,
            light_background_alpha: 0.,
            dark_background_alpha: 0.3,
            disabled_light_background_alpha: 0.,
            disabled_dark_background_alpha: 0.3,
        },
        Density::Compact => InputMetrics {
            radius: style.radii.xl,
            shadow: false,
            motion_kind: InputMotionKind::Colors,
            light_background_alpha: 0.,
            dark_background_alpha: 0.3,
            disabled_light_background_alpha: 0.5,
            disabled_dark_background_alpha: 0.8,
        },
        Density::Comfortable => InputMetrics {
            radius: style.radii.xl,
            shadow: false,
            motion_kind: InputMotionKind::Colors,
            light_background_alpha: 0.3,
            dark_background_alpha: 0.3,
            disabled_light_background_alpha: 0.3,
            disabled_dark_background_alpha: 0.3,
        },
    }
}

/// Renderable Input colors captured before a state transition begins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct InputPaintState {
    pub(super) background: Hsla,
    pub(super) border: Hsla,
    pub(super) ring: Hsla,
}

/// An active Input transition with enough timing data to resume after rerenders.
#[derive(Debug, Clone, Copy)]
struct ActiveInputTransition {
    from: InputPaintState,
    target: InputPaintState,
    started_at: Instant,
    duration: Duration,
    easing: MotionEasing,
    kind: InputMotionKind,
}

/// A renderable transition segment resolved from the current visual value.
#[derive(Debug, Clone, Copy)]
pub(super) struct InputTransition {
    pub(super) from: InputPaintState,
    pub(super) to: InputPaintState,
    pub(super) duration: Duration,
    pub(super) epoch: u64,
}

/// Previous and active paint state used for interruptible Input transitions.
#[derive(Debug, Clone, Copy)]
pub(super) struct InputMotionState {
    target: InputPaintState,
    active: Option<ActiveInputTransition>,
    epoch: u64,
}

impl InputMotionState {
    /// Creates stable motion state without animating the first render.
    pub(super) fn new(target: InputPaintState) -> Self {
        Self {
            target,
            active: None,
            epoch: 0,
        }
    }

    /// Resolves the currently visible value and clears completed transitions.
    fn current(&mut self, now: Instant) -> InputPaintState {
        let Some(active) = self.active else {
            return self.target;
        };
        let elapsed = now.saturating_duration_since(active.started_at);
        let linear_delta = if active.duration.is_zero() {
            1.
        } else {
            elapsed.as_secs_f32() / active.duration.as_secs_f32()
        };
        let current = interpolate_input_paint(
            active.from,
            active.target,
            active.easing.sample(linear_delta),
            active.kind,
        );
        if linear_delta >= 1. {
            self.active = None;
            active.target
        } else {
            current
        }
    }

    /// Records a target and resumes from the current visual value on interruption.
    pub(super) fn transition_to(
        &mut self,
        target: InputPaintState,
        now: Instant,
        duration: Duration,
        easing: MotionEasing,
        kind: InputMotionKind,
    ) -> Option<InputTransition> {
        let previous_active = self.active;
        let target_unchanged = self.target == target;
        let current = self.current(now);
        if target_unchanged && self.active.is_none() {
            return None;
        }

        self.target = target;
        self.epoch = self.epoch.wrapping_add(1);
        let duration = previous_active
            .map(|active| {
                let elapsed = now
                    .saturating_duration_since(active.started_at)
                    .min(active.duration);
                if target_unchanged {
                    active.duration.saturating_sub(elapsed)
                } else if target == active.from {
                    elapsed
                } else {
                    duration
                }
            })
            .unwrap_or(duration);
        if duration.is_zero() || current == target {
            self.active = None;
            return None;
        }
        self.active = Some(ActiveInputTransition {
            from: current,
            target,
            started_at: now,
            duration,
            easing,
            kind,
        });
        Some(InputTransition {
            from: current,
            to: target,
            duration,
            epoch: self.epoch,
        })
    }
}

/// Interpolates only properties covered by the active Style Preset transition.
fn interpolate_input_paint(
    from: InputPaintState,
    to: InputPaintState,
    delta: f32,
    kind: InputMotionKind,
) -> InputPaintState {
    match kind {
        InputMotionKind::Colors => InputPaintState {
            background: Lerp::lerp(&from.background, &to.background, delta),
            border: Lerp::lerp(&from.border, &to.border, delta),
            ring: to.ring,
        },
        InputMotionKind::Shadow => InputPaintState {
            background: to.background,
            border: to.border,
            ring: Lerp::lerp(&from.ring, &to.ring, delta),
        },
        InputMotionKind::ColorsAndShadow => InputPaintState {
            background: Lerp::lerp(&from.background, &to.background, delta),
            border: Lerp::lerp(&from.border, &to.border, delta),
            ring: Lerp::lerp(&from.ring, &to.ring, delta),
        },
    }
}

/// Keeps the active editing surface visible regardless of how it received focus.
///
/// Text-entry controls require keyboard input after pointer focus, so they retain
/// their focus treatment instead of using the keyboard-only policy of buttons.
pub(super) fn input_focus_visible(focused: bool) -> bool {
    focused
}

/// Returns whether the preset may paint its semantic color-transition surface.
///
/// Caller-provided paint values have higher priority than preset motion. GPUI backgrounds may be
/// arbitrary fills, so they cannot be safely interpolated as semantic solid colors. Disabling the
/// entire color surface also prevents an animated background child from covering a custom border.
pub(super) fn input_uses_semantic_color_motion(style: &StyleRefinement) -> bool {
    style.background.is_none() && style.border_color.is_none()
}

/// Resolves the shared Input-family feedback timing for the target focus state.
pub(super) fn input_motion_timing(ring_visible: bool, cx: &App) -> (Duration, MotionEasing) {
    let duration = if cx.reduce_motion() {
        Duration::ZERO
    } else {
        cx.theme().style.motion.fast()
    };
    let easing = if ring_visible {
        cx.theme().style.motion.enter_easing
    } else {
        cx.theme().style.motion.exit_easing
    };
    (duration, easing)
}

/// Derives an internal Input element ID from the stable state-backed root ID.
pub(super) fn input_child_id(id: &ElementId, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), name.into())
}

/// A text input element bind to an [`InputState`].
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    style: StyleRefinement,
    size: Size,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    height: Option<DefiniteLength>,
    appearance: bool,
    cleanable: bool,
    mask_toggle: bool,
    disabled: bool,
    read_only: bool,
    invalid: bool,
    bordered: bool,
    focus_bordered: bool,
    tab_index: isize,
    content_type: Option<InputContentType>,
    role: Option<Role>,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
    aria_numeric_value: Option<f64>,
    aria_numeric_value_step: Option<f64>,
    aria_min_numeric_value: Option<f64>,
    aria_max_numeric_value: Option<f64>,
    numeric_step_actions: bool,

    /// An optional context menu builder to allow a custom context menu on the input.
    ///
    /// If set, this overrides the built-in context menu.
    context_menu_builder: Option<Rc<dyn Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu>>,
}

impl Sizable for Input {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Input {
    /// Create a new [`Input`] element bind to the [`InputState`].
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
            prefix: None,
            suffix: None,
            height: None,
            appearance: true,
            cleanable: false,
            mask_toggle: false,
            disabled: false,
            read_only: false,
            invalid: false,
            bordered: true,
            focus_bordered: true,
            tab_index: 0,
            content_type: None,
            role: None,
            aria_label: None,
            aria_description: None,
            aria_numeric_value: None,
            aria_numeric_value_step: None,
            aria_min_numeric_value: None,
            aria_max_numeric_value: None,
            numeric_step_actions: false,
            context_menu_builder: None,
        }
    }

    /// Returns the state used by typed input-family composites.
    pub(super) fn state(&self) -> &Entity<InputState> {
        &self.state
    }

    /// Returns the configured disabled state before the element is rendered.
    pub(super) fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Returns the configured invalid state before the element is rendered.
    pub(super) fn is_invalid(&self) -> bool {
        self.invalid
    }

    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set full height of the input (Multi-line only).
    pub fn h_full(mut self) -> Self {
        self.height = Some(relative(1.));
        self
    }

    /// Set height of the input (Multi-line only).
    pub fn h(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Set the appearance of the input field, if false the input field will no border, background.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Set the bordered for the input, default: true
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set focus border for the input, default is true.
    pub fn focus_bordered(mut self, bordered: bool) -> Self {
        self.focus_bordered = bordered;
        self
    }

    /// Set whether to show the clear button when the input field is not empty, default is false.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }

    /// Set to enable toggle button for password mask state.
    pub fn mask_toggle(mut self) -> Self {
        self.mask_toggle = true;
        self
    }

    /// Set the semantic content type for password managers and autofill.
    ///
    /// This is a component-level semantic hint. It does not change the text
    /// value or masked rendering state.
    pub fn content_type(mut self, content_type: InputContentType) -> Self {
        self.content_type = Some(content_type);
        self
    }

    /// Override the accessible role for the input.
    ///
    /// If unset, the role is inferred from multi-line mode and content type.
    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    /// Sets the accessible name announced for the input.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets supporting text announced for the input.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Configures numeric accessibility metadata for a typed input-family composite.
    pub(super) fn numeric_accessibility(
        mut self,
        value: Option<f64>,
        step: Option<f64>,
        min: Option<f64>,
        max: Option<f64>,
        step_actions: bool,
    ) -> Self {
        self.aria_numeric_value = value;
        self.aria_numeric_value_step = step;
        self.aria_min_numeric_value = min;
        self.aria_max_numeric_value = max;
        self.numeric_step_actions = step_actions;
        self
    }

    /// Set to disable the input field.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the input to read-only while preserving focus, selection, and copy.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Set whether the input value is invalid.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Set the tab index for the input, default is 0.
    pub fn tab_index(mut self, index: isize) -> Self {
        self.tab_index = index;
        self
    }

    /// Sets a custom context menu builder for the input, shown as a native OS menu.
    ///
    /// If set, this overrides the built-in right-click context menu.
    pub fn context_menu(
        mut self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }

    fn render_toggle_mask_button(state: &Entity<InputState>, cx: &App) -> impl IntoElement {
        let masked = state.read(cx).masked;
        Button::new("toggle-mask")
            .aria_label(if masked {
                t!("Input.Show Password")
            } else {
                t!("Input.Hide Password")
            })
            .icon(if masked {
                IconName::Eye
            } else {
                IconName::EyeOff
            })
            .xsmall()
            .ghost()
            .tab_stop(false)
            .on_click({
                let state = state.clone();
                move |_, window, cx| {
                    state.update(cx, |state, cx| {
                        state.set_masked(!state.masked, window, cx);
                    })
                }
            })
    }

    fn mouse_down_handler(
        state: Entity<InputState>,
        content_type: Option<InputContentType>,
        disabled: bool,
    ) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
        move |event, window, cx| {
            if disabled {
                return;
            }
            sync_native_content_type(window, content_type, disabled);
            state.update(cx, |state, cx| state.on_mouse_down(event, window, cx));
        }
    }

    fn accessibility_role(
        is_multi_line: bool,
        content_type: Option<InputContentType>,
        role: Option<Role>,
    ) -> Role {
        if let Some(role) = role {
            return role;
        }

        if is_multi_line {
            return Role::MultilineTextInput;
        }

        match content_type {
            None => Role::TextInput,
            Some(InputContentType::TelephoneNumber) => Role::PhoneNumberInput,
            Some(InputContentType::EmailAddress) => Role::EmailInput,
            Some(InputContentType::Url) => Role::UrlInput,
            Some(InputContentType::Password | InputContentType::NewPassword) => Role::PasswordInput,
            Some(InputContentType::DateTime) => Role::DateTimeInput,
            Some(InputContentType::Birthdate) => Role::DateInput,
            Some(
                InputContentType::Name
                | InputContentType::NamePrefix
                | InputContentType::GivenName
                | InputContentType::MiddleName
                | InputContentType::FamilyName
                | InputContentType::NameSuffix
                | InputContentType::Nickname
                | InputContentType::JobTitle
                | InputContentType::OrganizationName
                | InputContentType::Location
                | InputContentType::FullStreetAddress
                | InputContentType::StreetAddressLine1
                | InputContentType::StreetAddressLine2
                | InputContentType::AddressCity
                | InputContentType::AddressState
                | InputContentType::AddressCityAndState
                | InputContentType::Sublocality
                | InputContentType::CountryName
                | InputContentType::PostalCode
                | InputContentType::CreditCardNumber
                | InputContentType::CreditCardName
                | InputContentType::CreditCardGivenName
                | InputContentType::CreditCardMiddleName
                | InputContentType::CreditCardFamilyName
                | InputContentType::CreditCardSecurityCode
                | InputContentType::CreditCardExpiration
                | InputContentType::CreditCardExpirationMonth
                | InputContentType::CreditCardExpirationYear
                | InputContentType::CreditCardType
                | InputContentType::Username
                | InputContentType::OneTimeCode
                | InputContentType::ShipmentTrackingNumber
                | InputContentType::FlightNumber
                | InputContentType::BirthdateDay
                | InputContentType::BirthdateMonth
                | InputContentType::BirthdateYear
                | InputContentType::CellularEid
                | InputContentType::CellularImei,
            ) => Role::TextInput,
        }
    }

    fn exposes_accessibility_value(masked: bool, content_type: Option<InputContentType>) -> bool {
        !masked
            && !matches!(
                content_type,
                Some(InputContentType::Password | InputContentType::NewPassword)
            )
    }

    fn handle_accessibility_set_value(
        state: &Entity<InputState>,
        data: Option<&gpui::accesskit::ActionData>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(gpui::accesskit::ActionData::Value(value)) = data else {
            return;
        };
        state.update(cx, |state, cx| {
            state.replace_all(value.to_string(), window, cx);
        });
    }

    /// Resolves the semantic Input surface for the current theme and state.
    pub(super) fn surface_background(metrics: InputMetrics, disabled: bool, cx: &App) -> Hsla {
        let alpha = match (disabled, cx.theme().is_dark()) {
            (false, false) => metrics.light_background_alpha,
            (false, true) => metrics.dark_background_alpha,
            (true, false) => metrics.disabled_light_background_alpha,
            (true, true) => metrics.disabled_dark_background_alpha,
        };
        cx.theme().input.opacity(alpha)
    }

    /// Resolves expanded corner radii for a layout-neutral outer ring.
    ///
    /// The ring is positioned outside the control by `ring_outset`. Its geometry
    /// must not include the control's border width, otherwise a visible gap is
    /// introduced even when the semantic ring offset is zero.
    pub(super) fn outer_ring_geometry(
        style: &StyleRefinement,
        ring_outset: Pixels,
        window: &Window,
    ) -> StyleRefinement {
        let rem_size = window.rem_size();
        let radii = Corners::<Pixels> {
            top_left: style
                .corner_radii
                .top_left
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
            top_right: style
                .corner_radii
                .top_right
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
            bottom_right: style
                .corner_radii
                .bottom_right
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
            bottom_left: style
                .corner_radii
                .bottom_left
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
        }
        .map(|radius| *radius + ring_outset);

        let mut ring_style = StyleRefinement::default();
        ring_style.corner_radii.top_left = Some(radii.top_left.into());
        ring_style.corner_radii.top_right = Some(radii.top_right.into());
        ring_style.corner_radii.bottom_right = Some(radii.bottom_right.into());
        ring_style.corner_radii.bottom_left = Some(radii.bottom_left.into());
        ring_style
    }

    /// This method must after the refine_style.
    fn render_editor(
        paddings: EdgesRefinement<DefiniteLength>,
        input_state: &Entity<InputState>,
        state: &InputState,
        window: &Window,
    ) -> impl IntoElement {
        let base_size = window.text_style().font_size;
        let rem_size = window.rem_size();

        let paddings = Edges {
            left: paddings
                .left
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
            right: paddings
                .right
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
            top: paddings
                .top
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
            bottom: paddings
                .bottom
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
        };

        state.editor_scrollbar_paddings.set(paddings);
        state.editor_scrollbar_snapshot.set(None);

        v_flex()
            .size_full()
            .children(state.search_panel.clone())
            .child(
                div()
                    .relative()
                    .flex_1()
                    .child(input_state.clone())
                    .child(EditorScrollbar::new(input_state.clone())),
            )
    }
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        const LINE_HEIGHT: Rems = Rems(1.25);
        let text_align = self.style.text.text_align.unwrap_or(TextAlign::Left);
        let uses_semantic_color_motion = input_uses_semantic_color_motion(&self.style);

        self.state.update(cx, |state, _| {
            state.context_menu_builder = self.context_menu_builder.clone();
            state.disabled = self.disabled;
            state.read_only = self.read_only;
            state.size = self.size;

            // Only for single line mode
            if state.mode.is_single_line() {
                state.text_align = text_align;
            }
        });

        let state = self.state.read(cx);
        let content_type = self.content_type;
        let disabled = self.disabled;
        let is_multi_line = state.mode.is_multi_line();
        let accessibility_role = Self::accessibility_role(is_multi_line, content_type, self.role);
        let accessibility_state = self.state.clone();
        // Materializing the whole rope is only observable through the
        // accessibility tree, so skip it when no client is listening.
        let accessibility_value = (window.is_a11y_active()
            && Self::exposes_accessibility_value(state.masked, content_type))
        .then(|| state.text.to_string());
        let focused = state.focus_handle.is_focused(window) && !state.disabled;
        let focus_visible = input_focus_visible(focused);
        if focused {
            sync_native_content_type(window, content_type, state.disabled);
        }

        let metrics = input_metrics(&cx.theme().style);
        let control_metrics = cx.theme().style.controls.for_size(self.size);
        let background = if state.mode.is_code_editor() {
            cx.theme().editor_background()
        } else {
            Self::surface_background(metrics, state.disabled, cx)
        };
        let disabled_opacity = if state.disabled { 0.5 } else { 1. };
        let invalid_border = cx
            .theme()
            .danger
            .opacity(if cx.theme().is_dark() { 0.5 } else { 1. });
        let border = if self.invalid {
            invalid_border
        } else if focus_visible && self.focus_bordered {
            cx.theme().ring
        } else {
            cx.theme().input
        }
        .opacity(disabled_opacity);
        let ring_visible = self.appearance
            && self.bordered
            && (self.invalid || (focus_visible && self.focus_bordered));
        let ring_color = if self.invalid {
            cx.theme()
                .danger
                .opacity(if cx.theme().is_dark() { 0.4 } else { 0.2 })
        } else {
            cx.theme().ring.opacity(0.5)
        }
        .opacity(disabled_opacity);
        let paint = InputPaintState {
            background: background.opacity(disabled_opacity),
            border,
            ring: if ring_visible {
                ring_color
            } else {
                ring_color.opacity(0.)
            },
        };
        let root_id: ElementId = ("input", self.state.entity_id()).into();

        let prefix = self.prefix;
        let suffix = self.suffix;
        let show_clear_button = self.cleanable
            && !state.disabled
            && !state.read_only
            && !state.loading
            && state.text.len() > 0
            && state.mode.is_single_line();
        let has_suffix = suffix.is_some() || state.loading || self.mask_toggle || show_clear_button;
        let appearance = self.appearance;
        let bordered = self.bordered;
        let size = self.size;
        let input_state = self.state.clone();
        let numeric_step_actions = self.numeric_step_actions;

        let mut element = div()
            .id(root_id.clone())
            .role(accessibility_role)
            .when_some(accessibility_value, |this, value| this.aria_value(value))
            .when_some(self.aria_numeric_value, |this, value| {
                this.aria_numeric_value(value)
            })
            .when_some(self.aria_numeric_value_step, |this, step| {
                this.aria_numeric_value_step(step)
            })
            .when_some(self.aria_min_numeric_value, |this, min| {
                this.aria_min_numeric_value(min)
            })
            .when_some(self.aria_max_numeric_value, |this, max| {
                this.aria_max_numeric_value(max)
            })
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .when_some(self.aria_description, |this, description| {
                this.aria_description(description)
            })
            .flex()
            .key_context(crate::input::CONTEXT)
            .track_focus(&state.focus_handle.clone())
            .tab_index(if state.disabled { -1 } else { self.tab_index })
            .when(!state.disabled && !state.read_only, |this| {
                this.on_a11y_action(AccessibleAction::SetValue, move |data, window, cx| {
                    Self::handle_accessibility_set_value(&accessibility_state, data, window, cx);
                })
            })
            .when(
                numeric_step_actions && !state.disabled && !state.read_only,
                |this| {
                    let increment_state = self.state.clone();
                    let decrement_state = self.state.clone();
                    this.on_a11y_action(AccessibleAction::Increment, move |_, window, cx| {
                        increment_state.update(cx, |state, cx| {
                            state.on_number_input_step(super::StepAction::Increment, window, cx);
                        });
                    })
                    .on_a11y_action(
                        AccessibleAction::Decrement,
                        move |_, window, cx| {
                            decrement_state.update(cx, |state, cx| {
                                state.on_number_input_step(
                                    super::StepAction::Decrement,
                                    window,
                                    cx,
                                );
                            });
                        },
                    )
                },
            )
            .when(!state.disabled, |this| {
                this.when(!state.read_only, |this| {
                    this.on_action(window.listener_for(&self.state, InputState::backspace))
                        .on_action(window.listener_for(&self.state, InputState::delete))
                        .on_action(
                            window
                                .listener_for(&self.state, InputState::delete_to_beginning_of_line),
                        )
                        .on_action(
                            window.listener_for(&self.state, InputState::delete_to_end_of_line),
                        )
                        .on_action(
                            window.listener_for(&self.state, InputState::delete_previous_word),
                        )
                        .on_action(window.listener_for(&self.state, InputState::delete_next_word))
                        .on_action(window.listener_for(&self.state, InputState::enter))
                        .on_action(window.listener_for(&self.state, InputState::escape))
                        .on_action(window.listener_for(&self.state, InputState::paste))
                        .on_action(window.listener_for(&self.state, InputState::cut))
                        .on_action(window.listener_for(&self.state, InputState::undo))
                        .on_action(window.listener_for(&self.state, InputState::redo))
                        .when(state.mode.is_multi_line(), |this| {
                            this.on_action(
                                window.listener_for(&self.state, InputState::indent_inline),
                            )
                            .on_action(window.listener_for(&self.state, InputState::outdent_inline))
                            .on_action(window.listener_for(&self.state, InputState::indent_block))
                            .on_action(window.listener_for(&self.state, InputState::outdent_block))
                        })
                        .on_action(
                            window.listener_for(
                                &self.state,
                                InputState::on_action_toggle_code_actions,
                            ),
                        )
                })
                .on_action(window.listener_for(&self.state, InputState::left))
                .on_action(window.listener_for(&self.state, InputState::right))
                .on_action(window.listener_for(&self.state, InputState::select_left))
                .on_action(window.listener_for(&self.state, InputState::select_right))
                .when(state.mode.is_multi_line(), |this| {
                    let result = this
                        .on_action(window.listener_for(&self.state, InputState::up))
                        .on_action(window.listener_for(&self.state, InputState::down))
                        .on_action(window.listener_for(&self.state, InputState::select_up))
                        .on_action(window.listener_for(&self.state, InputState::select_down))
                        .on_action(window.listener_for(&self.state, InputState::page_up))
                        .on_action(window.listener_for(&self.state, InputState::page_down));

                    let result = result.on_action(
                        window.listener_for(&self.state, InputState::on_action_go_to_definition),
                    );

                    result
                })
                .on_action(window.listener_for(&self.state, InputState::select_all))
                .on_action(window.listener_for(&self.state, InputState::select_to_start_of_line))
                .on_action(window.listener_for(&self.state, InputState::select_to_end_of_line))
                .on_action(window.listener_for(&self.state, InputState::select_to_previous_word))
                .on_action(window.listener_for(&self.state, InputState::select_to_next_word))
                .on_action(window.listener_for(&self.state, InputState::home))
                .on_action(window.listener_for(&self.state, InputState::end))
                .on_action(window.listener_for(&self.state, InputState::move_to_start))
                .on_action(window.listener_for(&self.state, InputState::move_to_end))
                .on_action(window.listener_for(&self.state, InputState::move_to_previous_word))
                .on_action(window.listener_for(&self.state, InputState::move_to_next_word))
                .on_action(window.listener_for(&self.state, InputState::select_to_start))
                .on_action(window.listener_for(&self.state, InputState::select_to_end))
                .on_action(window.listener_for(&self.state, InputState::show_character_palette))
                .on_action(window.listener_for(&self.state, InputState::copy))
                .on_action(window.listener_for(&self.state, InputState::on_action_search))
                .on_key_down(window.listener_for(&self.state, InputState::on_key_down))
                .on_mouse_down(
                    MouseButton::Left,
                    Self::mouse_down_handler(self.state.clone(), content_type, disabled),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    Self::mouse_down_handler(self.state.clone(), content_type, disabled),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    window.listener_for(&self.state, InputState::on_mouse_up),
                )
                .on_mouse_up(
                    MouseButton::Right,
                    window.listener_for(&self.state, InputState::on_mouse_up),
                )
                .on_mouse_move(window.listener_for(&self.state, InputState::on_mouse_move))
                .on_scroll_wheel(window.listener_for(&self.state, InputState::on_scroll_wheel))
            })
            .size_full()
            .line_height(LINE_HEIGHT)
            .input_px(self.size, cx)
            .input_py(self.size, cx)
            .input_h(self.size, cx)
            .input_text_size(self.size)
            .when(!self.disabled, |this| this.cursor_text())
            .items_center()
            .when(state.mode.is_multi_line(), |this| {
                this.h_auto()
                    .when_some(self.height, |this, height| this.h(height))
            })
            .when(appearance, |this| {
                this.bg(if uses_semantic_color_motion {
                    cx.theme().transparent
                } else {
                    paint.background
                })
                .rounded(metrics.radius)
                .when(metrics.shadow, |this| this.shadow_xs())
                .when(bordered, |this| {
                    this.border_color(if uses_semantic_color_motion {
                        cx.theme().transparent
                    } else {
                        paint.border
                    })
                    .border_1()
                })
            })
            .items_center()
            .gap(control_metrics.gap)
            .refine_style(&self.style);

        let motion_key = input_child_id(&root_id, "motion-state");
        let motion_state =
            window.use_keyed_state(motion_key, cx, |_, _| InputMotionState::new(paint));
        let (motion_duration, motion_easing) = input_motion_timing(ring_visible, cx);
        let transition = motion_state.update(cx, |state, _| {
            state.transition_to(
                paint,
                Instant::now(),
                motion_duration,
                motion_easing,
                InputMotionKind::ColorsAndShadow,
            )
        });

        if appearance && uses_semantic_color_motion {
            let mut surface_style = StyleRefinement::default();
            surface_style.corner_radii = element.style().corner_radii.clone();
            surface_style.border_widths = element.style().border_widths.clone();
            let surface = div()
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .left_0()
                .bg(paint.background)
                .border_color(paint.border)
                .refine_style(&surface_style)
                .into_any_element();
            let surface = if let Some(transition) = transition.filter(|transition| {
                transition.from.background != transition.to.background
                    || transition.from.border != transition.to.border
            }) {
                let from = transition.from;
                let to = transition.to;
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .left_0()
                    .bg(from.background)
                    .border_color(from.border)
                    .refine_style(&surface_style)
                    .with_animation(
                        input_child_id(&root_id, format!("surface-{}", transition.epoch)),
                        Animation::new(transition.duration)
                            .with_easing(move |delta| motion_easing.sample(delta)),
                        move |this, delta| {
                            this.bg(Lerp::lerp(&from.background, &to.background, delta))
                                .border_color(Lerp::lerp(&from.border, &to.border, delta))
                        },
                    )
                    .into_any_element()
            } else {
                surface
            };
            element = element.child(surface);
        }

        let ring_transition =
            transition.filter(|transition| transition.from.ring != transition.to.ring);
        if appearance && bordered && (ring_visible || ring_transition.is_some()) {
            let ring_width = cx.theme().style.focus.ring_width;
            let ring_outset = ring_width + cx.theme().style.focus.ring_offset;
            let ring_style = Self::outer_ring_geometry(element.style(), ring_outset, window);
            let ring = div()
                .absolute()
                .top(-ring_outset)
                .right(-ring_outset)
                .bottom(-ring_outset)
                .left(-ring_outset)
                .border(ring_width)
                .border_color(paint.ring)
                .refine_style(&ring_style);
            let ring = if let Some(transition) = ring_transition {
                let from = transition.from;
                let to = transition.to;
                ring.with_animation(
                    input_child_id(&root_id, format!("ring-{}", transition.epoch)),
                    Animation::new(transition.duration)
                        .with_easing(move |delta| motion_easing.sample(delta)),
                    move |this, delta| this.border_color(Lerp::lerp(&from.ring, &to.ring, delta)),
                )
                .into_any_element()
            } else {
                ring.into_any_element()
            };
            element = element.child(ring);
        }

        let state = self.state.read(cx);
        element = element
            .children(prefix.map(|p| {
                div()
                    .when(state.disabled, |this| this.opacity(0.5))
                    .child(p)
            }))
            .when(state.mode.is_multi_line(), |mut this| {
                let paddings = this.style().padding.clone();
                this.child(Self::render_editor(paddings, &self.state, state, window))
            })
            .when(!state.mode.is_multi_line(), |this| {
                this.child(input_state.clone())
            })
            .when(has_suffix, |this| {
                this.pr(size.input_px(cx)).child(
                    h_flex()
                        .id("suffix")
                        .gap(control_metrics.gap)
                        .items_center()
                        .cursor_default()
                        .when(state.disabled, |this| this.opacity(0.5))
                        .when(state.loading, |this| {
                            this.child(Spinner::new().color(cx.theme().muted_foreground))
                        })
                        .when(self.mask_toggle, |this| {
                            this.child(Self::render_toggle_mask_button(&input_state, cx))
                        })
                        .when(show_clear_button, |this| {
                            this.child(clear_button(cx).on_click({
                                let state = input_state.clone();
                                move |_, window, cx| {
                                    state.update(cx, |state, cx| {
                                        state.clean(window, cx);
                                        state.focus(window, cx);
                                    })
                                }
                            }))
                        })
                        .children(suffix),
                )
            });

        crate::accessibility::accessibility_state(
            element,
            self.invalid,
            self.read_only,
            self.disabled,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_types_map_to_accessibility_roles() {
        let cases = [
            (None, Role::TextInput),
            (Some(InputContentType::Name), Role::TextInput),
            (Some(InputContentType::NamePrefix), Role::TextInput),
            (Some(InputContentType::GivenName), Role::TextInput),
            (Some(InputContentType::MiddleName), Role::TextInput),
            (Some(InputContentType::FamilyName), Role::TextInput),
            (Some(InputContentType::NameSuffix), Role::TextInput),
            (Some(InputContentType::Nickname), Role::TextInput),
            (Some(InputContentType::JobTitle), Role::TextInput),
            (Some(InputContentType::OrganizationName), Role::TextInput),
            (Some(InputContentType::Location), Role::TextInput),
            (Some(InputContentType::FullStreetAddress), Role::TextInput),
            (Some(InputContentType::StreetAddressLine1), Role::TextInput),
            (Some(InputContentType::StreetAddressLine2), Role::TextInput),
            (Some(InputContentType::AddressCity), Role::TextInput),
            (Some(InputContentType::AddressState), Role::TextInput),
            (Some(InputContentType::AddressCityAndState), Role::TextInput),
            (Some(InputContentType::Sublocality), Role::TextInput),
            (Some(InputContentType::CountryName), Role::TextInput),
            (Some(InputContentType::PostalCode), Role::TextInput),
            (
                Some(InputContentType::TelephoneNumber),
                Role::PhoneNumberInput,
            ),
            (Some(InputContentType::EmailAddress), Role::EmailInput),
            (Some(InputContentType::Url), Role::UrlInput),
            (Some(InputContentType::CreditCardNumber), Role::TextInput),
            (Some(InputContentType::CreditCardName), Role::TextInput),
            (Some(InputContentType::CreditCardGivenName), Role::TextInput),
            (
                Some(InputContentType::CreditCardMiddleName),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardFamilyName),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardSecurityCode),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardExpiration),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardExpirationMonth),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardExpirationYear),
                Role::TextInput,
            ),
            (Some(InputContentType::CreditCardType), Role::TextInput),
            (Some(InputContentType::Username), Role::TextInput),
            (Some(InputContentType::Password), Role::PasswordInput),
            (Some(InputContentType::NewPassword), Role::PasswordInput),
            (Some(InputContentType::OneTimeCode), Role::TextInput),
            (
                Some(InputContentType::ShipmentTrackingNumber),
                Role::TextInput,
            ),
            (Some(InputContentType::FlightNumber), Role::TextInput),
            (Some(InputContentType::DateTime), Role::DateTimeInput),
            (Some(InputContentType::Birthdate), Role::DateInput),
            (Some(InputContentType::BirthdateDay), Role::TextInput),
            (Some(InputContentType::BirthdateMonth), Role::TextInput),
            (Some(InputContentType::BirthdateYear), Role::TextInput),
            (Some(InputContentType::CellularEid), Role::TextInput),
            (Some(InputContentType::CellularImei), Role::TextInput),
        ];

        for (content_type, role) in cases {
            assert_eq!(Input::accessibility_role(false, content_type, None), role);
        }
    }

    #[test]
    fn multiline_inputs_keep_multiline_accessibility_role() {
        assert_eq!(
            Input::accessibility_role(true, Some(InputContentType::Password), None),
            Role::MultilineTextInput
        );
    }

    #[test]
    fn explicit_accessibility_role_overrides_defaults() {
        assert_eq!(
            Input::accessibility_role(
                false,
                Some(InputContentType::Password),
                Some(Role::TextInput)
            ),
            Role::TextInput
        );
        assert_eq!(
            Input::accessibility_role(
                true,
                Some(InputContentType::Password),
                Some(Role::TextInput)
            ),
            Role::TextInput
        );
    }

    #[gpui::test]
    fn editable_input_offers_accessibility_write_action(cx: &mut gpui::TestAppContext) {
        use crate::ElementExt as _;
        use gpui::{AppContext as _, Element as _, IntoElement as _, Render};
        use std::sync::{Arc, Mutex};

        type EmittedState = Option<(Option<String>, bool)>;

        struct InputA11yProbe {
            state: Entity<InputState>,
            emitted: Arc<Mutex<EmittedState>>,
        }

        impl Render for InputA11yProbe {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                let state = self.state.clone();
                let emitted = self.emitted.clone();
                div().on_prepaint(move |_, window, cx| {
                    let input = Input::new(&state).render(window, cx).into_element();
                    let mut node = gpui::accesskit::Node::new(Role::TextInput);
                    input.write_a11y_info(&mut node);
                    *emitted.lock().unwrap() = Some((
                        node.value().map(ToOwned::to_owned),
                        node.supports_action(AccessibleAction::SetValue),
                    ));
                })
            }
        }

        cx.update(crate::init);
        let emitted = Arc::new(Mutex::new(None));
        let captured = emitted.clone();
        let (probe, cx) = cx.add_window_view(move |window, cx| InputA11yProbe {
            state: cx.new(|cx| InputState::new(window, cx).default_value("initial")),
            emitted,
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        // No assistive technology is attached in tests, so the value stays
        // unmaterialized while `SetValue` is still advertised.
        assert_eq!(*captured.lock().unwrap(), Some((None, true)));

        let state = probe.read_with(cx, |probe, _| probe.state.clone());
        cx.update(|window, cx| {
            Input::handle_accessibility_set_value(&state, None, window, cx);
        });
        assert_eq!(state.read_with(cx, |state, _| state.value()), "initial");

        let action = gpui::accesskit::ActionData::Value("updated".into());
        cx.update(|window, cx| {
            Input::handle_accessibility_set_value(&state, Some(&action), window, cx);
        });
        assert_eq!(state.read_with(cx, |state, _| state.value()), "updated");
    }

    #[gpui::test]
    fn numeric_input_exposes_spin_button_metadata(cx: &mut gpui::TestAppContext) {
        use crate::ElementExt as _;
        use gpui::{AppContext as _, Element as _, IntoElement as _, Render};
        use std::sync::{Arc, Mutex};

        type NumericMetadata = Option<(Option<f64>, Option<f64>, Option<f64>, Option<f64>, bool)>;

        struct NumericA11yProbe {
            state: Entity<InputState>,
            emitted: Arc<Mutex<NumericMetadata>>,
        }

        impl Render for NumericA11yProbe {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                let state = self.state.clone();
                let emitted = self.emitted.clone();
                div().on_prepaint(move |_, window, cx| {
                    let input = Input::new(&state)
                        .role(Role::SpinButton)
                        .numeric_accessibility(Some(5.), Some(0.5), Some(0.), Some(10.), true)
                        .render(window, cx)
                        .into_element();
                    let mut node = gpui::accesskit::Node::new(Role::SpinButton);
                    input.write_a11y_info(&mut node);
                    *emitted.lock().unwrap() = Some((
                        node.numeric_value(),
                        node.numeric_value_step(),
                        node.min_numeric_value(),
                        node.max_numeric_value(),
                        node.supports_action(AccessibleAction::Increment)
                            && node.supports_action(AccessibleAction::Decrement),
                    ));
                })
            }
        }

        cx.update(crate::init);
        let emitted = Arc::new(Mutex::new(None));
        let captured = emitted.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| NumericA11yProbe {
            state: cx.new(|cx| InputState::new(window, cx).default_value("5")),
            emitted,
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        assert_eq!(
            *captured.lock().unwrap(),
            Some((Some(5.), Some(0.5), Some(0.), Some(10.), true))
        );
    }

    #[test]
    fn accessibility_value_is_hidden_for_secret_inputs() {
        assert!(Input::exposes_accessibility_value(false, None));
        assert!(!Input::exposes_accessibility_value(true, None));
        assert!(!Input::exposes_accessibility_value(
            false,
            Some(InputContentType::Password)
        ));
        assert!(!Input::exposes_accessibility_value(
            false,
            Some(InputContentType::NewPassword)
        ));
    }

    #[test]
    fn input_metrics_match_builtin_shadcn_presets() {
        let vega = input_metrics(&StylePreset::vega());
        assert_eq!(vega.radius, px(8.));
        assert!(vega.shadow);
        assert_eq!(vega.motion_kind, InputMotionKind::Shadow);
        assert_eq!(vega.light_background_alpha, 0.);
        assert_eq!(vega.dark_background_alpha, 0.3);

        let nova = input_metrics(&StylePreset::nova());
        assert_eq!(nova.radius, px(10.));
        assert!(!nova.shadow);
        assert_eq!(nova.motion_kind, InputMotionKind::Colors);
        assert_eq!(nova.disabled_light_background_alpha, 0.5);
        assert_eq!(nova.disabled_dark_background_alpha, 0.8);

        let maia = input_metrics(&StylePreset::maia());
        assert_eq!(maia.radius, px(18.));
        assert!(!maia.shadow);
        assert_eq!(maia.motion_kind, InputMotionKind::Colors);
        assert_eq!(maia.light_background_alpha, 0.3);
        assert_eq!(maia.disabled_light_background_alpha, 0.3);
    }

    #[test]
    fn input_motion_state_skips_initial_render_and_advances_changed_targets() {
        let initial = InputPaintState {
            background: Hsla::white(),
            border: Hsla::black(),
            ring: Hsla::transparent_black(),
        };
        let target = InputPaintState {
            background: Hsla::black(),
            border: Hsla::red(),
            ring: Hsla::red(),
        };
        let mut state = InputMotionState::new(initial);
        let now = Instant::now();
        let duration = Duration::from_millis(100);

        assert!(
            state
                .transition_to(
                    initial,
                    now,
                    duration,
                    MotionEasing::Linear,
                    InputMotionKind::Colors,
                )
                .is_none()
        );
        let transition = state
            .transition_to(
                target,
                now,
                duration,
                MotionEasing::Linear,
                InputMotionKind::Colors,
            )
            .unwrap();
        assert_eq!(transition.from, initial);
        assert_eq!(transition.to, target);
        assert_eq!(transition.duration, duration);
        assert_eq!(transition.epoch, 1);
    }

    #[test]
    fn combined_input_motion_interpolates_surface_and_ring_together() {
        let from = InputPaintState {
            background: Hsla::white(),
            border: Hsla::black(),
            ring: Hsla::transparent_black(),
        };
        let to = InputPaintState {
            background: Hsla::black(),
            border: Hsla::red(),
            ring: Hsla::red(),
        };

        assert_eq!(
            interpolate_input_paint(from, to, 0.5, InputMotionKind::ColorsAndShadow),
            InputPaintState {
                background: Lerp::lerp(&from.background, &to.background, 0.5),
                border: Lerp::lerp(&from.border, &to.border, 0.5),
                ring: Lerp::lerp(&from.ring, &to.ring, 0.5),
            }
        );
    }

    #[test]
    fn input_motion_resumes_from_current_value_on_rerender_and_reverse() {
        let initial = InputPaintState {
            background: Hsla::white(),
            border: Hsla::white(),
            ring: Hsla::transparent_black(),
        };
        let target = InputPaintState {
            background: Hsla::black(),
            border: Hsla::black(),
            ring: Hsla::red(),
        };
        let now = Instant::now();
        let duration = Duration::from_millis(100);

        let mut rerendered = InputMotionState::new(initial);
        rerendered.transition_to(
            target,
            now,
            duration,
            MotionEasing::Linear,
            InputMotionKind::Colors,
        );
        let resumed = rerendered
            .transition_to(
                target,
                now + Duration::from_millis(50),
                duration,
                MotionEasing::Linear,
                InputMotionKind::Colors,
            )
            .unwrap();
        assert_eq!(
            resumed.from,
            interpolate_input_paint(initial, target, 0.5, InputMotionKind::Colors)
        );
        assert_eq!(resumed.duration, Duration::from_millis(50));

        let mut reversed = InputMotionState::new(initial);
        reversed.transition_to(
            target,
            now,
            duration,
            MotionEasing::Linear,
            InputMotionKind::Shadow,
        );
        let reverse = reversed
            .transition_to(
                initial,
                now + Duration::from_millis(75),
                duration,
                MotionEasing::Linear,
                InputMotionKind::Shadow,
            )
            .unwrap();
        assert_eq!(
            reverse.from,
            interpolate_input_paint(initial, target, 0.75, InputMotionKind::Shadow)
        );
        assert_eq!(reverse.to, initial);
        assert_eq!(reverse.duration, Duration::from_millis(75));
    }

    #[test]
    fn input_motion_reaches_target_immediately_when_reduced() {
        let initial = InputPaintState {
            background: Hsla::white(),
            border: Hsla::white(),
            ring: Hsla::transparent_black(),
        };
        let target = InputPaintState {
            background: Hsla::black(),
            border: Hsla::black(),
            ring: Hsla::red(),
        };
        let mut state = InputMotionState::new(initial);

        assert!(
            state
                .transition_to(
                    target,
                    Instant::now(),
                    Duration::ZERO,
                    MotionEasing::Linear,
                    InputMotionKind::Colors,
                )
                .is_none()
        );
        assert_eq!(state.target, target);
        assert!(state.active.is_none());
    }

    #[test]
    fn input_focus_ring_tracks_editing_focus() {
        assert!(!input_focus_visible(false));
        assert!(input_focus_visible(true));
    }

    #[test]
    fn caller_paint_overrides_disable_semantic_color_surface() {
        let default_style = StyleRefinement::default();
        assert!(input_uses_semantic_color_motion(&default_style));

        let mut background_override = StyleRefinement::default();
        background_override.background = Some(gpui::white().into());
        assert!(!input_uses_semantic_color_motion(&background_override));

        let mut border_override = StyleRefinement::default();
        border_override.border_color = Some(gpui::white());
        assert!(!input_uses_semantic_color_motion(&border_override));
    }

    #[test]
    fn input_internal_ids_preserve_structural_identity() {
        let structured = ElementId::NamedInteger("foo".into(), 1);
        let textual = ElementId::Name("foo-1".into());

        assert_eq!(structured.to_string(), textual.to_string());
        assert_ne!(
            input_child_id(&structured, "motion"),
            input_child_id(&textual, "motion")
        );
    }
}
