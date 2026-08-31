//! Price and context-window badges for a `/model` picker row. Split from
//! `modelpick.rs` (child module) to keep that file under the house line cap,
//! same reason as `modellive.rs`/`modelrecents.rs` beside it.

/// Dollars-per-Mtok badge, or an em dash when the rate is unknown.
pub(super) fn price_badge(price: Option<(u64, u64)>, free: bool) -> String {
    if free {
        return "free".to_string();
    }
    match price {
        Some((inp, out)) => format!("${}/${}", dollars(inp), dollars(out)),
        None => "\u{2014}".to_string(),
    }
}

/// µ$/Mtok → a short dollar string ("3", "0.4", "1.6"). A nonzero rate that
/// still rounds to "0.00" at two decimals renders `<0.01` instead — a live
/// sub-cent rate must never badge a paid row as free.
fn dollars(microusd: u64) -> String {
    let whole = microusd / 1_000_000;
    let tenths = (microusd % 1_000_000) / 100_000;
    let hundredths = (microusd % 100_000) / 10_000;
    match (whole, tenths, hundredths) {
        (0, 0, 0) if microusd > 0 => "<0.01".to_string(),
        (w, 0, 0) => w.to_string(),
        (w, t, 0) => format!("{w}.{t}"),
        (w, t, h) => format!("{w}.{t}{h}"),
    }
}

/// Context window as a short badge ("1M", "200k"); empty when unknown.
pub(super) fn context_badge(tokens: u32) -> String {
    match tokens {
        0 => String::new(),
        t if t >= 1_000_000 => format!("{}M", t / 1_000_000),
        t => format!("{}k", t / 1000),
    }
}

#[cfg(test)]
#[path = "modelbadge_tests.rs"]
mod tests;
