//! Filesystem operations behind the Far function keys: F5 copy and F6 move
//! into the other panel, F7 make-folder, F8 delete to trash. Mutations happen
//! in place and both panels reload so each side reflects the change.
use std::path::{Path, PathBuf};

use super::keys::FarAction;
use super::FarPane;

/// F5: copy the active panel's selection into the other panel's directory.
pub(crate) fn copy(p: &mut FarPane) -> FarAction {
    let Some((name, is_dir, src, dst)) = transfer_paths(p) else {
        return FarAction::Status("nothing to copy".into());
    };
    if dst.exists() {
        return FarAction::Status(format!("‘{name}’ already exists in the other panel"));
    }
    let res = if is_dir {
        copy_dir_all(&src, &dst)
    } else {
        std::fs::copy(&src, &dst).map(|_| ())
    };
    match res {
        Ok(()) => {
            p.reload_both();
            FarAction::Status(format!("copied ‘{name}’"))
        }
        Err(e) => FarAction::Status(format!("copy failed: {e}")),
    }
}

/// F6: move (rename) the active panel's selection into the other panel's dir.
pub(crate) fn rename_move(p: &mut FarPane) -> FarAction {
    let Some((name, is_dir, src, dst)) = transfer_paths(p) else {
        return FarAction::Status("nothing to move".into());
    };
    if dst.exists() {
        return FarAction::Status(format!("‘{name}’ already exists in the other panel"));
    }
    // `rename` is atomic on the same filesystem; fall back to copy-then-remove
    // across mounts (where it fails with EXDEV).
    let res = std::fs::rename(&src, &dst).or_else(|_| {
        let copied = if is_dir {
            copy_dir_all(&src, &dst)
        } else {
            std::fs::copy(&src, &dst).map(|_| ())
        };
        copied.and_then(|_| {
            if is_dir {
                std::fs::remove_dir_all(&src)
            } else {
                std::fs::remove_file(&src)
            }
        })
    });
    match res {
        Ok(()) => {
            p.reload_both();
            FarAction::Status(format!("moved ‘{name}’"))
        }
        Err(e) => FarAction::Status(format!("move failed: {e}")),
    }
}

/// F8: send the active panel's selection to the OS trash (recoverable).
pub(crate) fn delete(p: &mut FarPane) -> FarAction {
    let panel = p.panel(p.active);
    let Some(entry) = panel.entries.get(panel.sel) else {
        return FarAction::Status("nothing to delete".into());
    };
    if entry.is_parent {
        return FarAction::Status("can't delete the ‘..’ entry".into());
    }
    let name = entry.name.clone();
    let Some(dir) = panel.loc.local_path() else {
        return FarAction::Status("remote copy/move/delete lands in a later task".into());
    };
    let path = dir.join(&name);
    match trash::delete(&path) {
        Ok(()) => {
            p.reload_both();
            FarAction::Status(format!("deleted ‘{name}’ to trash"))
        }
        Err(e) => FarAction::Status(format!("delete failed: {e}")),
    }
}

/// F7 (on confirm): create `name` as a directory in the active panel.
pub(crate) fn make_dir(p: &mut FarPane, name: &str) -> FarAction {
    let Some(base) = p.panel(p.active).loc.local_path() else {
        return FarAction::Status("remote copy/move/delete lands in a later task".into());
    };
    let dir = base.join(name);
    if dir.exists() {
        return FarAction::Status(format!("‘{name}’ already exists"));
    }
    match std::fs::create_dir(&dir) {
        Ok(()) => {
            p.reload_both();
            FarAction::Status(format!("created ‘{name}/’"))
        }
        Err(e) => FarAction::Status(format!("mkdir failed: {e}")),
    }
}

/// `(name, is_dir, source, destination)` for a copy/move of the active panel's
/// selection into the other panel's directory, or `None` when there's nothing
/// transferable selected (empty list or the `..` row).
fn transfer_paths(p: &FarPane) -> Option<(String, bool, PathBuf, PathBuf)> {
    let src_panel = p.panel(p.active);
    let entry = src_panel.entries.get(src_panel.sel)?;
    if entry.is_parent {
        return None;
    }
    let name = entry.name.clone();
    let src_dir = src_panel.loc.local_path()?;
    let dst_dir = p.panel(p.other_side()).loc.local_path()?;
    let src = src_dir.join(&name);
    let dst = dst_dir.join(&name);
    Some((name, entry.is_dir, src, dst))
}

/// Recursively copy directory `src` to `dst` (std has no built-in for this).
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "fileops_tests.rs"]
mod tests;
