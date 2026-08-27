//! File references in pane output, marked as the links they already are.
//!
//! Agents cite files constantly — `src/main.rs:42`, `./run.sh`, `Cargo.toml` —
//! and Cmd+click has always resolved them. Nothing said so: a path and the
//! prose around it were the same ink, so the affordance existed and was
//! invisible.
//!
//! They are marked with a **dotted** rule rather than the solid one a URL
//! wears, because they are a different kind of link: a URL leaves for the
//! browser, a path opens here. Same colour, different rule.
use crew_render::CellView;
use crew_theme::deco::DecoLine;

/// Opening punctuation trimmed from a reference's left edge. Deliberately
/// without `.` and `~`: `./deploy.sh` and `~/notes.md` START with theirs.
const LEAD: &str = "\"'`([{<";
/// Closing punctuation and sentence marks trimmed from the right edge.
const TAIL: &str = "\"'`)]}>,;:.!?";

/// Character spans `[start, end)` of the file references in one row.
pub(crate) fn path_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        let (mut a, mut b) = (start, i);
        while a < b && LEAD.contains(chars[a]) {
            a += 1;
        }
        while b > a && TAIL.contains(chars[b - 1]) {
            b -= 1;
        }
        let tok: String = chars[a..b].iter().collect();
        if is_reference(&tok) {
            out.push((a, b));
        }
    }
    out
}

/// Whether `tok` reads as a path this terminal could open.
///
/// Conservative on purpose: a mark on `and/or` or on `e.g.` teaches people to
/// ignore the marks. Two shapes qualify — something with a directory
/// separator in it, or a bare filename with a real extension — and a trailing
/// `:line[:col]` is allowed on either.
pub(crate) fn is_reference(tok: &str) -> bool {
    if tok.contains("://") || tok.starts_with('-') || tok.len() < 3 {
        return false;
    }
    let path = strip_position(tok).0;
    if path.is_empty() {
        return false;
    }
    let has_dir = path.contains('/');
    let named = path.rsplit('/').next().is_some_and(has_extension);
    // A directory reference needs the separator to be believable; a bare name
    // needs an extension. `src/main.rs` has both, `and/or` has neither.
    named || (has_dir && tok != path)
}

/// A stem and a 2–6 character extension that is not all digits — enough to
/// pass `main.rs` and `Cargo.toml`, and to fail `e.g`, `Fig.2` and `v1.0`.
fn has_extension(name: &str) -> bool {
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty()
        && (2..=6).contains(&ext.chars().count())
        && ext.chars().all(|c| c.is_ascii_alphanumeric())
        && !ext.chars().all(|c| c.is_ascii_digit())
}

/// Split a `path:line[:col]` reference into its path and line number.
pub(crate) fn strip_position(tok: &str) -> (&str, Option<usize>) {
    let mut path = tok;
    let mut line = None;
    // Up to two trailing `:number` groups — `file.rs:42:7` is a column too.
    for _ in 0..2 {
        let Some((head, tail)) = path.rsplit_once(':') else {
            break;
        };
        let Ok(n) = tail.parse::<usize>() else { break };
        if head.is_empty() {
            break;
        }
        path = head;
        line = Some(n);
    }
    (path, line)
}

/// Mark every file reference in `lines`, the pane's rows already read off the
/// grid. Returns how many cells were marked. Runs before [`crate::linkhl`],
/// so a URL that happens to look like a path is re-marked as the URL it is.
///
/// The frame scans a pane's cells ONCE and hands the same rows to everything
/// that wants them (see `paneview`): three readers used to build the same
/// `Vec<Vec<char>>` from the same cells, one after another, every frame.
pub(crate) fn mark_in(cells: &mut [CellView], lines: &[Vec<char>]) -> usize {
    let ranges: Vec<(u16, usize, usize)> = lines
        .iter()
        .enumerate()
        .flat_map(|(r, line)| {
            path_spans(line)
                .into_iter()
                .map(move |(a, b)| (r as u16, a, b))
        })
        .collect();
    if ranges.is_empty() {
        return 0;
    }
    let fg = crate::linkhl::link_fg();
    let mut marked = 0;
    for c in cells.iter_mut() {
        if ranges
            .iter()
            .any(|&(r, a, b)| c.row == r && (a..b).contains(&(c.col as usize)))
        {
            c.fg = fg;
            c.deco.line = DecoLine::Dotted;
            marked += 1;
        }
    }
    marked
}

/// [`mark_in`] against a pane's cells, scanning them here. The app scans once
/// per frame and calls the `_in` form; this is the seam the tests use.
#[cfg(test)]
pub(crate) fn mark(cells: &mut [CellView], cols: u16, rows: u16) -> usize {
    mark_in(cells, &crate::gridrows::grid_lines(cells, cols, rows))
}

#[cfg(test)]
#[path = "pathhl_tests.rs"]
mod tests;
