//! Rows for the `/model` picker: the catalog grouped into provider sections,
//! filtered as the user types, each row badged with its price (or `—`), its
//! free/paid state, and how the active stack would serve it. Pure
//! string-in/rows-out — both surfaces (the input bar's value picker and the
//! composer's `Kind::Model` popup) render the same list.
use crew_hive::catalog::{catalog, ModelInfo, Vendor};

use crate::modelroute::{route_for, Route};
use crate::suggest::MenuItem;

#[path = "modelbadge.rs"]
mod modelbadge;
use modelbadge::{context_badge, price_badge};

/// Everything the query filters against: name, slug, alias, vendor, badges.
fn haystack(m: &ModelInfo) -> String {
    format!(
        "{} {} {} {} {}",
        m.name,
        m.slug,
        m.or_slug.unwrap_or(""),
        m.vendor.label(),
        if m.free { "free" } else { "paid" }
    )
    .to_lowercase()
}

fn matches(m: &ModelInfo, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let hay = haystack(m);
    hay.contains(query) || crate::suggest::is_subsequence(query, &hay)
}

/// The dim column: slug, price, context, current-mark, and route hint. Price,
/// free, and context are passed in already enriched (see [`enrich`]) rather
/// than read straight off `m`, so a live OpenRouter rate can fill a curated
/// `None`/`0` without this function needing to know where its numbers came
/// from.
fn desc(
    m: &ModelInfo,
    price: Option<(u64, u64)>,
    free: bool,
    context: u32,
    route: Route,
    current: bool,
) -> String {
    let mut parts: Vec<String> = vec![m.slug.to_string(), price_badge(price, free)];
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

#[path = "modelrecents.rs"]
mod modelrecents;
use modelrecents::recents_now;
pub(crate) use modelrecents::{set_recents, MAX_RECENTS};

/// The picker rows for `query`. `current` is the slug every agent is pinned to
/// (`None` when the roster disagrees or nothing is pinned) — it gets the `●`.
pub(crate) fn rows(query: &str, current: Option<&str>) -> Vec<MenuItem> {
    rows_with_recents(query, current, &recents_now())
}

/// `rows`, with the recent-picks list passed explicitly — split out so the
/// section is unit-testable without going through the process-global (see
/// `rows`'s doc comment for why the global exists at all).
pub(crate) fn rows_with_recents(
    query: &str,
    current: Option<&str>,
    recents: &[String],
) -> Vec<MenuItem> {
    let q = query.trim().to_lowercase();
    let (provider, probed) = crate::shellprobe::provider_now();
    let mut out = Vec::new();
    if "default".starts_with(&q) {
        out.push(default_row());
    }
    // A shortcut, not a move: a recent model still appears in its own vendor
    // section below too. An unknown slug (a model that left the catalog) is
    // skipped rather than rendered blank; if none survive, no header either
    // — this picker never emits an empty section. `seen` dedupes the raw
    // list first: a hand-edited config carrying the same slug twice must
    // still render exactly one recent row for it, not two identical ones.
    let mut seen = std::collections::HashSet::new();
    // A persisted recent may be the OpenRouter alias (`Route::fill_slug`
    // writes `m.or_slug` when OpenRouter is the active provider — both write
    // paths, the composer popup and the input bar, carry that value
    // verbatim), not the native `m.slug`. Match either, same as `is_current`
    // below, or an OpenRouter-primary user's recents never resolve to a row.
    // `.take` runs AFTER the catalog match (not before): an unresolvable
    // slug — a model that left the catalog — must not consume a recent slot.
    let recent: Vec<&ModelInfo> = recents
        .iter()
        .filter(|slug| seen.insert(slug.as_str()))
        .filter_map(|slug| {
            catalog()
                .iter()
                .find(|m| m.slug == *slug || m.or_slug == Some(slug.as_str()))
        })
        .filter(|m| matches(m, &q))
        .take(MAX_RECENTS)
        .collect();
    if !recent.is_empty() {
        out.push(header_row("recent"));
        for m in recent {
            let route = route_for(m, provider, probed);
            let is_current = current.is_some_and(|c| c == m.slug || Some(c) == m.or_slug);
            out.push(model_row(m, route, is_current));
        }
    }
    for vendor in Vendor::ORDER {
        let hits: Vec<&ModelInfo> = catalog()
            .iter()
            .filter(|m| m.vendor == *vendor && matches(m, &q))
            .collect();
        if hits.is_empty() {
            continue; // never emit an empty section
        }
        out.push(header_row(vendor.label()));
        for m in hits {
            let route = route_for(m, provider, probed);
            let is_current = current.is_some_and(|c| c == m.slug || Some(c) == m.or_slug);
            out.push(model_row(m, route, is_current));
        }
    }
    out
}

fn default_row() -> MenuItem {
    MenuItem {
        label: "default".to_string(),
        desc: "back to the provider default".to_string(),
        fill: "default".to_string(),
        submit: true,
        header: false,
        dim: false,
    }
}

fn header_row(label: &str) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        desc: String::new(),
        fill: String::new(),
        submit: false,
        header: true,
        dim: false,
    }
}

/// One model's row: label, badged desc, the slug to run, and whether the
/// active stack can't actually serve it (`route.unserveable()`) — the
/// composer popup renders that row dim rather than hiding it, so a user
/// still sees the model exists and what would fix it (see `desc`'s hint).
/// Factored out of `rows` so the dim wiring is unit-testable against an
/// explicit route, without the live provider probe `rows` itself depends on.
fn model_row(m: &ModelInfo, route: Route, current: bool) -> MenuItem {
    let (price, context, free) = modellive::enrich(m);
    MenuItem {
        label: m.name.to_string(),
        desc: desc(m, price, free, context, route, current),
        fill: route.fill_slug(m),
        submit: true,
        header: false,
        dim: route.unserveable(),
    }
}

#[path = "modellive.rs"]
mod modellive;
pub(crate) use modellive::set_live;

#[cfg(test)]
#[path = "modelpick_tests.rs"]
mod tests;
