//! The fuzzy half of the composer's completion: whether a typed run is a
//! subsequence of a candidate, the single unique match when there is one, and
//! extending what you typed to the longest shared prefix.
//!
//! Split from [`crate::chatcomplete`] for the line cap.

/// Extend `prefix` against `candidates`: the full name when exactly one
/// matches (`(name, true)`), else the longest common prefix when it grows the
/// input (`(lcp, false)`). Case-insensitive; `None` when nothing matches or
/// nothing would change.
pub(crate) fn extend(prefix: &str, candidates: &[&str]) -> Option<(String, bool)> {
    let low = prefix.to_lowercase();
    let hits: Vec<&&str> = candidates
        .iter()
        .filter(|c| c.to_lowercase().starts_with(&low))
        .collect();
    match hits.as_slice() {
        [] => None,
        [one] => Some((one.to_string(), true)),
        many => {
            let first = many[0].to_lowercase();
            let mut lcp = first.len();
            for c in many.iter().skip(1) {
                let c = c.to_lowercase();
                lcp = first
                    .chars()
                    .zip(c.chars())
                    .take(lcp)
                    .take_while(|(a, b)| a == b)
                    .count();
            }
            (lcp > prefix.len()).then(|| (first[..lcp].to_string(), false))
        }
    }
}

/// Case-insensitive subsequence match: is every char of `needle` found in
/// `hay` in order? (`"gl"` matches `"goal"`, `"pnr"` matches `"planner"`.)
/// Case-folds and delegates to `crate::suggest`'s identical (already
/// case-normalized-by-caller) helper.
pub(crate) fn is_subsequence(needle: &str, hay: &str) -> bool {
    crate::suggest::is_subsequence(&needle.to_lowercase(), &hay.to_lowercase())
}

/// The single candidate that fuzzy-matches `needle`, or `None` if zero or
/// more than one do.
pub(crate) fn fuzzy_unique<'a>(needle: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let mut hits = candidates.iter().filter(|c| is_subsequence(needle, c));
    let first = *hits.next()?;
    match hits.next() {
        Some(_) => None,
        None => Some(first),
    }
}
