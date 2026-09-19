//! The one interior hint row of the masked key prompt (`keyentry`): what it
//! says, if anything, for the variable being asked for.
//!
//! Split out because `keyentry.rs` sits on the line-cap debt list and the
//! hint used to be a single string literal there — fine while OpenRouter's
//! browser sign-in was the only prompt with something to say. The NVIDIA
//! prompt has something better to say: the key is free, and here is where.

/// The hint forms for `var`, longest first, or empty when the card should
/// stay three rows.
///
/// A browser sign-in in flight (`waiting`) wins over everything: that hint is
/// the only sign the flow is still live. Otherwise a variable with a free,
/// card-less way to get a key names it — a paste prompt titled
/// `NVIDIA_API_KEY` alone sends a keyless user to a search engine.
///
/// A LADDER rather than one string: this card is placed over a composer and
/// clamped to the pane, so on a tile the one form was cut — `waiting for
/// browser · or pa…`, which is the sign-in flow's only sign of life, ending
/// in an ellipsis. A shorter whole hint beats a longer cut one.
pub(crate) fn forms(var: &str, waiting: bool) -> &'static [&'static str] {
    if waiting {
        return &[
            "waiting for browser \u{b7} or paste the key",
            "waiting for browser",
            "waiting\u{2026}",
        ];
    }
    match var {
        "NVIDIA_API_KEY" => &[
            "free at build.nvidia.com \u{b7} no card",
            "free at build.nvidia.com",
            "build.nvidia.com",
        ],
        _ => &[],
    }
}

/// The longest form that fits `inner` columns, or the shortest when none do —
/// something whole and too long still says what it says; something cut does
/// not.
pub(crate) fn fitting(var: &str, waiting: bool, inner: usize) -> Option<&'static str> {
    let forms = forms(var, waiting);
    forms
        .iter()
        .find(|f| crate::chatwidth::str_w(f) <= inner)
        .or_else(|| forms.last())
        .copied()
}

/// The longest form there is — what the card asks for room for.
pub(crate) fn hint(var: &str, waiting: bool) -> Option<&'static str> {
    forms(var, waiting).first().copied()
}

/// Interior columns the prompt wants: room for a typical key from the start
/// (a paste should not wrap a row that was sized for nothing), the hint if
/// there is one, and every character typed so far — the card grows with the
/// key until the pane stops it (`popupplace::card_cols`). Plus the two
/// columns of the `❯ ` prompt at the field's head.
pub(crate) fn want(hint: Option<&str>, typed: usize) -> usize {
    2 + hint
        .map_or(0, |h| h.chars().count())
        .max(typed)
        .max(KEY_COLS)
}

/// Columns a typical provider key fills.
const KEY_COLS: usize = 48;

#[cfg(test)]
#[path = "keyhint_tests.rs"]
mod tests;
