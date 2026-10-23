//! `5 pm` read as `5pm`. A clock with its am/pm a word apart is how many
//! people type it, and apart neither half parsed: "call mom at 5 pm" stayed
//! titled so, with no due, and a mail's "by 5 pm" went unread.

/// `toks` with each number followed by a lone `am` or `pm` made one token,
/// spanning both, so the title loses them together.
pub(super) fn joined(toks: Vec<(usize, usize, String)>) -> Vec<(usize, usize, String)> {
    let mut out: Vec<(usize, usize, String)> = Vec::with_capacity(toks.len());
    for t in toks {
        let half = t.2 == "am" || t.2 == "pm";
        match out.last_mut() {
            Some(prev) if half && clock(&prev.2) => {
                prev.1 = t.1;
                prev.2.push_str(&t.2);
            }
            _ => out.push(t),
        }
    }
    out
}

/// `5`, `11`, `5:30`: what an am or pm may follow.
fn clock(w: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.len() <= 2 && s.chars().all(|c| c.is_ascii_digit());
    match w.split_once(':') {
        Some((h, m)) => digits(h) && m.len() == 2 && digits(m),
        None => digits(w),
    }
}

#[cfg(test)]
#[path = "clockword_tests.rs"]
mod tests;
