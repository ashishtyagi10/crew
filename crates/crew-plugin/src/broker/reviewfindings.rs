//! A review's findings are checked before they are shown.
//!
//! The reply used to reach the pane untouched, so a finding citing a file not in
//! the tree (or a line past its end), a stray `no findings` under real findings
//! (loop 39's S9 quirk) and a miscounted `verdict:` were all shown as fact. A
//! staff reviewer checks a citation before saying it. Only a provably wrong one
//! is dropped: a file we cannot look up is kept.
use std::path::Path;
use std::process::Command;

use crew_hive::childproc::no_console_window;

/// What looking up a cited file found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cited {
    /// There, with this many lines.
    Lines(usize),
    /// Not on disk and not in the tree's file list.
    Missing,
    /// No file list to tell by: the finding is kept.
    Unknown,
}

/// The reply as shown, plus one note per dropped finding.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Tidied {
    pub text: String,
    pub dropped: Vec<String>,
}

pub const SEVERITIES: &[&str] = &["blocker", "warn", "nit"];

/// A finding bullet split into (severity, file, line): `- blocker — a.rs:4 — why`.
pub fn finding(line: &str) -> Option<(&'static str, String, Option<usize>)> {
    let t = line.trim_start();
    let t = t
        .strip_prefix("- ")
        .or_else(|| t.strip_prefix("* "))
        .or_else(|| t.strip_prefix("• "))
        .or_else(|| {
            t.split_once(". ")
                .filter(|(n, _)| n.parse::<u32>().is_ok())
                .map(|(_, r)| r)
        })?;
    let (sev, rest) = t.split_once(char::is_whitespace)?;
    let sev = sev
        .trim_matches(|c| matches!(c, '*' | '`' | ':' | ','))
        .to_lowercase();
    let sev = SEVERITIES
        .iter()
        .find(|s| sev == **s || (**s == "warn" && sev == "warning"))?;
    // The citation is the first `—`/`–`/`-` separated field after the severity.
    let rest = rest.trim_start_matches(|c: char| c.is_whitespace() || "—–-*`".contains(c));
    let cite = rest.split([' ', ',', '(']).next().unwrap_or("");
    let cite = cite.trim_matches(|c| matches!(c, '`' | '*' | ':' | '.'));
    let (file, line) = match cite.split_once(':') {
        Some((f, l)) => (
            f,
            l.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok(),
        ),
        None => (cite, None),
    };
    (file.contains('.') || file.contains('/')).then(|| (*sev, file.to_string(), line))
}

fn is_stray_no_findings(line: &str) -> bool {
    let t: String = line
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect();
    t.trim().eq_ignore_ascii_case("no findings")
}

pub fn is_verdict(line: &str) -> bool {
    let t = line.trim().trim_start_matches(['*', '`', '_']);
    t.to_lowercase().starts_with("verdict")
}

/// The verdict recounted from the findings that were kept.
fn verdict(counts: [usize; 3]) -> String {
    let parts: Vec<String> = SEVERITIES
        .iter()
        .zip(counts)
        .filter(|(_, n)| *n > 0)
        .map(|(s, n)| format!("{n} {s}"))
        .collect();
    format!("verdict: {}", parts.join(", "))
}

/// Check every finding against `lookup`, drop the false ones, the stray
/// `no findings`, and recount the verdict. A reply with no finding bullets is
/// returned as it came.
pub(crate) fn tidy(reply: &str, lookup: &dyn Fn(&str) -> Cited) -> Tidied {
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    let mut counts = [0usize; 3];
    let mut any = false;
    for line in reply.lines() {
        let Some((sev, file, at)) = finding(line) else {
            kept.push(line.to_string());
            continue;
        };
        any = true;
        let cite = at.map_or(file.clone(), |n| format!("{file}:{n}"));
        match (lookup(&file), at) {
            (Cited::Missing, _) => dropped.push(format!("cited {cite}, no such file")),
            (Cited::Lines(len), Some(n)) if n == 0 || n > len => {
                dropped.push(format!("cited {cite}, past its end ({len} lines)"))
            }
            _ => {
                counts[SEVERITIES.iter().position(|s| *s == sev).unwrap_or(0)] += 1;
                kept.push(line.to_string());
            }
        }
    }
    if !any {
        return Tidied {
            text: reply.to_string(),
            dropped,
        };
    }
    let found = counts.iter().sum::<usize>() > 0;
    let mut text: Vec<String> = kept
        .into_iter()
        .filter(|l| !(found && is_stray_no_findings(l)) && !is_verdict(l))
        .collect();
    while text.last().is_some_and(|l| l.trim().is_empty()) {
        text.pop();
    }
    if found {
        text.push(String::new());
        text.push(verdict(counts));
    } else if !text.iter().any(|l| is_stray_no_findings(l)) {
        text.push("no findings".into());
    }
    Tidied {
        text: text.join("\n").trim_start().to_string(),
        dropped,
    }
}

/// [`tidy`] against the repository at `dir`: a cited path is looked up on
/// disk, then as a suffix of a tracked or untracked file (reviewers cite
/// `lib.rs` for `src/lib.rs`). A path the diff names (a deleted file) is
/// never called missing.
pub(crate) fn tidy_in(dir: &Path, diff: &str, reply: &str) -> Tidied {
    let files: Option<Vec<String>> = no_console_window(&mut Command::new("git"))
        .args(["ls-files", "-co", "--exclude-standard"])
        .current_dir(dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_string)
                .collect()
        });
    let count = |p: &Path| std::fs::read_to_string(p).ok().map(|s| s.lines().count());
    let lookup = |cited: &str| -> Cited {
        let cited = cited.trim_start_matches("./");
        if let Some(n) = count(&dir.join(cited)) {
            return Cited::Lines(n);
        }
        let tail = format!("/{cited}");
        let Some(files) = &files else {
            return Cited::Unknown;
        };
        // Several files can share a basename: the line is checked against the longest.
        let best = files
            .iter()
            .filter(|f| f.ends_with(&tail))
            .filter_map(|f| count(&dir.join(f)))
            .max();
        let in_diff = diff.lines().any(|l| {
            let path = l
                .strip_prefix("--- a/")
                .or_else(|| l.strip_prefix("+++ b/"));
            path.is_some_and(|p| p.trim_end() == cited || p.trim_end().ends_with(&tail))
        });
        match best {
            Some(n) => Cited::Lines(n),
            None if in_diff => Cited::Unknown,
            None => Cited::Missing,
        }
    };
    tidy(reply, &lookup)
}

/// The one smith line for what was dropped, `None` when nothing was.
pub(crate) fn dropped_note(dropped: &[String]) -> Option<String> {
    let n = dropped.len();
    (n > 0).then(|| {
        let s = if n == 1 { "" } else { "s" };
        format!("{n} finding{s} dropped — {}", dropped.join(" · "))
    })
}

#[cfg(test)]
#[path = "reviewfindings_tests.rs"]
mod tests;
