# /smith `/model` picker — design

**Date:** 2026-07-25
**Goal:** Typing `/model` in the agent smith composer opens an opencode-style
picker: models grouped by provider, real display names, free/paid and per-Mtok
cost badges, and a mark on rows the current provider stack can actually serve.

## Today

- **Chat pane composer** (`chatpalette`): the leading-`/` palette lists
  `chatcomplete::CONSTRUCTS`, which includes `/model`. It closes the moment a
  space is typed (`pending_palette` returns `None` once the input contains
  whitespace), so `/model ` offers **nothing** — the user must know a slug.
- **App input bar** (`suggest::menu_items` → `options_for("/model")`): a flat,
  hardcoded list of 7 slugs with one-line hints. No grouping, no prices, no
  free/paid signal. A pick routes through `chatspawn::set_model_cmd`, which
  forwards `/model all <slug>` to the open smith pane.
- **Broker** (`broker/commands.rs::model_cmd`): `/model <agent> <slug>` pins one
  agent, `/model all <slug>` pins the whole roster, `default` clears the pin.
  Freeform slugs always work. **`/model <slug>` with no agent is a usage error** —
  the composer fill must therefore be `/model all <slug>`, not `/model <slug>`.
- **Provider stack** (`broker/discover.rs::pick_provider`): exactly ONE provider
  backs every API agent — mock, then `CREW_PROVIDER` (dashscope|openrouter|
  anthropic), then key auto-discovery in that order. So a model is serveable only
  if the active provider can route it.
- **Pricing** (`crew_hive::pricing::RATES`): µ$/Mtok by longest-substring match,
  unknown → 0 (the footer hides `$` rather than invent a number).

## Decision

One catalog, two surfaces, three columns.

### 1. Catalog (`crew-hive`)

New `catalog.rs` (+ `catalog/data.rs` for the table, to respect the 200-line
cap) exposing:

```rust
pub struct ModelInfo {
    pub name: &'static str,        // "Claude Sonnet 5"
    pub slug: &'static str,        // native slug, e.g. "claude-sonnet-5"
    pub or_slug: Option<&'static str>, // OpenRouter alias, e.g. "anthropic/claude-sonnet-5"
    pub vendor: Vendor,            // grouping key
    pub price: Option<(u64, u64)>, // µ$/Mtok in/out; None = unknown
    pub free: bool,
    pub context: u32,              // context window in tokens, 0 = unknown
}
pub enum Vendor { Anthropic, OpenAI, Google, Alibaba, Moonshot, DeepSeek,
                  Mistral, Meta, XAI, HuggingFace, OpenRouter, Other }
pub fn catalog() -> &'static [ModelInfo];
```

Curated, ~40 rows. Prices come from `pricing::RATES` where a pattern already
exists and from first-party rate cards where verified; **anything unverified is
`None`, never a guess** — the badge renders `—`. Two `RATES` corrections land
with this work: `claude-opus` 15/75 → **5/25** and `claude-fable` 15/75 →
**10/50** (both stale as of 2026-07). Free rows are OpenRouter `:free` variants
and HF-served endpoints, `free: true`, `price: Some((0, 0))`.

### 2. Serviceability (`crew-plugin` + `crew-app`)

`pick_provider` becomes reachable from the app: `crew_plugin::active_provider(
has_key: impl Fn(&str) -> bool) -> Option<Provider>` (pub re-export of the
existing enum + fn; broker behaviour unchanged).

The app can't just read its own env — the broker hydrates missing keys from the
login shell (`broker/shellenv.rs`), so a Finder-launched app may not see a key
the broker will. `modelkeys.rs` mirrors `cmdcheck::init_shell_path`: a
background thread runs `$SHELL -ilc env` once at startup and stores which
provider keys are non-empty in a `OnceLock`. **Never on the winit thread.**
Until it lands, serviceability is `Unknown` and no row is dimmed — we never
claim "no key" on evidence we don't have.

```rust
enum Route { Direct(&'static str), ViaOpenRouter, Mock, Unknown, Missing(&'static str) }
```

- active Anthropic + `claude-*` → `Direct("anthropic")`
- active DashScope + `qwen*` → `Direct("dashscope")`
- active OpenRouter + row has an `or_slug` (or is already `vendor/model`) →
  `ViaOpenRouter`
- active Mock → `Mock` (everything serveable)
- otherwise → `Missing("<KEY>_API_KEY")`, row dim, reason in the desc column

The **fill slug follows the route**: `ViaOpenRouter` sends `or_slug`, everything
else sends `slug`. Picking a `Missing` row still sends (freeform always worked)
but flashes the reason first.

### 3. Rows and grouping (`crew-app::modelpick`)

`MenuItem` gains `header: bool`. Header rows carry the vendor name, render dim +
bold with no `›` marker, and are **not selectable** — `suggest::step_sel` /
`first_selectable` skip them, and both popup key handlers plus the input bar's
`menu_sel` navigation route through those helpers.

Row shape (label / desc):

```
  Claude Sonnet 5      claude-sonnet-5 · $3/$15 · 1M ● current
  Qwen Max             qwen-max · $1.6/$6.4 · dashscope
  Llama 3.3 70B        meta-llama/…:free · free · via openrouter
  GPT-4.1 Mini         gpt-4.1-mini · $0.4/$1.6 · needs OPENAI_API_KEY
```

Order: `default` row, then **Recent** (if any), then vendor sections in a fixed
order with flagship → mini → free inside each. Filtering reuses
`chatmention::filter`'s prefix > substring > subsequence ranking over a haystack
of `name + slug + vendor + badges`, so `free`, `anthropic`, and `sonnet` all
narrow usefully; a section header is dropped when it has no surviving rows.

The current model is marked `●` and the cursor opens on it (from the pane's
`AgentInfo.model`; when agents disagree, no mark).

### 4. Composer + input bar wiring

`chatpalette::Kind` gains `Model`. `pending_palette` recognises the arg phase:
input starts with `/model` + a space, and the remainder holds **at most one
whitespace-free token** → `Some((Kind::Model, arg))`. `/model all qwen` (two
tokens) returns `None`, so the per-agent and explicit-`all` freeform forms keep
working untouched.

Rows are `submit: true`. `popup_key` gains `PaletteKey::Submit`: it sets
`*input = "/model all <slug>"`, closes the palette, and `chat::on_input` then
re-enters its own `ChatInput::Enter` path (palette is `None`, so no recursion).
Card legend: `"models"` (`render::palette_card_title`).

The input bar keeps its existing route — `options_for("/model")` returns the
same catalog rows, so `chatspawn::set_model_cmd` is unchanged (it already
prefixes `all`).

### 5. Live enrichment (OpenRouter `/models`)

`crew_hive::catalog::fetch_openrouter(key) -> Result<Vec<ModelInfo>>` (async,
reuses the crate's `reqwest`) parses `data[]` → `id`, `name`, `pricing.prompt`,
`pricing.completion` (USD per token, string) → µ$/Mtok, `context_length`.

App side, `modelfetch.rs` follows `swarm/plan.rs`: a worker thread owning a
current-thread tokio runtime, result over an `mpsc` channel drained in the frame
poll. Fired **lazily on first picker open**, once per process. Results merge into
a process-global `Mutex<Vec<ModelInfo>>` that both surfaces read; they enrich
price/context and add free rows, never removing curated ones. Cached to
`models-openrouter.json` beside the config with a fetched-at stamp, TTL 24h, and
the whole path degrades silently to the static catalog when offline or keyless.

### Non-goals

Per-agent model matrix UI (the freeform `/model <agent> <slug>` covers it); API
key entry/management; favourites; sort chips; benchmark or quality rankings;
auto-switching by task; cost estimation beyond the badge.

## Error handling

Missing/unparseable cache → static catalog. Fetch failure, timeout, or absent
`OPENROUTER_API_KEY` → static catalog, no user-visible error (the picker is not
a network feature). Key probe failure → `Unknown` route, nothing dimmed. Empty
filter result → palette closes, as every other picker does.

## Testing

Pure-logic tests beside each module, matching existing patterns:

1. `crew-hive`: catalog invariants (slugs unique, every vendor represented, no
   row both `free` and priced non-zero); `pricing` regression pinning the
   corrected Anthropic rates; `fetch_openrouter`'s parser against a captured
   JSON fixture (string prices → µ$/Mtok, `"0"` → free).
2. `crew-plugin`: `active_provider` honours mock > `CREW_PROVIDER` > key order
   (the existing `pick_provider` tests, re-pointed at the public fn).
3. `crew-app::modelroute`: each route arm, including `Unknown` before the probe
   lands and `Missing` naming the right key; fill-slug follows the route.
4. `crew-app::modelpick`: sections ordered and headers emitted only for
   non-empty vendors; `free` / vendor-name / slug queries all narrow; `●` marks
   the current model and only when the roster agrees; `default` row is first.
5. `crew-app::suggest`: `step_sel` / `first_selectable` skip headers in both
   directions and at both ends.
6. `crew-app::chatpalette`: `/model ` opens `Kind::Model`; `/model qwen` filters;
   `/model all qwen` (two tokens) does NOT open; accept fills `/model all <slug>`
   and returns `Submit`.
7. `crew-app::render`: model card legend is `"models"`; header rows render dim
   and carry no selection marker.
