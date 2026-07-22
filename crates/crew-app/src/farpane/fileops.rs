//! Filesystem operations behind the Far function keys: F5 copy and F6 move
//! into the other panel, F7 make-folder, F8 delete to trash. Mutations happen
//! in place and both panels reload so each side reflects the change.
use std::path::Path;

use super::keys::FarAction;
use super::FarPane;

/// F5: copy the active panel's selection into the other panel's directory —
/// synchronously via `std::fs` when both endpoints are local, or via
/// `rclone copy`/`copyto` (async, see `remote::begin_transfer`) when either
/// side is a remote panel.
pub(crate) fn copy(p: &mut FarPane) -> FarAction {
    let src_panel = p.panel(p.active);
    let Some(entry) = src_panel.entries.get(src_panel.sel) else {
        return FarAction::Status("nothing to copy".into());
    };
    if entry.is_parent {
        return FarAction::Status("can't copy the ‘..’ entry".into());
    }
    let name = entry.name.clone();
    let is_dir = entry.is_dir;
    let src = src_panel.loc.child(&name);
    let dst = p.panel(p.other_side()).loc.child(&name);
    if src.is_remote() || dst.is_remote() {
        let argv = super::rclone::argv_copy(&src, &dst, is_dir);
        let note = format!(
            "copying {} \u{2192} {}",
            src.rclone_addr(),
            dst.rclone_addr()
        );
        return p.begin_transfer(argv, "copied", note);
    }
    let src = src.local_path().expect("checked not remote above");
    let dst = dst.local_path().expect("checked not remote above");
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

/// F6: move (rename) the active panel's selection into the other panel's
/// directory — synchronously via `std::fs` when both endpoints are local, or
/// via `rclone move`/`moveto` (async, see `remote::begin_transfer`) when
/// either side is a remote panel.
pub(crate) fn rename_move(p: &mut FarPane) -> FarAction {
    let src_panel = p.panel(p.active);
    let Some(entry) = src_panel.entries.get(src_panel.sel) else {
        return FarAction::Status("nothing to move".into());
    };
    if entry.is_parent {
        return FarAction::Status("can't move the ‘..’ entry".into());
    }
    let name = entry.name.clone();
    let is_dir = entry.is_dir;
    let src = src_panel.loc.child(&name);
    let dst = p.panel(p.other_side()).loc.child(&name);
    if src.is_remote() || dst.is_remote() {
        let argv = super::rclone::argv_move(&src, &dst, is_dir);
        let note = format!(
            "moving {} \u{2192} {}",
            src.rclone_addr(),
            dst.rclone_addr()
        );
        return p.begin_transfer(argv, "moved", note);
    }
    let src = src.local_path().expect("checked not remote above");
    let dst = dst.local_path().expect("checked not remote above");
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

/// F8: send the active panel's selection to the OS trash (recoverable), or
/// run `rclone deletefile`/`purge` for a remote panel.
pub(crate) fn delete(p: &mut FarPane) -> FarAction {
    if p.panel(p.active).loc.is_remote() {
        let panel = p.panel(p.active);
        let Some(entry) = panel.entries.get(panel.sel) else {
            return FarAction::Status("nothing to delete".into());
        };
        if entry.is_parent {
            return FarAction::Status("can't delete the ‘..’ entry".into());
        }
        let target = panel.loc.child(&entry.name);
        let is_dir = entry.is_dir;
        let side = p.active;
        return p.begin_simple(
            super::rclone::argv_delete(&target, is_dir),
            side,
            "deleted",
            format!("deleting {}", target.rclone_addr()),
        );
    }
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

/// F7 (on confirm): create `name` as a directory in the active panel, or run
/// `rclone mkdir` for a remote panel.
pub(crate) fn make_dir(p: &mut FarPane, name: &str) -> FarAction {
    if p.panel(p.active).loc.is_remote() {
        let target = p.panel(p.active).loc.child(name);
        let side = p.active;
        return p.begin_simple(
            super::rclone::argv_mkdir(&target),
            side,
            "created folder",
            format!("mkdir {}", target.rclone_addr()),
        );
    }
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
