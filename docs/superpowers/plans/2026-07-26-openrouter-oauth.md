# Provider Auth SP2 — OpenRouter OAuth (PKCE) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Accepting a dimmed OpenRouter row opens the browser; the user approves; crew receives their own OpenRouter key and stores it — no typing, no shared quota, no secret in the binary.

**Architecture:** `crew-hive` owns PKCE and the token exchange (it has `reqwest`); `crew-app` owns the browser launch and a single-shot loopback callback listener (it has `open`). The flow runs on a spawned thread and delivers one outcome through an `mpsc::Receiver` polled non-blockingly in `poll.rs`, exactly as `modelfetch` already does. Success reuses SP1's `chatkeystore::store_provider_key`, so both entry paths share one save.

**Tech Stack:** Rust, `reqwest`, `sha2`, `base64`, `getrandom`, `open`, `std::net::TcpListener`.

**Spec:** `docs/superpowers/specs/2026-07-26-openrouter-oauth-design.md`

## Global Constraints

- **NEVER run `cargo build --release` or `cargo clean`** — disk is tight on this machine. `cargo test` / `cargo clippy` (dev profile) only.
- **Never log, print, echo or `{:?}`-format** the PKCE verifier, the authorization code, or the returned key. Failure messages describe the FLOW (timeout, network, non-2xx status), never a credential.
- **Nothing blocking may run on the winit thread.** All work in this app is synchronous on that thread; a blocking `accept()` there freezes every pane. The listener, the browser launch and the exchange all live on a spawned thread; the main thread only ever calls `try_recv()`.
- **The listener binds `127.0.0.1:0`** — loopback only, never `0.0.0.0`, ephemeral port.
- Exact endpoints: authorize `https://openrouter.ai/auth`, exchange `POST https://openrouter.ai/api/v1/auth/keys`. Exact parameters: `callback_url`, `code_challenge`, `code_challenge_method=S256`; body `{code, code_verifier, code_challenge_method}`; response field `key`. **No `client_id` — OpenRouter requires no app registration.**
- Tests must not mutate process-global environment, touch the real config directory, or reach the network.
- Keep source files under ~200 lines; tests in a sibling `<name>_tests.rs`.
- Run `cargo fmt` before committing (a pre-commit hook enforces it).
- `cargo clippy --workspace --all-targets -- -D warnings` green, no `#[allow(...)]` added.
- Never commit keys — this repo is public; use obvious fakes.

---

### Task 1: PKCE and the token exchange (`crew-hive`)

**Files:**
- Create: `crates/crew-hive/src/oauth.rs`, `crates/crew-hive/src/oauth_tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (declare + re-export), `crates/crew-hive/Cargo.toml` (add `base64`, `getrandom`), root `Cargo.toml` (workspace entries if that is the convention there — `sha2` already exists at line 45)

**Interfaces:**
- Consumes: nothing.
- Produces: `crew_hive::oauth::{Pkce, pkce, exchange_openrouter_code}` — Task 3 calls `pkce()` and `exchange_openrouter_code(code, verifier)`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/oauth_tests.rs`:

```rust
use super::*;

/// RFC 7636 Appendix B's S256 vector. This pins the ENCODING (base64url,
/// no padding, `-`/`_` not `+`/`/`) against an external authority rather than
/// against our own implementation — a self-consistent test would pass just as
/// happily with standard base64, which OpenRouter would reject.
#[test]
fn the_challenge_matches_the_rfc_7636_vector() {
    assert_eq!(
        challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

#[test]
fn a_verifier_is_long_url_safe_and_unpredictable() {
    let a = pkce();
    let b = pkce();
    assert_eq!(a.verifier.chars().count(), 64);
    assert!(
        a.verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "verifier must be URL-safe: {}",
        a.verifier
    );
    assert_ne!(a.verifier, b.verifier, "two flows must not share a verifier");
    assert_eq!(a.challenge, challenge_for(&a.verifier));
}

#[test]
fn a_challenge_carries_no_padding_or_unsafe_characters() {
    let p = pkce();
    assert!(!p.challenge.contains('='), "no padding: {}", p.challenge);
    assert!(!p.challenge.contains('+'), "url-safe alphabet: {}", p.challenge);
    assert!(!p.challenge.contains('/'), "url-safe alphabet: {}", p.challenge);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p crew-hive oauth`
Expected: compile error — the module does not exist.

- [ ] **Step 3: Add the dependencies**

In `crates/crew-hive/Cargo.toml` add `base64` and `getrandom`. Both are already in `Cargo.lock` transitively, so pick the versions already resolved there (`grep -A1 'name = "base64"' Cargo.lock`) rather than introducing a second major version. Follow the file's existing style — if its other entries use `x = { workspace = true }`, add the crates to the root `[workspace.dependencies]` first, beside `sha2` at `Cargo.toml:45`.

**API note:** `getrandom` 0.2 exposes `getrandom::getrandom(&mut buf)`; 0.3 renamed it to `getrandom::fill(&mut buf)`. Use whichever the resolved version provides — the compiler will tell you immediately.

- [ ] **Step 4: Write the module**

Create `crates/crew-hive/src/oauth.rs`:

```rust
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
```

Declare it in `crates/crew-hive/src/lib.rs` (`pub mod oauth;`) following the file's existing style.

- [ ] **Step 5: Add a test for the URL builder**

Append to `oauth_tests.rs`:

```rust
#[test]
fn the_authorize_url_carries_exactly_the_documented_parameters() {
    let url = authorize_url("http://127.0.0.1:8731/abc", "CHAL");
    assert!(url.starts_with("https://openrouter.ai/auth?"), "{url}");
    assert!(url.contains("code_challenge=CHAL"), "{url}");
    assert!(url.contains("code_challenge_method=S256"), "{url}");
    // The callback must be escaped, or the `:` and `/` truncate the parameter.
    assert!(
        url.contains("callback_url=http%3A%2F%2F127.0.0.1%3A8731%2Fabc"),
        "{url}"
    );
    // No client_id: OpenRouter requires no app registration, and inventing one
    // would break the flow.
    assert!(!url.contains("client_id"), "{url}");
}
```

- [ ] **Step 6: Run tests, lint, commit**

Run: `cargo test -p crew-hive` then `cargo clippy --workspace --all-targets -- -D warnings`. Both green.

```bash
git add crates/crew-hive Cargo.toml Cargo.lock
git commit -m "feat(hive): PKCE and the OpenRouter code exchange"
```

---

### Task 2: The callback parser (`crew-app`)

Pure functions with no I/O, so the flow's trickiest logic is fully testable before any socket exists.

**Files:**
- Create: `crates/crew-app/src/oauth.rs`, `crates/crew-app/src/oauth_tests.rs`
- Modify: `crates/crew-app/src/main.rs` (`mod oauth;`, alphabetical)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces, `pub(crate)` in `crate::oauth`: `enum Callback { Code(String), Denied(String), Ignore }` and `fn parse_request(line: &str, nonce: &str) -> Callback` — Task 3 feeds it the first line of an HTTP request.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/oauth_tests.rs`:

```rust
use super::*;

#[test]
fn a_matching_path_with_a_code_yields_it() {
    match parse_request("GET /n0nce?code=abc123 HTTP/1.1", "n0nce") {
        Callback::Code(c) => assert_eq!(c, "abc123"),
        other => panic!("expected a code, got {other:?}"),
    }
}

#[test]
fn a_wrong_nonce_is_ignored() {
    // Any local process can reach the port. The nonce is what stops another
    // one feeding us a code — the loopback equivalent of an OAuth `state`.
    assert!(matches!(
        parse_request("GET /guessed?code=abc123 HTTP/1.1", "n0nce"),
        Callback::Ignore
    ));
}

#[test]
fn a_denial_is_reported_not_swallowed() {
    match parse_request("GET /n0nce?error=access_denied HTTP/1.1", "n0nce") {
        Callback::Denied(e) => assert_eq!(e, "access_denied"),
        other => panic!("expected a denial, got {other:?}"),
    }
}

#[test]
fn junk_and_empty_queries_are_ignored() {
    for line in [
        "GET /n0nce HTTP/1.1",
        "GET /n0nce?code= HTTP/1.1",
        "GET / HTTP/1.1",
        "",
        "not an http request at all",
        "POST /n0nce?code=abc HTTP/1.1",
    ] {
        assert!(
            matches!(parse_request(line, "n0nce"), Callback::Ignore),
            "should ignore: {line:?}"
        );
    }
}

#[test]
fn extra_parameters_around_the_code_do_not_confuse_it() {
    match parse_request("GET /n0nce?state=x&code=abc&scope=y HTTP/1.1", "n0nce") {
        Callback::Code(c) => assert_eq!(c, "abc"),
        other => panic!("expected a code, got {other:?}"),
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p crew-app oauth`
Expected: compile error — the module does not exist.

- [ ] **Step 3: Write the parser**

Create `crates/crew-app/src/oauth.rs` with the module doc and:

```rust
/// What the single callback request turned out to be.
#[derive(Debug, PartialEq)]
pub(crate) enum Callback {
    Code(String),
    Denied(String),
    /// Not ours, or carries nothing usable — answer 404 and keep waiting.
    Ignore,
}

/// Parse the request line of the one callback we expect.
///
/// `nonce` guards the port: it is loopback, but any process on this machine
/// can connect to it, so a request whose path is not exactly our nonce is not
/// ours. This is the `state` equivalent for a localhost callback.
pub(crate) fn parse_request(line: &str, nonce: &str) -> Callback {
    let mut parts = line.split_whitespace();
    if parts.next() != Some("GET") {
        return Callback::Ignore;
    }
    let Some(target) = parts.next() else {
        return Callback::Ignore;
    };
    let Some((path, query)) = target.split_once('?') else {
        return Callback::Ignore;
    };
    if path.trim_start_matches('/') != nonce {
        return Callback::Ignore;
    }
    let mut code = None;
    let mut error = None;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("code", v)) if !v.is_empty() => code = Some(v.to_string()),
            Some(("error", v)) if !v.is_empty() => error = Some(v.to_string()),
            _ => {}
        }
    }
    match (code, error) {
        (Some(c), _) => Callback::Code(c),
        (None, Some(e)) => Callback::Denied(e),
        _ => Callback::Ignore,
    }
}

#[cfg(test)]
#[path = "oauth_tests.rs"]
mod tests;
```

Register `mod oauth;` in `main.rs` in its alphabetical slot.

- [ ] **Step 4: Run, lint, commit**

Run: `cargo test -p crew-app oauth` (all five pass), then `cargo clippy --workspace --all-targets -- -D warnings`.

```bash
git add crates/crew-app/src/oauth.rs crates/crew-app/src/oauth_tests.rs crates/crew-app/src/main.rs
git commit -m "feat(chat): parse the OpenRouter OAuth callback"
```

---

### Task 3: The flow driver — listener, browser, exchange

**Files:**
- Modify: `crates/crew-app/src/oauth.rs` (add the driver), `crates/crew-app/src/oauth_tests.rs` (listener test)

**Interfaces:**
- Consumes: `crew_hive::oauth::{pkce, authorize_url, exchange_openrouter_code}` (Task 1); `parse_request`/`Callback` (Task 2).
- Produces: `pub(crate) enum OauthOutcome { Key(String), Failed(String) }` and `pub(crate) fn spawn() -> Option<Receiver<OauthOutcome>>` — Task 4 polls it.

- [ ] **Step 1: Write the failing test**

Append to `oauth_tests.rs`. It drives a REAL loopback listener from the test, so it is hermetic and needs no network:

```rust
#[test]
fn the_listener_returns_the_code_from_a_matching_request() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let h = std::thread::spawn(move || await_callback(listener, "n0nce", TEST_TIMEOUT));
    // A wrong-nonce request must not end the wait...
    let _ = ureq_like_get(port, "/wrong?code=nope");
    let _ = ureq_like_get(port, "/n0nce?code=the-code");
    assert_eq!(h.join().unwrap(), Callback::Code("the-code".into()));
}

#[test]
fn the_listener_gives_up_rather_than_waiting_forever() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let started = std::time::Instant::now();
    let got = await_callback(listener, "n0nce", std::time::Duration::from_millis(300));
    assert!(matches!(got, Callback::Ignore), "{got:?}");
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "did not time out");
}

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Minimal HTTP GET over a raw socket — the app has no HTTP client and this
/// test needs no dependency to make one request.
fn ureq_like_get(port: u16, target: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port))?;
    write!(s, "GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
    Ok(())
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-app oauth`
Expected: compile error — `await_callback` does not exist.

- [ ] **Step 3: Write the driver**

Append to `crates/crew-app/src/oauth.rs`:

```rust
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// How long the user gets to approve before crew stops holding the port.
const FLOW_TIMEOUT: Duration = Duration::from_secs(180);
/// How often the listener wakes to re-check the deadline.
const POLL_GAP: Duration = Duration::from_millis(100);

pub(crate) enum OauthOutcome {
    Key(String),
    /// A FLOW failure — timeout, network, non-2xx. Never a credential.
    Failed(String),
}

/// Start the browser sign-in. Returns immediately; `None` only if the loopback
/// listener could not bind, in which case the caller falls back to pasting.
///
/// EVERYTHING blocking happens on the spawned thread: this app runs
/// synchronously on the winit thread, so a blocking accept there would freeze
/// every pane. The caller polls the receiver with `try_recv`.
pub(crate) fn spawn() -> Option<Receiver<OauthOutcome>> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    let pkce = crew_hive::oauth::pkce();
    let nonce = nonce();
    let callback = format!("http://127.0.0.1:{port}/{nonce}");
    let url = crew_hive::oauth::authorize_url(&callback, &pkce.challenge);
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        // If the browser can't be opened the user can still paste, so this is
        // not fatal on its own — the wait below simply times out.
        let _ = open::that(&url);
        let outcome = match await_callback(listener, &nonce, FLOW_TIMEOUT) {
            Callback::Code(code) => exchange(&code, &pkce.verifier),
            Callback::Denied(e) => OauthOutcome::Failed(e),
            Callback::Ignore => OauthOutcome::Failed("timed out".into()),
        };
        // A failed send means the user cancelled and dropped the receiver.
        let _ = tx.send(outcome);
    });
    Some(rx)
}

/// 32 URL-safe random characters guarding the callback path.
fn nonce() -> String {
    crew_hive::oauth::random_token(32)
}

/// Serve requests until one matches `nonce` or `timeout` elapses. Answers
/// every request (matching or not) so no browser tab hangs, but only a
/// matching one ends the wait.
fn await_callback(
    listener: std::net::TcpListener,
    nonce: &str,
    timeout: Duration,
) -> Callback {
    use std::io::{BufRead, BufReader, Write};
    if listener.set_nonblocking(true).is_err() {
        return Callback::Ignore;
    }
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                let _ = reader.read_line(&mut line);
                let got = parse_request(line.trim_end(), nonce);
                let body = match got {
                    Callback::Ignore => "not this crew window",
                    _ => "crew is signed in — you can close this tab.",
                };
                let mut s = &stream;
                let _ = write!(
                    s,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                if !matches!(got, Callback::Ignore) {
                    return got;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(POLL_GAP)
            }
            Err(_) => return Callback::Ignore,
        }
    }
    Callback::Ignore
}

/// Redeem the code on a current-thread runtime, the same shape `modelfetch`
/// uses for its one async call.
fn exchange(code: &str, verifier: &str) -> OauthOutcome {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return OauthOutcome::Failed("could not start the http runtime".into());
    };
    match rt.block_on(crew_hive::oauth::exchange_openrouter_code(code, verifier)) {
        Ok(key) => OauthOutcome::Key(key),
        Err(e) => OauthOutcome::Failed(e.to_string()),
    }
}
```

- [ ] **Step 4: Run, lint, commit**

Run: `cargo test -p crew-app oauth` — all pass. Then `cargo clippy --workspace --all-targets -- -D warnings`.

If `spawn` trips `dead_code` (no caller until Task 4), do NOT add `#[allow]` and do NOT delete it — fold Task 4 into this commit and say so in the commit message.

```bash
git add crates/crew-app/src/oauth.rs crates/crew-app/src/oauth_tests.rs
git commit -m "feat(chat): drive the OpenRouter browser sign-in off the winit thread"
```

---

### Task 4: Wire it into the prompt

**Files:**
- Modify: `crates/crew-app/src/keyentry.rs` (a `waiting` flag + its interior line), `crates/crew-app/src/chat.rs` (start the flow on the OpenRouter row; hold the receiver), `crates/crew-app/src/poll.rs` (poll it), `crates/crew-app/src/keyentry_tests.rs`, `crates/crew-app/src/chat_tests.rs`

**Interfaces:**
- Consumes: `oauth::{spawn, OauthOutcome}` (Task 3); `chatkeystore::store_provider_key` (shipped in SP1).
- Produces: nothing — this is the last task.

- [ ] **Step 1: Write the failing tests**

Append to `keyentry_tests.rs`:

```rust
#[test]
fn a_waiting_prompt_says_so_and_still_masks_what_is_typed() {
    let mut e = KeyEntry::new("OPENROUTER_API_KEY".into());
    e.set_waiting(true);
    let drawn: String = e.card(60).iter().map(|c| c.c).collect();
    assert!(drawn.contains("waiting for browser"), "{drawn}");
    // Typing must still work — the browser may never have opened.
    typed(&mut e, "sk-typed");
    let cells = e.card(60);
    let interior: String = cells
        .iter()
        .filter(|c| c.row == 1)
        .map(|c| c.c)
        .collect();
    for ch in "sk-typed".chars() {
        assert!(!interior.contains(ch), "character {ch:?} reached the screen");
    }
}

#[test]
fn typing_clears_the_waiting_state() {
    // Once the user starts pasting, the card should stop claiming to wait.
    let mut e = KeyEntry::new("OPENROUTER_API_KEY".into());
    e.set_waiting(true);
    typed(&mut e, "s");
    let drawn: String = e.card(60).iter().map(|c| c.c).collect();
    assert!(!drawn.contains("waiting for browser"), "{drawn}");
}
```

Append to `chat_tests.rs`:

```rust
#[test]
fn an_oauth_key_is_stored_through_the_same_path_as_a_typed_one() {
    // One save path for both entry methods, or pinning and re-resolution
    // could diverge between them.
    let mut p = pane();
    let dir = std::env::temp_dir().join(format!("crew-oauth-store-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = dir.join("credentials.json");
    crate::chatkeystore::store_provider_key_at(
        &mut p,
        &store,
        "OPENROUTER_API_KEY",
        "sk-test-not-a-real-key",
    );
    let loaded = crew_plugin::credentials::load_from(&store);
    assert_eq!(
        loaded.keys.get("OPENROUTER_API_KEY").map(String::as_str),
        Some("sk-test-not-a-real-key")
    );
    assert_eq!(loaded.provider.as_deref(), Some("openrouter"));
    let _ = std::fs::remove_dir_all(&dir);
}
```

`store_provider_key_at` is currently private to `chatkeystore`; make it `pub(crate)` for this test rather than duplicating its logic.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p crew-app keyentry && cargo test -p crew-app chat_tests`
Expected: compile errors — no `set_waiting`, `store_provider_key_at` private.

- [ ] **Step 3: Add the waiting state**

In `keyentry.rs`, add `waiting: bool` to `KeyEntry` (default `false` in `new`), and:

```rust
    /// Show that a browser sign-in is in flight. Cleared as soon as the user
    /// types, since that means they are pasting instead.
    pub(crate) fn set_waiting(&mut self, waiting: bool) {
        self.waiting = waiting;
    }
```

Set `self.waiting = false;` in the `ChatInput::Char` arm of `key`. Raise `ROWS` to `4` and, in `card`, draw the hint on row 2 when `waiting`:

```rust
        if self.waiting {
            let hint = "waiting for browser · or paste the key";
            for (i, ch) in hint.chars().take(inner).enumerate() {
                cells.push(CellView {
                    col: 1 + i as u16,
                    row: 2,
                    c: ch,
                    fg: t.text_muted,
                    bg: t.page_bg,
                    bold: false,
                    italic: false,
                });
            }
        }
```

The secret is still drawn only on row 1, so the existing row-scoped leak assertion keeps its meaning.

- [ ] **Step 4: Start the flow and poll it**

In `chat.rs`, add a field beside `keyentry`:

```rust
    /// An in-flight OpenRouter browser sign-in (see `oauth`). Dropped when the
    /// prompt closes, which is what cancels the worker thread.
    pub(crate) oauth: Option<std::sync::mpsc::Receiver<crate::oauth::OauthOutcome>>,
```

Initialise it to `None`, and clear it wherever `keyentry` is cleared (Escape, submit) so cancelling really cancels.

In the `PaletteKey::NeedsKey(var)` arm, start the browser flow when the row wants the OpenRouter key:

```rust
            crate::chatpalette::PaletteKey::NeedsKey(var) => {
                let mut entry = crate::keyentry::KeyEntry::new(var.clone());
                if var == "OPENROUTER_API_KEY" {
                    // OpenRouter is the one provider with a real third-party
                    // OAuth flow. The paste prompt still opens underneath, so
                    // a failed browser launch is never a dead end.
                    self.oauth = crate::oauth::spawn();
                    entry.set_waiting(self.oauth.is_some());
                }
                self.keyentry = Some(entry);
                return None;
            }
```

In `poll.rs`, beside the existing `model_fetch` `try_recv` branch, poll the focused chat pane's `oauth` receiver and route the outcome:

```rust
            match outcome {
                crate::oauth::OauthOutcome::Key(key) => {
                    crate::chatkeystore::store_provider_key(pane, "OPENROUTER_API_KEY", &key);
                    pane.keyentry = None;
                }
                crate::oauth::OauthOutcome::Failed(why) => {
                    pane.push_note(format!("openrouter sign-in failed: {why}"));
                    if let Some(e) = pane.keyentry.as_mut() {
                        e.set_waiting(false);
                    }
                }
            }
            pane.oauth = None;
```

`poll.rs` already binds `let focused = self.focused;` (`poll.rs:97`) and indexes `self.panes[…]`; reach the pane that way rather than adding a new traversal. Poll only the FOCUSED pane's receiver — the prompt is modal to its pane, and a background pane cannot have started a flow.

- [ ] **Step 4b: Prove the receiver is dropped on cancel**

Add to `chat_tests.rs`:

```rust
#[test]
fn escaping_the_prompt_cancels_an_in_flight_sign_in() {
    // Dropping the receiver is what makes the worker thread's send fail and
    // exit, so a cancelled flow must not leave it set.
    let mut p = pane();
    p.keyentry = Some(crate::keyentry::KeyEntry::new("OPENROUTER_API_KEY".into()));
    let (_tx, rx) = std::sync::mpsc::channel();
    p.oauth = Some(rx);
    p.on_input(ChatInput::Close, std::path::Path::new("."));
    assert!(p.keyentry.is_none(), "escape closes the prompt");
    assert!(p.oauth.is_none(), "escape cancels the sign-in");
}
```

- [ ] **Step 5: Run, lint, commit**

Run: `cargo test -p crew-app`, then `cargo clippy --workspace --all-targets -- -D warnings`. Both green.

```bash
git add crates/crew-app/src
git commit -m "feat(chat): open the browser for OpenRouter, paste still available"
```

---

## Verification (after all four tasks)

- [ ] `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` — green.
- [ ] **Manual, and it matters** — the automated tests cannot reach the browser, the real exchange, or `poll.rs`'s wiring. Launch `target/debug/crew` (a dev launch spawns ITSELF as the broker; NEVER overwrite `~/.local/bin/crew`). In a `/smith` pane, `/model`, pick a dimmed OpenRouter row, and confirm: the browser opens to `openrouter.ai/auth`; the prompt says it is waiting; approving stores the key and the row goes live; the pane notes `OPENROUTER_API_KEY saved · openrouter pinned` and never the key itself.
- [ ] Confirm Escape during the wait cancels cleanly and stores nothing.
- [ ] Confirm typing during the wait still pastes, and that the hint disappears when you do.
- [ ] Delete the resulting `credentials.json` afterwards if the key was a throwaway.
