//! The project card: which repository the agents are standing in.
//!
//! Measured live (2026-09-27): asked "what does crew-term do?" in the crew
//! repo, a relay agent described "a collaborative terminal for teams" and a
//! swarm worker SEARCHED THE WEB for the word. Both had the tools to read the
//! crate; neither knew there was a crate. No prompt said where the agent was,
//! what the project is, or what its parts are called — the one thing every
//! agentic tool before crew states up front.
//!
//! So every task carries a short card: the directory, the README's opening
//! paragraph, the workspace's members with each one's own one-line summary,
//! and the top-level entries. Gathered from files only — no subprocess, no
//! model — and cached briefly, because it sits on the path of every message.
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Whole-card budget: the card orients, it does not document.
const CAP: usize = 1_800;
/// Chars of the README's opening paragraph.
const README_CAP: usize = 320;
/// Chars of one member's summary line.
const MEMBER_CAP: usize = 90;
/// Top-level entries named.
const TOP_MAX: usize = 24;
/// How long a card is reused before the files are read again.
const TTL: Duration = Duration::from_secs(30);

/// Entries never worth naming at the top level.
const SKIP: &[&str] = &["target", "node_modules", "dist", "build", "__pycache__"];

/// The card for the broker's project dir, or `None` when there is nothing
/// to say (an empty directory). Cached for [`TTL`].
pub(crate) fn block() -> Option<String> {
    static CACHE: Mutex<Option<(PathBuf, Instant, Option<String>)>> = Mutex::new(None);
    let dir = std::env::var("CREW_PROJECT_DIR")
        .map(PathBuf::from)
        .or_else(|_| std::env::current_dir())
        .ok()?;
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((d, at, card)) = c.as_ref() {
        if *d == dir && at.elapsed() < TTL {
            return card.clone();
        }
    }
    let card = card_at(&dir);
    *c = Some((dir, Instant::now(), card.clone()));
    card
}

/// The card for `dir` — the testable core of [`block`].
pub(crate) fn card_at(dir: &Path) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(r) = readme_gist(dir) {
        parts.push(r);
    }
    let members = members(dir);
    if !members.is_empty() {
        parts.push(format!("parts:\n{}", members.join("\n")));
    }
    let top = top_level(dir);
    if !top.is_empty() {
        parts.push(format!("top level: {}", top.join(", ")));
    }
    if parts.is_empty() {
        return None;
    }
    let body = clip(&parts.join("\n"), CAP);
    Some(format!(
        "PROJECT (the repository you are working in \u{2014} {}; relative paths \
         resolve here, and questions about \u{201c}this project\u{201d} or one of its \
         parts are about THIS code):\n{body}",
        dir.display()
    ))
}

/// The README's first prose paragraph: headings, badges and blank lines
/// skipped, markdown emphasis and link targets shed, clipped.
fn readme_gist(dir: &Path) -> Option<String> {
    let text = ["README.md", "README", "readme.md"]
        .iter()
        .find_map(|n| std::fs::read_to_string(dir.join(n)).ok())?;
    let para: Vec<&str> = text
        .lines()
        .map(str::trim)
        .skip_while(|l| {
            l.is_empty() || l.starts_with('#') || l.starts_with("[!") || l.starts_with("![")
        })
        .take_while(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let flat = plain(&para.join(" "));
    (!flat.is_empty()).then(|| clip(&flat, README_CAP))
}

/// `- path — summary` for each member of a Cargo workspace.
fn members(dir: &Path) -> Vec<String> {
    cargo_members(dir)
        .into_iter()
        .map(|rel| match member_gist(&dir.join(&rel)) {
            Some(g) => format!("- {rel} \u{2014} {}", clip(&g, MEMBER_CAP)),
            None => format!("- {rel}"),
        })
        .collect()
}

/// The `members = [...]` of a Cargo workspace, globs of the form `dir/*`
/// expanded, in the manifest's order.
fn cargo_members(dir: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
        return Vec::new();
    };
    let Some(start) = text.find("members") else {
        return Vec::new();
    };
    let rest = &text[start..];
    let Some(list) = rest
        .find('[')
        .and_then(|a| rest[a..].find(']').map(|b| &rest[a + 1..a + b]))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for raw in list.split(',') {
        let m = raw.trim().trim_matches('"').trim();
        if m.is_empty() || m.starts_with('#') {
            continue;
        }
        match m.strip_suffix("/*") {
            Some(parent) => {
                let mut kids: Vec<String> = std::fs::read_dir(dir.join(parent))
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|e| e.path().join("Cargo.toml").exists())
                    .map(|e| format!("{parent}/{}", e.file_name().to_string_lossy()))
                    .collect();
                kids.sort();
                out.extend(kids);
            }
            None => out.push(m.to_string()),
        }
    }
    out
}

/// A member's own summary: its manifest's `description`, else the first
/// prose paragraph of its crate doc (`//!`), headings skipped — a paragraph,
/// not a line, because a doc sentence wraps ("…holds every UI colour, and").
fn member_gist(path: &Path) -> Option<String> {
    if let Ok(toml) = std::fs::read_to_string(path.join("Cargo.toml")) {
        let desc = toml.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == "description").then(|| v.trim().trim_matches('"').to_string())
        });
        if let Some(d) = desc.filter(|d| !d.is_empty()) {
            return Some(d);
        }
    }
    let src = ["src/lib.rs", "src/main.rs"]
        .iter()
        .find_map(|f| std::fs::read_to_string(path.join(f)).ok())?;
    let para: Vec<&str> = src
        .lines()
        .map(str::trim)
        .take_while(|l| l.starts_with("//!"))
        .filter_map(|l| l.strip_prefix("//!").map(str::trim))
        .skip_while(|l| l.is_empty() || l.starts_with('#'))
        .take_while(|l| !l.is_empty())
        .collect();
    (!para.is_empty()).then(|| plain(&para.join(" ")))
}

/// Non-hidden top-level entries, directories marked `/`, noise skipped.
fn top_level(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with('.') || SKIP.contains(&n.as_str()) {
                return None;
            }
            Some(if e.path().is_dir() {
                format!("{n}/")
            } else {
                n
            })
        })
        .collect();
    names.sort();
    names.truncate(TOP_MAX);
    names
}

/// Markdown shed: `**`, backticks, and `[text](url)` → `text`.
fn plain(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("](") {
        let (head, tail) = rest.split_at(i);
        out.push_str(head);
        rest = tail.find(')').map_or("", |j| &tail[j + 1..]);
    }
    out.push_str(rest);
    out.replace(['[', '`'], "").replace("**", "")
}

/// Char-boundary clip with an ellipsis.
fn clip(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        None => s.to_string(),
        Some((i, _)) => format!("{}\u{2026}", s[..i].trim_end()),
    }
}

#[cfg(test)]
#[path = "projectcard_tests.rs"]
mod tests;
