//! Where a history search matched, for the popup to mark.
//!
//! Ctrl+R's popup listed `cargo clippy --workspace` under a query of `car`
//! and marked nothing — and its second rank is a SUBSEQUENCE match, so a row
//! like `why is the sidebar chart a smear…` came up with no way to see why.
//! The palette marks its fuzzy hits for exactly this reason; these are the
//! positions its row drawing ([`crate::cmdrow`]) washes.

/// Char indices of `label` that `query` matched, case-insensitively: the
/// first substring run (the filter's rank 0), else the greedy leftmost
/// subsequence (rank 1). Empty when it does not match at all.
pub(crate) fn hits(label: &str, query: &str) -> Vec<usize> {
    let lower = |c: char| c.to_lowercase().next().unwrap_or(c);
    let l: Vec<char> = label.chars().map(lower).collect();
    let q: Vec<char> = query.chars().map(lower).collect();
    if q.is_empty() || q.len() > l.len() {
        return Vec::new();
    }
    if let Some(at) = (0..=l.len() - q.len()).find(|&i| l[i..i + q.len()] == q[..]) {
        return (at..at + q.len()).collect();
    }
    let mut out = Vec::new();
    let mut rest = l.iter().enumerate();
    for want in &q {
        match rest.by_ref().find(|(_, c)| *c == want) {
            Some((i, _)) => out.push(i),
            None => return Vec::new(),
        }
    }
    out
}

#[cfg(test)]
#[path = "histhits_tests.rs"]
mod tests;
