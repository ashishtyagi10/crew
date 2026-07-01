# Crew pane UI enhancement — 10-iteration loop

**Goal:** transform the `/crew` multi-agent pane from a bland sender:text list
into a rich, modern multi-model/multi-agent UI, taking inspiration from
Claude Code, Cursor, Antigravity, Groq, and other market leaders.

**Cadence:** one iteration per hour, driven by a scheduled trigger. Each
iteration adds a full, self-contained feature set to the crew pane, then
**verifies** (`cargo fmt --check`, `cargo clippy --workspace --all-targets`,
`cargo test -p crew-app`) and **commits**. After iteration 10, cut a patch
release (bump version, tag `vX.Y.Z`, push) and **delete the trigger to stop**.

**Files:** `crates/crew-app/src/chat.rs` (pane state + `cells()`),
`chatlayout.rs` (message/input layout), and new `chat*` submodules (keep every
file ≤200 lines; split by responsibility). `layout_cells` is tested directly —
compose new UI in `ChatPane::cells()` and leave `layout_cells` intact where
possible.

**Guardrails:** if an iteration can't be made to pass verification, STOP and
report — never commit broken code, never skip the gates, never auto-advance past
a failure. Keep each iteration shippable on its own.

## Progress

- [x] **1 — Header status bar.** A top row: title (`crew · <channel>`) left;
  right-aligned live status — connection dot, message count, animated "thinking"
  spinner while a reply is pending.
- [x] **2 — Agent roster + model badges.** Track the connected agents (from the
  broker `Ready`/hop events) and show them as colored chips with role + model
  labels and an active-agent highlight (à la Cursor's agent bar).
- [x] **3 — Role-styled message cards.** Distinct treatment per sender (user vs
  each agent vs system): a colored gutter/rule, a role label line, and clear
  separation between messages rather than inline `sender: text`.
- [x] **4 — Handoff / relay visualization.** Render `@next`/`@done` control hops
  as inline `from → to` connectors/pills so the conversation flow is legible.
  Plus: structured `Activity` events (agent + state) replace "calling X…"
  transcript spam; the header names the thinking agent with elapsed seconds and
  the roster highlights it (▸ + bold).
- [x] **5 — Code block rendering.** Detect fenced ```code``` in messages and
  render it in a bordered monospace card with a dimmed background and language tag.
  Bonus: message bodies are now newline-aware (multi-line replies render as real
  lines), and code wraps verbatim (hard chunking, no dropped break spaces).
- [x] **6 — Rich input area.** A framed composer with an `@agent` target hint, a
  affordance bar of available agents + slash actions, and send/enter hints.
  Tall panes get an affordance bar (`@agent` chips in roster colours, key hints
  right-aligned) above a `❯` prompt with a caret; a valid leading `@mention`
  renders bold in that agent's colour.
- [x] **7 — Scrollbar + new-message affordance.** A visual scroll indicator and a
  "N new ↓" pill when scrolled up, with a jump-to-latest. Proportional thumb in
  the last column while the transcript overflows; the `↓ N new` pill counts
  messages that arrive out of view and clears on return to the live bottom
  (sending also snaps to latest).
- [x] **8 — Thinking/stream timeline.** A per-turn activity timeline: which agent
  is working, elapsed time, and a live token/cost meter in the header. The broker
  times each agent call and ends every turn with a structured `Stats` event plus
  a `turn done — planner 4.2s → coder 8.1s · …` summary line; the header shows a
  running `~N tok` meter.
- [x] **9 — Timestamps + per-message metadata.** Relative timestamps and a subtle
  metadata line (tokens/latency) per message when available. The broker stamps
  every message with epoch-ms `ts` and the reply's latency in a new `meta`
  field; card headers show `▍sender · 2m ago · 4.2s`.
- [x] **10 — Empty-state onboarding + polish.** A welcoming empty state (detected
  agents, example prompts, quick-start), plus a final theming/consistency pass.
  Then cut the release and stop the loop. A fresh pane now shows "Your crew is
  ready" with the agent roster + roles, a how-to line, and a `Try: @planner …`
  example; connecting and no-key states explain themselves.

All 10 iterations shipped: 1 in v0.5.14 (header), 2/3/4/8/9 in v0.5.15
(roster, cards, activity, timeline/meter, timestamps), 5/6/7/10 in v0.5.16
(code blocks, composer, scroll affordances, onboarding). Loop complete.

## How to continue (trigger fires here each hour)

1. Read this file; find the first unchecked iteration.
2. Implement its full feature set on the crew pane.
3. Run the three gates; fix until green.
4. Commit (`feat(crew-app): crew pane UI — <iteration title>`), then check the
   box in this file and commit the roadmap update.
5. If that was iteration 10: bump the version, tag, push (release), then delete
   the scheduled trigger to stop the loop. Otherwise wait for the next firing.
