mod field;
mod form;

pub use field::*;
pub use form::*;

/// Create a new [`Form`] with a vertical layout.
pub fn v_form() -> Form {
    Form::vertical()
}

/// Create a new [`Form`] with a horizontal layout.
pub fn h_form() -> Form {
    Form::horizontal()
}

/// Creates a new [`Field`] with a stable element ID.
pub fn field(id: impl Into<gpui::ElementId>) -> Field {
    Field::new(id)
}
