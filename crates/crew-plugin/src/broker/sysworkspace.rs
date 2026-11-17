//! Optional path confinement for built-in file tools.
use std::path::{Component, Path, PathBuf};

/// Resolve a tool argument inside Crew's current working directory when enabled.
pub(super) fn checked(path: &str, tool: &str) -> Result<String, String> {
    let path = Path::new(path);
    let resolved = if super::systools::workspace_only() {
        let root = std::env::current_dir()
            .and_then(|p| p.canonicalize())
            .map_err(|e| format!("sys:{tool}: cannot resolve working directory: {e}"))?;
        inside(&root, path).ok_or_else(|| outside(tool, path))?
    } else {
        path.to_path_buf()
    };
    resolved
        .into_os_string()
        .into_string()
        .map_err(|_| format!("sys:{tool}: path is not valid UTF-8"))
}

/// Check a discovered file too: git may list a symlink whose target is elsewhere.
pub(super) fn discovered(path: &Path) -> bool {
    if !super::systools::workspace_only() {
        return true;
    }
    let Ok(root) = std::env::current_dir().and_then(|p| p.canonicalize()) else {
        return false;
    };
    discovered_in_workspace(&root, path)
}

fn discovered_in_workspace(root: &Path, path: &Path) -> bool {
    inside(root, path).is_some()
}

fn inside(root: &Path, path: &Path) -> Option<PathBuf> {
    let relative = if path.is_absolute() {
        path.strip_prefix(root).ok()?.to_path_buf()
    } else {
        path.to_path_buf()
    };
    let mut current = root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => return None,
            Component::CurDir => {}
            Component::ParentDir => {
                if current == root {
                    return None;
                }
                current.pop();
            }
            Component::Normal(name) => {
                let next = current.join(name);
                if std::fs::symlink_metadata(&next).is_ok() {
                    current = next.canonicalize().ok()?;
                    if !current.starts_with(root) {
                        return None;
                    }
                } else {
                    current = next;
                }
            }
        }
    }
    current.starts_with(root).then_some(current)
}

fn outside(tool: &str, path: &Path) -> String {
    format!(
        "sys:{tool} path {} is outside the working directory (CREW_SYS_MODE=workspace)",
        path.display()
    )
}

#[cfg(test)]
#[path = "sysworkspace_tests.rs"]
mod tests;
