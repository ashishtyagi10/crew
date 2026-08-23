//! The value list behind `auto`'s per-appearance pairing pickers
//! (`theme_dark` / `theme_light`).
//!
//! Unlike the light-hours boxes beside them these are a closed set, so they
//! are ‹ › pickers rather than typed fields. The list is every value the
//! config accepts, and that completeness is load-bearing: a picker that
//! could not reach `crt-green` would still have to DISPLAY it for someone who
//! set it in the config file, and a value the form can show but not produce
//! is a value the next Save silently drops.
//!
//! Both sides share one list, in one order. A light palette on the dark side
//! is legal (crew serves it while the OS is dark) and perverse, but filtering
//! it out per side would mean a config holding it could not round-trip
//! through the form — the exact failure above, reintroduced for tidiness.
use crew_theme::{RandomMode, Selection, ALL_THEMES};

/// Shown for the unset side: no pairing, so `auto` uses its built-in pool for
/// that appearance. Not a config value — unset is `None`.
pub(super) const DEFAULT_LABEL: &str = "default";

/// Every value a pairing side can hold, in picker order: unset, then the
/// three rotation pools, then every palette.
pub(super) fn values() -> Vec<Option<Selection>> {
    let mut v = vec![None];
    for m in [RandomMode::Dark, RandomMode::Light, RandomMode::Crt] {
        v.push(Some(Selection::Mode(m)));
    }
    v.extend(ALL_THEMES.into_iter().map(|id| Some(Selection::Fixed(id))));
    v
}

/// The picker's text for a value.
pub(super) fn label(v: Option<Selection>) -> &'static str {
    v.map_or(DEFAULT_LABEL, |s| s.label())
}

/// Step a side one place through [`values`], returning the config string to
/// store (`None` = unset). `cur` is the side's PARSED value, so an
/// unrecognised config string enters the cycle at `default` rather than
/// wedging it.
pub(super) fn cycle(cur: Option<Selection>, back: bool) -> Option<String> {
    let all = values();
    let i = all.iter().position(|&v| v == cur).unwrap_or(0);
    let next = if back {
        (i + all.len() - 1) % all.len()
    } else {
        (i + 1) % all.len()
    };
    all[next].map(|s| s.label().to_string())
}

#[cfg(test)]
#[path = "pairing_tests.rs"]
mod tests;
