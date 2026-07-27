# Provider Auth SP2 — OpenRouter OAuth (PKCE)

**Status:** approved 2026-07-26
**Scope:** `crew-hive` (PKCE + token exchange) + `crew-app` (callback listener, flow driver, prompt state)

## 0. Where this sits

SP1 shipped in v0.6.35: a masked prompt writes a provider key to an owner-only
store, and the broker resolves keys from it per request. SP2 removes the typing.

| | Sub-project | State |
| --- | --- | --- |
| SP1 | Credential store + key popup | shipped v0.6.35 (+ rotation fix v0.6.36) |
| **SP2** | **OpenRouter OAuth PKCE (this spec)** | this change |
| SP3 | Anthropic `ant auth login` profile reuse | queued |

OpenRouter is the one provider where crew can be a real OAuth client. Verified
against OpenRouter's own docs: the flow needs **no app registration and no
`client_id`**, and localhost callbacks are supported on any port. Anthropic
offers no third-party OAuth (SP3 reuses an existing CLI profile instead) and
DashScope none, so this flow is OpenRouter-only by nature, not by choice.

## 1. The flow

1. The user accepts a dimmed OpenRouter row (`Route::needs_key() ==
   "OPENROUTER_API_KEY"`).
2. crew generates a PKCE verifier and its S256 challenge, binds a single-shot
   loopback listener, and opens the browser at
   `https://openrouter.ai/auth?callback_url=<cb>&code_challenge=<challenge>&code_challenge_method=S256`.
3. The paste prompt opens underneath in a **waiting** state, so a user who
   would rather paste — or whose browser never opened — is never stuck.
4. The user approves in the browser; OpenRouter redirects to the callback with
   `?code=…`.
5. crew POSTs `{code, code_verifier, code_challenge_method: "S256"}` to
   `https://openrouter.ai/api/v1/auth/keys` and receives `{"key": "…"}` — a key
   belonging to that user, on their own quota, revocable by them.
6. The key goes through SP1's existing save path: stored, provider pinned,
   probe overlay updated, a note naming the variable. The row goes live.

## 2. Why this replaces the embedded key

The original goal asked for a key compiled into the binary for free models.
That was dropped on evidence (SP1 §0): OpenRouter governs rate limits per
**account**, not per key, so one baked-in key gives every install a shared 50
requests/day — and it is extractable from a public release with `strings`. This
flow gives each user their own quota in one click and ships no secret at all.

## 3. Architecture

The split follows what each crate already has, so no dependency moves:
`crew-hive` owns the HTTP (it has `reqwest`), `crew-app` owns the browser and
the listener (it has `open`).

### 3.1 `crew-hive` — PKCE and exchange

```rust
/// A PKCE pair. `verifier` NEVER leaves this process; only `challenge` is
/// put in a URL.
pub struct Pkce { pub verifier: String, pub challenge: String }

/// 64 URL-safe random characters from the OS CSPRNG, and their S256
/// challenge: base64url-nopad(sha256(verifier)).
pub fn pkce() -> Pkce;

/// `chars` URL-safe random characters from the OS CSPRNG — shared by the PKCE
/// verifier and by crew-app's callback nonce.
pub fn random_token(chars: usize) -> String;

/// Exchange an authorization code for the user's own OpenRouter key.
/// Never logs `code`, `verifier` or the returned key.
pub async fn exchange_openrouter_code(code: &str, verifier: &str)
    -> anyhow::Result<String>;
```

**New dependencies:** `base64` and `getrandom`, added to `crew-hive` only. Both
are already in `Cargo.lock` transitively, so this costs no new compilation.
`sha2` is already a workspace dependency. `getrandom` rather than `rand`: the
only need is raw OS entropy, and base64url-encoding those bytes gives a uniform
URL-safe token with no modulo bias to reason about.

### 3.2 `crew-app/src/oauth.rs` — the flow driver

Modelled on `modelfetch::spawn` (`modelfetch.rs:30-45`), which is the
established shape for off-thread work in this app:

```rust
pub(crate) enum OauthOutcome { Key(String), Failed(String) }

/// Start the browser flow. Returns immediately with a receiver the app polls;
/// `None` if the loopback listener could not bind.
pub(crate) fn spawn() -> Option<Receiver<OauthOutcome>>;
```

**Everything blocking happens on a spawned thread.** All work in this app runs
synchronously on the winit thread, so a blocking `accept()` there would freeze
every pane. The thread binds, opens the browser, blocks on one connection,
exchanges the code in a current-thread tokio runtime (as `modelfetch` does),
and sends one `OauthOutcome`.

`poll.rs` polls it with `try_recv()` beside the existing `model_fetch`
receiver — non-blocking, one branch, no new machinery.

### 3.3 The callback listener

- Binds `127.0.0.1:0` — loopback only, never `0.0.0.0`, and an ephemeral port
  so it cannot collide with anything the user is running.
- The callback path carries **32 random URL-safe characters**:
  `http://127.0.0.1:<port>/<nonce>`. Any other local process can reach the
  port, so a request whose path does not match the nonce is answered politely
  and ignored — it does NOT end the wait. This is the CSRF/`state` equivalent
  for a loopback callback.
- **Single-shot.** One matching request is served, then the listener drops.
- **Three-minute timeout** via `set_nonblocking` + a poll loop, so an abandoned
  flow never leaks a thread or holds a port. On timeout the outcome is
  `Failed("timed out")`.
- The reply is one line of `text/plain` telling the user to return to crew,
  with `Connection: close`. It contains no key and no code, and needs no HTML.
- Only `code` is read from the query. An `error=` response yields
  `Failed(<error>)`.

### 3.4 Prompt integration

`KeyEntry` gains a `waiting: bool`. When the flow starts, the prompt for
`OPENROUTER_API_KEY` opens with `waiting = true`; its interior reads
`waiting for browser · or paste the key`. Typing clears `waiting` and the
prompt behaves exactly as SP1 shipped it — the fallback is always live.

The card still masks everything typed. Nothing about the OAuth path renders a
key, and the "waiting" line is not a secret.

**Cancellation.** Escape closes the prompt and drops the receiver. The worker
thread's `send` then fails and it exits; if it is mid-exchange it finishes and
discards the result. A user who Escapes has, by construction, no key stored.

### 3.5 Success and failure

On `Key(k)`, the app calls SP1's existing `chatkeystore::store_provider_key`
path — store, pin, `note_key`, `note_pin`, and a note naming the variable. One
save path for both entry methods, so pinning and re-resolution cannot diverge.

On `Failed(reason)`, the pane notes `openrouter sign-in failed: <reason>` and
the prompt stays open in paste mode. The reason is a flow error (timeout,
network, non-2xx) — never a credential.

## 4. Security properties

- **No secret ships in the binary.** No `client_id` exists to embed.
- **The verifier never leaves the process**; only its SHA-256 challenge is put
  in a URL, which is the entire point of PKCE.
- **The listener is loopback-only, nonce-guarded, single-shot and bounded.**
- **The key is never logged, printed, echoed, rendered, exported, or written
  anywhere but the SP1 store** (0600 file in a 0700 directory).
- **Nothing is auto-approved.** The user explicitly approves in their browser,
  and the key that comes back is theirs to revoke.

## 5. Testing

Pure logic first, since the network and the browser are not unit-testable:

**crew-hive.** `pkce()` produces a 64-character verifier, a challenge that is
base64url-nopad with no padding or `+`/`/`, and a challenge that equals a known
SHA-256 of a fixed verifier (test the inner function against an RFC 7636 vector
so the encoding is pinned, not merely self-consistent). Two calls differ.

**crew-app.** The callback parser: a request for the nonce path with `?code=x`
yields `x`; a wrong path yields nothing; `?error=access_denied` yields that
error; a request with no query yields nothing. The auth-URL builder produces
exactly the documented parameter set and percent-encodes the callback.

**Not unit-tested, stated plainly:** the browser opening, the real exchange, and
`poll.rs`'s wiring. The listener/timeout behaviour is tested by driving a real
loopback listener from the test itself, which is hermetic and fast.

## 6. Non-goals

- OAuth for any other provider (Anthropic has none; SP3 uses a CLI profile).
- Refresh tokens or expiry handling — OpenRouter returns a durable user key.
- Storing anything beyond that key in the existing store.
- Revoking or listing keys from inside crew.
- A browser flow for the input-bar model picker (SP1's `needs` is already
  ignored there; unchanged here).
