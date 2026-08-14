use std::{rc::Rc, sync::Arc};

use crate::{
    ActiveTheme, Density, Disableable, IconName, Selectable, Sizable, Size, StylePreset,
    StyledExt as _, animation::Lerp, icon::IconNamed, text::Text, tooltip::ComponentTooltip,
    v_flex,
};
use gpui::{
    Animation, AnimationExt, AnyElement, App, Background, Div, ElementId, Hsla, InteractiveElement,
    IntoElement, ParentElement, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Toggled, Window, div, prelude::FluentBuilder as _, px, relative, svg,
};

/// The CSS transition family used by a Checkbox Style Preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckboxMotionKind {
    Colors,
    Shadow,
}

/// Geometry and elevation resolved from the active Style Preset without preset ID checks.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CheckboxMetrics {
    edge: Pixels,
    indicator_edge: Pixels,
    radius: Pixels,
    label_gap: Pixels,
    content_gap: Pixels,
    shadow: bool,
    motion_kind: CheckboxMotionKind,
}

/// Resolves shadcn default geometry and the existing GPUI-specific size extensions.
fn checkbox_metrics(size: Size, style: &StylePreset) -> CheckboxMetrics {
    let edge = match size {
        Size::Size(edge) => edge,
        Size::XSmall => px(12.),
        Size::Small => px(14.),
        Size::Medium => px(16.),
        Size::Large => px(18.),
    };
    let indicator_edge = px((edge.as_f32() - 2.).max(1.));

    CheckboxMetrics {
        edge,
        indicator_edge,
        radius: match style.density {
            Density::Compact => style.radii.sm,
            Density::Standard | Density::Comfortable => (style.radii.sm - px(2.)).max(px(0.)),
        }
        .min(edge * 0.5),
        label_gap: px(8.),
        content_gap: px(12.),
        shadow: style.elevation.enabled && style.density == Density::Standard,
        motion_kind: match style.density {
            Density::Compact => CheckboxMotionKind::Colors,
            Density::Standard | Density::Comfortable => CheckboxMotionKind::Shadow,
        },
    }
}

/// Renderable Checkbox colors captured before a state transition starts.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CheckboxPaintState {
    background: Background,
    border: Hsla,
    foreground: Hsla,
    ring: Hsla,
}

/// Previous and target paint states used to animate controlled Checkbox updates.
#[derive(Debug, Clone, Copy)]
struct CheckboxMotionState {
    target: CheckboxPaintState,
    epoch: u64,
}

impl CheckboxMotionState {
    /// Creates stable motion state without animating the first render.
    fn new(target: CheckboxPaintState) -> Self {
        Self { target, epoch: 0 }
    }

    /// Records a new target and returns the transition endpoints when state changed.
    fn transition_to(
        &mut self,
        target: CheckboxPaintState,
    ) -> Option<(CheckboxPaintState, CheckboxPaintState, u64)> {
        if self.target == target {
            return None;
        }

        let from = self.target;
        self.target = target;
        self.epoch = self.epoch.wrapping_add(1);
        Some((from, target, self.epoch))
    }
}

/// Derives an internal Checkbox element ID without flattening the caller's structural ID.
fn checkbox_child_id(id: &ElementId, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), name.into())
}

/// A Checkbox element.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    base: Div,
    style: StyleRefinement,
    label: Option<Text>,
    aria_label: Option<SharedString>,
    children: Vec<AnyElement>,
    checked: bool,
    indeterminate: bool,
    invalid: bool,
    disabled: bool,
    size: Size,
    tab_stop: bool,
    tab_index: isize,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    tooltip: ComponentTooltip,
}

impl Checkbox {
    /// Create a new Checkbox with the given id.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style: StyleRefinement::default(),
            label: None,
            aria_label: None,
            children: Vec::new(),
            checked: false,
            indeterminate: false,
            invalid: false,
            disabled: false,
            size: Size::default(),
            on_click: None,
            tab_stop: true,
            tab_index: 0,
            tooltip: ComponentTooltip::default(),
        }
    }

    /// Set tooltip text for the checkbox.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip.text = Some((tooltip.into(), None));
        self
    }

    /// Set the label for the checkbox.
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the accessible name used when the checkbox has no visible label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set the checked state for the checkbox.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set the indeterminate state for the checkbox.
    ///
    /// Indeterminate takes precedence over `checked` for rendering and
    /// accessibility. Activating an indeterminate checkbox selects it.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    /// Set whether the checkbox value is invalid.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Set the click handler for the checkbox.
    ///
    /// The `&bool` parameter indicates the new checked state after the click.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set the tab stop for the checkbox, default is true.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Set the tab index for the checkbox, default is 0.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    fn handle_click(
        on_click: &Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
        checked: bool,
        indeterminate: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let new_checked = indeterminate || !checked;
        if let Some(f) = on_click {
            (f)(&new_checked, window, cx);
        }
    }
}

impl InteractiveElement for Checkbox {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}
impl StatefulInteractiveElement for Checkbox {}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl Disableable for Checkbox {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Checkbox {
    fn selected(self, selected: bool) -> Self {
        self.checked(selected)
    }

    fn is_selected(&self) -> bool {
        self.checked
    }
}

impl ParentElement for Checkbox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for Checkbox {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

/// Maps visual checkbox state to the AccessKit tri-state value.
fn checkbox_toggled(checked: bool, indeterminate: bool) -> Toggled {
    if indeterminate {
        Toggled::Mixed
    } else {
        checked.into()
    }
}

/// Renders the static shadcn Checkbox indicator with an explicit semantic foreground color.
fn checkbox_indicator(
    indicator_edge: Pixels,
    selected: bool,
    foreground: Hsla,
) -> impl IntoElement {
    svg()
        .size(indicator_edge)
        .text_color(foreground)
        .when(selected, |this| this.path(IconName::Check.path()))
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let indeterminate = self.indeterminate;
        let selected = checked || indeterminate;

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);
        let focus_visible = is_focused && window.last_input_was_keyboard();
        let metrics = checkbox_metrics(self.size, &cx.theme().style);
        let has_content = !self.children.is_empty();
        let invalid_border = cx
            .theme()
            .danger
            .opacity(if cx.theme().is_dark() { 0.5 } else { 1. });
        let border_color = if self.invalid {
            if selected {
                cx.theme().primary
            } else {
                invalid_border
            }
        } else if focus_visible {
            cx.theme().ring
        } else if selected {
            cx.theme().primary
        } else {
            cx.theme().input
        };
        let ring_visible = self.invalid || focus_visible;
        let ring_width = cx.theme().style.focus.ring_width;
        let ring_inset = ring_width + cx.theme().style.focus.ring_offset;
        let ring_color = if self.invalid {
            cx.theme()
                .danger
                .opacity(if cx.theme().is_dark() { 0.4 } else { 0.2 })
        } else {
            cx.theme().ring.opacity(0.5)
        };
        let background = if selected {
            cx.theme().tokens.primary.background
        } else {
            cx.theme().input_background().into()
        };
        let foreground = if selected {
            cx.theme().primary_foreground
        } else {
            cx.theme().foreground
        };
        let paint = CheckboxPaintState {
            background,
            border: border_color,
            foreground,
            ring: if ring_visible {
                ring_color
            } else {
                ring_color.opacity(0.)
            },
        };
        let motion_key = checkbox_child_id(&self.id, "checkbox-motion");
        let motion_state =
            window.use_keyed_state(motion_key, cx, |_, _| CheckboxMotionState::new(paint));
        let transition = motion_state.update(cx, |state, _| state.transition_to(paint));
        let accessible_label = self
            .aria_label
            .or_else(|| self.label.as_ref().map(|label| label.get_text(cx)));
        let on_click = self.on_click.clone();

        let ring_transition = transition.filter(|(from, to, _)| {
            metrics.motion_kind == CheckboxMotionKind::Shadow && from.ring != to.ring
        });
        let show_ring = ring_visible || ring_transition.is_some();
        let ring = show_ring.then(|| {
            let ring = div()
                .absolute()
                .top(-ring_inset)
                .right(-ring_inset)
                .bottom(-ring_inset)
                .left(-ring_inset)
                .border(ring_width)
                .border_color(paint.ring)
                .rounded(metrics.radius + ring_width);

            if let Some((from, to, epoch)) = ring_transition {
                let easing = cx.theme().style.motion.move_easing;
                let animation_id = checkbox_child_id(&self.id, format!("checkbox-ring-{epoch}"));
                ring.with_animation(
                    animation_id,
                    Animation::new(cx.theme().style.motion.normal())
                        .with_easing(move |delta| easing.sample(delta)),
                    move |this, delta| this.border_color(Lerp::lerp(&from.ring, &to.ring, delta)),
                )
                .into_any_element()
            } else {
                ring.into_any_element()
            }
        });

        let control = div()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .size(metrics.edge)
            .flex_shrink_0()
            .border_1()
            .border_color(paint.border)
            .text_color(paint.foreground)
            .bg(paint.background)
            .rounded(metrics.radius)
            .when(has_content, |this| this.mt_px())
            .when(metrics.shadow, |this| this.shadow_xs())
            .when_some(ring, |this, ring| this.child(ring))
            .child(checkbox_indicator(
                metrics.indicator_edge,
                selected,
                paint.foreground,
            ));

        let color_transition = transition.filter(|(from, to, _)| {
            metrics.motion_kind == CheckboxMotionKind::Colors
                && (from.background != to.background
                    || from.border != to.border
                    || from.foreground != to.foreground)
        });
        let control = if let Some((from, to, epoch)) = color_transition {
            let easing = cx.theme().style.motion.move_easing;
            let solid_backgrounds = from.background.as_solid().zip(to.background.as_solid());
            let animation_id = checkbox_child_id(&self.id, format!("checkbox-colors-{epoch}"));
            control
                .with_animation(
                    animation_id,
                    Animation::new(cx.theme().style.motion.normal())
                        .with_easing(move |delta| easing.sample(delta)),
                    move |this, delta| {
                        this.map(|this| {
                            if let Some((from, to)) = solid_backgrounds {
                                this.bg(Lerp::lerp(&from, &to, delta))
                            } else {
                                this
                            }
                        })
                        .border_color(Lerp::lerp(&from.border, &to.border, delta))
                        .text_color(Lerp::lerp(
                            &from.foreground,
                            &to.foreground,
                            delta,
                        ))
                    },
                )
                .into_any_element()
        } else {
            control.into_any_element()
        };

        let element = self
            .base
            .id(self.id.clone())
            .role(Role::CheckBox)
            .aria_toggled(checkbox_toggled(checked, indeterminate))
            .when_some(accessible_label, |this, label| this.aria_label(label))
            .when(!self.disabled, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_stop(self.tab_stop)
                        .tab_index(self.tab_index),
                )
            })
            .h_flex()
            .gap(if has_content {
                metrics.content_gap
            } else {
                metrics.label_gap
            })
            .when(has_content, |this| this.items_start())
            .when(!has_content, |this| this.items_center())
            .line_height(relative(1.))
            .text_color(cx.theme().foreground)
            .map(|this| match self.size {
                Size::XSmall => this.text_xs(),
                Size::Small | Size::Medium | Size::Size(_) => this.text_sm(),
                Size::Large => this.text_base(),
            })
            .refine_style(&self.style)
            .when(self.disabled, |this| this.opacity(0.5))
            .child(control)
            .when(self.label.is_some() || !self.children.is_empty(), |this| {
                this.child(
                    v_flex()
                        .flex_1()
                        .overflow_hidden()
                        .line_height(relative(1.2))
                        .gap_1()
                        .map(|this| {
                            if let Some(label) = self.label {
                                this.child(
                                    div()
                                        .size_full()
                                        .text_color(cx.theme().foreground)
                                        .font_medium()
                                        .line_height(relative(1.))
                                        .child(label),
                                )
                            } else {
                                this
                            }
                        })
                        .children(self.children),
                )
            })
            .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                // Avoid focus on mouse down.
                window.prevent_default();
            })
            .when(!self.disabled, |this| {
                this.on_click(move |_, window, cx| {
                    window.prevent_default();
                    Self::handle_click(&on_click, checked, indeterminate, window, cx);
                })
            })
            .map(|this| self.tooltip.apply(&self.id, this));

        crate::accessibility::accessibility_state(element, self.invalid, false, self.disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementExt as _;
    use gpui::{
        AppContext as _, Element as _, KeyDownEvent, KeyUpEvent, Keystroke, Render, TestAppContext,
        VisualTestContext, accesskit,
    };
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    #[gpui::test]
    fn checkbox_builder_preserves_public_configuration(_cx: &mut TestAppContext) {
        let checkbox = Checkbox::new("builder-checkbox")
            .label("Visible label")
            .aria_label("Accessible label")
            .checked(true)
            .indeterminate(true)
            .invalid(true)
            .disabled(true)
            .large()
            .tab_stop(false)
            .tab_index(2)
            .tooltip("More information")
            .on_click(|_, _, _| {});

        assert!(checkbox.label.is_some());
        assert_eq!(checkbox.aria_label.as_deref(), Some("Accessible label"));
        assert!(checkbox.checked);
        assert!(checkbox.indeterminate);
        assert!(checkbox.invalid);
        assert!(checkbox.disabled);
        assert_eq!(checkbox.size, Size::Large);
        assert!(!checkbox.tab_stop);
        assert_eq!(checkbox.tab_index, 2);
        assert!(checkbox.tooltip.text.is_some());
        assert!(checkbox.on_click.is_some());
    }

    #[test]
    fn checkbox_metrics_match_builtin_shadcn_presets() {
        let vega = checkbox_metrics(Size::Medium, &StylePreset::vega());
        assert_eq!(vega.edge, px(16.));
        assert_eq!(vega.indicator_edge, px(14.));
        assert_eq!(vega.radius, px(4.));
        assert!(vega.shadow);
        assert_eq!(vega.motion_kind, CheckboxMotionKind::Shadow);

        let nova = checkbox_metrics(Size::Medium, &StylePreset::nova());
        assert_eq!(nova.edge, px(16.));
        assert_eq!(nova.radius, px(4.));
        assert!(!nova.shadow);
        assert_eq!(nova.motion_kind, CheckboxMotionKind::Colors);

        let maia = checkbox_metrics(Size::Medium, &StylePreset::maia());
        assert_eq!(maia.edge, px(16.));
        assert_eq!(maia.radius, px(6.));
        assert!(!maia.shadow);
        assert_eq!(maia.motion_kind, CheckboxMotionKind::Shadow);
    }

    #[test]
    fn checkbox_motion_state_skips_initial_render_and_advances_changed_targets() {
        let initial = CheckboxPaintState {
            background: Hsla::white().into(),
            border: Hsla::red(),
            foreground: Hsla::black(),
            ring: Hsla::transparent_black(),
        };
        let target = CheckboxPaintState {
            background: Hsla::black().into(),
            border: Hsla::black(),
            foreground: Hsla::white(),
            ring: Hsla::red(),
        };
        let mut state = CheckboxMotionState::new(initial);

        assert!(state.transition_to(initial).is_none());
        assert_eq!(state.transition_to(target), Some((initial, target, 1)));
        assert!(state.transition_to(target).is_none());
    }

    #[test]
    fn checkbox_internal_ids_preserve_structural_identity() {
        let structured = ElementId::NamedInteger("foo".into(), 1);
        let textual = ElementId::Name("foo-1".into());

        assert_eq!(structured.to_string(), textual.to_string());
        assert_ne!(
            checkbox_child_id(&structured, "checkbox-motion"),
            checkbox_child_id(&textual, "checkbox-motion")
        );
        assert_ne!(
            checkbox_child_id(&structured, "checkbox-ring-1"),
            checkbox_child_id(&textual, "checkbox-ring-1")
        );
        assert_ne!(
            checkbox_child_id(&structured, "checkbox-colors-1"),
            checkbox_child_id(&textual, "checkbox-colors-1")
        );
    }

    #[test]
    fn indeterminate_state_is_exposed_as_mixed() {
        assert_eq!(checkbox_toggled(false, false), Toggled::False);
        assert_eq!(checkbox_toggled(true, false), Toggled::True);
        assert_eq!(checkbox_toggled(false, true), Toggled::Mixed);
        assert_eq!(checkbox_toggled(true, true), Toggled::Mixed);
    }

    #[derive(Debug, PartialEq)]
    struct CheckboxAccessibility {
        role: Role,
        explicit_label: Option<String>,
        visible_label: Option<String>,
        toggled: Option<Toggled>,
        invalid: bool,
        disabled: bool,
    }

    struct AccessibilityProbe {
        metadata: Arc<Mutex<Option<CheckboxAccessibility>>>,
    }

    impl Render for AccessibilityProbe {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let metadata = self.metadata.clone();
            div().on_prepaint(move |_, window, cx| {
                let mut node = accesskit::Node::new(Role::CheckBox);
                let checkbox = Checkbox::new("accessible-checkbox")
                    .aria_label("Select all rows")
                    .indeterminate(true)
                    .invalid(true)
                    .disabled(true)
                    .render(window, cx)
                    .into_element();
                let role = checkbox
                    .a11y_role()
                    .expect("checkbox must expose its accessibility role");
                checkbox.write_a11y_info(&mut node);

                let mut visible_label_node = accesskit::Node::new(Role::CheckBox);
                Checkbox::new("visible-label-checkbox")
                    .label("Visible option")
                    .checked(true)
                    .render(window, cx)
                    .into_element()
                    .write_a11y_info(&mut visible_label_node);

                *metadata.lock().unwrap() = Some(CheckboxAccessibility {
                    role,
                    explicit_label: node.label().map(ToOwned::to_owned),
                    visible_label: visible_label_node.label().map(ToOwned::to_owned),
                    toggled: node.toggled(),
                    invalid: node.invalid() == Some(accesskit::Invalid::True),
                    disabled: node.is_disabled(),
                });
            })
        }
    }

    #[gpui::test]
    fn checkbox_exposes_name_tristate_invalid_and_disabled(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let metadata = Arc::new(Mutex::new(None));
        let captured = metadata.clone();
        let (_, cx) = cx.add_window_view(move |_, _| AccessibilityProbe { metadata });
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        assert_eq!(
            *captured.lock().unwrap(),
            Some(CheckboxAccessibility {
                role: Role::CheckBox,
                explicit_label: Some("Select all rows".into()),
                visible_label: Some("Visible option".into()),
                toggled: Some(Toggled::Mixed),
                invalid: true,
                disabled: true,
            })
        );
    }

    struct KeyboardFixture {
        calls: Arc<AtomicUsize>,
        checked: Arc<AtomicBool>,
    }

    impl Render for KeyboardFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            let checked = self.checked.clone();
            div()
                .child(
                    Checkbox::new("disabled-checkbox")
                        .label("Disabled")
                        .disabled(true),
                )
                .child(
                    Checkbox::new("keyboard-checkbox")
                        .label("Enable notifications")
                        .on_click(move |value, _, _| {
                            calls.fetch_add(1, Ordering::SeqCst);
                            checked.store(*value, Ordering::SeqCst);
                        }),
                )
        }
    }

    #[gpui::test]
    fn space_activates_enabled_checkbox_once_and_ignores_key_repeat(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let calls = Arc::new(AtomicUsize::new(0));
        let checked = Arc::new(AtomicBool::new(false));
        let captured_calls = calls.clone();
        let captured_checked = checked.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let fixture = cx.new(|_| KeyboardFixture { calls, checked });
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
        assert!(captured_checked.load(Ordering::SeqCst));
    }
}
