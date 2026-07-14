# Inter-Pane Ask — Design (v1, targeted + local)

**Status:** Approved design, pre-implementation.
**Date:** 2026-07-13.
**North star:** this is v1 of the [Sentinel Network](../../vision/sentinel-network.md) — the local, targeted seed of a global agent-to-agent network. Every v1 decision below is chosen to *extend* into broadcast (v2) and cross-instance federation (v3) without a rewrite.

## Purpose

Let an agent working in one crew pane ask an agent in another pane a question and get its answer back — visibly, in-session, with a wait governed by the target's *liveness* rather than a fixed timeout. If the target never answers, the asker gets a clear negative verdict and reasons about a fallback instead of hanging.

Concretely, agent 1 runs one command:

```
$ crew ask schema "which API version does the client target?"
ANSWERED: v2 — see api/v2/client.rs
```

and either uses the answer or, on `NO_ANSWER …`, finds another route.

## Non-goals (v1)

- **Broadcast / "ask anyone"** (`--any`) — deferred to v2 (it is the highest-value feature; see the vision doc). v1 is single-target.
- **Cross-instance / remote panes** — deferred to v3. v1 resolves addresses locally only.
- **Structured out-of-band queries** that bypass the target's visible session — explicitly rejected: the user *wants* the exchange visible in pane 2's transcript.
- Picking the target *for* the agent — crew provides a decidable roster; the agent chooses.

## Architecture & data flow

```
agent 1 (in pane 1's shell)                 crew GUI process (winit thread)
  │                                            │
  │  $ crew ask schema "…"                     │
  │  (new subcommand, client mode)             │
  │───── connect Unix socket ─────────────────▶│  ipc.rs: accept, parse Envelope
  │      {from, to:"schema", question, id}     │
  │                                            │  askroute.rs: resolve "schema"→pane p2,
  │                                            │    inject wrapped question into p2 (visible)
  │                                            │  askwait.rs: register pending {id, p2, baseline}
  │                                            │
  │        (blocks on socket read)             │  ── poll tick loop (already ~60Hz) ──
  │                                            │  askwait.rs: watch p2 output + liveness,
  │                                            │    until sentinel | idle | stalled | busy
  │◀──── Verdict {ANSWERED text | NO_ANSWER r}─│  ipc.rs: write verdict, close
  │  prints verdict to stdout, exits           │
  ▼                                            ▼
agent 1 reads stdout, continues / falls back
```

### Components (each independently testable)

| File | Responsibility | Depends on |
|---|---|---|
| `crew-app/src/ipc.rs` (new) | Unix-domain socket listener; parse `Envelope`, write `Verdict`. Transport-agnostic message types live here. | std net, serde |
| `crew-app/src/askroute.rs` (new) | Resolve address → pane; inject the wrapped question via the existing `send_to_label` path; register a pending request. | `spawn.rs::send_to_label`, pane roster |
| `crew-app/src/askwait.rs` (new) | The liveness/verdict engine; advanced once per poll tick; owns pending requests + sentinel scanning. | pane activity signals |
| `crew-app/src/panes_roster.rs` (new) | Build the `crew panes` roster from live pane state. | pane list |
| `crew-app/src/main.rs` (edit) | `crew ask …` and `crew panes` **client** subcommands: connect, send, print, exit (before any GUI init, like `--list-fonts`). | ipc types |
| `crew-app/src/poll.rs` (edit) | Call `askwait::tick()` each cycle; accept new socket connections non-blocking. | askwait |
| `crew-app/src/handler.rs` (edit) | Bind the socket on startup; unlink on exit. | ipc |

### Transport & message envelope (forward-compat)

The envelope is defined independently of the socket, so a network relay can carry the identical bytes in v3.

```rust
// ipc.rs
struct AskRequest { v: u32, from: String, to: String, question: String, id: String }
enum Verdict {
    Answered { text: String },
    NoAnswer { reason: NoAnswerReason, partial: Option<String> },
}
enum NoAnswerReason { IdleNoEngage, Stalled, BusyElsewhere, Unreachable }
struct RosterRequest { v: u32 }
struct RosterReply { panes: Vec<PaneCard> }
struct PaneCard { id: String, label: Option<String>, kind: String, running: Option<String>, dir: Option<String>, busy: bool }
```

- **Socket path:** `${XDG_RUNTIME_DIR:-~/.local/state/crew}/crew-ipc.sock`, unlinked on clean exit; stale socket reclaimed on bind. One socket per running GUI (v1 = one instance).
- **Client mode** (`crew ask` / `crew panes`) short-circuits in `main.rs` before GUI init (same pattern as `--list-fonts`, `--broker-plugin`). If the socket is absent/refused → print `NO_ANSWER unreachable` (no crew running) and exit non-zero.

## Addressing & discovery

- **Address** is an opaque string resolved by `askroute::resolve`. v1 accepts a **stable pane id** (`p1`, `p2`, … assigned per pane, always present) or a **`/name` label** (friendly alias). v3 widens the same field to `label@instance` — callers and protocol unchanged.
- **Discovery:** `crew panes` returns the roster (`RosterReply`) — id, label, kind (`terminal`/`swarm`/`far`), the foreground agent (`claude`/`codex`/…), dir, and busy state. Everything already lives in pane state (`label`, `title_text`, `PaneContent`, `procnames`, activity).
- **Decision** is the asking agent's job; crew only makes the roster decidable. `/name`-ing panes by role (`schema`, `payments-api`) makes the choice near-automatic.
- **Capability bootstrap:** agents must *know* these commands exist. Two mechanisms: (a) when crew spawns an agent pane it seeds a one-line hint into that agent's context ("run `crew panes` to list sibling panes, `crew ask <id> \"q\"` to query one"); (b) expose `ask`/`panes` through crew's existing MCP relay so MCP-speaking agents see them as tools. (a) is required for v1; (b) is a fast-follow.

## Delivery — visible, cooperative sentinel

`askroute` injects into the target pane, via the existing `send_to_label` PTY path (swarm target = a broker message, phase 2):

```
⇐ ask from "builder" [q7]: which API version does the client target?
  reply between <CREW-ANS q7> and </CREW-ANS q7>
```

- Lands in the target's **live transcript** — the user sees the exchange happen (an explicit product choice). Rendered with a distinct "⇐ from <pane>" affordance so it reads as an inter-pane ask, not the user's own input.
- The `id` (`q7`) namespaces the sentinel so concurrent asks to the same pane don't collide.
- Instructable agents (LLM CLIs, swarm agents) honor the markers → clean answer boundary. Non-agent panes (plain shell) simply never emit the sentinel → resolved by the liveness engine as `idle_no_engage`.

## Liveness / verdict engine (`askwait`)

Runs on the existing poll tick (already ~60Hz — cheap, no new thread). Per pending request it tracks a baseline output offset and `last_progress_ms`.

**Signals (already exposed by crew):**
- Terminal pane: PTY output progress (`try_read() > 0`, `has_pending()`); foreground-idle detection (`agent_done`, foreground pid → shell) = the target's turn ended.
- Swarm pane: `Activity{state}` (`thinking`/`idle`), `is_busy()`.

**Rule — while the target genuinely emits output, keep waiting.** Resolve when:
- Closing `</CREW-ANS id>` seen → **Answered(text between markers)**.
- Target returns to idle without the sentinel → **IdleNoEngage** (produced nothing) or **Stalled** (produced but never closed; return any partial).
- Target still active but silent for an **adaptive** quiet window (a function of its own observed output cadence — a stream running 2 min earns more patience than one silent from the start), no sentinel → **Stalled(partial)**.
- Target busy on its own work when the ask arrives → wait a short grace for idle, then inject; if still busy past the grace → **BusyElsewhere**.
- Absolute safety ceiling (generous, e.g. a few minutes) → **Stalled**; liveness is expected to resolve long before this.

**Concurrency:** multiple pending asks (to different or same panes) coexist, keyed by `id`; each scans the target's output tail for *its* sentinel.

## Edge cases

- **No crew running** → client prints `NO_ANSWER unreachable`, exits non-zero.
- **Unknown address** → `Unreachable`.
- **Target closes mid-wait** → `Stalled(partial)`.
- **Asker aborts** (Ctrl-C / socket drop) → app drops the pending request; the already-injected question stays in the target's transcript (harmless, visible).
- **Ask to self** (`from == to`) → rejected client-side with a usage error.
- **Malformed/oversized question** → bounded length; reject over cap.
- **Sentinel appears in the question echo** → scan only output *after* the injected question offset; require the *closing* tag.

## Testing

- `ipc.rs`: envelope round-trips (serde); stale-socket reclaim.
- `askroute.rs`: resolve id and label; unknown → `Unreachable`; self-ask rejected; wrapped-question formatting includes the id.
- `askwait.rs` (the core, pure over injected signals): given a scripted sequence of (output chunk, active?) ticks → asserts each verdict — sentinel→Answered; idle-no-output→IdleNoEngage; output-then-idle-no-close→Stalled(partial); active-but-silent→Stalled; busy-throughout→BusyElsewhere; and the adaptive window grows with cadence.
- `panes_roster.rs`: roster reflects labels/kind/running/busy for a mixed pane set.
- Integration (harness): two isolated terminal panes; `crew ask` from one to the other returns `Answered` for a cooperating stub and `IdleNoEngage` for a plain shell.

## Phased build

1. **v1a — plumbing:** socket + `crew panes` roster + `crew ask` client + address resolution. Ships discovery immediately.
2. **v1b — terminal ask:** inject + sentinel capture + liveness engine for terminal panes. The full targeted round-trip.
3. **v1c — swarm target + bootstrap:** deliver to crew/swarm panes via the broker; seed the capability hint on agent-pane spawn; MCP relay exposure.

Then v2 (broadcast) and v3 (federation) per the vision doc — additive over this engine.
