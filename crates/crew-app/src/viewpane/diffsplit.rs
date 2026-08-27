//! The review, side by side: what was there on the left, what is there now on
//! the right.
//!
//! A unified diff is a compression — the two versions of a file interleaved
//! into one column, which is what makes it fit in an email and what makes it
//! hard to read. Crew's diff rung already recovers most of what that
//! compression costs: it pairs each removed line with the added line that
//! replaced it and dims the text the two share, so the change is the only
//! thing at full strength. What it cannot recover is *position* — a removed
//! line and its replacement occupy the same place in the file, and stacking
//! them says they happen one after another.
//!
//! So the pairs are laid out where they belong: the old line on the left, the
//! new one on the right, on the same row. Everything the unified rung knows
//! comes with it — the pairing, the word-level refinement, the hunk headings
//! — because both rungs read the same [`super::diffpaint`].
//!
//! ## Line numbers
//!
//! Each side carries the file's own numbering, tracked from the hunk header
//! (`@@ -12,7 +12,9 @@`): a removed line advances the old count, an added
//! line the new one, and context both. A unified diff's gutter can only count
//! rows of the diff; a split one can say where in the file you are, which is
//! the number you quote to someone.
//!
//! ## Width
//!
//! Half a pane each, and half a pane is not always enough: below
//! [`MIN_COLS`] there is no honest split and the caller falls back to the
//! unified rung. A pair wraps BOTH sides at the half width and pads the
//! shorter to the taller — wrapping each side independently and zipping the
//! results would slide the two versions out of step exactly where the lines
//! are long enough to need the help.
use crate::viewpane::diffpaint::{self, Kind};
use crate::viewpane::lines::GUTTER_W;

/// Narrowest text column a side is worth having. Below this a line of code is
/// more continuation than line.
const MIN_TEXT: usize = 24;

/// Narrowest pane the split is offered on: two gutters, two text columns, and
/// the divider between them.
pub(crate) const MIN_COLS: usize = 2 * (GUTTER_W + MIN_TEXT) + 1;

/// The rule between the two halves.
pub(super) const DIVIDER: char = '\u{2502}';

/// One row of the split, before it is laid out.
#[derive(Debug, PartialEq)]
pub(super) enum Row<'a> {
    /// A file or hunk header — it is about both sides, so it spans them.
    Full(&'a str, Kind),
    /// `(old line number, old text, new line number, new text)`. Either side
    /// is absent where a run of removals is longer than the run of additions
    /// that replaced it, or the other way round.
    Pair(
        Option<usize>,
        Option<&'a str>,
        Option<usize>,
        Option<&'a str>,
    ),
}

/// Split `text` into rows, tracking each side's line number.
///
/// Removals and additions are gathered into maximal runs and paired index by
/// index — the same correspondence [`diffpaint::paint`] refines — with the
/// overhang of the longer run landing on its own side alone.
pub(super) fn rows(text: &str) -> Vec<Row<'_>> {
    let lines: Vec<&str> = text.split('\n').collect();
    let kinds: Vec<Kind> = lines.iter().map(|l| diffpaint::kind_of(l)).collect();
    let mut out = Vec::with_capacity(lines.len());
    let (mut old, mut new) = (0usize, 0usize);
    let mut i = 0;
    while i < lines.len() {
        match kinds[i] {
            Kind::Hunk => {
                if let Some((o, n)) = hunk_start(lines[i]) {
                    (old, new) = (o, n);
                }
                out.push(Row::Full(lines[i], Kind::Hunk));
                i += 1;
            }
            Kind::File => {
                out.push(Row::Full(lines[i], Kind::File));
                i += 1;
            }
            Kind::Context => {
                out.push(Row::Pair(
                    Some(old),
                    Some(lines[i]),
                    Some(new),
                    Some(lines[i]),
                ));
                old += 1;
                new += 1;
                i += 1;
            }
            Kind::Removed | Kind::Added => {
                let del = run(&kinds, i, Kind::Removed);
                let add = run(&kinds, del, Kind::Added);
                for k in 0..(del - i).max(add - del) {
                    let l = (i + k < del).then(|| lines[i + k]);
                    let r = (del + k < add).then(|| lines[del + k]);
                    out.push(Row::Pair(l.map(|_| old + k), l, r.map(|_| new + k), r));
                }
                old += del - i;
                new += add - del;
                i = add.max(del).max(i + 1);
            }
        }
    }
    out
}

/// The end of the run of `want` starting at `from`.
fn run(kinds: &[Kind], from: usize, want: Kind) -> usize {
    let mut i = from;
    while i < kinds.len() && kinds[i] == want {
        i += 1;
    }
    i
}

/// `(old start, new start)` from a `@@ -12,7 +12,9 @@` header.
///
/// Tokens that are not a signed count are SKIPPED rather than failing the
/// parse: the header opens with `@@` and often closes with a function name,
/// and a header crew cannot read leaves the counters where they were rather
/// than resetting them to nothing.
fn hunk_start(line: &str) -> Option<(usize, usize)> {
    let mut old = None;
    let mut new = None;
    for tok in line.split_whitespace() {
        let sign = tok.chars().next().unwrap_or(' ');
        if sign != '-' && sign != '+' {
            continue;
        }
        let Ok(n) = tok[1..].split(',').next().unwrap_or("").parse::<usize>() else {
            continue;
        };
        match sign {
            '-' if old.is_none() => old = Some(n),
            '+' if new.is_none() => new = Some(n),
            _ => {}
        }
    }
    Some((old?, new?))
}

#[cfg(test)]
#[path = "diffsplit_tests.rs"]
mod tests;
