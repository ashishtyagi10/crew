# /smith Claude-style statusline footer

**Date:** 2026-07-22
**Status:** Approved

## Goal

Replace the `/smith` pane's summary footer (labelled `model/git/ctx/usage/agents`
block in `chatsummary.rs`) with a Claude-Code-style 3-line statusline:

```
qwen3-coder | main | $0.129 | 41.6K in / 314 out
5h:3h52m | 7d:3d23h | ░░░░░░░░ 3% (5h) | ░░░░░░░░ 4% (ctx)
▶▶ swarm mode · / for constructs · @ to relay to an agent
```

## Lines

### Line 1 — identity & spend
`model | branch | $cost | {in} in / {out} out`

- **model**: existing `short_model` logic; `mixed (N)` for mixed rosters; omitted when roster empty.
- **branch**: existing `git_branch` watch (never run git in the render path).
- **cost**: session dollar spend, `$0.129` style (3 decimals < $10, 2 above).
  Shown only when at least one priced reply landed — unpriced models never
  show a fake `$0.000`.
- **tokens**: session in/out split via `fmt_tokens`, always shown (fresh pane
  shows `0 in / 0 out`) — this keeps the always-on-footer guarantee.

### Line 2 — rolling windows & bars
`5h:{left} | 7d:{left} | {bar} {n}% (5h) | {bar} {n}% (ctx)`

- **Windows**: crew tracks its own usage. A window opens at the first request
  after the previous window expired (Claude session semantics): a 5-hour block
  and a 7-day block. `{left}` = `XhYYm` / `XdYYh` until that block ends. No
  open window (no usage inside one) → `5h:--`.
- **5h bar**: % of the 5h token budget spent in the current block.
- **ctx bar**: tightest-agent context fill, reusing the existing calc.
- Bars are dithered glyph runs (`░` empty / `▓` filled), ~8 cells wide,
  degrading gracefully on narrow panes (drop bars before countdowns).

### Line 3 — routing mode & hints
- `▶▶ swarm mode` when the composer would route to the swarm engine (default).
- `▶▶ @{name} relay` live, when the composer text starts with a valid
  `@mention` of a rostered agent.
- Followed by `· / for constructs · @ to relay to an agent`. The composer
  placeholder drops the hints it currently carries (no duplication).

### Height degradation
`summary_rows` grants 3 → 2 → 1 lines as the pane shrinks (same
`MIN_ROWS`-safe budget rule as today, `MAX_BLOCK` becomes 3). Priority:
line 1, then line 2, then line 3. The 1-line fallback IS line 1 (replaces
the old dense `summary_text`).

### Color
Per-segment theme accents (model=cyan, branch=yellow, cost=green,
tokens=magenta, windows=blue, bars=muted fill, mode=yellow) sourced from the
theme palette and passed through the existing contrast floor. Separators `|`
stay muted. Light/dark both derive from `crew_theme::theme()` — no hardcoded
RGB in crew-app.

## Plumbing

### Protocol: in/out/cost on `Stats`
`PluginEvent::Stats` gains optional fields (serde defaults, back-compat both
directions):

- `tok_in: u64` — reply prompt tokens
- `tok_out: u64` — reply completion tokens
- `cost_microusd: u64` — broker-computed cost in micro-USD (0 = unknown)

Existing `tokens`/`ctx` stay untouched so an old GUI ↔ new broker (and
reverse) keeps working.

### Broker computes cost
Pricing lives broker-side (crew-hive, near the providers, which know the
model):

- Small built-in `$/Mtok` table (input & output rates) for common models:
  Qwen/DashScope tiers, Claude, GPT, DeepSeek, etc. Unknown model → cost 0.
- OpenRouter: prefer the exact per-request cost the API reports when present.
- Mock provider reports zeros.

### GUI usage ledger
- Append-only `usage.jsonl` beside `config.toml` (`dirs::config_dir()/crew/`,
  matching `config.rs`): `{ts, in, out, cost_microusd}` per Stats
  event, written by the GUI (single process = natural aggregator across
  panes; brokers are per-pane children and would race on the file).
- Loaded at startup, entries older than 7d pruned (rewrite on load).
- Window math (`block start`, `% of budget`, `time left`) in a pure module
  with an injected clock.
- Budgets in config with defaults: `usage_budget_5h = 5_000_000` tokens,
  `usage_budget_7d = 25_000_000` tokens.

### Pane state
`ChatPane` accumulates `tok_in`, `tok_out`, `cost_microusd` alongside the
existing `tokens`. Ledger writes happen where Stats events are applied.

## Not in scope
- No user-cyclable mode (shift+tab) — line 3 reflects routing state, it does
  not add a mode machine.
- No server-side quota integration.
- Other pane types keep their own footers.

## Testing
- Pure line builders: formatting, segment omission (no cost, empty roster,
  fresh pane), width degradation, mixed models.
- Window math: block open/expiry/rollover with injected clocks.
- Protocol: round-trip + old-payload (missing fields) decode tests.
- Ledger: append/prune with a temp dir.
- Visual: `/verify` GUI harness screenshot at the end.
