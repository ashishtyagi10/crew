# Provider Auth SP1 — Credential Store + Key Popup

**Status:** approved 2026-07-26
**Scope:** `crew-plugin` (credential store, broker consumption) + `crew-app` (popup, probe overlay)

## 0. Where this sits

The provider-auth goal is three subsystems. It decomposes into three sub-projects
that share one foundation:

| | Sub-project | Delivers |
| --- | --- | --- |
| **SP1** | **Credential store + key popup (this spec)** | Somewhere to put a credential, and a way to type one in |
| SP2 | OpenRouter OAuth PKCE | One-click browser auth; crew receives the user's own key |
| SP3 | Anthropic profile reuse | Detect an existing `ant auth login` profile; no key at all |

SP1 is first because both later parts need a place to store what they obtain.

**The embedded free-tier key was dropped, on evidence.** OpenRouter governs rate
limits per *account*, not per key — "Making additional accounts or API keys will
not affect your rate limits, as we govern capacity globally" — with free models
capped at 20 req/min and 50 req/day under $10 lifetime credits. One key baked
into a public binary therefore gives every crew install in the world a shared 50
requests a day, on top of being extractable with `strings`. SP2's OAuth flow
gives each user their own quota at one click, which is what the "free models just
work" goal actually wanted.

## 1. The problem

Keys reach crew from environment variables only. `broker/discover.rs::pick_provider`
probes `DASHSCOPE_API_KEY` → `OPENROUTER_API_KEY` → `ANTHROPIC_API_KEY`, and
`broker/shellenv.rs::hydrate` back-fills them from a login shell because a
Dock-launched app inherits none. App-side, `modelroute::route_for` computes
`Route::Missing("<VAR>")` per catalog row and `modelpick::model_row` renders it
dim with a "needs `<VAR>`" hint.

So the app already knows exactly which key each unusable row wants — and there is
no way to supply one from inside the app. The user must find a shell rc file,
export a var, and restart. That is the friction this removes.

## 2. What SP1 delivers

Accepting a dimmed model row opens a masked field naming the key it needs. On
submit the key is stored, the row goes live, and the next message runs on it —
with no broker restart and no app restart.

The no-restart property is not a trick: the broker rebuilds its registry on every
request (`session.registry()` → `Registry::discover_with` → `roster_with` →
`pick_provider`), so a credential the broker can see takes effect on the next
message.

## 3. The credential store

New public module `crew-plugin/src/credentials.rs`. It lives in `crew-plugin`
because both consumers can reach it: the broker is `crew-plugin` itself, and
`crew-app` already depends on the crate. Both crates already have `dirs`,
`serde` and `serde_json` — no new dependencies.

```rust
/// The only variables the store will hold. A key typed into the UI can never
/// name an arbitrary environment variable.
pub const VARS: [&str; 3] = [
    "DASHSCOPE_API_KEY",
    "OPENROUTER_API_KEY",
    "ANTHROPIC_API_KEY",
];

#[derive(Default, Serialize, Deserialize)]
pub struct Store {
    /// Provider to force when `CREW_PROVIDER` is unset (§5).
    pub provider: Option<String>,
    pub keys: BTreeMap<String, String>,
}

pub fn path() -> Option<PathBuf>;                    // <config_dir>/crew/credentials.json
pub fn load() -> Store;                              // Default on ANY failure
pub fn save_key(var: &str, value: &str, provider: Option<&str>) -> anyhow::Result<()>;

/// The provider a variable authenticates, as `pick_provider`/`CREW_PROVIDER`
/// spell it: `DASHSCOPE_API_KEY` → `"dashscope"`, `OPENROUTER_API_KEY` →
/// `"openrouter"`, `ANTHROPIC_API_KEY` → `"anthropic"`. `None` for anything
/// outside `VARS`. This is what §5's pin is derived from, so the mapping lives
/// beside `VARS` rather than being restated at the call site.
pub fn provider_for(var: &str) -> Option<&'static str>;
```

**Location.** `dirs::config_dir()/crew/credentials.json`, a sibling of the
existing `config.toml` (`configio.rs:15`). Deliberately **not** inside
`CrewConfig`: that file is user-visible, hand-edited and safe to share, and a
credential in it would leak the first time someone pasted their config.

**Not the macOS Keychain.** crew ships Linux release binaries too, so Keychain
would mean two code paths and a platform-specific failure mode for a v1. A
0600 file is what `gh` and `aws` do. Keychain remains open as a later upgrade.

**Permissions.** The parent directory is created `0700`, the file `0600`, on
`#[cfg(unix)]`. Writes are atomic and never expose a readable window: write to
`credentials.json.tmp` in the same directory, `set_permissions(0600)` on it
**before** any content is written, then `rename` over the target. A crash leaves
either the old file or the temp file, never a truncated one.

**Failure is silent and safe.** `load()` returns `Store::default()` for a missing
file, unreadable file, or malformed JSON — an unparseable credentials file must
never stop crew from starting. `save_key` returns `Err` and the UI reports it.

**Rejections.** `save_key` errors if `var` is not in `VARS`. An empty `value`
removes that key rather than storing a blank (an empty string is exactly the trap
that makes `ANTHROPIC_API_KEY=""` outrank a valid OAuth profile).

## 4. Precedence, and how the broker sees it

**A real environment variable set at launch → the credentials file → the
login-shell hydration.**

An explicitly exported variable is the most deliberate signal and keeps winning.
A key typed into the app beats a stale value in a shell rc file — otherwise the
user types a key, sees nothing change, and has no way to find out why.

Two passes, in two different places, both applying that precedence.

At broker startup, `shellenv::hydrate` already only fills variables *missing*
from the process environment, so:

```
hydrate():
    1. for each VAR in credentials.json: set_var if missing-or-empty in the process env
    2. existing login-shell pass: set_var if still missing
```

Step 1 runs inside `hydrate()`, which is already documented as running before the
broker spawns any thread — `set_var` is process-global and unsound once threads
are live. Nothing new about the threading contract.

**But `hydrate()` alone is not enough, and this was originally missed.** It runs
exactly ONCE per broker process, and the broker is a long-lived child spawned
when the chat pane opens. The key prompt is only reachable *from inside* a chat
pane — so the file is always written after the only import that would have
picked it up. A key saved this session would never reach the running broker at
all; worse, because the *pin* is re-read per request, saving one would resolve a
provider whose key the process could not see, `provider_and_model_for` would
return `None`, and `roster_with` would fall back to plugins only — the user's
working specialist roster vanishing until they reopened the pane.

So `discover.rs` reads the store per request as well, with the same precedence:
`key_for(store, VAR)` returns the process environment's value when it is set
non-empty, else the stored one. That is the same shape `forced_provider()`
already had for the pin, and it is what makes a saved key take effect on the
next message.

**With one exception, for keys crew injected itself.** `hydrate` still exports
stored keys into the environment, because child processes inherit it — a plugin
agent that shells out to a CLI reads `ANTHROPIC_API_KEY` from there and would
otherwise never see an in-app key. But that export is crew's own copy of the
store, not user intent, and `hydrate` runs once per process while the store is
re-read every request. Treating it as user intent broke **rotation**: paste a
replacement key in a later session and the store updated while crew's stale
startup injection kept winning every request — unfixable 401s until the user
quit crew. So `hydrate` records which variables it injected, and for those the
store is authoritative (falling back to the environment only if the store has
since been emptied). A value the *user* exported still wins over everything.

## 5. Provider pinning

`pick_provider`'s order is fixed: DashScope → OpenRouter → Anthropic. So a user
who picks an Anthropic model, is asked for `ANTHROPIC_API_KEY`, and supplies it
would see *nothing change* if a DashScope key were already present — the pick
silently keeps routing to Qwen.

So supplying a key for provider P also pins P: `save_key` records
`Store::provider`, and the broker's forced-provider resolution becomes
`CREW_PROVIDER` (env, unchanged precedence) **else** `Store::provider`. The pin is
stated in the UI when it happens, never silent (§6).

`CREW_PROVIDER` still wins, so an explicit pin from the environment is never
overridden by a stored one.

## 6. App integration

**Only real key names open the popup.** `Route::Missing` does not always hold an
environment variable: `modelroute.rs:76` produces
`Missing("a model OpenRouter serves")`, a human phrase. A new
`Route::needs_key() -> Option<&'static str>` returns `Some` only when the payload
is one of `credentials::VARS`. Everything else keeps flashing its reason.

**Carrying it to the accept path.** `suggest::MenuItem` gains
`needs: Option<String>`, filled by `modelpick::model_row` from `route.needs_key()`.
`chatpalette::popup_key` gains a `PaletteKey::NeedsKey(String)` outcome: when the
accepted row carries `needs`, the palette closes, the input is **not** filled, and
nothing is submitted — the model is not chosen until it can actually run.
`chat.rs`'s existing `match` over `PaletteKey` (`chat.rs:362`) gains that arm.

**The popup.** New `crew-app/src/keyentry.rs`:

```rust
pub(crate) struct KeyEntry { pub var: String, buf: String }
pub(crate) enum KeyOutcome { Consumed, Cancelled, Submit(String) }

impl KeyEntry {
    pub(crate) fn new(var: String) -> Self;
    pub(crate) fn key(&mut self, k: &ChatInput) -> KeyOutcome;
    pub(crate) fn card(&self, cols: u16) -> Vec<CellView>;
}
```

`ChatPane` gains `keyentry: Option<KeyEntry>`, routed **before** the palette and
mention handlers in `on_input`, since it is modal.

Rendering follows the house style — a bordered fieldset card with a legend on the
top border, not a floating title-barred popup. It reuses `boxdraw::titled_card`
exactly as `cmdmenu::menu_card` does, with the legend naming the variable
(`paste ANTHROPIC_API_KEY`) and one interior row. `render.rs` gains a block
mirroring the palette block at `render.rs:196-232`, positioned the same way
(above the composer).

**Masking.** The interior row renders one `•` per character. The buffer is never
drawn in plaintext, never logged, never written to the session log, chat export
or `/dump`. There is no reveal toggle in v1: the row going live is the
confirmation that the key was right.

Keys: printable chars append (paste arrives as chars), Backspace deletes, Enter
submits a non-empty buffer, Esc cancels and discards.

**On submit:** `credentials::save_key(var, &buf, credentials::provider_for(&var))`, then
`shellprobe::note_key(var, value)` so the app re-resolves without waiting on
anything, then a system line in the pane stating what happened, naming the
variable and the pin but **never** the value:

```
ANTHROPIC_API_KEY saved · anthropic pinned
```

On `Err`, the same line reports the failure and the popup stays open.

## 7. Live re-resolution

`shellprobe`'s cache is a `OnceLock<Probed>` and cannot be re-set. Rather than
convert it to a lock, `shellprobe` gains a small overlay:

```rust
static ENTERED: RwLock<BTreeMap<String, String>>;   // seeded from credentials::load()
pub(crate) fn note_key(var: &str, value: &str);
```

`provider_now()` resolves against the union of the probed keys and the overlay.
The overlay is seeded on first use from `credentials::load()`, so keys entered in
an earlier session make their rows live at startup.

**Honest limitation:** the `probed` flag is unchanged. If the login-shell probe
has not landed yet, rows still render `Unknown` even with a key in the overlay —
crew does not claim a route on evidence it hasn't finished gathering. In practice
the probe lands within its 3-second bound at startup. The stored key still
reaches the broker regardless, because the broker's per-request resolution reads
the file itself (§4) — *not* because of the startup `hydrate()` pass, which has
already run by the time any key can be typed.

The overlay carries the stored **pin** as well as the key names
(`shellprobe::note_pin`, seeded from `credentials::load().provider` at startup).
Without it the app would keep resolving its own fixed discovery order while the
broker honoured the pin: the row the user just supplied a key for would stay dim,
accepting it would re-open the same prompt, and the two halves of crew would
disagree permanently. `CREW_PROVIDER` still outranks the stored pin here, exactly
as it does in the broker.

## 8. Testing

**Store (`crew-plugin`).** Round-trip save→load; `save_key` rejects a variable
outside `VARS`; an empty value removes rather than stores; malformed JSON loads as
`Store::default()` instead of erroring; a missing file loads as default; the
written file is mode `0600` and the parent `0700` (unix); the write is atomic —
after `save_key`, no `.tmp` remains.

**Precedence (`crew-plugin`).** The credentials pass fills only variables missing
or empty in the process environment; a variable already set is left untouched.
Test the pure resolution helper rather than mutating process-global env in a
parallel test run.

**Provider pin (`crew-plugin`).** Forced-provider resolution prefers
`CREW_PROVIDER` over `Store::provider`, and uses `Store::provider` when the env is
unset; `None`/unknown values fall through to auto-discovery unchanged.

**Route (`crew-app`).** `needs_key()` returns the variable for
`Missing("ANTHROPIC_API_KEY")` and `None` for `Missing("a model OpenRouter
serves")` — the case that would otherwise open a popup for a nonsense variable.

**Palette (`crew-app`).** A row with `needs` yields `PaletteKey::NeedsKey(var)`,
leaves `input` unchanged, and does not submit. A row without `needs` behaves
exactly as before.

**Popup (`crew-app`).** Chars append; Backspace deletes; Esc yields `Cancelled`;
Enter on a non-empty buffer yields `Submit` with the exact text; Enter on an empty
buffer does not submit. `card()` renders `buf.chars().count()` mask glyphs and —
asserted explicitly — **no cell containing any character of the secret**.

## 9. Non-goals

- OAuth of any kind (SP2/SP3).
- macOS Keychain storage.
- Validating a key against the provider before storing it.
- Editing, listing or removing stored keys from the UI (delete the file).
- Windows support (`0600` is `#[cfg(unix)]`; crew releases darwin + linux).
- Changing `pick_provider`'s discovery order.
