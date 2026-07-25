//! Background enrichment of the model catalog. The fetch is an async HTTP call
//! in `crew-hive`; here it runs on a short-lived worker thread owning its own
//! current-thread tokio runtime (the `swarm::plan` pattern) and delivers over
//! an mpsc channel drained each frame — the winit thread never blocks. A disk
//! cache beside the config makes the second launch instant and keeps the
//! picker useful offline.
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use crew_hive::catalog::LiveModel;

/// How long a cached catalog stays fresh.
const TTL: Duration = Duration::from_secs(24 * 60 * 60);

fn cache_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("models-openrouter.json"))
}

/// Spawn the enrichment worker: cache first, network only when stale.
/// Returns immediately; `None` when there's nothing to do (no API key).
pub(crate) fn spawn() -> Option<Receiver<Vec<LiveModel>>> {
    let key = std::env::var("OPENROUTER_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        if let Some(cached) = read_cache() {
            let _ = tx.send(cached);
            return;
        }
        let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        if let Ok(models) = rt.block_on(crew_hive::catalog::fetch_openrouter(&key)) {
            write_cache(&models);
            let _ = tx.send(models);
        }
    });
    Some(rx)
}

/// The cached catalog when it exists and is younger than [`TTL`].
fn read_cache() -> Option<Vec<LiveModel>> {
    let path = cache_path()?;
    let age = std::fs::metadata(&path)
        .ok()?
        .modified()
        .ok()?
        .elapsed()
        .ok()?;
    if age > TTL {
        return None;
    }
    let raw = std::fs::read_to_string(&path).ok()?;
    crew_hive::catalog::parse_models(&raw).ok()
}

/// Best-effort cache write — a failure just means a fetch next launch. The
/// file's own mtime (set by `fs::write`) is the freshness stamp `read_cache`
/// checks; nothing else needs to record when this ran.
fn write_cache(models: &[LiveModel]) {
    let Some(path) = cache_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, cache_body(models));
}

/// Serialise into the same `{"data": [{"pricing": {"prompt": "...", ...}}]}`
/// shape [`crew_hive::catalog::parse_models`] reads, so a written cache reads
/// back byte-for-byte identical in price and context — split out from
/// `write_cache` so the round trip is testable without touching disk.
fn cache_body(models: &[LiveModel]) -> String {
    serde_json::json!({
        "data": models.iter().map(|m| serde_json::json!({
            "id": m.id,
            "name": m.name,
            "context_length": m.context,
            "pricing": {
                "prompt": m.price.map_or(String::new(), |(i, _)| per_token(i)),
                "completion": m.price.map_or(String::new(), |(_, o)| per_token(o)),
            },
        })).collect::<Vec<_>>(),
    })
    .to_string()
}

/// µ$/Mtok → the USD-per-token decimal string shape `parse_models` reads back.
/// µ$/Mtok is an integer number of millionths of a dollar per million tokens,
/// i.e. `1e-12` dollars per token exactly — so it's rendered as a fixed-point
/// string at 12 decimal places rather than through `f64`, which can't
/// represent most of these fractions exactly and would drift on read-back.
fn per_token(microusd_per_mtok: u64) -> String {
    let whole = microusd_per_mtok / 1_000_000_000_000;
    let frac = microusd_per_mtok % 1_000_000_000_000;
    format!("{whole}.{frac:012}")
}

#[cfg(test)]
#[path = "modelfetch_tests.rs"]
mod tests;
