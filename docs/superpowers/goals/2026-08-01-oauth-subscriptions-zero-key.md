# Goal — sign in, don't paste keys: subscriptions become first-class model providers

**Status: COMPLETE** — shipped v0.12.0 with the field fix in v0.12.1: `/login` and `/logout`, a
signed-out device-flow provider as a numbered row in `/model`, and a grant that outranks a key.

**Set:** 2026-08-01 by the user.

Today crew's model access is KEY-SHAPED: the broker discovers `DASHSCOPE_API_KEY` /
`OPENROUTER_API_KEY` in the environment (`backend()` in `broker/swarm.rs`, hydrated from `$SHELL`
via `CREW_SHELL_ENV`), `CREW_PROVIDER` pins one, and if no key exists the pane falls back to
relaying through whatever agent CLIs are installed. But the people crew is for ALREADY PAY for
models — a Claude Pro/Max seat, a ChatGPT Plus seat that Codex signs into, a Copilot seat, the
Gemini and Qwen free tiers that want only a Google/Qwen account. Every one of those is an OAuth
sign-in, not a key. Asking a subscriber to go mint a pay-per-token API key — new billing, new
secret to manage — just to automate what their subscription already covers is the single biggest
onboarding wall this product has. "People don't want to get a key to automate."

THE GOAL: a fresh machine with ONE existing subscription reaches a working smith pane WITHOUT ANY
API KEY — crew walks the user through a sign-in, remembers it, refreshes it, and `/model` simply
shows what their accounts can serve.

THE HONEST CONSTRAINT THAT SHAPES THE DESIGN: most subscription OAuth is contractually tied to
the vendor's own client — Anthropic and OpenAI tokens are licensed to Claude Code and Codex, not
to arbitrary third-party apps. Crew therefore has TWO RUNGS, and picking the right one per
provider IS the feature. Rung one, CLI-DELEGATED: where a vendor CLI owns the login (Claude Code,
Codex), crew drives THAT CLI as the execution engine — `crew` triggers the CLI's own login flow,
detects its signed-in state, and routes smith work through it, exactly the pattern `/far` uses by
letting rclone own Google Drive's OAuth. The subscription is used INSIDE the client it is licensed
to; crew orchestrates, never impersonates. Rung two, NATIVE DEVICE FLOW: where the provider openly
permits third-party OAuth (device-code flow — Qwen does, Gemini's CLI tier does, others will),
crew runs the flow itself: show a code, open a URL, poll, store. NEVER rung zero: crew does not
scrape another app's token store or replay tokens against provider terms — consent-based
integration only, and a provider with no permitted path simply stays key-only and says so.

PROVIDERS ARE DATA, NOT FORKS: one auth registry where each entry declares its modes
(`cli-delegated` / `oauth-device` / `api-key`), its models, and how to probe signed-in state.
Adding a provider is a registry entry plus at most a thin adapter — the broker, planner, and
intent router never learn provider names. Discovery order stays: explicit pin (`CREW_PROVIDER`),
then signed-in subscriptions, then keys, then installed CLIs — and the EXISTING keyless relay and
mock provider paths are the fallback rungs, untouched.

THE UX BAR, because configuration is half the ask: `/model` becomes the whole story — it lists
concrete, usable models grouped by where they come from ("your subscriptions", "your keys",
"installed CLIs"), signed-out providers appear grayed with a one-keystroke "sign in" that starts
the right flow IN THE PANE (device codes render as a card; browser opens; pane updates when the
poll succeeds), and picking a model is picking a number. No config file edited by hand, no env
var required on the happy path.

### Done means
1. ZERO-KEY ONBOARDING, tested end-to-end: a clean `HOME` with one signed-in provider (stub or
   real) and no `*_API_KEY` anywhere reaches a smith pane that answers, plans, and fans out.
2. The auth registry exists; the grep-level check is that no provider name appears in broker
   routing/planning code outside the registry and its adapters.
3. CLI-delegated rung works for at least Claude Code and Codex: crew detects signed-in state,
   offers the CLI's own login when signed out, and smith work routes through the CLI with the
   full construct surface (shapes, judges, swarm) intact.
4. Native device flow works for at least one provider that permits it, fully in-pane: code card,
   poll, stored token, refresh — proven by a stubbed OAuth server in tests (no network, no key).
5. Token lifecycle is invisible until it can't be: automatic refresh; on hard expiry ONE re-auth
   prompt; `/doctor` shows per-provider auth state (signed in / expired / key / none) and
   never prints a secret.
6. Secrets live in the OS keychain, or a 0600 file only where no keychain exists; nothing
   token-shaped ever lands in session logs, crash logs, or the repo — asserted by a test that
   greps every log sink after an auth round-trip.
7. `/model` shows the grouped picker with sign-in affordance; the chosen model drives ALL model
   calls (classify, planner, workers, judges, summarizer) and persists across restarts.

NON-NEGOTIABLE: consent-based only — every sign-in is user-initiated in the moment, no token is
read from another app's private store, and providers whose terms forbid third-party use are
represented truthfully (key-only) rather than worked around. The existing keyless CLI relay and
`CREW_BROKER_MOCK_REPLY`/mock provider paths keep working unchanged — tests and CI never need a
subscription. Auth flows run OFF the winit thread (the blocking rule) — a polling device flow
must never freeze a pane. And this composes with the autonomy goal, not against it: no new
slash commands beyond what `/model` already is; sign-in is a flow inside `/model` and `/doctor`,
not new vocabulary to learn.
