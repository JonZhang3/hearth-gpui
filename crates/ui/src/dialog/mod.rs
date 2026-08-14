// Modified from the original gpui-component project in commit 4caf2023.
// Changes:
// - Exported the aligned dialog presentation and accessibility APIs.
mod alert_dialog;
mod content;
mod description;
mod dialog;
mod footer;
mod header;
mod modal;
mod title;

pub use alert_dialog::*;
pub use content::DialogContent;
pub use description::DialogDescription;
pub use dialog::*;
pub use footer::*;
pub use header::DialogHeader;
pub(crate) use modal::modal_overlay;
pub use title::DialogTitle;
