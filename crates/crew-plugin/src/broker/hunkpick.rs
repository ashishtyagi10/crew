//! Keeping SOME hunks of a file change the person was asked about.
//!
//! The question shows the diff (`hunkshow`); this is the other half. The preview computed when
//! the question is asked is held, and when the answer names hunks the file is written as the
//! original plus ONLY those. Before writing, the preview is recomputed: if the file or the call's
//! result moved since the question, the numbers the person typed no longer mean what they saw, so
//! nothing is written and the model is told to look again. Guessing here would write a change
//! nobody approved.
use super::hunkdiff::{self, Hunk};
use super::hunkshow;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Hunks chosen for approvals this process asked, keyed by approval id. A process global for the
/// reason the answer mailbox is one (`approval`): the stdin loop delivers, a worker takes.
static CHOSEN: Mutex<BTreeMap<String, Vec<u32>>> = Mutex::new(BTreeMap::new());

/// An answer from the host: the hunks first, then the yes, so a worker that wakes on the yes
/// always finds the hunks that belong to it.
pub(crate) fn deliver(id: &str, granted: bool, hunks: Option<Vec<u32>>) {
    if let Some(h) = hunks {
        CHOSEN
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.to_string(), h);
    }
    super::approval::deliver_answer(id, granted);
}

/// The hunks chosen for `id`, once. Always taken, so a refusal cannot leave one behind.
pub(crate) fn take(id: &str) -> Option<Vec<u32>> {
    CHOSEN.lock().unwrap_or_else(|e| e.into_inner()).remove(id)
}

/// What a file-changing call would do, worked out before it is asked about.
pub(crate) struct Preview {
    name: String,
    args: String,
    /// Where the file really is, after the workspace check.
    path: String,
    old: Option<String>,
    new: String,
    hunks: Vec<Hunk>,
}

/// The preview of a `sys:edit` or `sys:write_file`; `None` for any other call, and for one that
/// would fail anyway (the tool then fails after the answer, as it always did).
pub(crate) fn preview(name: &str, args: &str) -> Option<Preview> {
    let mut v: serde_json::Value = serde_json::from_str(args).ok()?;
    let tool = name
        .strip_prefix("sys:")
        .filter(|t| matches!(*t, "edit" | "write_file"))?;
    let path = super::sysworkspace::checked(v.get("path")?.as_str()?, tool).ok()?;
    let (old, new) = match tool {
        "edit" => {
            v["path"] = path.clone().into();
            let (before, after) = super::editargs::dry(&v).ok()?;
            (Some(before), after)
        }
        _ => {
            let old = match std::fs::read_to_string(&path) {
                Ok(s) => Some(s),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(_) => return None,
            };
            (old, v.get("content")?.as_str()?.to_string())
        }
    };
    let hunks = old
        .as_deref()
        .map_or_else(Vec::new, |o| hunkdiff::diff(o, &new));
    Some(Preview {
        name: name.into(),
        args: args.into(),
        path,
        old,
        new,
        hunks,
    })
}

impl Preview {
    /// The words for the question: what follows the dash, and the rows below it.
    pub(crate) fn more(&self) -> (String, String) {
        hunkshow::show(self.old.as_deref(), &self.new, &self.hunks)
    }
}

/// A change to write in part: the preview and the 1-based hunks to keep.
pub(crate) struct Picked {
    shown: Preview,
    keep: Vec<u32>,
}

/// Turn the host's choice into what to do. `Ok(None)` = run the call whole (no hunks named, or
/// all of them); `Err` = the choice cannot be honoured and nothing may run.
pub(crate) fn choose(
    shown: Option<Preview>,
    keep: Option<Vec<u32>>,
) -> Result<Option<Picked>, String> {
    let Some(keep) = keep else {
        return Ok(None);
    };
    let Some(shown) = shown.filter(|p| !p.hunks.is_empty()) else {
        return Err(
            "hunks were chosen, but this call shows none to choose from, so nothing ran".into(),
        );
    };
    let n = shown.hunks.len() as u32;
    if let Some(k) = keep.iter().find(|&&k| k == 0 || k > n) {
        return Err(format!(
            "hunk {k} does not exist (the change has {n}), so nothing was written"
        ));
    }
    let all = (1..=n).all(|k| keep.contains(&k));
    Ok((!all).then_some(Picked { shown, keep }))
}

impl Picked {
    /// Write the file as it was plus the chosen hunks, and tell the model what landed.
    pub(crate) fn run(&self) -> Result<String, String> {
        let p = &self.shown;
        let again = preview(&p.name, &p.args);
        if !again.is_some_and(|a| a.old == p.old && a.new == p.new) {
            return Err(format!(
                "{} changed after the question was asked, so the hunks the user chose no longer \
                 match it and NOTHING was written \u{2014} read the file again and redo the change",
                p.path
            ));
        }
        let keep = |k: usize| self.keep.contains(&(k as u32 + 1));
        let text = hunkdiff::apply(p.old.as_deref().unwrap_or_default(), &p.hunks, keep);
        std::fs::write(&p.path, text).map_err(|e| format!("{}: {e}", p.path))?;
        let list = |want: bool| {
            let rows: Vec<String> = (0..p.hunks.len())
                .filter(|&k| keep(k) == want)
                .map(|k| {
                    let h = &p.hunks[k];
                    format!(
                        "hunk {} (line {}, -{} +{})",
                        k + 1,
                        h.at + 1,
                        h.old.len(),
                        h.new.len()
                    )
                })
                .collect();
            rows.join(", ")
        };
        Ok(format!(
            "{}: the user kept only some of the change. APPLIED: {}. REJECTED by the user: {} \
             \u{2014} those edits are NOT in the file, so do not assume your whole change landed; \
             read the file before building on it",
            p.path,
            list(true),
            list(false)
        ))
    }
}

#[cfg(test)]
#[path = "hunkpick_tests.rs"]
mod tests;
