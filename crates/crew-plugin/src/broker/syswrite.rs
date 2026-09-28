//! `sys:write_file`: the whole file, and the directories it goes in.
//!
//! A new file in a new directory used to fail, because `std::fs::write` makes
//! no parents: `src/feature/mod.rs` came back `No such file or directory` and
//! the agent spent its next round on a `sys:run mkdir -p`, which is the one
//! built-in that asks for approval. An editor makes the directory when you
//! save into one that is not there yet, and so does this.
//!
//! Only inside the working directory. Every other sys tool takes any path, and
//! this one still writes anywhere a directory already exists, so it is not a
//! sandbox; what it will not do is make new directories outside the project,
//! where a mistyped absolute path would leave them wherever it happened to
//! point and nothing in the task's diff or `git status` would show them.
//!
//! Whether an existing file may be replaced at all is the session's question,
//! not this file's (`readset`): it needs to know what the task has read.
use std::path::{Component, Path, PathBuf};

#[cfg(test)]
#[path = "syswrite_tests.rs"]
mod tests;

/// `sys:write_file`, against the broker's own working directory. With none
/// (it was deleted under the broker) a relative path fails as it always did,
/// and no directory is made.
pub(super) fn write_file(path: &str, content: &str) -> Result<String, String> {
    write_in(&std::env::current_dir().unwrap_or_default(), path, content)
}

/// [`write_file`] against `cwd`, handed in so the rule can be tested in a
/// directory of the test's own: moving the process there would move every
/// other test running beside it.
pub(super) fn write_in(cwd: &Path, path: &str, content: &str) -> Result<String, String> {
    let mut to = cwd.join(path);
    if to
        .parent()
        .is_some_and(|d| !d.as_os_str().is_empty() && !d.is_dir())
    {
        to = make_parents(cwd, path, &to)?;
    }
    std::fs::write(&to, content).map_err(|e| super::syspath::with_hint("write", path, e))?;
    Ok(format!("wrote {} bytes to {path}", content.len()))
}

/// Makes the directories `to` is missing and says where the file now goes, or
/// refuses when they would be made outside `cwd`. The answer is the real
/// path, because a `..` among the missing names means nothing to the OS until
/// the directory before it exists.
fn make_parents(cwd: &Path, path: &str, to: &Path) -> Result<PathBuf, String> {
    let (Some(to), Some(root)) = (real(to), real(cwd)) else {
        return Err(outside(path, cwd));
    };
    let dir = match to.parent() {
        Some(dir) if to.starts_with(&root) => dir,
        _ => return Err(outside(path, &root)),
    };
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("write {path}: could not make {}: {e}", dir.display()))?;
    Ok(to)
}

/// Where `p` really is: its deepest part that exists, as the OS resolves it
/// (symlinks and `..` included), then the rest folded by name, which is safe
/// only because none of the rest exists yet, so none of it can be a link.
/// Compared whole components at a time, so `proj2` is not inside `proj`.
fn real(p: &Path) -> Option<PathBuf> {
    let parts: Vec<Component> = p.components().collect();
    let base = |n: usize| parts[..n].iter().collect::<PathBuf>().canonicalize().ok();
    let n = (1..=parts.len()).rev().find(|&n| base(n).is_some())?;
    let mut out = base(n)?;
    for c in &parts[n..] {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            c => out.push(c),
        }
    }
    Some(out)
}

/// The refusal, with its reason and the way through when the directory really
/// is meant; plus the not-found hint, since a path outside the project whose
/// directory is missing is most often a typo one level up.
fn outside(path: &str, root: &Path) -> String {
    let hint = super::syspath::hint(path).map_or(String::new(), |h| format!(" ({h})"));
    format!(
        "write {path}: its directory does not exist{hint}, and crew makes new directories only inside the working directory, {} \u{2014} outside it a mistyped path would leave them wherever it pointed. Write inside the project, or make the directory with sys:run mkdir -p if it is meant",
        root.display()
    )
}
