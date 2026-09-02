//! The near misses. A model that invents a tool name is one round from
//! recovering if the error names what it probably meant — and a wall of
//! every name it may call, at two hundred tools, is not that. One sentence,
//! one shape, for a tool, a server or a slash command nobody has.
use std::fmt::Write as _;

/// Classic edit distance (insert/delete/substitute, unit cost).
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != cb);
            cur.push(sub.min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Up to three of `names` nearest `typo`, nearest first. Near means within an
/// edit distance of a third of the name (two at least) or sharing its first
/// four characters — `sys__run` for `sys_run`, not `sys__run` for `weather`.
pub fn nearest<'a>(names: &[&'a str], typo: &str) -> Vec<&'a str> {
    let typo = typo.to_ascii_lowercase();
    let mut scored: Vec<(usize, &str)> = names
        .iter()
        .filter_map(|n| {
            let low = n.to_ascii_lowercase();
            let d = levenshtein(&typo, &low);
            let prefix = typo
                .chars()
                .zip(low.chars())
                .take_while(|(a, b)| a == b)
                .count();
            (d <= (low.chars().count() / 3).max(2) || prefix >= 4).then_some((d, *n))
        })
        .collect();
    scored.sort_by_key(|&(d, n)| (d, n));
    scored.into_iter().take(3).map(|(_, n)| n).collect()
}

/// Names a short list can just be; a long one names the near misses and its
/// size instead. `unknown tool “x” — did you mean a, b? (200 tools in all)`.
pub fn unknown(kind: &str, typo: &str, names: &[&str], tail: &str) -> String {
    let mut s = format!("unknown {kind} \u{201c}{typo}\u{201d} \u{2014} ");
    if names.is_empty() {
        let _ = write!(s, "none available{tail}");
    } else if names.len() <= 8 {
        let mut all = names.to_vec();
        all.sort_unstable();
        let _ = write!(s, "available: {}{tail}", all.join(", "));
    } else {
        let near = nearest(names, typo);
        if !near.is_empty() {
            let _ = write!(s, "did you mean {}? ", near.join(", "));
        }
        let _ = write!(s, "({} {kind}s in all{tail})", names.len());
    }
    s
}

#[cfg(test)]
#[path = "near_tests.rs"]
mod tests;
