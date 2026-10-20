//! Google sign-in for the inbox: the desktop flow Google documents for an
//! installed app ("OAuth 2.0 for iOS & Desktop Apps", read 2026-10-20). The
//! browser opens on Google's consent page; Google sends the user back to a
//! listener on `http://127.0.0.1:<random port>` (no port is registered for a
//! desktop client) with a code bound to a PKCE S256 challenge and to a random
//! `state`; the code is redeemed for a refresh token. The listener is
//! OpenRouter's (`oauth::await_callback`): one hardened wait, two flows.
//!
//! Everything blocking runs on a thread of its own, as `oauth::spawn` does:
//! the app runs on the winit thread.
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::Value;

use super::grant::{Client, Grant};
use crate::oauth::Waited;

/// Read, file (archive, read, star) and draft — never delete, which needs the
/// full-mail scope. `gmail.modify` covers `drafts.create` too, so one consent
/// serves every phase. Restricted, so the client is the user's.
pub(crate) const SCOPE: &str = "https://www.googleapis.com/auth/gmail.modify";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub(crate) const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub(crate) const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
/// How long the browser gets before the listener lets the port go.
const FLOW: Duration = Duration::from_secs(180);
const HTTP: Duration = Duration::from_secs(20);

/// The consent page's URL. `access_type=offline` and `prompt=consent` ask for
/// a refresh token every time, so a second sign-in after a lost token gets
/// one too.
pub(crate) fn authorize_url(
    client_id: &str,
    redirect: &str,
    challenge: &str,
    state: &str,
) -> String {
    let q = [
        ("client_id", client_id),
        ("redirect_uri", redirect),
        ("response_type", "code"),
        ("scope", SCOPE),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
        ("access_type", "offline"),
        ("prompt", "consent"),
    ];
    let q: Vec<String> = q
        .iter()
        .map(|(k, v)| format!("{k}={}", super::query::enc(v)))
        .collect();
    format!("{AUTH_URL}?{}", q.join("&"))
}

/// Start the sign-in. `None` only when no loopback port could be bound.
pub(crate) fn spawn(client: Client) -> Option<Receiver<Result<Grant, String>>> {
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    let pkce = crew_hive::oauth::pkce();
    let state = crew_hive::oauth::random_token(32);
    let redirect = format!("http://127.0.0.1:{port}");
    let url = authorize_url(&client.id, &redirect, &pkce.challenge, &state);
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = open::that_detached(&url);
        let got = match crate::oauth::await_callback(listener, &state, FLOW) {
            Waited::Code(code) => {
                let code = super::query::dec(&code);
                exchange(TOKEN_URL, &client, &code, &pkce.verifier, &redirect)
            }
            Waited::Denied(e) => Err(format!("Google said no ({e})")),
            Waited::TimedOut => Err("timed out waiting for the browser".into()),
            Waited::Broken(why) => Err(why.into()),
        };
        let _ = tx.send(got);
    });
    Some(rx)
}

/// Redeem `code` at `token_url` for the grant.
pub(crate) fn exchange(
    token_url: &str,
    client: &Client,
    code: &str,
    verifier: &str,
    redirect: &str,
) -> Result<Grant, String> {
    let v = post(
        token_url,
        &[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect),
            ("client_id", &client.id),
            ("client_secret", &client.secret),
        ],
    )?;
    let refresh = v["refresh_token"]
        .as_str()
        .ok_or("Google sent no refresh token \u{2014} sign in again")?;
    Ok(Grant {
        client: client.clone(),
        refresh_token: refresh.to_string(),
        email: String::new(),
    })
}

/// A fresh access token for `grant`, and its life in seconds.
pub(crate) fn refresh(token_url: &str, grant: &Grant) -> Result<(String, u64), String> {
    let v = post(
        token_url,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", &grant.refresh_token),
            ("client_id", &grant.client.id),
            ("client_secret", &grant.client.secret),
        ],
    )?;
    let token = v["access_token"]
        .as_str()
        .ok_or("Google sent no access token")?;
    Ok((token.to_string(), v["expires_in"].as_u64().unwrap_or(3600)))
}

/// Withdraw the grant at Google (best effort: signing out locally does not
/// wait on it).
pub(crate) fn revoke(revoke_url: &str, grant: &Grant) -> Result<(), String> {
    post(revoke_url, &[("token", &grant.refresh_token)]).map(|_| ())
}

/// One form POST, answered as JSON. An error says Google's own `error` and
/// `error_description`, clipped — never the request, which holds secrets.
fn post(url: &str, form: &[(&str, &str)]) -> Result<Value, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not start the http runtime".to_string())?;
    rt.block_on(async {
        let r = reqwest::Client::new()
            .post(url)
            .form(form)
            .timeout(HTTP)
            .send()
            .await
            .map_err(|e| format!("could not reach Google: {}", e.without_url()))?;
        let ok = r.status().is_success();
        let v: Value = r.json().await.unwrap_or(Value::Null);
        match ok {
            true => Ok(v),
            false => Err(google_error(&v)),
        }
    })
}

/// `invalid_grant: Token has been expired or revoked.`
pub(crate) fn google_error(v: &Value) -> String {
    let code = v["error"].as_str().unwrap_or("error");
    let text = match v["error_description"].as_str() {
        Some(d) => format!("{code}: {d}"),
        None => code.to_string(),
    };
    crate::oauth::short_reason(&text)
}

#[cfg(test)]
#[path = "signin_tests.rs"]
mod tests;
