//! Which project a file belongs to — the directory a server is started in
//! and told is the workspace. Wrong roots are the usual reason a language
//! server answers "unknown" to everything, so the rule is written down: the
//! nearest ancestor holding a `.git`, else the nearest holding a build
//! manifest, else the file's own directory.
use std::path::{Path, PathBuf};

/// Files that make a directory a project, in no particular order.
const MARKERS: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "go.mod",
    "pyproject.toml",
    "setup.py",
    "tsconfig.json",
];

/// The root for `file`, per the module rule. `file` need not exist, but its
/// ancestors are read.
pub fn for_file(file: &Path) -> PathBuf {
    let start = if file.is_dir() {
        file
    } else {
        file.parent().unwrap_or(file)
    };
    if let Some(git) = start.ancestors().find(|d| d.join(".git").exists()) {
        return git.to_path_buf();
    }
    start
        .ancestors()
        .find(|d| MARKERS.iter().any(|m| d.join(m).is_file()))
        .unwrap_or(start)
        .to_path_buf()
}

#[cfg(test)]
#[path = "root_tests.rs"]
mod tests;
