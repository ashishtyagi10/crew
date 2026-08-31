//! Live catalog enrichment from the OpenRouter `/models` API: real list
//! prices, context windows, and the current free tier. Parsing is split from
//! the request so it's testable without a network. Prices arrive as
//! USD-per-token decimal strings; an unparseable one stays `None` (the badge
//! renders `—`) rather than becoming a wrong number.
use super::LiveModel;

const ENDPOINT: &str = "https://openrouter.ai/api/v1/models";

/// USD-per-token string → µ$ per 1M tokens. `None` when it isn't a number.
fn per_mtok(raw: &str) -> Option<u64> {
    let usd: f64 = raw.trim().parse().ok()?;
    if !usd.is_finite() || usd < 0.0 {
        return None;
    }
    Some((usd * 1_000_000.0 * 1_000_000.0).round() as u64)
}

/// Parse a `/models` response body.
pub fn parse_models(body: &str) -> Result<Vec<LiveModel>, String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let data = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "no `data` array".to_string())?;
    Ok(data
        .iter()
        .filter_map(|m| {
            let id = m.get("id")?.as_str()?.to_string();
            let name = m
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or(&id)
                .to_string();
            let pricing = m.get("pricing");
            let inp = pricing
                .and_then(|p| p.get("prompt"))
                .and_then(|p| p.as_str())
                .and_then(per_mtok);
            let out = pricing
                .and_then(|p| p.get("completion"))
                .and_then(|p| p.as_str())
                .and_then(per_mtok);
            let price = inp.zip(out);
            // `try_from` rather than `as u32`: a garbage value past u32::MAX
            // (e.g. `4294967296`) must fall back to "unknown" (0), not wrap
            // silently into a small, wrong context window.
            let context = m
                .get("context_length")
                .and_then(|c| c.as_u64())
                .and_then(|c| u32::try_from(c).ok())
                .unwrap_or(0);
            Some(LiveModel {
                free: price == Some((0, 0)),
                id,
                name,
                price,
                context,
            })
        })
        .collect())
}

/// Fetch the live catalog. Bounded; any failure is the caller's cue to keep
/// using the static catalog.
pub async fn fetch(api_key: &str) -> Result<Vec<LiveModel>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let body = client
        .get(ENDPOINT)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    parse_models(&body)
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
