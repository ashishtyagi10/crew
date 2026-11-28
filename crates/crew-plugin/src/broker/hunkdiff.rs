//! A line diff, cut into hunks a person can keep or drop one by one.
//!
//! WHY here and not a crate: an approval question must show the change before it is made, and
//! "keep hunks 1 and 3" must write EXACTLY the original plus those hunks. Both come from this one
//! diff, so what the question showed and what lands cannot drift apart. No dependency is wanted
//! for a few dozen lines of LCS.
//!
//! Lines keep their line ending, so a missing final newline is a change like any other and
//! applying every hunk rebuilds the new text byte for byte.

/// Longest middle (old lines x new lines) the LCS table is built for. Past it the whole middle
/// is ONE hunk: a coarse diff is still honest, a 10 000 x 10 000 table is a stalled broker.
const MAX_CELLS: usize = 4_000_000;

/// One run of changed lines: `old` lines at line index `at` become `new` lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hunk {
    pub at: usize,
    pub old: Vec<String>,
    pub new: Vec<String>,
}

/// The hunks that turn `old` into `new`, in file order; none when they are equal.
pub(crate) fn diff(old: &str, new: &str) -> Vec<Hunk> {
    let (a, b) = (lines(old), lines(new));
    let head = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let tail = a[head..]
        .iter()
        .rev()
        .zip(b[head..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (ma, mb) = (&a[head..a.len() - tail], &b[head..b.len() - tail]);
    if ma.is_empty() && mb.is_empty() {
        return Vec::new();
    }
    if ma.len().saturating_mul(mb.len()) > MAX_CELLS {
        let own = |s: &[&str]| s.iter().map(|l| l.to_string()).collect::<Vec<_>>();
        return vec![Hunk {
            at: head,
            old: own(ma),
            new: own(mb),
        }];
    }
    walk(ma, mb, head)
}

fn lines(s: &str) -> Vec<&str> {
    s.split_inclusive('\n').collect()
}

/// Walk the LCS table of the differing middle, closing a hunk at every shared line.
fn walk(a: &[&str], b: &[&str], base: usize) -> Vec<Hunk> {
    let w = b.len() + 1;
    let mut t = vec![0u32; (a.len() + 1) * w];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            t[i * w + j] = match a[i] == b[j] {
                true => t[(i + 1) * w + j + 1] + 1,
                false => t[(i + 1) * w + j].max(t[i * w + j + 1]),
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::<Hunk>::new());
    let mut open: Option<Hunk> = None;
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            out.extend(open.take());
            (i, j) = (i + 1, j + 1);
            continue;
        }
        let h = open.get_or_insert_with(|| Hunk {
            at: base + i,
            old: Vec::new(),
            new: Vec::new(),
        });
        if j == b.len() || (i < a.len() && t[(i + 1) * w + j] >= t[i * w + j + 1]) {
            h.old.push(a[i].to_string());
            i += 1;
        } else {
            h.new.push(b[j].to_string());
            j += 1;
        }
    }
    out.extend(open);
    out
}

/// `old` with only the hunks `keep` says yes to applied; the rest stay as they were.
pub(crate) fn apply(old: &str, hunks: &[Hunk], keep: impl Fn(usize) -> bool) -> String {
    let a = lines(old);
    let (mut out, mut at) = (String::with_capacity(old.len()), 0);
    for (k, h) in hunks.iter().enumerate() {
        out.extend(a[at..h.at].iter().copied());
        match keep(k) {
            true => out.extend(h.new.iter().map(String::as_str)),
            false => out.extend(a[h.at..h.at + h.old.len()].iter().copied()),
        }
        at = h.at + h.old.len();
    }
    out.extend(a[at..].iter().copied());
    out
}

#[cfg(test)]
#[path = "hunkdiff_tests.rs"]
mod tests;
