//! Per-character paint for the diff rung: what a review needs a diff to show.
//!
//! A unified diff painted a line at a time — green for added, red for removed
//! — tells you *that* a line changed and leaves you to find *what* changed by
//! eye, one character at a time. So this pairs each removed line with the
//! added line that replaced it, finds the run that actually differs, and
//! draws the rest of the pair dimmed: the shared text recedes and the change
//! is the only thing at full strength. It is the same reading `git diff
//! --word-diff` offers, without the bracket syntax that makes code unreadable.
//!
//! Refinement is deliberately conservative. Two runs of different length are
//! not paired at all (there is no honest correspondence to draw), and a pair
//! that differs almost everywhere is left alone (marking the whole line is
//! not a mark).
use crate::viewpane::codepaint::CharPaint;

/// Above this share of the longer line, a "change" is a rewrite and marking
/// it says nothing.
const MAX_CHANGED: f32 = 0.7;

/// What kind of line this is, by its first character — the only classification
/// a unified diff supports, and the one every tool uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    File,
    Hunk,
    Added,
    Removed,
    Context,
}

pub(crate) fn kind_of(line: &str) -> Kind {
    if line.starts_with("@@") {
        return Kind::Hunk;
    }
    // `+++`/`---` are the file headers, not a one-line addition and removal.
    if line.starts_with("+++") || line.starts_with("---") || line.starts_with("diff ") {
        return Kind::File;
    }
    if line.starts_with("index ")
        || line.starts_with("new file")
        || line.starts_with("deleted file")
    {
        return Kind::File;
    }
    match line.chars().next() {
        Some('+') => Kind::Added,
        Some('-') => Kind::Removed,
        _ => Kind::Context,
    }
}

/// The char range of `line` that differs from `other`, as `(start, end)` into
/// each — common prefix and suffix trimmed, then grown out to word edges so a
/// change inside an identifier marks the identifier. `None` when there is no
/// useful difference to mark.
pub(crate) fn refine(old: &str, new: &str) -> Option<((usize, usize), (usize, usize))> {
    let (a, b): (Vec<char>, Vec<char>) = (old.chars().collect(), new.chars().collect());
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let pre = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let max_suf = a.len().min(b.len()) - pre;
    let suf = (0..max_suf)
        .take_while(|i| a[a.len() - 1 - i] == b[b.len() - 1 - i])
        .count();
    let (mut a0, mut a1) = (pre, a.len() - suf);
    let (mut b0, mut b1) = (pre, b.len() - suf);
    if a0 >= a1 && b0 >= b1 {
        return None; // the lines are the same
    }
    let word = |c: &char| c.is_alphanumeric() || *c == '_';
    let grow = |s: &mut usize, e: &mut usize, v: &[char]| {
        while *s > 0 && word(&v[*s - 1]) && v.get(*s).is_some_and(word) {
            *s -= 1;
        }
        while *e < v.len() && word(&v[*e]) && *e > 0 && word(&v[*e - 1]) {
            *e += 1;
        }
    };
    grow(&mut a0, &mut a1, &a);
    grow(&mut b0, &mut b1, &b);
    let changed = (a1 - a0).max(b1 - b0) as f32;
    let longest = a.len().max(b.len()) as f32;
    (changed / longest <= MAX_CHANGED).then_some(((a0, a1), (b0, b1)))
}

/// How far a shared run recedes toward the page.
const DIM: f32 = 0.45;

/// Paint for every line of `text`, one entry per character.
pub(crate) fn paint(text: &str) -> Vec<Vec<CharPaint>> {
    let t = crew_theme::theme();
    let lines: Vec<&str> = text.split('\n').collect();
    let kinds: Vec<Kind> = lines.iter().map(|l| kind_of(l)).collect();
    // Marked ranges per line, filled in for the pairs that refine.
    let mut marks: Vec<Option<(usize, usize)>> = vec![None; lines.len()];
    let mut i = 0;
    while i < lines.len() {
        if kinds[i] != Kind::Removed {
            i += 1;
            continue;
        }
        let del = i..lines[i..]
            .iter()
            .enumerate()
            .find(|(k, _)| kinds[i + k] != Kind::Removed)
            .map_or(lines.len(), |(k, _)| i + k);
        let add_start = del.end;
        let add = add_start
            ..lines[add_start..]
                .iter()
                .enumerate()
                .find(|(k, _)| kinds[add_start + k] != Kind::Added)
                .map_or(lines.len(), |(k, _)| add_start + k);
        // Only equal-length runs correspond line for line. Anything else is a
        // guess, and a guess drawn as a mark is a lie about what changed.
        if del.len() == add.len() {
            for (d, a) in del.clone().zip(add.clone()) {
                if let Some((r, w)) = refine(&lines[d][1..], &lines[a][1..]) {
                    marks[d] = Some((r.0 + 1, r.1 + 1));
                    marks[a] = Some((w.0 + 1, w.1 + 1));
                }
            }
        }
        i = add.end.max(del.end);
    }
    lines
        .iter()
        .enumerate()
        .map(|(n, line)| line_paint(line, kinds[n], marks[n], t))
        .collect()
}

/// One line's characters, given its kind and the range that changed.
fn line_paint(
    line: &str,
    kind: Kind,
    mark: Option<(usize, usize)>,
    t: &crew_theme::Theme,
) -> Vec<CharPaint> {
    let base = match kind {
        Kind::File => t.ink,
        Kind::Hunk => t.ansi[6],
        Kind::Added => t.ansi[2],
        Kind::Removed => t.ansi[1],
        Kind::Context => t.ink,
    };
    let bold = matches!(kind, Kind::File | Kind::Hunk);
    if kind == Kind::Hunk {
        // `@@ -1,7 +1,9 @@ fn main` — the range is the heading, the trailing
        // context is a note about where you are, and reads as one.
        let end = line.find("@@ ").map(|i| i + 3).unwrap_or(0);
        let end = line[end..]
            .find("@@")
            .map(|i| end + i + 2)
            .unwrap_or(line.len());
        return line
            .chars()
            .enumerate()
            .map(|(i, _)| match i < end {
                true => (base, true),
                false => (t.text_muted, false),
            })
            .collect();
    }
    let Some((s, e)) = mark else {
        return line.chars().map(|_| (base, bold)).collect();
    };
    let quiet = crate::anim::lerp_rgb(base, t.page_bg, DIM);
    line.chars()
        .enumerate()
        .map(|(i, _)| match (s..e).contains(&i) || i == 0 {
            true => (base, true),
            false => (quiet, false),
        })
        .collect()
}

#[cfg(test)]
#[path = "diffpaint_tests.rs"]
mod tests;
