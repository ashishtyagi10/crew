//! Print the current crate version.

use self_update::cargo_crate_version;

/// Print the current version.
pub fn print_version() {
    println!("farx {}", cargo_crate_version!());
}
