//! Token budgets as the number a person would say.
//!
//! `usage_budget_5h` / `usage_budget_7d` are raw token counts — 5000000 and
//! 25000000 by default. Nobody reads an eight-digit field, and the footer
//! those budgets feed never shows the figure at all: it draws a percentage
//! bar against them. So the form types MILLIONS, the same trade the opacity
//! box already makes by typing a percentage over a stored fraction.
//!
//! The hazard that buys is silent quantisation. A config hand-set to 5123456
//! displays as `5.12`, and committing that back would round it to 5120000 —
//! the form rewriting a value the user never touched. [`commit`] closes it
//! by treating a buffer that still reads as the stored value as no edit at
//! all.

/// Smallest budget worth having, in tokens — `CrewConfig::clamped`'s floor,
/// restated here so the form cannot accept what the config will raise.
pub(super) const FLOOR: u64 = 10_000;

/// A token count as millions, trimmed: 5000000 → `5`, 7500000 → `7.5`,
/// 5123456 → `5.12`. Two decimals is 10k resolution, which is [`FLOOR`].
pub(super) fn label(tokens: u64) -> String {
    let m = tokens as f64 / 1_000_000.0;
    let s = format!("{m:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() { "0" } else { s }.to_string()
}

/// The token count `typed` (in millions) means, or `None` if it is not a
/// number. Clamped up to [`FLOOR`] so a typed `0` becomes the smallest real
/// budget rather than a divide-by-zero the footer has to defend against.
pub(super) fn parse(typed: &str) -> Option<u64> {
    let m: f64 = typed.trim().parse().ok()?;
    if !m.is_finite() || m < 0.0 {
        return None;
    }
    Some(((m * 1_000_000.0).round() as u64).max(FLOOR))
}

/// The value `typed` should commit to, given what is already stored.
///
/// Returns `prev` untouched when the buffer still reads as `prev` — the
/// no-quantisation rule from the module docs — and when the buffer is not a
/// number at all, matching the accent box, which keeps the previous colour
/// rather than guessing at unparseable hex.
pub(super) fn commit(typed: &str, prev: u64) -> u64 {
    if typed.trim() == label(prev) {
        return prev;
    }
    parse(typed).unwrap_or(prev)
}

/// The task spend cap in dollars as typed: 10 → `10`, 2.5 → `2.5`.
pub(super) fn dollars(usd: f64) -> String {
    let s = format!("{usd:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The cap `typed` commits to, given what is stored: `prev` when the buffer
/// still reads as it (the same no-quantisation rule as [`commit`]: a hand-set
/// `2.555` shows as `2.56` and must not be rewritten unasked), or when it is
/// not a number. `0` is a real answer here, "off".
pub(super) fn commit_usd(typed: &str, prev: f64) -> f64 {
    match typed.trim().parse::<f64>() {
        _ if typed.trim() == dollars(prev) => prev,
        Ok(v) if v.is_finite() => v.clamp(0.0, 100_000.0),
        _ => prev,
    }
}

#[cfg(test)]
#[path = "tokens_tests.rs"]
mod tests;
