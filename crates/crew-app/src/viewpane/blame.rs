//! Who last touched each line, in the viewer's gutter.
//!
//! Reading a file in a repository, the question that follows "what does this
//! do" is "when did it become this, and who was there" — and answering it
//! today means leaving the pane, running `git blame`, and reading the file a
//! second time in a shell with none of the viewer's colouring. The answer is
//! per-line data about text already on screen, which is what a gutter is for.
//!
//! ## Runs, not lines
//!
//! A blame column that repeats the same commit down forty rows is forty rows
//! of noise hiding the one row that matters. Every editor's blame view
//! collapses runs for that reason, and [`labels`] does the same: a line is
//! labelled only when its commit differs from the line above it, so the
//! column reads as *boundaries* — this block arrived here, that block arrived
//! there — which is the shape of the question being asked.
//!
//! ## Width
//!
//! A pane is a tile in a grid, not a full-screen editor, so the column has to
//! survive being narrow. It degrades rather than truncating mid-field: the
//! full form is `sha author`, a narrow pane gets the sha alone, and below
//! that there is no honest label and the gutter is not drawn at all (`/blame`
//! says so rather than silently doing nothing).
use std::collections::HashMap;

/// Where one line came from. `sha` is already abbreviated; `author` is the
/// first word of the author name, which is what fits and what people say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Line {
    pub sha: String,
    pub author: String,
}

/// Abbreviated sha length — git's own default for `--abbrev-commit`.
const SHA: usize = 7;

/// Widest author name the column carries. Beyond this a name is truncated;
/// eight columns holds the overwhelming majority of first names whole.
const AUTHOR: usize = 8;

/// The full column: `sha` + a space + `author`.
pub(crate) const WIDE: usize = SHA + 1 + AUTHOR;

/// The narrow column: the sha alone. Below this the column is not drawn.
pub(crate) const NARROW: usize = SHA;

/// Parse `git blame --line-porcelain` output into one entry per line.
///
/// The porcelain format opens each line with `<sha> <orig-line> <final-line>
/// [<count>]`, follows it with header lines (`author X`, `summary X`, …), and
/// closes it with the source line itself prefixed by a TAB. Only the header
/// block of the FIRST line of a commit's run carries `author`; later lines
/// repeat the sha with the headers elided, so the author is remembered per
/// sha rather than re-read.
///
/// A line whose sha is all zeros is an uncommitted change — git's own marker
/// for "not committed yet" — and is labelled as such rather than shown with a
/// meaningless sha.
pub(crate) fn parse(out: &str) -> Vec<Line> {
    let mut by_sha: HashMap<String, String> = HashMap::new();
    let mut lines = Vec::new();
    let mut sha: Option<String> = None;
    for raw in out.lines() {
        if raw.starts_with('\t') {
            // The source line closes the block: emit what the headers said.
            if let Some(s) = sha.take() {
                let author = by_sha.get(&s).cloned().unwrap_or_default();
                lines.push(match s.chars().all(|c| c == '0') {
                    true => Line {
                        sha: "-".repeat(SHA),
                        author: "uncommitted".into(),
                    },
                    false => Line {
                        sha: s.chars().take(SHA).collect(),
                        author,
                    },
                });
            }
            continue;
        }
        if let Some(name) = raw.strip_prefix("author ") {
            if let Some(s) = &sha {
                // The first word is the one that fits and the one people say.
                let first = name.split_whitespace().next().unwrap_or(name);
                by_sha.insert(s.clone(), first.to_string());
            }
            continue;
        }
        // A header block opens with the sha; every other header is ignored.
        if sha.is_none() {
            let head = raw.split(' ').next().unwrap_or("");
            if head.len() >= SHA && head.chars().all(|c| c.is_ascii_hexdigit()) {
                sha = Some(head.to_string());
            }
        }
    }
    lines
}

/// The gutter label for each source line, padded to `width`, with runs from
/// one commit collapsed to their first line. `width` is [`WIDE`] or
/// [`NARROW`]; anything narrower has no honest label and gets none.
pub(crate) fn labels(lines: &[Line], width: usize) -> Vec<String> {
    if width < NARROW {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut prev: Option<&str> = None;
    for l in lines {
        // A run's continuation lines are blank: the column marks where one
        // commit's work ends and the next begins, not how long each run is.
        if prev == Some(l.sha.as_str()) {
            out.push(" ".repeat(width));
            continue;
        }
        prev = Some(&l.sha);
        let mut s = l.sha.clone();
        if width >= WIDE {
            // A gutter that cuts "Ashish Tyagi" to "Ashish T" has invented a
            // person; the ellipsis says the name goes on.
            let author = crate::chatwidth::clip_w(&l.author, AUTHOR);
            s = format!("{s} {author}");
        }
        // Clip before padding: a wide author on a narrow budget must not
        // push the text column right and desynchronise every row.
        let s = crate::chatwidth::clip_w(&s, width);
        out.push(format!("{s:<width$}"));
    }
    out
}

/// The column width `cols` can afford, or `None` when it cannot afford one.
/// The gutter is never allowed past a third of the pane: a blame column that
/// crowds out the code it annotates has answered the wrong question.
pub(crate) fn width_for(cols: usize) -> Option<usize> {
    let budget = cols / 3;
    if budget >= WIDE {
        Some(WIDE)
    } else if budget >= NARROW {
        Some(NARROW)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "blame_tests.rs"]
mod tests;
