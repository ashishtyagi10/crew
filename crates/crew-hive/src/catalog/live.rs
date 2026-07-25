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
            let context = m
                .get("context_length")
                .and_then(|c| c.as_u64())
                .unwrap_or(0) as u32;
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
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{"data":[
      {"id":"anthropic/claude-sonnet-5","name":"Anthropic: Claude Sonnet 5",
       "context_length":1000000,
       "pricing":{"prompt":"0.000003","completion":"0.000015"}},
      {"id":"meta-llama/llama-3.3-70b-instruct:free","name":"Meta: Llama 3.3 70B (free)",
       "context_length":131072,
       "pricing":{"prompt":"0","completion":"0"}},
      {"id":"weird/no-pricing","name":"No Pricing","context_length":0,
       "pricing":{"prompt":"","completion":""}}
    ]}"#;

    #[test]
    fn parses_per_token_strings_into_microusd_per_mtok() {
        let got = parse_models(FIXTURE).expect("fixture parses");
        let sonnet = got
            .iter()
            .find(|m| m.id == "anthropic/claude-sonnet-5")
            .unwrap();
        // $0.000003/token * 1M tokens = $3 = 3_000_000 µ$.
        assert_eq!(sonnet.price, Some((3_000_000, 15_000_000)));
        assert!(!sonnet.free);
        assert_eq!(sonnet.context, 1_000_000);
    }

    #[test]
    fn zero_price_is_free_and_unparseable_price_is_unknown() {
        let got = parse_models(FIXTURE).unwrap();
        let llama = got.iter().find(|m| m.id.ends_with(":free")).unwrap();
        assert!(llama.free);
        assert_eq!(llama.price, Some((0, 0)));
        let weird = got.iter().find(|m| m.id == "weird/no-pricing").unwrap();
        assert_eq!(weird.price, None); // never invent a number
        assert!(!weird.free);
    }

    #[test]
    fn malformed_json_is_an_error_not_a_panic() {
        assert!(parse_models("not json").is_err());
        assert!(parse_models("{}").is_err());
    }
}
