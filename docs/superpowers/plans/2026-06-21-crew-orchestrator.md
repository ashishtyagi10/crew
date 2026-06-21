# Crew Agent Orchestrator (pane-driving plugins) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Let a plugin not just chat but **drive the agent grid** — spawn agent panes and send them commands — and ship a **reference orchestrator plugin** that, on a prompt, spawns sub-agent panes and delegates. This is the design doc's "orchestrator drives N panes," built on the subprocess-plugin pipe from the chat milestone. Real LLM orchestrators become later plugins (same swap as the echo plugin).

**Design (grounded in the Crew design doc + the chat plugin infra):** Extend the plugin protocol with two plugin→host events — `spawn_pane` and `send_pane`. The plugin host already streams `PluginEvent`s; a chat/orchestrator pane's `poll()` now returns a list of **host actions** (spawn/send) for the app to execute, while still handling chat `message`/`ready`/`error` internally. The app spawns labeled terminal panes and routes `send_pane` to them by label. `Cmd+O` opens an **orchestrator pane** (a chat-style pane bound to the orchestrator plugin). The reference `crew-orchestrator-plugin`, on a prompt, emits a plan message + two `spawn_pane` events — proving the spawn-and-delegate loop headlessly (no LLM).

**Tech Stack:** Rust 2021, existing crates + `serde`/`serde_json`.

## Global Constraints
- **`crew-render` imports neither `crew-term` nor `crew-plugin`.**
- **Every `.rs` ≤ 200 lines (HARD).**
- **Reserved keys = Super-chords only** (now incl. `Cmd+O`); non-Super keys go to the focused pane.
- `cargo clippy --workspace --all-targets` ZERO warnings; **no `#[allow]`** of any kind.
- Gate: compile + clippy clean + `cargo test` + timeboxed non-panic launch (`timeout 6 cargo run -p crew-app` → exit 124).

---

### Task 1: protocol — `spawn_pane` / `send_pane` events (TDD)

**Files:** Modify `crates/crew-plugin/src/protocol.rs`.

**Interfaces:** Add to `PluginEvent`: `SpawnPane { command: String, args: Vec<String>, label: String }` and `SendPane { label: String, text: String }` (same `#[serde(tag="type", rename_all="snake_case")]`, so tags `"spawn_pane"` / `"send_pane"`).

- [ ] **Step 1: Failing tests** — `serde_json::to_string(&PluginEvent::SpawnPane{command:"sh".into(),args:vec!["-c".into()],label:"a".into()})` contains `"type":"spawn_pane"`; a `{"type":"send_pane","label":"a","text":"hi"}` line deserializes into `SendPane{label:"a",text:"hi"}`.
- [ ] **Step 2: Run → fail.**
- [ ] **Step 3: Add the two variants.**
- [ ] **Step 4: Run → pass;** `cargo test -p crew-plugin` (all prior tests still green); clippy clean; protocol.rs ≤200.
- [ ] **Step 5: Commit** — `feat(crew-plugin): spawn_pane/send_pane protocol events`.

---

### Task 2: reference orchestrator plugin (TDD)

**Files:** Create `crates/crew-plugin/src/orchestrator.rs`, `crates/crew-plugin/src/bin/crew-orchestrator-plugin.rs`; modify `lib.rs` (`mod orchestrator; pub use orchestrator::plan;`).

**Interfaces:** `pub fn plan(cmd: &PluginCommand) -> Vec<PluginEvent>`:
- `Hello` → `[Ready { v:1, provider:"orchestrator", channels:vec!["plan"] }]`.
- `Send { text, .. }` → `[ Message { channel:"plan", sender:"orchestrator", text: format!("Plan: spawning 2 agents for: {text}"), ts:String::new() }, SpawnPane{ command:"sh", args:["-c", format!("echo agent-A on: {text}; sleep 30")], label:"agent-A" }, SpawnPane{ command:"sh", args:["-c", format!("echo agent-B on: {text}; sleep 30")], label:"agent-B" } ]`.
- `Subscribe` → `[]`.

- [ ] **Step 1: Failing tests** — `plan(&Hello{v:1})` → one `Ready` provider `"orchestrator"`; `plan(&Send{channel:"plan",text:"build X"})` → first a `Message` whose text contains `"build X"`, then two `SpawnPane` with labels `agent-A`/`agent-B` and commands containing `"build X"`.
- [ ] **Step 2: Run → fail.**
- [ ] **Step 3: Implement `plan`.**
- [ ] **Step 4: The bin** `crew-orchestrator-plugin.rs` — identical loop to the echo bin but calls `plan(&cmd)` instead of `respond(&cmd)` (read stdin JSON lines, skip parse errors, write each event as a JSON line + flush). Cargo auto-discovers the bin.
- [ ] **Step 5: Run → pass;** `cargo build -p crew-plugin` (bin builds); optional sanity `echo '{"type":"send","channel":"plan","text":"hi"}' | cargo run -q --bin crew-orchestrator-plugin` shows a message + 2 spawn_pane lines; clippy clean; files ≤200.
- [ ] **Step 6: Commit** — `feat(crew-plugin): reference orchestrator plugin`.

---

### Task 3: `ChatPane.poll` returns host actions (TDD)

**Files:** Modify `crates/crew-app/src/chat.rs` (+ a `chataction.rs` if needed).

**Interfaces:**
- `pub enum HostAction { SpawnPane { command: String, args: Vec<String>, label: String }, SendPane { label: String, text: String } }` (crew-app's own type).
- Change `ChatPane::poll(&mut self) -> Vec<HostAction>` — drain `plugin.try_recv()`: `Ready`/`Message`/`Error` handled internally as today (mark changed for redraw via an internal flag or return value tweak); `SpawnPane`/`SendPane` → pushed to the returned `Vec<HostAction>`. Keep a `pub fn took_event(&self) -> bool`? Simpler: have `poll` return `Vec<HostAction>` AND set an internal `dirty` flag readable via the existing redraw logic — OR return `(bool, Vec<HostAction>)`. Choose `-> Vec<HostAction>` and treat "any events drained" as needing redraw by also returning a bool: define `poll(&mut self) -> PollResult { changed: bool, actions: Vec<HostAction> }`.
- Pure helper to TDD without a real plugin: `pub fn classify(ev: &PluginEvent) -> Option<HostAction>` mapping `SpawnPane`/`SendPane` → `Some(HostAction…)`, others → `None`.

- [ ] **Step 1: Failing tests** — `classify(&PluginEvent::SpawnPane{command:"sh",args:vec![],label:"x"})` → `Some(HostAction::SpawnPane{label:"x",..})`; `classify(&PluginEvent::Message{..})` → `None`. (And a test that a sequence drained through a small helper separates chat-display events from host actions.)
- [ ] **Step 2: Run → fail.**
- [ ] **Step 3: Implement** `HostAction`, `classify`, and refactor `poll` to a `PollResult { changed, actions }` (update the field/struct; keep `Ready`/`Message`/`Error` handling). Keep files ≤200 (split a `chataction.rs` if needed).
- [ ] **Step 4: Run → pass;** `cargo test -p crew-app` green; clippy clean (note: callers of `poll` in handler.rs will break — that's Task 4; for THIS task keep crew-app compiling by updating the single `poll()` call site minimally to ignore actions, OR do the full wiring here. Prefer: update the call site to consume `result.changed` and ignore actions for now so the workspace compiles).
- [ ] **Step 5: Commit** — `feat(crew-app): ChatPane.poll surfaces host actions`.

---

### Task 4: app executes host actions + `Cmd+O` (build-run-observe)

**Files:** Modify `crates/crew-app/src/pane.rs`, `app.rs`, `handler.rs`.

**Interfaces:** `Pane` gains `pub label: Option<String>`. `spawn_pane` sets `label: None`. New `CrewApp::spawn_labeled_terminal(&mut self, command, args, label)` and `CrewApp::send_to_label(&mut self, label, text)`. `spawn_chat_pane` parameterized by plugin command so `Cmd+J` (echo) and `Cmd+O` (orchestrator) share it.

- [ ] **Step 1: Pane label + helpers** — add `label` to `Pane`; `spawn_labeled_terminal` spawns a terminal pane (reuse the `TermPane` spawn path) running `command args`, sets its `label`, pushes, relayout; `send_to_label` finds the pane whose `label == Some(label)` and, if Terminal, writes `text + "\n"` to its `input`.
- [ ] **Step 2: Execute host actions** — in `about_to_wait`, collect `PollResult.actions` from every chat pane; after polling, execute: `SpawnPane{command,args,label}` → `spawn_labeled_terminal(...)`; `SendPane{label,text}` → `send_to_label(...)`. Redraw if any pane changed OR any action ran.
- [ ] **Step 3: `Cmd+O`** — resolve the orchestrator plugin (`CREW_ORCHESTRATOR_PLUGIN` env → sibling `crew-orchestrator-plugin`); call the parameterized `spawn_chat_pane(cmd)`. Add `Cmd+O` to the Super-chord handler (Key "o"). `Cmd+J` keeps using the echo plugin.
- [ ] **Step 4: Gate** — `cargo build -p crew-app` + `cargo clippy --workspace --all-targets` ZERO warnings; `cargo test --workspace` green; every `.rs` ≤200 (split if needed); `timeout 6 cargo run -p crew-app` exit 124.
- [ ] **Step 5: Commit** — `feat(crew-app): execute plugin host actions + Cmd+O orchestrator pane`.

---

### Task 5: Cleanup + milestone verification

- [ ] **Step 1:** `cargo fmt --all`; `cargo clippy --workspace --all-targets` ZERO warnings.
- [ ] **Step 2:** `cargo test --workspace` all green.
- [ ] **Step 3:** Every `.rs` in `crates/crew-*` ≤200.
- [ ] **Step 4:** Manual smoke (record in commit): `Cmd+O` opens an orchestrator pane; typing a prompt + Enter shows a "Plan:" message AND spawns two `agent-A`/`agent-B` terminal panes that tile in; `Cmd+J` echo chat still works; terminals still work; `Cmd+W` closes panes.
- [ ] **Step 5:** Commit milestone — `chore: Crew agent orchestrator (pane-driving plugins) milestone`.

---

## Notes for the next phase
- A real LLM orchestrator plugin (decompose via an API; needs provider keys) — drop-in on this protocol.
- Result-gathering from sub-agent panes (the design's sentinel/file convention) → host→plugin `pane_output` event.
- Plugin kill-on-drop; disconnect UI; inline images (Kitty/Sixel); LRU minimized tile strip.
