//! App-weite UI-Bausteine (Ersatz für `src/components`).

pub mod button;
pub mod icons;
mod icons_data;
pub mod markdown;
pub mod modal;
pub mod page_header;
pub mod password_input;
pub mod section_tabs;
pub mod skeleton;
pub mod toast;

pub use button::{Button, ButtonSize, ButtonVariant};
pub use icons::Icon;
#[allow(unused_imports)]
pub use modal::ConfirmDialog;
pub use modal::Modal;
pub use page_header::PageHeader;
pub use password_input::PasswordInput;
pub use skeleton::Skeleton;
pub mod command_palette;
pub mod crash_watch;
pub mod global_search;
pub mod whats_new;
