//! Path-argument expansion shared by `/edit`, `/open`, and `/dump`: expand
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
mod tests {
    use super::expand_path;
    use std::path::{Path, PathBuf};

    #[test]
    fn keeps_absolute_and_joins_relative() {
        let base = Path::new("/work");
        assert_eq!(expand_path(base, "/etc/hosts"), Path::new("/etc/hosts"));
        assert_eq!(expand_path(base, "src/main.rs"), base.join("src/main.rs"));
    }

    #[test]
    fn expands_tilde_and_env() {
        std::env::set_var("HOME", "/home/u");
        assert_eq!(expand_path(Path::new("/x"), "~"), PathBuf::from("/home/u"));
        assert_eq!(
            expand_path(Path::new("/x"), "~/notes.md"),
            PathBuf::from("/home/u/notes.md")
        );
        std::env::set_var("CREW_PE_DIR", "/data");
        // `$VAR` expands, then is treated as the (absolute) path.
        assert_eq!(
            expand_path(Path::new("/x"), "$CREW_PE_DIR/f.txt"),
            PathBuf::from("/data/f.txt")
        );
    }
}
