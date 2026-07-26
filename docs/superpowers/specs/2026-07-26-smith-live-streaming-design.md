# /smith live reply streaming — design

**Date:** 2026-07-26
**Goal:** Replies appear in the agent smith pane as they are generated, not all
at once when the hop ends. Every agent call path streams — plain `/smith`
messages (swarm), `@agent` relays, `/fan`, and tool-call follow-ups.

## Today

The pane goes quiet, then the whole reply lands at once. The pieces to fix it
are already in place; only the last mile is missing.

- **Provider** (`crew-hive/src/provider/openai_http.rs`): real SSE streaming
  exists and is well tested — `request_with_retry_streaming` (:111) and
  `consume_sse` (:216) forward each text delta to an `on_chunk: ChunkFn`
  (`provider/mod.rs:94`), with byte-safe line splitting across chunk
  boundaries and a `[DONE]`/usage-frame tail.
- **`ApiAgent`** (`crew-hive/src/apiagent/mod.rs:98`) — the agent behind swarm
  tasks — calls the **non-streaming** `provider.complete()` and publishes
  **one** `HiveEvent::OutputChunk` holding the entire reply (:105). This is the
  big bang.
- **Broker, swarm path**: `swarmmsg::translate` (:54) maps that single
  `OutputChunk` to one whole `PluginEvent::Message`.
- **Broker, relay/fan/toolcall paths**: these already stream.
  `apiadapter::call_with_usage_ticked` (:155) hands the provider an `on_chunk`
  that counts characters and **discards the text** (:169-175), keeping only a
  chars/4 token estimate. `engine.rs:132`, `engine.rs:162`, `fan.rs:62` and
  `toolcall.rs:191` all call it.
- **Ticks** already flow end-to-end: `tick::hop_ticker` (:21) rate-gates a
  running token estimate into `PluginEvent::StatsTick` at one per 150 ms, rides
  the dedicated `tick_emit` side-channel that writes straight to `Out`
  (`stdio.rs:254-258`), and the app **currently throws it away**
  (`chat.rs:174`). The wire, the pacing and the threading are proven; only text
  is missing.
- **App**: the live area is one status row (`chatswarmview`) — spinner, running
  task, elapsed. No reply text until the `Message` arrives (`chat.rs:175-193`).

Two facts that shape the design:

1. `host.rs:35` parses with `if let Ok(ev) = serde_json::from_str::<PluginEvent>`
   — **unknown event types are silently skipped**. A new event is therefore safe
   against version skew, which matters here because the live app spawns the
   *installed* broker, not the dev build.
2. `chatmsgs::card_lines` (:158) already re-renders the whole transcript through
   the markdown engine every frame (`body_lines`, :192). A bubble that grows
   costs nothing architecturally — no render cache is needed, and adding one is
   a separate concern.

## Decision

### 0. The invariant

**Deltas are advisory; the end-of-hop `Message` is authoritative.** This is the
doctrine `StatsTick` already documents (`protocol.rs:71-77`), and it is what
makes the rest safe: a coalescer may drop a trailing fragment, a `Lagged`
broadcast may skip frames (`bus/mod.rs:19`, `swarm.rs:234-237`), a delta may
arrive for an agent whose reply never lands — none of it can corrupt the
transcript, because the settled `Message` replaces streamed text wholesale.

Every rate-limit and buffering decision below leans on this. Nothing streamed
is ever load-bearing.

### 1. Wire protocol — one additive event

`PluginEvent` gains:

```rust
/// Mid-reply text: `agent` produced `text` since the previous Delta of this
/// hop. Advisory — the end-of-hop `Message` carries the full normalized reply
/// and replaces anything streamed here (same doctrine as `StatsTick`).
Delta { agent: String, text: String },
```

No existing event changes shape. New broker + old app degrades to today's
behaviour silently, per `host.rs:35`.

### 2. `crew-hive` — make `ApiAgent` stream

`ApiAgent::run` switches from `provider.complete(req)` to
`provider.complete_streaming(req, on_chunk)`. The `on_chunk` closure publishes
fragments on the bus.

`HiveEvent` gains a variant rather than overloading the existing one:

```rust
OutputDelta { agent: AgentId, text: String },  // NEW: one streamed fragment
OutputChunk { agent: AgentId, text: String },  // unchanged: COMPLETE output
```

`OutputChunk`'s name says "chunk" but its meaning today is "this agent's
finished output" — it is what becomes the transcript bubble. Adding
`OutputDelta` preserves that meaning, so `chatswarm.rs:121-123`,
`telemetry/mod.rs:81` and `remoteagent/mod.rs:47-52` need no behavioural
change, only an ignore arm. The existing final `OutputChunk` publish stays
exactly where it is.

Fragments are published raw here; pacing is the broker's job (§3), because the
broker is where the process boundary and the app's frame budget are.

### 3. `crew-plugin` broker — one gate, four call paths

**`tick.rs` gains a `TextGate`** and, wrapping it, `hop_texter` — the text
sibling of `hop_ticker`: accumulate incoming fragments in a buffer and flush
the buffer as one `PluginEvent::Delta` at most once per `TEXT_GAP_MS` (80 ms).
Same shape as `hop_ticker` — clock as a parameter, per-hop state, pure gate
helper — so it stays unit-testable.

`TextGate` is exposed separately from the `hop_texter` closure because the two
call shapes need it differently: a relay/fan/toolcall hop is a single call and
wants a self-contained closure, while the swarm drain multiplexes many agents
over one loop and needs to own a gate per agent (below). Both share one
implementation and one set of tests.

**The one way it must differ from `hop_ticker`:** a token estimate is
monotonic, so `hop_ticker` can simply *skip* a tick and the next one carries
the truth. Text is cumulative, so the gate must **buffer and concatenate,
never skip**. Dropping a fragment mid-hop would corrupt the visible text until
the settled `Message` healed it.

No end-of-hop flush. The final fragment may be dropped by the gate, and §0 says
that is fine — the settled `Message` supersedes it milliseconds later. This is
deliberate: it keeps the gate a pure function of its inputs with no lifecycle.

**Swarm path:** `swarmmsg::translate` maps `HiveEvent::OutputDelta` →
`PluginEvent::Delta`, naming the agent with the existing `agent_name` helper so
delta senders match the roster exactly as `StatsTick`'s already do (:49-52).

Pacing here needs its own home, because `translate` is called once per bus
event and holds no clock. The swarm drain loop (`swarm.rs:221-226`) already
threads `&mut HashMap` state (`agent_task`) through `translate`; it gains a
second such map, `HashMap<String, TextGate>` keyed by agent name, so each
agent's fragments coalesce independently while a parallel run interleaves them.
Without this the swarm path would emit one `Delta` per SSE fragment — the exact
per-token flood the 150 ms `StatsTick` gate exists to prevent, and the one
thing that could stall the winit thread, which polls these events synchronously
and cannot afford an unbounded queue per frame.

**Relay / fan / toolcall:** widen the streaming hook. `Adapter::
call_with_usage_ticked`'s `on_tokens: Arc<dyn Fn(u64) + Send + Sync>`
(`adapter.rs:67-75`) becomes a pair, so both signals ride one parameter:

```rust
/// The two live signals of one in-flight hop. Both advisory (see the Delta
/// and StatsTick docs); a backend that cannot stream simply never calls them.
pub struct HopStream {
    pub on_tokens: Arc<dyn Fn(u64) + Send + Sync>,
    pub on_text:   Arc<dyn Fn(&str) + Send + Sync>,
}
```

Call sites: `engine.rs:132`, `engine.rs:162`, `fan.rs:62`, `toolcall.rs:191`,
plus `apiadapter`'s impl and its two self-tests. `CliAdapter` keeps the default
no-op impl, so external CLIs (claude/codex/opencode) are unaffected — they have
no stream to forward.

**Emission channel:** deltas ride the existing `tick_emit` side-channel
(`stdio.rs:254-258`), which already writes straight to `Out` and deliberately
bypasses the `counting` wrapper. That is exactly right: a delta must not be
stamped with a task id (`counting` does that to every `Message`, :264-274) and
must not be counted as tokens.

**Kill switch:** `CREW_STREAM_TEXT=0` makes `hop_texter` a no-op, so the
pre-streaming behaviour stays one env var away for a regressed run or a
deterministic test.

### 4. `crew-app` — provisional cards, kept out of the transcript

The streamed text lives **outside** `messages`:

```rust
/// Agents mid-reply: one provisional card each, accumulating Delta text.
/// NEVER stored in `messages` — the transcript only ever holds settled
/// replies, so export, /restore, the session log, compact view and the swarm
/// fold are all correct here by construction.
pub(crate) streaming: Vec<Message>,
```

This is the reason for the shape. Putting a `streaming: bool` flag on `Message`
would work but leaves a half-written reply reachable by `/export`, `/restore`
and the session log, each of which would then need its own guard. Keeping
provisional cards in a separate list makes "the transcript holds only settled
messages" true by construction instead of by vigilance.

- **`Delta` arrives** → find this agent's provisional card (matched on
  `sender.split(" → ").next()`, the same normalization `chatflow::note_reply`
  already uses at :88, so a relay's `"coder → user"` sender matches a bare
  `"coder"` delta) or open one; append the text; report `changed`.
- **`Message` arrives** → drop that agent's provisional card, then push the
  settled `Message` exactly as today (`chat.rs:187`). Any fragment the gate
  dropped is healed by the replacement, and a reply that never streamed at all
  (external CLI, mock, `CREW_STREAM_TEXT=0`) takes the untouched path.
- **Turn ends / broker dies** → `flush_active_hops` and the `Error` arm clear
  `streaming`, so an interrupted hop cannot leave a card stranded forever.
- **`unread` and `scroll`** stay driven by the settled `Message` only. A
  provisional card must never bump the `↓ N new` pill.

**Render composition.** `card_lines`, `card_line_count`, `message_cells` and
`chatplace::placed_lines` move from `&[Message]` to `&[&Message]`; the pane
composes `messages.iter().chain(streaming.iter()).collect()` once per frame —
pointer copies, no `String` clones. Threading one composed slice through all
four keeps scroll math, the scrollbar, link hit-tests and the unread pill
automatically in agreement, exactly as the `View` struct already does
(`chatmsgs.rs:39-52`).

Provisional cards therefore render **below** all settled messages: history
above, live region at the bottom next to the spinner. When one agent of a
parallel run settles while another still streams, the settled card lands above
the still-live one — a small reorder, and the right way round.

**The dimmed tail.** The `chatswarmview` status row grows a dimmed tail — the
last 2 rows of the **most recently updated** streaming card (the agent whose
delta arrived last, not the one that started first), in `text_muted`,
front-ellipsized — but **only when the growing card cannot be seen**: the user
has scrolled up
(`scroll > 0`), or more than one agent is streaming (the scheduler's cap is 4)
so the newest is not the one drawing the eye. The tail is an overflow view, not
a second copy of what is already on screen.

### Non-goals

- **Reasoning / thinking tokens.** Neither provider parses `delta.reasoning`
  nor Anthropic thinking blocks, and nothing in `crew-hive` models a reasoning
  channel. "Thinking" here means the reply text as it is produced. A real
  reasoning stream is a separate, later goal.
- A markdown render cache (the transcript already re-renders per frame).
- Streaming for external CLI adapters — a single `claude -p` invocation has no
  incremental stdout to forward.
- Persisting partial replies across a restart, or streaming into `/export`.
- Syntax colouring of streamed code — that is the queued semantic-palette goal.

## Error handling

Partial markdown renders as-is: an unterminated ``` fence briefly shows an open
code card and self-heals on the next delta. Front-truncation is never applied
to the card (only to the tail), so a long reply scrolls normally as it grows.

A `Lagged` broadcast (`swarm.rs:234-237`) already emits its own visible note
and now also means some deltas were skipped — visible as a jump in the growing
card, healed by the settled `Message`. A hop that errors mid-stream keeps
whatever streamed; the `✗ failed:` message replaces the provisional card via
the same path as a success, since it arrives as a normal `Message`
(`swarmmsg.rs:61-66`). A provider transport error partway through the stream is
still returned as-is by `consume_sse` — no partial success is ever synthesized,
and that stays true.

## Testing

1. **`crew-hive::apiagent`** — `MockProvider` already splits its canned reply
   and calls `on_chunk` per group (`provider/mock.rs:29-49`), so this is
   headless: assert `ApiAgent` publishes **several `OutputDelta`s** and
   **exactly one `OutputChunk`** whose text is the complete reply, and that
   `TokenDelta`/`CostDelta` still land unchanged.
2. **`crew-plugin::tick`** — pure gate tests for `hop_texter`: the first flush
   passes; a flush inside the gap buffers rather than drops (concatenate every
   flushed payload and assert it equals the full input minus at most the
   unflushed tail — **no character lost at a flush boundary**); clock skew
   saturates without panicking, mirroring the existing `should_tick` test;
   `CREW_STREAM_TEXT=0` emits nothing.
3. **`crew-plugin::swarmmsg`** — `OutputDelta` translates to `Delta` with the
   task's specialty as the agent name (not its title), and `OutputChunk` still
   translates to a whole `Message`. Plus the multiplexing case: two agents
   interleaving fragments coalesce into **per-agent** `Delta`s (one agent's
   text never leaks into the other's), and a burst inside one gap yields one
   `Delta`, not one per fragment.
4. **`crew-plugin::protocol`** — `Delta` round-trips with a `"type":"delta"`
   tag, and an unknown event type is still skipped rather than fatal.
5. **`crew-plugin`** relay/fan/toolcall — extend the existing `StatsTick`
   assertions (`relay_tests.rs:271`, `toolcall_tests.rs:447`) to assert a
   `Delta` also lands for the same hop, so the widened `HopStream` is wired on
   every path and not just compiling.
6. **`crew-app::chat`** — a `Delta` opens a provisional card; a second appends
   to the same card, not a new one; a `Message` **replaces** it (transcript
   grows by exactly one, `streaming` empties); a `Message` with no preceding
   delta still lands; a relay `"coder → user"` sender matches a bare `"coder"`
   delta; a provisional card does **not** bump `unread` while scrolled up; the
   `Error` arm and turn-end idle both clear `streaming`.
7. **`crew-app::chatexport` / restore / session log** — a regression test that
   a transcript with an in-flight provisional card exports and restores
   **without** the partial text, pinning the §4 invariant.
8. **`crew-app::chatswarmview`** — the tail appears only when scrolled up or
   with 2+ streaming agents, is dimmed, and is clamped to 2 rows.
9. **Live GUI pass** via the `verify` skill (isolated HOME, frontmost-PID
   guard, screenshots): the real risk surface is frame pacing and text jitter
   under a live provider, which no unit test covers.
