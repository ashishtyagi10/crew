//! The lines an edit touched, laid out as numbered rows for
//! [`super::editshow`]: `  41│ text`, right-aligned to the widest number
//! shown, two lines either side, and regions that touch or overlap run
//! together so one edit's context is not printed twice beside the next's.
//!
//! Capped on rows AND chars, because either alone lets one kind of edit come
//! back whole: 200 short lines, or forty 200-char ones. Past the cap a pointer
//! names the `sys:read_file` call that shows the rest.

/// Lines shown either side of an edit: enough to see what it joined onto.
const AROUND: usize = 2;
/// Most rows, and chars, the shown part takes: a few hundred chars more per
/// edit is the price of checking it, a second copy of a big paste is not.
const MAX_ROWS: usize = 40;
const MAX_CHARS: usize = 2_000;
/// A longer line is cut, or one minified line would spend the whole budget.
const LINE_CAP: usize = 200;

/// One edit in lines of the file now: `first..=last`, or, for `cut` whole
/// lines taken out, the empty `first..=first - 1`, meaning just before `first`.
pub(super) struct Edit {
    pub(super) first: usize,
    pub(super) last: usize,
    pub(super) cut: usize,
}

/// The rows for `edits` in `after`, in file order, then a pointer to any
/// edited lines the cap left out. Empty only when there is no line to show.
pub(super) fn layout(mut edits: Vec<Edit>, after: &str) -> String {
    let text: Vec<&str> = after
        .split_inclusive('\n')
        .map(|l| l.trim_end_matches(['\n', '\r']))
        .collect();
    edits.sort_by_key(|e| (e.first, e.last));
    let rows = rows(&edits, text.len());
    // Costed at the file's widest number, so the rows shown only come in under.
    let most = digits(text.len());
    let kept = fit(&rows, |r| r.render(most, &text).chars().count() + 1);
    let w = rows[..kept].iter().map(Row::digits).max().unwrap_or(1);
    let mut out: Vec<String> = rows[..kept].iter().map(|r| r.render(w, &text)).collect();
    out.extend(pointer(&rows[kept..], &edits));
    out.join("\n")
}

fn digits(n: usize) -> usize {
    n.max(1).to_string().len()
}

enum Row {
    Line(usize),
    /// `.1` lines taken out just before line `.0`.
    Cut(usize, usize),
    /// Between two regions that do not touch.
    Break,
}

impl Row {
    fn digits(&self) -> usize {
        if let Row::Line(n) = self {
            digits(*n)
        } else {
            1
        }
    }

    fn render(&self, w: usize, text: &[&str]) -> String {
        match *self {
            Row::Line(n) => match text[n - 1].char_indices().nth(LINE_CAP) {
                Some((cut, _)) => format!("{n:>w$}\u{2502} {}\u{2026}", &text[n - 1][..cut]),
                None => format!("{n:>w$}\u{2502} {}", text[n - 1]),
            },
            Row::Cut(_, 1) => format!("{:w$}\u{2502} \u{2504} 1 line removed", ""),
            Row::Cut(_, k) => format!("{:w$}\u{2502} \u{2504} {k} lines removed", ""),
            Row::Break => format!("{:>w$}", "\u{22ee}"),
        }
    }
}

/// Every row to show, in file order: each edit's lines with [`AROUND`] either
/// side, regions that touch or overlap as one, and a `Break` between the rest.
fn rows(edits: &[Edit], total: usize) -> Vec<Row> {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for e in edits {
        let lo = e.first.saturating_sub(AROUND).max(1);
        let hi = (e.last + AROUND).min(total);
        match spans.last_mut() {
            Some(s) if lo <= s.1 + 1 => s.1 = s.1.max(hi),
            _ => spans.push((lo, hi)),
        }
    }
    let cut = |n: usize| -> usize { edits.iter().filter(|e| e.first == n).map(|e| e.cut).sum() };
    let mut out = Vec::new();
    for (lo, hi) in spans {
        if !out.is_empty() {
            out.push(Row::Break);
        }
        // To `hi + 1`: lines taken out after the last line sit past it.
        for n in lo..=hi + 1 {
            if cut(n) > 0 {
                out.push(Row::Cut(n, cut(n)));
            }
            if n <= hi {
                out.push(Row::Line(n));
            }
        }
    }
    out
}

/// How many rows fit: whole regions while they do, or, when not even the
/// first does, as much of it as fits. Half a region is harder to read than
/// one left to the pointer, unless it is all there is.
fn fit(rows: &[Row], cost: impl Fn(&Row) -> usize) -> usize {
    let (mut kept, mut chars) = (0, 0);
    while kept < rows.len() {
        let next = rows[kept + 1..]
            .iter()
            .position(|r| matches!(r, Row::Break));
        let end = next.map_or(rows.len(), |p| kept + 1 + p);
        let more: usize = rows[kept..end].iter().map(&cost).sum();
        if end <= MAX_ROWS && chars + more <= MAX_CHARS {
            (kept, chars) = (end, chars + more);
            continue;
        }
        // The first region's head, one row at least: an edit that shows
        // nothing checks nothing.
        let room = if kept == 0 { end.min(MAX_ROWS) } else { 0 };
        while kept < room && (kept == 0 || chars + cost(&rows[kept]) <= MAX_CHARS) {
            chars += cost(&rows[kept]);
            kept += 1;
        }
        break;
    }
    kept
}

/// Where the edited lines the cap left out start, as the `sys:read_file`
/// call that shows them, so the check the cap cut short is one call away.
fn pointer(rest: &[Row], edits: &[Edit]) -> Option<String> {
    let edited = |n: usize| edits.iter().any(|e| (e.first..=e.last).contains(&n));
    let at = rest.iter().find_map(|r| match *r {
        Row::Line(n) if edited(n) => Some(n),
        Row::Cut(n, _) => Some(n.saturating_sub(1).max(1)),
        _ => None,
    })?;
    let lines = rest
        .iter()
        .filter(|r| matches!(r, Row::Line(n) if edited(*n)));
    let cuts = rest
        .iter()
        .map(|r| if let Row::Cut(_, k) = r { *k } else { 0 });
    let (n, what) = match lines.count() {
        0 => (cuts.sum(), "removed"),
        n => (n, "edited"),
    };
    let s = if n == 1 { "" } else { "s" };
    Some(format!(
        "\u{2026} (+{n} more line{s} {what}; sys:read_file with \"line\": {at} shows them)"
    ))
}

/// Whether `old` was copied out of a shown block, prefixes and all: every
/// non-blank line opens with a right-aligned number and `│ `. The model sees
/// those rows after every edit, and the next `old` it writes can carry them;
/// saying so beats the generic "not in the file".
pub(super) fn carries_numbers(old: &str) -> bool {
    let mut rows = old.lines().filter(|l| !l.trim().is_empty()).peekable();
    rows.peek().is_some()
        && rows.all(|l| {
            let t = l.trim_start();
            let digits = t.chars().take_while(char::is_ascii_digit).count();
            digits > 0 && t[digits..].starts_with('\u{2502}')
        })
}

#[cfg(test)]
#[path = "editrows_tests.rs"]
mod tests;
