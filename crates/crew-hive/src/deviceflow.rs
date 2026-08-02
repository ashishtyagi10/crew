//! RFC 8628 device-authorization grant: the HTTP half. Start a flow (the
//! user code + verification URL), poll the token endpoint, refresh an
//! expired access token. Pure protocol over injected endpoint DATA — which
//! provider permits this flow, and at which URLs, is the broker's auth
//! registry's business (`crew-plugin`), never hardcoded here.
//!
//! Lives beside `oauth.rs` for the same reason that module exists: this
//! crate owns the HTTP client (reqwest), the broker owns the flow's timing
//! and storage, the pane owns the rendering.
//!
//! NEVER log a device code, an access token or a refresh token. Every error
//! this module returns names the HTTP status only — response bodies and
//! request parameters can echo secrets and stay out of error text.
use std::time::Duration;

/// Per-request ceiling (`oauth::EXCHANGE_TIMEOUT`'s rationale): the polling
/// budget covers the HUMAN; this covers a hung server pinning a worker.
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

/// Where one provider's device flow lives. Values come from the broker's
/// auth registry (or a test's stub server) — data, not constants here.
#[derive(Clone, Debug)]
pub struct DeviceEndpoints {
    /// The device-authorization endpoint (RFC 8628 §3.1).
    pub device_url: String,
    /// The token endpoint polled with the device-code grant (§3.4).
    pub token_url: String,
    pub client_id: String,
    pub scope: String,
}

/// What §3.2 hands back: everything the pane's code card shows, plus the
/// polling contract (interval, lifetime).
#[derive(Clone)]
pub struct DeviceStart {
    /// Secret — pairs this poll loop with the user's approval. Never shown.
    pub device_code: String,
    /// The short code the USER types — the one deliberately visible field.
    pub user_code: String,
    pub verification_uri: String,
    /// The uri with the code already embedded, when the server offers one.
    pub verification_uri_complete: Option<String>,
    /// Seconds between polls (§3.5; the server's word, default 5).
    pub interval: u64,
    /// Seconds until the device code expires (default 900).
    pub expires_in: u64,
}

/// A granted token set. `Debug` is hand-written so no token can ever reach a
/// panic message, `dbg!` or an `anyhow` context (`credentials::Store`'s rule).
#[derive(Clone)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Seconds of validity, when the server says.
    pub expires_in: Option<u64>,
    /// Some providers (Qwen) return the API host this token is valid
    /// against, which differs from their key-shaped endpoint.
    pub resource_url: Option<String>,
}

impl std::fmt::Debug for TokenSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let refresh = self.refresh_token.as_ref().map(|_| "<redacted>");
        f.debug_struct("TokenSet")
            .field("access_token", &"<redacted>")
            .field("refresh_token", &refresh)
            .field("expires_in", &self.expires_in)
            .field("resource_url", &self.resource_url)
            .finish()
    }
}

/// One poll's verdict (§3.5).
#[derive(Debug)]
pub enum DevicePoll {
    Ready(TokenSet),
    /// `authorization_pending` — keep polling at the current interval.
    Pending,
    /// `slow_down` — keep polling, interval + 5 seconds.
    SlowDown,
    /// `expired_token` — the device code died; the flow must restart.
    Expired,
    /// `access_denied` — the user refused.
    Denied,
}

/// POST `params` form-encoded, returning the status and parsed JSON body.
/// A body that isn't JSON parses as `null` — the RFC error mapping below
/// then falls through to the status-only error.
async fn post_form(
    url: &str,
    params: &[(&str, &str)],
) -> anyhow::Result<(reqwest::StatusCode, serde_json::Value)> {
    let client = reqwest::Client::builder().timeout(HTTP_TIMEOUT).build()?;
    let resp = client.post(url).form(params).send().await?;
    let status = resp.status();
    let body = resp.json().await.unwrap_or(serde_json::Value::Null);
    Ok((status, body))
}

fn field(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key).and_then(|s| s.as_str()).map(str::to_string)
}

fn num(v: &serde_json::Value, key: &str, or: u64) -> u64 {
    v.get(key).and_then(serde_json::Value::as_u64).unwrap_or(or)
}

/// Parse a 2xx token response. Absent `access_token` is a protocol error —
/// reported without quoting the body.
fn token_set(body: &serde_json::Value) -> anyhow::Result<TokenSet> {
    Ok(TokenSet {
        access_token: field(body, "access_token")
            .filter(|t| !t.is_empty())
            .ok_or_else(|| anyhow::anyhow!("token response carried no access token"))?,
        refresh_token: field(body, "refresh_token"),
        expires_in: body.get("expires_in").and_then(serde_json::Value::as_u64),
        resource_url: field(body, "resource_url").or_else(|| field(body, "endpoint")),
    })
}

/// §3.1: request a device + user code pair. `challenge` is a PKCE S256
/// challenge (`oauth::pkce`), sent when the provider's flow uses one (Qwen's
/// does); a plain RFC 8628 server ignores the extra parameters.
pub async fn device_start(
    e: &DeviceEndpoints,
    challenge: Option<&str>,
) -> anyhow::Result<DeviceStart> {
    let mut params = vec![
        ("client_id", e.client_id.as_str()),
        ("scope", e.scope.as_str()),
    ];
    if let Some(c) = challenge {
        params.push(("code_challenge", c));
        params.push(("code_challenge_method", "S256"));
    }
    let (status, body) = post_form(&e.device_url, &params).await?;
    if !status.is_success() {
        anyhow::bail!("device authorization returned {status}");
    }
    Ok(DeviceStart {
        device_code: field(&body, "device_code")
            .filter(|c| !c.is_empty())
            .ok_or_else(|| anyhow::anyhow!("device authorization carried no device code"))?,
        user_code: field(&body, "user_code").unwrap_or_default(),
        verification_uri: field(&body, "verification_uri").unwrap_or_default(),
        verification_uri_complete: field(&body, "verification_uri_complete"),
        interval: num(&body, "interval", 5),
        expires_in: num(&body, "expires_in", 900),
    })
}

/// §3.4/§3.5: one poll of the token endpoint. `verifier` is the PKCE
/// verifier paired with the `device_start` challenge.
pub async fn device_poll(
    e: &DeviceEndpoints,
    device_code: &str,
    verifier: Option<&str>,
) -> anyhow::Result<DevicePoll> {
    let mut params = vec![
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", device_code),
        ("client_id", e.client_id.as_str()),
    ];
    if let Some(v) = verifier {
        params.push(("code_verifier", v));
    }
    let (status, body) = post_form(&e.token_url, &params).await?;
    if status.is_success() {
        return Ok(DevicePoll::Ready(token_set(&body)?));
    }
    match field(&body, "error").as_deref() {
        Some("authorization_pending") => Ok(DevicePoll::Pending),
        Some("slow_down") => Ok(DevicePoll::SlowDown),
        Some("expired_token") => Ok(DevicePoll::Expired),
        Some("access_denied") => Ok(DevicePoll::Denied),
        _ => anyhow::bail!("token endpoint returned {status}"),
    }
}

/// RFC 6749 §6: trade a refresh token for a fresh set. Every failure is
/// hard from the caller's point of view — the broker treats an `Err` as
/// "sign in again", never retries with the same refresh token.
pub async fn device_refresh(e: &DeviceEndpoints, refresh_token: &str) -> anyhow::Result<TokenSet> {
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", e.client_id.as_str()),
    ];
    let (status, body) = post_form(&e.token_url, &params).await?;
    if !status.is_success() {
        anyhow::bail!("token refresh returned {status}");
    }
    token_set(&body)
}

#[cfg(test)]
#[path = "deviceflow_tests.rs"]
mod tests;
