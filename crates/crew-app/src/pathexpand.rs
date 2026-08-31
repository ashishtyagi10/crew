//! Path-argument expansion shared by `/dump` and Cmd+click file opening: expand
//! `$VAR`/`${VAR}` and a leading `~`, keep absolute paths, and resolve relative
//! ones against a base directory. Unlike [`crate::cwd::resolve`] it does not
//! canonicalise or require the path to exist (the target may be new).
use std::path::{Path, PathBuf};

/// Expand `arg` to a path against `base`. `$VAR`/`${VAR}` are expanded first,
/// then `~`/`~/x` → `$HOME`; an absolute path is kept; anything else joins `base`.
pub(crate) fn expand_path(base: &Path, arg: &str) -> PathBuf {
    let expanded = crate::envexpand::expand_env(arg);
    let arg = expanded.as_str();
    let home = || std::env::var_os("HOME").map(PathBuf::from);
    if arg == "~" {
        if let Some(h) = home() {
            return h;
        }
    }
    if let Some(rest) = arg.strip_prefix("~/") {
        if let Some(h) = home() {
            return h.join(rest);
        }
    }
    let p = Path::new(arg);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(arg)
    }
}

#[cfg(test)]
#[path = "pathexpand_tests.rs"]
mod tests;
