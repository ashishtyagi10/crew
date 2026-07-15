//! The project-local specialist store: `./.crew/specialists.json`, alongside
//! `session-live.md`.
//!
//! This file is not merely persistence — it is the accumulation mechanism.
//! `Session::registry()` calls `Registry::discover_with` on every hello, send
//! and construct, rebuilding from scratch each time, so there is no long-lived
//! registry to hold a growing roster. Re-reading a file per rebuild makes
//! accumulation and durability one thing, and keeps new mutable state out of
//! `Session`.
//!
//! Every write is best-effort: specialists are a convenience, not a run's
//! product, so a failure here must never fail a run.
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Most specialists kept. Sized against the prompt spike's worst case: ~5 new
/// specialists per run with little name reuse (28 distinct / 32 tasks), so 24
/// is about five runs of history before the LRU trims a tail. A tighter cap
/// turns the roster over every couple of runs — churn, not a network.
pub(crate) const CAP: usize = 24;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct Specialist {
    /// The `@`-handle — always a valid `agentname::slug`.
    pub name: String,
    /// Prose craft hint. May be empty.
    pub role: String,
    /// Unix ms of last use, for LRU eviction. Bumped by a run that invents the
    /// name (`record`) and by an `@`-dial (`touch`).
    pub last_used: u64,
}

fn path(base: &Path) -> PathBuf {
    base.join(".crew").join("specialists.json")
}

/// The project dir the store lives under. `CREW_PROJECT_DIR` overrides the
/// process CWD — the seam tests use, since lib tests share one CWD and cannot
/// each chdir. Production never sets it: the broker's CWD *is* the project.
fn base_dir() -> PathBuf {
    std::env::var("CREW_PROJECT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub(crate) fn load() -> Vec<Specialist> {
    load_at(&base_dir())
}

/// Read the store. Absent, unreadable or corrupt → empty: a broken file must
/// degrade to "no specialists yet", never break the broker.
pub(crate) fn load_at(base: &Path) -> Vec<Specialist> {
    let Ok(raw) = std::fs::read_to_string(path(base)) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub(crate) fn record(seen: &[(String, String)]) {
    record_at(&base_dir(), seen)
}

/// Merge `(name, role)` pairs into the store: a name already present keeps its
/// original role and is bumped to now; a new name is inserted. Over [`CAP`],
/// the least-recently-used are dropped. Best-effort — errors are swallowed.
pub(crate) fn record_at(base: &Path, seen: &[(String, String)]) {
    let mut all = load_at(base);
    let now = now_ms();
    for (name, role) in seen {
        // Defence in depth: parse_plan already slugs, but this store is also
        // the e2e seam and a hand-written file could carry anything.
        let Some(name) = crew_hive::agentname::slug(name) else {
            continue;
        };
        match all.iter().position(|s| s.name == name) {
            Some(i) => {
                // Move-to-front, not update-in-place: `save_at`'s sort is
                // stable, so among entries sharing a `last_used` millisecond
                // physical order decides who is evicted. A just-touched entry
                // must outrank an equally-stamped older one.
                let mut s = all.remove(i);
                s.last_used = now;
                all.insert(0, s);
            }
            None => all.insert(
                0,
                Specialist {
                    name,
                    role: crew_hive::agentname::role_clamp(role),
                    last_used: now,
                },
            ),
        }
    }
    save_at(base, all);
}

pub(crate) fn touch(name: &str) {
    touch_at(&base_dir(), name)
}

/// Bump `name`'s recency without inventing it — the `@`-dial path, so that use
/// defers eviction and not only re-invention does.
pub(crate) fn touch_at(base: &Path, name: &str) {
    let mut all = load_at(base);
    let Some(i) = all.iter().position(|s| s.name == name) else {
        return;
    };
    // Move-to-front, matching `record_at`: a stable sort needs the
    // just-touched entry to outrank equally-stamped older ones.
    let mut s = all.remove(i);
    s.last_used = now_ms();
    all.insert(0, s);
    save_at(base, all);
}

/// Sort newest-first, trim to [`CAP`], write atomically (tmp + rename) so a
/// crash mid-write can't leave a torn file. Every failure is ignored.
fn save_at(base: &Path, mut all: Vec<Specialist>) {
    all.sort_by_key(|s| std::cmp::Reverse(s.last_used));
    all.truncate(CAP);
    let p = path(base);
    let Some(dir) = p.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(json) = serde_json::to_string_pretty(&all) else {
        return;
    };
    let tmp = p.with_extension("json.tmp");
    if std::fs::write(&tmp, json).is_ok() {
        let _ = std::fs::rename(&tmp, &p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);

    /// A fresh project dir per test — these run in parallel against a
    /// process-wide filesystem. Mirrors `tests/common::unique_dir`.
    fn tmp() -> PathBuf {
        let id = SEQ.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("crew-spec-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn absent_store_loads_empty() {
        assert!(load_at(&tmp()).is_empty());
    }

    #[test]
    fn corrupt_store_loads_empty_instead_of_panicking() {
        let base = tmp();
        std::fs::create_dir_all(base.join(".crew")).unwrap();
        std::fs::write(path(&base), "{not json").unwrap();
        assert!(load_at(&base).is_empty());
    }

    #[test]
    fn record_then_load_roundtrips() {
        let base = tmp();
        record_at(&base, &[("archivist".into(), "records, retrieval".into())]);
        let got = load_at(&base);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "archivist");
        assert_eq!(got[0].role, "records, retrieval");
        assert!(got[0].last_used > 0);
    }

    #[test]
    fn record_merges_by_name_rather_than_suffixing() {
        let base = tmp();
        record_at(&base, &[("analyst".into(), "first".into())]);
        record_at(&base, &[("analyst".into(), "second".into())]);
        let got = load_at(&base);
        assert_eq!(got.len(), 1, "same name is the same specialist: {got:?}");
        assert_eq!(got[0].role, "first", "the original role is kept");
    }

    #[test]
    fn record_skips_names_that_are_not_slugs() {
        let base = tmp();
        // "@#$" is `agentname::slug`'s own canonical unsalvageable example
        // (see `slug_or_derives_from_id_when_unsalvageable`): every char is
        // dropped, leaving nothing. A name with plain whitespace, like "Not A
        // Slug", is deliberately *salvageable* (hyphenated to "not-a-slug"),
        // so it wouldn't exercise this skip path.
        record_at(
            &base,
            &[("@#$".into(), "x".into()), ("ok-name".into(), "y".into())],
        );
        let got = load_at(&base);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "ok-name");
    }

    #[test]
    fn evicts_least_recently_used_at_cap() {
        let base = tmp();
        // Fill past the cap, oldest first.
        for i in 0..(CAP + 3) {
            record_at(&base, &[(format!("agent-{i:02}"), String::new())]);
        }
        let got = load_at(&base);
        assert_eq!(got.len(), CAP);
        let names: Vec<&str> = got.iter().map(|s| s.name.as_str()).collect();
        assert!(!names.contains(&"agent-00"), "oldest should be evicted");
        assert!(names.contains(&"agent-26"), "newest should survive");
    }

    #[test]
    fn touch_defers_eviction_for_a_dialed_specialist() {
        // Without touch, last_used only moves when a run re-invents a name, so
        // a specialist you @-dial daily would be evicted by unrelated churn.
        let base = tmp();
        record_at(&base, &[("favourite".into(), String::new())]);
        for i in 0..(CAP - 1) {
            record_at(&base, &[(format!("filler-{i:02}"), String::new())]);
        }
        touch_at(&base, "favourite");
        // Two more push past the cap; `favourite` must outlive the fillers.
        record_at(&base, &[("newcomer-a".into(), String::new())]);
        record_at(&base, &[("newcomer-b".into(), String::new())]);
        let names: Vec<String> = load_at(&base).into_iter().map(|s| s.name).collect();
        assert!(names.contains(&"favourite".to_string()), "got {names:?}");
    }

    #[test]
    fn touch_on_an_unknown_name_is_a_no_op() {
        let base = tmp();
        record_at(&base, &[("archivist".into(), String::new())]);
        touch_at(&base, "nobody");
        assert_eq!(load_at(&base).len(), 1);
    }

    #[test]
    fn a_same_millisecond_tie_evicts_the_earliest_recorded() {
        // One call ⇒ every name shares a `last_used`, so only physical order
        // can break the tie. The newest write must never be the one evicted.
        let base = tmp();
        let names: Vec<(String, String)> = (0..(CAP + 2))
            .map(|i| (format!("agent-{i:02}"), String::new()))
            .collect();
        record_at(&base, &names);
        let got: Vec<String> = load_at(&base).into_iter().map(|s| s.name).collect();
        assert_eq!(got.len(), CAP);
        assert!(
            !got.contains(&"agent-00".to_string()),
            "earliest recorded evicted: {got:?}"
        );
        assert!(
            got.contains(&format!("agent-{:02}", CAP + 1)),
            "latest recorded survives: {got:?}"
        );
    }
}
