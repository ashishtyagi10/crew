//! The one interior hint row of the masked key prompt (`keyentry`): what it
//! says, if anything, for the variable being asked for.
//!
//! Split out because `keyentry.rs` sits on the line-cap debt list and the
//! hint used to be a single string literal there — fine while OpenRouter's
//! browser sign-in was the only prompt with something to say. The NVIDIA
//! prompt has something better to say: the key is free, and here is where.

/// The hint text for `var`, or `None` when the card should stay three rows.
///
/// A browser sign-in in flight (`waiting`) wins over everything: that hint is
/// the only sign the flow is still live. Otherwise a variable with a free,
/// card-less way to get a key names it — a paste prompt titled
/// `NVIDIA_API_KEY` alone sends a keyless user to a search engine.
pub(crate) fn hint(var: &str, waiting: bool) -> Option<&'static str> {
    if waiting {
        return Some("waiting for browser \u{b7} or paste the key");
    }
    match var {
        "NVIDIA_API_KEY" => Some("free at build.nvidia.com \u{b7} no card"),
        _ => None,
    }
}

#[cfg(test)]
#[path = "keyhint_tests.rs"]
mod tests;
