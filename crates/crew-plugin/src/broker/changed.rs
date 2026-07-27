//! What a task did to your files, said without being asked.
//!
//! The pane already REPORTS rather than answers questions — the footer lists
//! running tasks (so `/tasks` went), the working directory leads it (so `/cwd`
//! went), the roster is on it (so `/agents` went). The one thing a user cares
//! about most after an agent runs was still a question they had to ask:
//! `/diff`. An agent that edited four files and said "done" left no trace of
//! WHICH four anywhere on screen.
//!
//! It costs nothing to know. A checkpoint already pins the working tree before
//! every task (`stdio::auto_checkpoint`), so the pre-task tree is sitting in
//! `Session::last_tree`; the answer is one `diff-tree` against it.
use std::path::Path;

use super::checkpoint::{git, worktree_tree};

/// One changed path and git's status letter for it (`A`, `M`, `D`, `R`, …).
pub(crate) type Change = (char, String);

/// Paths that differ from the tree `base`, computed the same way the
/// checkpoint itself is: through a throwaway index, so HEAD, the user's index
/// and every branch are untouched. An empty list means the task changed
/// nothing, which is the common case for a question.
pub(crate) fn since(dir: &Path, base: &str) -> Result<Vec<Change>, String> {
    let now = worktree_tree(dir)?;
    if now == base {
        return Ok(Vec::new());
    }
    let out = git(dir, &["diff-tree", "-r", "--name-status", base, &now], None)?;
    Ok(parse(&out)
        .into_iter()
        .filter(|(_, p)| !is_crew_artifact(p))
        .collect())
}

/// Crew's own bookkeeping, which is not something a task did to YOUR files.
///
/// `sessionlog` rewrites `./.crew/session-live.md` as every reply streams, so
/// without this the answer to "what changed?" would be "the transcript" after
/// every single question — including the questions that touched nothing, which
/// is the case this feature most needs to stay quiet for. It goes unnoticed in
/// crew's own repo, where `.crew/` is gitignored and so never in the tree at
/// all; in a user's repo it is not ignored and would be the only line.
pub(super) fn is_crew_artifact(path: &str) -> bool {
    path == ".crew" || path.starts_with(".crew/")
}

/// The same exclusion as a git pathspec, for the commands that hand the job to
/// git instead of filtering rows themselves (`/diff`, `/commit`, `/review`).
/// One constant, so a view of "your work" can never start including crew's
/// own transcript in one place and not another.
pub(super) const NOT_CREW: &str = ":!.crew";

/// `diff-tree --name-status` rows: a status letter, a tab, a path. Rename and
/// copy rows carry a similarity score (`R096`) and TWO paths; the destination
/// is the one that exists now, which is what a reader wants.
fn parse(out: &str) -> Vec<Change> {
    out.lines()
        .filter_map(|line| {
            let mut cols = line.split('\t');
            let status = cols.next()?.chars().next()?;
            let path = cols.next_back()?.trim();
            (!path.is_empty()).then(|| (status, path.to_string()))
        })
        .collect()
}

/// How many paths to name before summarising the rest. Enough to be the whole
/// answer for an ordinary edit, few enough that the note stays one line.
const NAMED: usize = 4;

/// The note for a finished task, or `None` when it touched nothing.
///
/// `hint` adds where to go next; the caller passes it once per session, for
/// the same reason the checkpoint note is said once — the file list is new
/// information every time, "and here is what /diff does" is not.
pub(crate) fn summary(changes: &[Change], hint: bool) -> Option<String> {
    if changes.is_empty() {
        return None;
    }
    let named: Vec<String> = changes
        .iter()
        .take(NAMED)
        .map(|(st, p)| format!("{} {p}", mark(*st)))
        .collect();
    let mut line = format!(
        "{} file{} changed: {}",
        changes.len(),
        if changes.len() == 1 { "" } else { "s" },
        named.join(", "),
    );
    if let Some(rest) = changes.len().checked_sub(NAMED).filter(|n| *n > 0) {
        line.push_str(&format!(", +{rest} more"));
    }
    if hint {
        line.push_str(" \u{2014} /diff shows them, /restore puts them back");
    }
    Some(line)
}

/// Git's status letter as something readable at a glance. Anything unexpected
/// keeps its letter rather than being flattened into a wrong symbol.
fn mark(status: char) -> String {
    match status {
        'A' => "+".to_string(),
        'D' => "\u{2212}".to_string(), // a real minus, not a hyphen in a path
        'M' | 'T' => "~".to_string(),
        'R' => "\u{2192}".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
#[path = "changed_tests.rs"]
mod repo_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_rows() {
        let out = "M\tsrc/main.rs\nA\tsrc/new.rs\nD\told.rs";
        assert_eq!(
            parse(out),
            vec![
                ('M', "src/main.rs".to_string()),
                ('A', "src/new.rs".to_string()),
                ('D', "old.rs".to_string()),
            ]
        );
    }

    /// A rename row has two paths; the one that exists now is the useful one.
    #[test]
    fn a_rename_reports_where_the_file_ended_up() {
        assert_eq!(
            parse("R096\tsrc/old.rs\tsrc/new.rs"),
            vec![('R', "src/new.rs".to_string())]
        );
    }

    #[test]
    fn blank_and_malformed_rows_are_dropped() {
        assert!(parse("").is_empty());
        assert!(parse("\n\n").is_empty());
        assert!(
            parse("M\t").is_empty(),
            "a status with no path says nothing"
        );
    }

    #[test]
    fn crews_own_files_are_not_the_users_files() {
        assert!(is_crew_artifact(".crew/session-live.md"));
        assert!(is_crew_artifact(".crew/specialists.json"));
        assert!(is_crew_artifact(".crew"));
        // A path that merely BEGINS like one is somebody's real file.
        assert!(!is_crew_artifact(".crewmate/notes.md"));
        assert!(!is_crew_artifact("src/.crew/x"), "only at the repo root");
    }

    #[test]
    fn nothing_changed_is_no_note_at_all() {
        assert_eq!(summary(&[], true), None);
    }

    #[test]
    fn one_file_is_singular_and_named() {
        let s = summary(&[('M', "src/main.rs".into())], false).unwrap();
        assert_eq!(s, "1 file changed: ~ src/main.rs");
    }

    #[test]
    fn the_hint_is_the_callers_to_add() {
        let c = [('A', "a.rs".into())];
        assert!(summary(&c, true).unwrap().contains("/restore"));
        assert!(
            !summary(&c, false).unwrap().contains("/restore"),
            "the second task must not repeat the lesson"
        );
    }

    #[test]
    fn a_long_list_names_a_few_and_counts_the_rest() {
        let changes: Vec<Change> = (0..7).map(|i| ('M', format!("f{i}.rs"))).collect();
        let s = summary(&changes, false).unwrap();
        assert!(s.starts_with("7 files changed: "), "{s}");
        assert!(s.contains("~ f3.rs"), "names the first four: {s}");
        assert!(!s.contains("f4.rs"), "and not the fifth: {s}");
        assert!(s.ends_with(", +3 more"), "{s}");
    }

    #[test]
    fn every_status_reads_as_something() {
        for (st, want) in [('A', "+"), ('D', "\u{2212}"), ('M', "~"), ('R', "\u{2192}")] {
            assert_eq!(mark(st), want);
        }
        // An unexpected letter keeps itself rather than becoming a wrong sign.
        assert_eq!(mark('U'), "U");
    }
}
