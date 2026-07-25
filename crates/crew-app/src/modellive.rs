//! The live OpenRouter overlay backing `modelpick::model_row`'s enrichment:
//! a process-global published by `poll.rs`'s drain of the background fetch
//! (`modelfetch`), and the merge that fills a curated row's gaps from it.
//! Split from `modelpick.rs` (child module) to keep that file under the
//! house line cap, same reason as `modelrecents.rs` beside it.
use std::sync::Mutex;

use crew_hive::catalog::{LiveModel, ModelInfo};

/// OpenRouter rows keyed by their `id`. Empty until the first fetch lands
/// (or forever, offline/no key) — every reader treats that the same as "no
/// enrichment available", never as an error.
static LIVE: Mutex<Vec<LiveModel>> = Mutex::new(Vec::new());

/// Publish freshly fetched/cached rows, replacing whatever was there. Called
/// only from the poll drain — never blocks a picker render on the network.
pub(crate) fn set_live(models: Vec<LiveModel>) {
    if let Ok(mut g) = LIVE.lock() {
        *g = models;
    }
}

/// Live price/context/free for a catalog row, matched on its OpenRouter
/// alias against the process-global overlay. Thin wrapper around
/// [`enrich_with`] so the merge logic itself is unit-testable against an
/// explicit live list, without going through the shared `Mutex` (which
/// every test in the binary contends for — see `rows` vs. `rows_with_recents`
/// in `modelpick.rs` for the same split, same reason).
pub(crate) fn enrich(m: &ModelInfo) -> (Option<(u64, u64)>, u32, bool) {
    let Some(alias) = m.or_slug else {
        return (m.price, m.context, m.free);
    };
    let Ok(live) = LIVE.lock() else {
        return (m.price, m.context, m.free);
    };
    enrich_with(m, alias, &live)
}

/// The additive merge itself: a live row fills a curated `None` price or `0`
/// context, but never overwrites a curated value that's already known, and a
/// row with no live match just falls back to the curated numbers untouched
/// — nothing here can delete or replace a catalog row. `free` is derived
/// from whichever price wins, the same way `parse_models` derives it, rather
/// than OR'd in from the live row separately — that would let a live
/// `free: true` override a curated *paid* price the merge just decided to
/// keep.
fn enrich_with(m: &ModelInfo, alias: &str, live: &[LiveModel]) -> (Option<(u64, u64)>, u32, bool) {
    match live.iter().find(|l| l.id == alias) {
        Some(l) => {
            let price = m.price.or(l.price);
            let context = if m.context > 0 { m.context } else { l.context };
            (price, context, price == Some((0, 0)))
        }
        None => (m.price, m.context, m.free),
    }
}

#[cfg(test)]
#[path = "modellive_tests.rs"]
mod tests;
