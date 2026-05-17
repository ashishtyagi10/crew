//! Self-update support: check GitHub Releases and install new versions
//! into `~/.local/bin` without requiring sudo.

mod check;
mod install;
mod version;

/// GitHub repository owner.
pub(crate) const REPO_OWNER: &str = "ashishtyagi10";
/// GitHub repository name.
pub(crate) const REPO_NAME: &str = "farx";

pub use check::{check_and_auto_update_async, UpdateStatus};
pub use install::perform_update;
pub use version::print_version;
