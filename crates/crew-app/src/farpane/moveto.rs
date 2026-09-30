//! F6 in `/far`: Far Manager's "Rename or move" box.
//!
//! F6 used to move the selection into the other panel at once, with nothing
//! asked — and with both panels on the same folder, which is how `/far`
//! opens, that was a refusal and no way to rename at all. Far Manager asks:
//! F6 opens a box whose input is the other panel's folder, so Enter as it
//! stands moves there, a new name renames the file where it is, and any path
//! moves it there. This is that box's logic; `render/movebox.rs` draws it.
use std::path::{Path, PathBuf};

use super::keys::FarAction;
use super::location::Location;
use super::{FarPane, Prompt, PromptKind};

/// F6: open the box on the active panel's selection, its input the other
/// panel's folder.
pub(crate) fn open_move(p: &mut FarPane) -> Option<FarAction> {
    let panel = p.panel(p.active);
    let Some(entry) = panel.entries.get(panel.sel) else {
        return Some(FarAction::Status("nothing to move".into()));
    };
    if entry.is_parent {
        return Some(FarAction::Status("cannot move the ‘..’ entry".into()));
    }
    let offered = folder_text(&p.panel(p.other_side()).loc);
    let kind = PromptKind::Move {
        name: entry.name.clone(),
        is_dir: entry.is_dir,
        offered: offered.clone(),
    };
    p.prompt = Some(Prompt {
        kind,
        input: offered,
    });
    None
}

/// Enter in an open prompt: make the folder (F7), or rename/move (F6). An
/// empty input does nothing, as in Far.
pub(crate) fn submit_prompt(p: &mut FarPane) -> Option<FarAction> {
    let prompt = p.prompt.take()?;
    let input = prompt.input.trim();
    if input.is_empty() {
        return None;
    }
    match prompt.kind {
        PromptKind::MkDir => Some(super::fileops::make_dir(p, input)),
        PromptKind::Move {
            name,
            is_dir,
            offered,
        } => Some(match target(p, &name, input, &offered) {
            Ok(dst) => super::fileops::move_entry(p, &name, is_dir, dst),
            Err(why) => FarAction::Status(why),
        }),
    }
}

/// A folder as the box writes it: `~/…` for a local one, `remote:path` for a
/// remote, ending in a separator so it reads as "into this folder".
pub(crate) fn folder_text(loc: &Location) -> String {
    let s = loc.shown();
    match s.ends_with(['/', '\\', ':']) {
        true => s,
        false => format!("{s}/"),
    }
}

/// Where the typed `input` sends `name`: the other panel while it is left as
/// offered (with or without its last `/`); a bare name renames it in the
/// active panel's folder, local or remote; a path moves it — into the folder
/// it names, or to that exact path when it names none. A path is local only.
pub(crate) fn target(
    p: &FarPane,
    name: &str,
    input: &str,
    offered: &str,
) -> Result<Location, String> {
    let bare = |s: &str| s.trim_end_matches(['/', '\\']).to_string();
    if bare(input) == bare(offered) {
        return Ok(p.panel(p.other_side()).loc.child(name));
    }
    let here = &p.panel(p.active).loc;
    if !input.contains(['/', '\\']) && !matches!(input, "~" | "." | "..") {
        return Ok(here.child(input));
    }
    let Some(dir) = here.local_path() else {
        return Err("on a remote panel, type a new name or keep the offered folder".into());
    };
    let path = resolve(&dir, input);
    let into = input.ends_with(['/', '\\']) || path.is_dir();
    Ok(Location::local(&if into { path.join(name) } else { path }))
}

/// `input` as a path: `~` is home, and a relative path starts from the
/// active panel's folder, not the process's.
fn resolve(dir: &Path, input: &str) -> PathBuf {
    let path = match input {
        "~" => dirs::home_dir().unwrap_or_else(|| PathBuf::from(input)),
        _ => crate::cmdcheck::expand_home(input),
    };
    match path.is_absolute() {
        true => path,
        false => dir.join(path),
    }
}

#[cfg(test)]
#[path = "moveto_tests.rs"]
mod tests;
