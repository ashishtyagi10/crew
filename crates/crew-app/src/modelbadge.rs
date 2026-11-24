//! Price and context-window badges for a `/model` picker row. Split from
//! `modelpick.rs` (child module) to keep that file under the house line cap,
//! same reason as `modellive.rs`/`modelrecents.rs` beside it.
use crew_hive::catalog::sunset::sunset;
use crew_hive::catalog::ModelInfo;

use crate::modelroute::Route;

/// The dim column: slug, retirement, price, context, route hint, and
/// current-mark. Price, free, and context are passed in already enriched
/// (`modellive::enrich`) rather than read straight off `m`, so a live
/// OpenRouter rate can fill a curated `None`/`0` without this function
/// needing to know where its numbers came from.
///
/// A vendor's shutdown date rides right after the slug, not at the end: the
/// row is cut from the right on a narrow card, and `retires Dec 11` is the
/// one part that changes whether the pick is worth making. It goes in the
/// description, not the label, because the label column is the list's
/// widest label — a badge there would push every row's description over.
pub(super) fn desc(
    m: &ModelInfo,
    (price, free, context): (Option<(u64, u64)>, bool, u32),
    route: Route,
    current: bool,
    today: i64,
) -> String {
    let mut parts: Vec<String> = vec![m.slug.to_string()];
    parts.extend(sunset(m.slug, today).map(|n| n.badge()));
    parts.push(price_badge(price, free));
    let ctx = context_badge(context);
    if !ctx.is_empty() {
        parts.push(ctx);
    }
    let hint = route.hint();
    if !hint.is_empty() {
        parts.push(hint);
    }
    if current {
        parts.push("\u{25cf} current".to_string());
    }
    parts.join(" \u{b7} ")
}

/// Dollars-per-Mtok badge, or an em dash when the rate is unknown.
fn price_badge(price: Option<(u64, u64)>, free: bool) -> String {
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
fn context_badge(tokens: u32) -> String {
    match tokens {
        0 => String::new(),
        t if t >= 1_000_000 => format!("{}M", t / 1_000_000),
        t => format!("{}k", t / 1000),
    }
}

#[cfg(test)]
#[path = "modelbadge_tests.rs"]
mod tests;
