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
mod tests {
    use super::*;

    #[test]
    fn a_live_sub_cent_rate_never_badges_as_free() {
        // 5_000 µ$/Mtok = $0.000005/Mtok — nonzero, but rounds to "0.00" at
        // two decimals. Must render as a paid row, not "$0/$0" (free-looking).
        assert_eq!(dollars(5_000), "<0.01");
        let badge = price_badge(Some((5_000, 5_000)), false);
        assert_eq!(badge, "$<0.01/$<0.01");
        assert_ne!(badge, "$0/$0");
    }

    #[test]
    fn ordinary_rates_are_unaffected() {
        assert_eq!(dollars(0), "0");
        assert_eq!(dollars(3_000_000), "3");
        assert_eq!(dollars(400_000), "0.4");
        assert_eq!(dollars(1_650_000), "1.65");
        assert_eq!(dollars(5_000_000), "5");
    }
}
