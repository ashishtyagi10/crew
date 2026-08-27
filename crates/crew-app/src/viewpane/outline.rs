//! Structure worth jumping to in a rendered document.
//!
//! A review is not read top to bottom; it is walked file by file and hunk by
//! hunk. `]` and `[` step that structure, which means the viewer needs to know
//! where the structure IS — and in *rendered* rows, not source lines, because
//! a wrapped line occupies several of them.
/// One place `]` / `[` can land, as a rendered row.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Mark {
    pub row: usize,
    pub label: String,
}

/// The jumpable lines of a unified diff, as `(source line index, label)`:
/// every file header and every hunk. Both, because the first hunk of the next
/// file is the thing a reviewer means by "next" at the end of a file.
pub(crate) fn diff_marks(text: &str) -> Vec<(usize, String)> {
    text.split('\n')
        .enumerate()
        .filter_map(|(i, line)| label(line).map(|l| (i, l)))
        .collect()
}

/// What this line is called in the outline, or `None` if it is not a landmark.
fn label(line: &str) -> Option<String> {
    if let Some(rest) = line.strip_prefix("diff --git ") {
        // `a/src/main.rs b/src/main.rs` — the second path is where it is now,
        // which is the name a rename should be listed under.
        let name = rest.split_whitespace().last().unwrap_or(rest);
        return Some(name.strip_prefix("b/").unwrap_or(name).to_string());
    }
    let rest = line.strip_prefix("@@")?;
    // `@@ -12,7 +12,9 @@ fn main() {` — the context after the closing `@@`
    // is what a person calls the hunk. Without one, the range itself is.
    let (range, context) = rest.split_once("@@")?;
    Some(match context.trim().is_empty() {
        true => format!("@@ {} @@", range.trim()),
        false => context.trim().to_string(),
    })
}

/// The row to scroll to for the next (`down`) or previous mark, relative to
/// `from`. `None` when there is none that way — the view stays where it is
/// rather than wrapping, because a review has an end and jumping back to the
/// top from it is how you lose your place.
pub(crate) fn step(marks: &[Mark], from: usize, down: bool) -> Option<&Mark> {
    match down {
        true => marks.iter().find(|m| m.row > from),
        false => marks.iter().rev().find(|m| m.row < from),
    }
}

#[cfg(test)]
#[path = "outline_tests.rs"]
mod tests;
