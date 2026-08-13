use gpui::{App, Axis, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window};

use crate::{
    ActiveTheme as _, Disableable, Sizable, Size, StyledExt as _,
    form::{Field, FieldMetrics, FieldProps},
    v_flex,
};

/// GPUI-native form layout container for composable Fields.
#[derive(IntoElement)]
pub struct Form {
    style: StyleRefinement,
    fields: Vec<Field>,
    props: FieldProps,
}

impl Form {
    fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            props: FieldProps::default(),
            fields: Vec::new(),
        }
    }

    /// Creates a Form whose Fields inherit horizontal orientation.
    pub fn horizontal() -> Self {
        Self::new().layout(Axis::Horizontal)
    }

    /// Creates a Form whose Fields inherit vertical orientation.
    pub fn vertical() -> Self {
        Self::new().layout(Axis::Vertical)
    }

    /// Sets the orientation inherited by Fields without an explicit override.
    pub fn layout(mut self, layout: Axis) -> Self {
        self.props.layout = layout;
        self
    }

    /// Adds one Field to the form grid.
    pub fn child(mut self, field: impl Into<Field>) -> Self {
        self.fields.push(field.into());
        self
    }

    /// Adds multiple Fields to the form grid.
    pub fn children(mut self, fields: impl IntoIterator<Item = Field>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Sets the grid column count. Values below one resolve to one column.
    pub fn columns(mut self, columns: usize) -> Self {
        self.props.columns = columns.max(1);
        self
    }
}

impl Styled for Form {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Form {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.props.size = size.into();
        self
    }
}

impl Disableable for Form {
    fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }
}

impl RenderOnce for Form {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let props = self.props;
        let metrics = FieldMetrics::resolve(cx.theme().style.density, props.size);

        v_flex()
            .w_full()
            .min_w_0()
            .grid()
            .grid_cols(props.columns.max(1) as u16)
            .gap_x(metrics.fieldset_gap)
            .gap_y(metrics.group_gap)
            .refine_style(&self.style)
            .children(self.fields.into_iter().map(|field| field.props(props)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_are_normalized() {
        assert_eq!(Form::vertical().columns(0).props.columns, 1);
    }

    #[test]
    fn disabled_form_propagates_through_field_props() {
        let form = Form::vertical().disabled(true);
        assert!(form.props.disabled);
    }
}
