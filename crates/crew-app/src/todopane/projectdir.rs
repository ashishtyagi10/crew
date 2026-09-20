//! Where an `@project` IS: the registry and the inference behind it.
//!
//! An `@project` tag was a free-form word. For a todo to run itself the word
//! has to name a directory, and crew must never guess one silently — a
//! coding agent let loose in the wrong checkout is the worst outcome this
//! feature has. So resolution is three rungs, each one something the user
//! can see: a binding they made (`/todo project crew ~/code/crew`, kept in
//! `projects.toml` beside `todos.toml`); a pane already open in a directory
//! of that name, or in a checkout whose git root has that name; a sibling
//! of such a directory (`~/code/crew` when a pane sits in `~/code/hive`).
//! The first success is written back as a binding, so the next resolution
//! is a lookup and the file is the one place to correct a wrong one.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
struct ProjectsFile {
    #[serde(default)]
    projects: BTreeMap<String, PathBuf>,
}

/// `projects.toml` beside the todo list.
pub(crate) fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("projects.toml"))
}

/// The bindings in `p`; a missing or unreadable file is no bindings.
pub(crate) fn load_at(p: Option<&Path>) -> BTreeMap<String, PathBuf> {
    let Some(p) = p else {
        return BTreeMap::new();
    };
    let Ok(text) = std::fs::read_to_string(p) else {
        return BTreeMap::new();
    };
    toml::from_str::<ProjectsFile>(&text)
        .unwrap_or_default()
        .projects
}

/// Rewrite `p` with `projects` (creating the config dir if needed).
pub(crate) fn save_at(p: Option<&Path>, projects: &BTreeMap<String, PathBuf>) {
    let Some(p) = p else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = toml::to_string(&ProjectsFile {
        projects: projects.clone(),
    }) {
        let _ = std::fs::write(p, s);
    }
}

/// The bindings on disk. Read on every run rather than cached: a run is a
/// deliberate, rare act and the file is tiny, so a stale cache would be the
/// only way to be wrong.
pub(crate) fn bound() -> BTreeMap<String, PathBuf> {
    load_at(path().as_deref())
}

/// Bind `name` to `dir` on disk. Never writes the real file under test
/// (the `CrewConfig::save` rule).
pub(crate) fn bind(name: &str, dir: &Path) {
    if cfg!(test) {
        return;
    }
    let mut all = bound();
    all.insert(name.to_lowercase(), dir.to_path_buf());
    save_at(path().as_deref(), &all);
}

/// The checkout `dir` is in: the nearest ancestor (or `dir` itself) holding
/// a `.git`. `None` outside any checkout.
pub(crate) fn git_root(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|a| a.join(".git").exists())
        .map(Path::to_path_buf)
}

fn base_is(p: &Path, name: &str) -> bool {
    p.file_name()
        .is_some_and(|b| b.to_string_lossy().eq_ignore_ascii_case(name))
}

/// The places a project named `name` could be, given the directories crew's
/// panes are in — in the order [`resolve`] tries them, without duplicates.
/// The rungs after the bindings: a pane's directory or its git root wearing
/// the name, then a same-named sibling of either.
pub(crate) fn candidates(name: &str, dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut push = |p: PathBuf| {
        if p.is_dir() && !out.contains(&p) {
            out.push(p);
        }
    };
    let roots: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|d| [Some(d.clone()), git_root(d)])
        .flatten()
        .collect();
    for r in roots.iter().filter(|r| base_is(r, name)) {
        push(r.clone());
    }
    for r in &roots {
        if let Some(parent) = r.parent() {
            let sibling = parent.join(name);
            if base_is(&sibling, name) {
                push(sibling);
            }
        }
    }
    out
}

/// The directory `name` resolves to: a binding first, then the first
/// candidate. `None` is an honest "crew cannot place this project", and the
/// caller says so rather than running anywhere.
pub(crate) fn resolve(
    name: &str,
    bound: &BTreeMap<String, PathBuf>,
    dirs: &[PathBuf],
) -> Option<PathBuf> {
    if let Some(p) = bound
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
    {
        return Some(p);
    }
    candidates(name, dirs).into_iter().next()
}

#[cfg(test)]
#[path = "projectdir_tests.rs"]
mod tests;
