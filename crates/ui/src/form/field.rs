// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Added public types: `FieldState`, `FieldBody`, `FieldOrientation`, `FieldSet`,
//   `FieldLegendVariant`, `FieldLegend`, `FieldGroup`, `FieldLabel` and 4 more.
// - Removed public types: `FieldBuilder`.
// - Added public methods: `disabled`, `invalid`, `apply_disabled`, `orientation`, `aria_label`,
//   `aria_description`, `content`, `variant` and 3 more.
// - Removed public methods: `label_indent`, `label_fn`, `description`, `description_fn`.
// - Reworked Field around accessibility semantics and ARIA state, semantic Style Preset geometry
//   and density, invalid and validation state handling.
use std::collections::HashSet;

use gpui::{
    AlignItems, AnyElement, App, Axis, ElementId, FocusHandle, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative,
};

use crate::{
    ActiveTheme as _, Density, Disableable, Size, StyledExt as _, h_flex, separator::Separator,
    v_flex,
};

/// Layout values shared by the Form and Field family.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FieldMetrics {
    pub(super) field_gap: gpui::Pixels,
    pub(super) content_gap: gpui::Pixels,
    pub(super) group_gap: gpui::Pixels,
    pub(super) fieldset_gap: gpui::Pixels,
    pub(super) label_gap: gpui::Pixels,
}

impl FieldMetrics {
    /// Resolves the pinned shadcn geometry through semantic preset density.
    pub(super) fn resolve(density: Density, size: Size) -> Self {
        let mut metrics = match density {
            Density::Compact => Self {
                field_gap: px(8.),
                content_gap: px(2.),
                group_gap: px(20.),
                fieldset_gap: px(16.),
                label_gap: px(8.),
            },
            Density::Standard => Self {
                field_gap: px(12.),
                content_gap: px(4.),
                group_gap: px(28.),
                fieldset_gap: px(24.),
                label_gap: px(8.),
            },
            Density::Comfortable => Self {
                field_gap: px(12.),
                content_gap: px(4.),
                group_gap: px(28.),
                fieldset_gap: px(24.),
                label_gap: px(8.),
            },
        };

        let scale = match size {
            Size::XSmall => 0.75,
            Size::Small => 0.875,
            Size::Large => 1.125,
            _ => 1.,
        };
        metrics.field_gap *= scale;
        metrics.content_gap *= scale;
        metrics.group_gap *= scale;
        metrics.fieldset_gap *= scale;
        metrics
    }
}

#[derive(Clone, Copy)]
pub(super) struct FieldProps {
    pub(super) size: Size,
    pub(super) layout: Axis,
    pub(super) columns: usize,
    pub(super) disabled: bool,
}

impl Default for FieldProps {
    fn default() -> Self {
        Self {
            layout: Axis::Vertical,
            size: Size::default(),
            columns: 1,
            disabled: false,
        }
    }
}

/// Effective state inherited by content rendered inside a Field or FieldSet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FieldState {
    disabled: bool,
    invalid: bool,
    required: bool,
}

impl FieldState {
    fn new(disabled: bool, invalid: bool, required: bool) -> Self {
        Self {
            disabled,
            invalid,
            required,
        }
    }

    /// Returns whether controls in this scope must be disabled.
    pub fn disabled(self) -> bool {
        self.disabled
    }

    /// Returns whether controls in this scope are invalid.
    pub fn invalid(self) -> bool {
        self.invalid
    }

    /// Returns whether labels and controls in this scope are required.
    pub fn required(self) -> bool {
        self.required
    }

    /// Applies the inherited disabled state to a compatible control.
    pub fn apply_disabled<T: Disableable>(self, control: T) -> T {
        control.disabled(self.disabled)
    }
}

/// Children produced by a state-aware Field or FieldSet content builder.
#[derive(Default)]
pub struct FieldBody {
    children: Vec<AnyElement>,
}

impl FieldBody {
    /// Creates an empty body for state-aware Field content.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for FieldBody {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// The orientation of a Field's label and content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FieldOrientation {
    #[default]
    Vertical,
    Horizontal,
}

impl From<FieldOrientation> for Axis {
    fn from(value: FieldOrientation) -> Self {
        match value {
            FieldOrientation::Vertical => Axis::Vertical,
            FieldOrientation::Horizontal => Axis::Horizontal,
        }
    }
}

/// Composable root for one form control and its supporting content.
#[derive(IntoElement)]
pub struct Field {
    id: ElementId,
    props: FieldProps,
    orientation: Option<FieldOrientation>,
    style: StyleRefinement,
    content: Option<Box<dyn FnOnce(FieldState) -> FieldBody>>,
    visible: bool,
    disabled: bool,
    required: bool,
    invalid: bool,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
    align_items: Option<AlignItems>,
    col_span: u16,
    col_start: Option<i16>,
    col_end: Option<i16>,
}

impl Field {
    /// Creates a Field with a stable ID used by GPUI and AccessKit.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            props: FieldProps::default(),
            orientation: None,
            style: StyleRefinement::default(),
            content: None,
            visible: true,
            disabled: false,
            required: false,
            invalid: false,
            aria_label: None,
            aria_description: None,
            align_items: None,
            col_span: 1,
            col_start: None,
            col_end: None,
        }
    }

    /// Overrides the orientation inherited from Form.
    pub fn orientation(mut self, orientation: FieldOrientation) -> Self {
        self.orientation = Some(orientation);
        self
    }

    /// Sets the accessible name for the grouped field.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets the accessible description for the grouped field.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Sets whether the field is invalid.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets whether the field is required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Sets whether the field participates in layout and accessibility.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Builds all Field content with the effective Form and Field state.
    ///
    /// Controls must consume the relevant state explicitly, for example with
    /// `Input::disabled(state.disabled())` and `Input::invalid(state.invalid())`.
    pub fn content(mut self, content: impl FnOnce(FieldState) -> FieldBody + 'static) -> Self {
        self.content = Some(Box::new(content));
        self
    }

    /// Applies Form-owned layout properties while preserving Field overrides.
    pub(super) fn props(mut self, props: FieldProps) -> Self {
        self.props = props;
        self
    }

    /// Aligns the Field's direct children to the start.
    pub fn items_start(mut self) -> Self {
        self.align_items = Some(AlignItems::Start);
        self
    }

    /// Aligns the Field's direct children to the end.
    pub fn items_end(mut self) -> Self {
        self.align_items = Some(AlignItems::End);
        self
    }

    /// Aligns the Field's direct children to the center.
    pub fn items_center(mut self) -> Self {
        self.align_items = Some(AlignItems::Center);
        self
    }

    /// Sets the grid column span used by Form.
    pub fn col_span(mut self, col_span: u16) -> Self {
        self.col_span = col_span.max(1);
        self
    }

    /// Sets the grid column start used by Form.
    pub fn col_start(mut self, col_start: i16) -> Self {
        self.col_start = Some(col_start);
        self
    }

    /// Sets the grid column end used by Form.
    pub fn col_end(mut self, col_end: i16) -> Self {
        self.col_end = Some(col_end);
        self
    }
}

impl Styled for Field {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for Field {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Field {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.visible {
            return gpui::Empty.into_any_element();
        }

        let state = FieldState::new(
            self.disabled || self.props.disabled,
            self.invalid,
            self.required,
        );
        let children = self
            .content
            .map(|content| content(state).children)
            .unwrap_or_default();
        let orientation = self
            .orientation
            .map(Axis::from)
            .unwrap_or(self.props.layout);
        let metrics = FieldMetrics::resolve(cx.theme().style.density, self.props.size);
        let element = if orientation == Axis::Horizontal {
            h_flex().items_start().gap(metrics.field_gap)
        } else {
            v_flex().gap(metrics.field_gap)
        };

        let element = element
            .id(self.id)
            .role(Role::Group)
            .w_full()
            .min_w_0()
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .when_some(self.aria_description, |this, description| {
                this.aria_description(description)
            })
            .when_some(self.align_items, |this, align| {
                this.map(|this| match align {
                    AlignItems::Start => this.items_start(),
                    AlignItems::End => this.items_end(),
                    AlignItems::Center => this.items_center(),
                    AlignItems::Baseline => this.items_baseline(),
                    _ => this,
                })
            })
            .col_span(self.col_span)
            .when_some(self.col_start, |this, start| this.col_start(start))
            .when_some(self.col_end, |this, end| this.col_end(end))
            .when(state.invalid, |this| this.text_color(cx.theme().danger))
            .refine_style(&self.style)
            .children(children);

        crate::accessibility::accessibility_field_state(
            element,
            state.invalid,
            state.disabled,
            state.required,
        )
        .into_any_element()
    }
}

/// Groups related fields and exposes one accessible group surface.
#[derive(IntoElement)]
pub struct FieldSet {
    id: ElementId,
    style: StyleRefinement,
    aria_label: Option<SharedString>,
    disabled: bool,
    content: Option<Box<dyn FnOnce(FieldState) -> FieldBody>>,
}

impl FieldSet {
    /// Creates an empty related-field group with a stable accessibility ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            aria_label: None,
            disabled: false,
            content: None,
        }
    }

    /// Sets the accessible name for the grouped fields.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Builds all grouped content with the effective FieldSet state.
    pub fn content(mut self, content: impl FnOnce(FieldState) -> FieldBody + 'static) -> Self {
        self.content = Some(Box::new(content));
        self
    }
}

impl Disableable for FieldSet {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for FieldSet {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for FieldSet {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = FieldMetrics::resolve(cx.theme().style.density, Size::Medium);
        let state = FieldState::new(self.disabled, false, false);
        let children = self
            .content
            .map(|content| content(state).children)
            .unwrap_or_default();
        let element = v_flex()
            .id(self.id)
            .role(Role::Group)
            .w_full()
            .min_w_0()
            .gap(metrics.fieldset_gap)
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .refine_style(&self.style)
            .children(children);
        crate::accessibility::accessibility_state(element, false, self.disabled, false)
    }
}

/// Typography variant for FieldLegend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FieldLegendVariant {
    #[default]
    Legend,
    Label,
}

/// Heading for a FieldSet.
#[derive(IntoElement)]
pub struct FieldLegend {
    style: StyleRefinement,
    text: SharedString,
    variant: FieldLegendVariant,
}

impl FieldLegend {
    /// Creates a FieldSet heading using the default legend typography.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            style: StyleRefinement::default(),
            text: text.into(),
            variant: FieldLegendVariant::Legend,
        }
    }

    /// Sets the semantic typography variant.
    pub fn variant(mut self, variant: FieldLegendVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Styled for FieldLegend {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for FieldLegend {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .font_medium()
            .when(self.variant == FieldLegendVariant::Legend, |this| {
                this.text_base()
            })
            .when(self.variant == FieldLegendVariant::Label, |this| {
                this.text_sm()
            })
            .refine_style(&self.style)
            .child(self.text)
    }
}

macro_rules! field_container {
    ($name:ident, $gap:ident) => {
        #[derive(IntoElement)]
        pub struct $name {
            style: StyleRefinement,
            children: Vec<AnyElement>,
        }

        impl $name {
            /// Creates an empty compositional container.
            pub fn new() -> Self {
                Self {
                    style: StyleRefinement::default(),
                    children: Vec::new(),
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(elements);
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl RenderOnce for $name {
            fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
                let metrics = FieldMetrics::resolve(cx.theme().style.density, Size::Medium);
                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(metrics.$gap)
                    .refine_style(&self.style)
                    .children(self.children)
            }
        }
    };
}

field_container!(FieldContent, content_gap);

/// Vertical collection of related Fields or selection controls.
#[derive(IntoElement)]
pub struct FieldGroup {
    style: StyleRefinement,
    children: Vec<AnyElement>,
    selection: bool,
}

impl FieldGroup {
    /// Creates a standard vertical group of related Fields.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
            selection: false,
        }
    }

    /// Uses the tighter shadcn spacing for checkbox and radio collections.
    pub fn selection(mut self) -> Self {
        self.selection = true;
        self
    }
}

impl Default for FieldGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for FieldGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for FieldGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for FieldGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = FieldMetrics::resolve(cx.theme().style.density, Size::Medium);
        v_flex()
            .w_full()
            .min_w_0()
            .gap(if self.selection {
                px(12.)
            } else {
                metrics.group_gap
            })
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// Clickable label for a Field control.
#[derive(IntoElement)]
pub struct FieldLabel {
    style: StyleRefinement,
    text: SharedString,
    focus_target: Option<FocusHandle>,
    required: bool,
    disabled: bool,
}

impl FieldLabel {
    /// Creates a visible text label for a Field control.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            style: StyleRefinement::default(),
            text: text.into(),
            focus_target: None,
            required: false,
            disabled: false,
        }
    }

    /// Associates pointer activation with the target control's focus handle.
    pub fn for_focus(mut self, focus_target: &FocusHandle) -> Self {
        self.focus_target = Some(focus_target.clone());
        self
    }

    /// Shows the semantic required indicator next to the label.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
}

impl Styled for FieldLabel {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for FieldLabel {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for FieldLabel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = FieldMetrics::resolve(cx.theme().style.density, Size::Medium);
        div()
            .h_flex()
            .items_center()
            .gap(metrics.label_gap)
            .text_sm()
            .font_medium()
            .line_height(relative(1.))
            .when(self.disabled, |this| this.opacity(0.5))
            .refine_style(&self.style)
            .child(self.text)
            .when(self.required, |this| {
                this.child(div().text_color(cx.theme().danger).child("*"))
            })
            .when_some(
                if self.disabled {
                    None
                } else {
                    self.focus_target
                },
                |this, focus_target| {
                    this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        focus_target.focus(window, cx);
                    })
                },
            )
    }
}

/// Medium-weight title used inside label or choice-card compositions.
#[derive(IntoElement)]
pub struct FieldTitle {
    style: StyleRefinement,
    text: SharedString,
}

impl FieldTitle {
    /// Creates a medium-weight title for compound Field content.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            style: StyleRefinement::default(),
            text: text.into(),
        }
    }
}

impl Styled for FieldTitle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for FieldTitle {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .text_sm()
            .font_medium()
            .refine_style(&self.style)
            .child(self.text)
    }
}

/// Muted supporting text for a Field.
#[derive(IntoElement)]
pub struct FieldDescription {
    style: StyleRefinement,
    text: SharedString,
}

impl FieldDescription {
    /// Creates muted supporting text for a Field.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            style: StyleRefinement::default(),
            text: text.into(),
        }
    }
}

impl Styled for FieldDescription {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for FieldDescription {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .child(self.text)
    }
}

/// Live validation message for a Field.
#[derive(IntoElement)]
pub struct FieldError {
    id: ElementId,
    style: StyleRefinement,
    errors: Vec<SharedString>,
}

impl FieldError {
    /// Creates one live validation message with a stable accessibility ID.
    pub fn new(id: impl Into<ElementId>, error: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            errors: vec![error.into()],
        }
    }

    /// Replaces the messages and removes duplicate text while preserving order.
    pub fn errors(mut self, errors: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        let mut seen = HashSet::new();
        self.errors = errors
            .into_iter()
            .map(Into::into)
            .filter(|error| seen.insert(error.to_string()))
            .collect();
        self
    }
}

impl Styled for FieldError {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for FieldError {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let announcement = self
            .errors
            .iter()
            .map(|error| error.as_ref())
            .collect::<Vec<_>>()
            .join(". ");
        v_flex()
            .id(self.id)
            .role(Role::Alert)
            .aria_label(announcement)
            .text_sm()
            .text_color(cx.theme().danger)
            .refine_style(&self.style)
            .children(self.errors)
    }
}

/// Separator between related Field sections, optionally carrying text.
#[derive(IntoElement)]
pub struct FieldSeparator {
    style: StyleRefinement,
    label: Option<SharedString>,
}

impl FieldSeparator {
    /// Creates an unlabeled horizontal separator between Field sections.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            label: None,
        }
    }

    /// Adds centered text to the separator.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Default for FieldSeparator {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for FieldSeparator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for FieldSeparator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .h(px(20.))
            .flex()
            .items_center()
            .refine_style(&self.style)
            .child(
                Separator::horizontal()
                    .when_some(self.label, |separator, label| separator.label(label)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_match_preset_density_contract() {
        let vega = FieldMetrics::resolve(Density::Standard, Size::Medium);
        let nova = FieldMetrics::resolve(Density::Compact, Size::Medium);
        let maia = FieldMetrics::resolve(Density::Comfortable, Size::Medium);
        assert_eq!(vega.field_gap, px(12.));
        assert_eq!(vega.group_gap, px(28.));
        assert_eq!(nova.field_gap, px(8.));
        assert_eq!(nova.group_gap, px(20.));
        assert_eq!(maia.fieldset_gap, px(24.));
    }

    #[test]
    fn errors_are_deduplicated_in_order() {
        let error =
            FieldError::new("field-error", "unused").errors(["Required", "Required", "Invalid"]);
        assert_eq!(error.errors.len(), 2);
        assert_eq!(error.errors[0].as_ref(), "Required");
        assert_eq!(error.errors[1].as_ref(), "Invalid");
    }

    #[test]
    fn grid_span_is_never_zero() {
        assert_eq!(Field::new("field").col_span(0).col_span, 1);
    }

    #[test]
    fn field_state_exposes_effective_values() {
        let state = FieldState::new(true, true, true);
        assert!(state.disabled());
        assert!(state.invalid());
        assert!(state.required());
    }

    #[test]
    fn form_disabled_state_is_inherited_by_field() {
        let mut field = Field::new("field").disabled(false);
        field.props.disabled = true;
        let state = FieldState::new(
            field.disabled || field.props.disabled,
            field.invalid,
            field.required,
        );
        assert!(state.disabled());
    }

    #[test]
    fn explicit_alignment_overrides_horizontal_default() {
        assert_eq!(Field::new("field").align_items, None);
        assert_eq!(
            Field::new("field").items_center().align_items,
            Some(AlignItems::Center)
        );
    }

    #[gpui::test]
    fn hidden_field_has_no_element_identity(cx: &mut gpui::TestAppContext) {
        use gpui::Element as _;

        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let field = Field::new("hidden-field")
                .visible(false)
                .render(window, cx)
                .into_element();
            assert!(field.id().is_none());
        });
    }
}
