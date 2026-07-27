//! OpenRouter's OAuth PKCE flow: the parts that are pure computation or HTTP.
//! The browser launch and the loopback callback live in `crew-app`, which owns
//! the UI; this module is what makes the exchange correct and testable.
//!
//! crew needs no `client_id` and no app registration for this flow — verified
//! against OpenRouter's own documentation. Anthropic offers no third-party
//! OAuth (SP3 reuses an existing CLI profile instead) and DashScope none, so
//! this is OpenRouter-specific by nature.
//!
//! NEVER log the verifier, the code, or the returned key.
use base64::Engine;
use sha2::{Digest, Sha256};

/// Where the user approves, and where the code is redeemed.
const AUTHORIZE_URL: &str = "https://openrouter.ai/auth";
const EXCHANGE_URL: &str = "https://openrouter.ai/api/v1/auth/keys";

/// A PKCE pair. `verifier` NEVER leaves this process; only `challenge` is put
/// in a URL — that asymmetry is the whole point of PKCE, since an attacker who
/// intercepts the redirect still cannot redeem the code.
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

/// A fresh verifier (64 URL-safe characters, 384 bits from the OS CSPRNG) and
/// its S256 challenge.
///
/// The verifier is base64url of 48 random bytes rather than random draws from
/// an alphabet: 48 bytes encode to exactly 64 characters with no padding, and
/// it sidesteps the modulo bias a naive `byte % alphabet.len()` would carry.
pub fn pkce() -> Pkce {
    let verifier = random_token(64);
    let challenge = challenge_for(&verifier);
    Pkce {
        verifier,
        challenge,
    }
}

/// `chars` URL-safe random characters from the OS CSPRNG. Shared by the PKCE
/// verifier and by `crew-app`'s callback nonce, so both draw from one audited
/// source instead of one of them improvising.
///
/// Encoding random BYTES as base64url — rather than indexing an alphabet with
/// `byte % len` — keeps the distribution uniform: 3 bytes yield exactly 4
/// characters, and there is no modulo bias to argue about.
pub fn random_token(chars: usize) -> String {
    let bytes = chars.div_ceil(4) * 3;
    let mut buf = vec![0u8; bytes];
    fill_random(&mut buf);
    let mut s = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&buf);
    s.truncate(chars);
    s
}

/// RFC 7636 S256: base64url-nopad(sha256(verifier)).
fn challenge_for(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// The URL the browser opens. `callback_url` is percent-encoded because it
/// carries a `:` and `/`.
pub fn authorize_url(callback_url: &str, challenge: &str) -> String {
    format!(
        "{AUTHORIZE_URL}?callback_url={}&code_challenge={challenge}&code_challenge_method=S256",
        percent_encode(callback_url)
    )
}

/// Minimal percent-encoding for a query VALUE: everything outside the
/// unreserved set is escaped. Hand-rolled rather than pulling in a URL crate
/// for one call site.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Redeem an authorization code for the user's own OpenRouter key.
///
/// Never logs `code`, `verifier` or the key: on a non-2xx the error names the
/// STATUS only, deliberately not the body, which could echo the code back.
pub async fn exchange_openrouter_code(code: &str, verifier: &str) -> anyhow::Result<String> {
    let body = serde_json::json!({
        "code": code,
        "code_verifier": verifier,
        "code_challenge_method": "S256",
    });
    let resp = reqwest::Client::new()
        .post(EXCHANGE_URL)
        .json(&body)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("openrouter returned {status}");
    }
    let parsed: serde_json::Value = resp.json().await?;
    parsed
        .get("key")
        .and_then(|k| k.as_str())
        .filter(|k| !k.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("openrouter returned no key"))
}

fn fill_random(buf: &mut [u8]) {
    getrandom::getrandom(buf).expect("OS CSPRNG unavailable");
}

#[cfg(test)]
#[path = "oauth_tests.rs"]
mod tests;
