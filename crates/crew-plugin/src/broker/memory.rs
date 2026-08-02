//! Project memory (à la Claude Code's `#` shortcut): a `#note` line in the
//! `/crew` pane appends a standing preference to `./.crew/memory.md`, and
//! every relay/fan prompt carries the merged memory (user-level
//! `~/.config/crew/memory.md` first, project second) — so the crew follows
//! your conventions without being retold each task. Asking ("what do you
//! remember?") shows it — the memory rides the relay prompt, so any reply
//! surfaces it. Unlike skills (per-task playbooks), memory is always on.
use std::path::{Path, PathBuf};

/// Merged-memory budget interpolated into prompts. Clipped with a marker so a
/// sprawling memory file can't crowd out the task.
const MEM_CAP: usize = 2048;

/// The project dir the memory file lives under. Mirrors `sessionlog::base_dir`
/// exactly, and for the same reason: `CREW_PROJECT_DIR` overrides the process
/// CWD (the test seam); production never sets it.
fn base_dir() -> PathBuf {
    std::env::var("CREW_PROJECT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// `#note` handler: append to the project memory file. Returns the one-line
/// confirmation (or error) to show in the pane.
pub(crate) fn remember(note: &str) -> String {
    remember_at(&base_dir(), note)
}

/// Testable core of [`remember`]: append `- note` to `base/.crew/memory.md`,
/// creating the directory on first use.
pub(crate) fn remember_at(base: &Path, note: &str) -> String {
    let note = note.trim();
    if note.is_empty() {
        return "usage: #<note to remember> — appended to ./.crew/memory.md".into();
    }
    let dir = base.join(".crew");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return format!("memory: cannot create {}: {e}", dir.display());
    }
    let path = dir.join("memory.md");
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    text.push_str(&format!("- {note}\n"));
    match std::fs::write(&path, &text) {
        Ok(()) => format!(
            "remembered ({} note{}) — .crew/memory.md",
            text.lines().count(),
            if text.lines().count() == 1 { "" } else { "s" }
        ),
        Err(e) => format!("memory: cannot write {}: {e}", path.display()),
    }
}

/// The merged memory text: user file then project file, clipped to
/// [`MEM_CAP`]. `None` when neither exists or both are blank.
pub(crate) fn load() -> Option<String> {
    let user = dirs::config_dir().map(|d| d.join("crew").join("memory.md"));
    load_from(user.as_deref(), &base_dir().join(".crew").join("memory.md"))
}

/// Testable core of [`load`].
pub(crate) fn load_from(user: Option<&Path>, project: &Path) -> Option<String> {
    let read = |p: &Path| {
        std::fs::read_to_string(p)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let mut merged = String::new();
    if let Some(u) = user.and_then(read) {
        merged.push_str(&u);
    }
    if let Some(p) = read(project) {
        if !merged.is_empty() {
            merged.push('\n');
        }
        merged.push_str(&p);
    }
    if merged.is_empty() {
        return None;
    }
    if merged.len() > MEM_CAP {
        let mut cut = MEM_CAP;
        while !merged.is_char_boundary(cut) {
            cut -= 1;
        }
        merged.truncate(cut);
        merged.push_str("\n… (memory clipped)");
    }
    Some(merged)
}

/// Prepend the loaded memory to a task prompt; the task passes through
/// untouched when no memory exists.
pub(crate) fn with_memory(task: &str) -> String {
    prepend(load(), task)
}

/// Testable core of [`with_memory`].
pub(crate) fn prepend(mem: Option<String>, task: &str) -> String {
    match mem {
        None => task.to_string(),
        Some(m) => format!(
            "STANDING MEMORY (the user's saved preferences — always follow \
             them):\n{m}\n\nTASK:\n{task}"
        ),
    }
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
