# Inter-Pane Ask v1 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans (inline). Steps use `- [ ]`.

**Goal:** `crew ask <id|label> "<q>"` — an agent in one pane queries an agent in another, visibly and in-session, with a liveness-governed wait returning `ANSWERED`/`NO_ANSWER{reason}`; plus `crew panes` for discovery.

**Architecture:** A dedicated IPC thread owns a Unix socket (all blocking I/O off the winit thread); it bridges to the poll loop via channels. The poll loop resolves the target, injects a sentinel-wrapped question into that pane (only if the pane is idle), and advances a pure liveness engine each tick until the sentinel closes, the target goes idle, or it stalls. Client subcommands (`crew ask`, `crew panes`) short-circuit in `main.rs` before GUI init.

**Tech Stack:** Rust, std `UnixListener`/`UnixStream`, `std::sync::mpsc`, serde_json, winit poll tick.

## Global Constraints

- No blocking I/O on the winit thread (memory: it freezes every pane) — socket I/O lives on the IPC thread.
- `.rs` ≤200 source lines; `cargo test --workspace`, clippy 0 warnings, fmt clean before merge.
- Never inject into a busy target pane → `BusyElsewhere` (protects the user's in-progress work).
- v1 = terminal-pane targets + local, single-target. Swarm-target delivery and `--any` broadcast are out of scope (v1c/v2).
- Socket path: `${XDG_RUNTIME_DIR:-<config_dir>/crew}/crew-ipc.sock`.

---

### Task 1: IPC message types (`ipc_types.rs`)

**Files:** Create `crates/crew-app/src/ipc_types.rs`; Test: inline `#[cfg(test)]`.

**Interfaces — Produces:**
```rust
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
#[serde(tag = "op")]
pub enum Request {
    Ask { v: u32, from: String, to: String, question: String, id: String },
    Panes { v: u32 },
}
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub enum NoAnswer { IdleNoEngage, Stalled, BusyElsewhere, Unreachable }
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
#[serde(tag = "kind")]
pub enum Reply {
    Answered { text: String },
    NoAnswer { reason: NoAnswer, partial: Option<String> },
    Roster { panes: Vec<PaneCard> },
}
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct PaneCard { pub id: String, pub label: Option<String>, pub kind: String,
    pub running: Option<String>, pub dir: Option<String>, pub busy: bool }
pub const PROTOCOL_V: u32 = 1;
```

- [ ] Step 1: Write test `request_and_reply_round_trip` — serialize an `Ask` and a `Reply::NoAnswer{IdleNoEngage,None}` to JSON, deserialize, assert equal; assert `serde_json::to_string(&Reply::Answered{text:"hi".into()})` contains `"Answered"`.
- [ ] Step 2: Run `cargo test -p crew-app -- ipc_types` → FAIL (module missing).
- [ ] Step 3: Create the file with the types above + `mod ipc_types;` in main.rs (after `mod input;` alphabetically).
- [ ] Step 4: Run → PASS.
- [ ] Step 5: Commit `feat(ask): ipc message types`.

---

### Task 2: Address resolution + question wrap + answer scan (`askroute.rs`)

**Files:** Create `crates/crew-app/src/askroute.rs`; Test: inline.

**Interfaces — Produces:**
```rust
/// Resolve an address to a pane index: exact `label` match, else `p{i}` index form.
pub(crate) fn resolve(panes: &[crate::pane::Pane], addr: &str) -> Option<usize>;
/// The sentinel-wrapped question injected into the target pane.
pub(crate) fn wrap(from: &str, id: &str, question: &str) -> String;
/// Scan captured output for a closed `<CREW-ANS id> … </CREW-ANS id>`; return inner text.
pub(crate) fn scan_answer(captured: &str, id: &str) -> Option<String>;
```
- Consumes: `Pane.label: Option<String>` (pane.rs:48).

- [ ] Step 1: Write tests:
```rust
#[test] fn resolve_by_label_then_index() {
    let p = panes_fixture(); // helper builds 2 terminal panes, pane[1].label = Some("schema")
    assert_eq!(resolve(&p, "schema"), Some(1));
    assert_eq!(resolve(&p, "p0"), Some(0));
    assert_eq!(resolve(&p, "nope"), None);
}
#[test] fn wrap_includes_from_id_and_sentinels() {
    let w = wrap("builder", "q7", "which API?");
    assert!(w.contains("builder") && w.contains("q7") && w.contains("which API?"));
    assert!(w.contains("<CREW-ANS q7>") && w.contains("</CREW-ANS q7>"));
}
#[test] fn scan_extracts_between_markers_only_when_closed() {
    assert_eq!(scan_answer("noise <CREW-ANS q7>v2</CREW-ANS q7> tail", "q7"), Some("v2".into()));
    assert_eq!(scan_answer("<CREW-ANS q7>partial no close", "q7"), None);
    assert_eq!(scan_answer("<CREW-ANS q9>other</CREW-ANS q9>", "q7"), None);
}
```
- [ ] Step 2: Run → FAIL.
- [ ] Step 3: Implement. `resolve`: first `panes.iter().position(|p| p.label.as_deref()==Some(addr))`, else parse `addr.strip_prefix('p')?.parse::<usize>().ok().filter(|&i| i<panes.len())`. `wrap`: `format!("\n[⇐ ask from \"{from}\" · {id}] {question}\nReply between <CREW-ANS {id}> and </CREW-ANS {id}>.\n")`. `scan_answer`: find `format!("<CREW-ANS {id}>")` open, then `format!("</CREW-ANS {id}>")` close after it, slice between, trim.
- [ ] Step 4: Run → PASS. Add the `panes_fixture` helper (build `Pane` literals — copy field set from pane_tests.rs).
- [ ] Step 5: Commit `feat(ask): address resolve + sentinel wrap/scan`.

---

### Task 3: Roster builder (`panes_roster.rs`)

**Files:** Create `crates/crew-app/src/panes_roster.rs`; Test: inline.

**Interfaces — Produces:**
```rust
pub(crate) fn roster(panes: &[crate::pane::Pane], procnames: &crate::procname::ProcNames,
    focused: usize) -> Vec<crate::ipc_types::PaneCard>;
```
- Consumes: `PaneContent` variants; `Pane.label/name/dir`; `pty.foreground_pid()`; `procnames.name(pid)`.

- [ ] Step 1: Write test `roster_reports_id_label_kind_and_busy` — fixture of a labeled terminal pane + a chat pane; assert card[0].id=="p0", kind=="terminal", card carries the label; chat pane kind=="swarm".
- [ ] Step 2: Run → FAIL.
- [ ] Step 3: Implement: map each pane to `PaneCard { id: format!("p{i}"), label: p.name.clone().or(p.label.clone()), kind: match &p.content { Terminal=>"terminal", Chat=>"swarm", Far=>"far", _=>"other" }, running: terminal fg pid→procnames.name, dir: p.dir…file_name, busy: <pane active heuristic> }`. Busy heuristic: terminal → `procnames.name(fg).is_some()` (a foreground agent running); chat → `c.is_busy()`.
- [ ] Step 4: Run → PASS.
- [ ] Step 5: Commit `feat(ask): panes roster builder`.

---

### Task 4: Liveness engine — the core (`askwait.rs`)

**Files:** Create `crates/crew-app/src/askwait.rs`; Test: inline (heavy).

**Interfaces — Produces:**
```rust
pub(crate) struct PendingAsk {
    pub id: String, pub target: usize, pub captured: String,
    pub produced_any: bool, pub first_out_ms: Option<u64>, pub last_progress_ms: u64,
}
pub(crate) struct Obs<'a> { pub new_output: &'a str, pub idle_transition: bool, pub now_ms: u64 }
pub(crate) enum Step { Wait, Answered(String), Stalled(Option<String>), IdleNoEngage }
impl PendingAsk {
    pub(crate) fn new(id: String, target: usize, now_ms: u64) -> Self;
    pub(crate) fn observe(&mut self, o: Obs) -> Step;
}
const BASE_QUIET_MS: u64 = 4_000;
```
- Consumes: `askroute::scan_answer`.

- [ ] Step 1: Write tests (the heart — script Obs sequences):
```rust
#[test] fn sentinel_close_yields_answered() {
    let mut a = PendingAsk::new("q7".into(), 0, 0);
    assert!(matches!(a.observe(Obs{new_output:"working…", idle_transition:false, now_ms:100}), Step::Wait));
    let s = a.observe(Obs{new_output:"<CREW-ANS q7>v2</CREW-ANS q7>", idle_transition:false, now_ms:200});
    assert!(matches!(s, Step::Answered(t) if t=="v2"));
}
#[test] fn idle_with_no_output_is_idle_no_engage() {
    let mut a = PendingAsk::new("q7".into(), 0, 0);
    assert!(matches!(a.observe(Obs{new_output:"", idle_transition:true, now_ms:50}), Step::IdleNoEngage));
}
#[test] fn idle_after_output_without_close_is_stalled_with_partial() {
    let mut a = PendingAsk::new("q7".into(), 0, 0);
    a.observe(Obs{new_output:"thinking about it", idle_transition:false, now_ms:100});
    let s = a.observe(Obs{new_output:"", idle_transition:true, now_ms:200});
    assert!(matches!(s, Step::Stalled(Some(p)) if p.contains("thinking")));
}
#[test] fn active_but_silent_past_adaptive_budget_is_stalled() {
    let mut a = PendingAsk::new("q7".into(), 0, 0);
    a.observe(Obs{new_output:"x", idle_transition:false, now_ms:0});
    // silent from ms 0; base budget 4000 → still waiting at 3999, stalled at 4001
    assert!(matches!(a.observe(Obs{new_output:"", idle_transition:false, now_ms:3_999}), Step::Wait));
    assert!(matches!(a.observe(Obs{new_output:"", idle_transition:false, now_ms:4_001}), Step::Stalled(_)));
}
#[test] fn long_stream_earns_more_patience() {
    let mut a = PendingAsk::new("q7".into(), 0, 0);
    // stream from ms 0..10000, last progress 10000; adaptive budget = base + span
    for t in (0..=10_000).step_by(1000) { a.observe(Obs{new_output:"chunk ", idle_transition:false, now_ms:t}); }
    // silent after 10000; span=10000 so budget ≈ 4000+10000; still waiting at 12000
    assert!(matches!(a.observe(Obs{new_output:"", idle_transition:false, now_ms:12_000}), Step::Wait));
}
```
- [ ] Step 2: Run → FAIL.
- [ ] Step 3: Implement `observe`: append `new_output` to `captured`; if non-empty set `produced_any=true`, `first_out_ms.get_or_insert(now_ms)`, `last_progress_ms=now_ms`. Then: if `scan_answer(&captured,&id)` → `Answered`. If `idle_transition` → `produced_any ? Stalled(Some(captured.clone())) : IdleNoEngage`. Else adaptive silence check: `span = now_ms - first_out_ms.unwrap_or(now_ms)`; `budget = BASE_QUIET_MS + span`; if `produced_any && now_ms - last_progress_ms > budget` → `Stalled(Some(captured.clone()))`; else `Wait`.
- [ ] Step 4: Run → PASS.
- [ ] Step 5: Commit `feat(ask): liveness/verdict engine`.

---

### Task 5: IPC socket thread (`ipc.rs`)

**Files:** Create `crates/crew-app/src/ipc.rs`; Test: inline (socket round-trip with a temp path).

**Interfaces — Produces:**
```rust
pub(crate) struct Incoming { pub req: crate::ipc_types::Request, pub reply: std::sync::mpsc::Sender<crate::ipc_types::Reply> }
pub(crate) struct IpcHandle { pub rx: std::sync::mpsc::Receiver<Incoming> } // held by app; drained each tick
pub(crate) fn socket_path() -> std::path::PathBuf;
pub(crate) fn spawn() -> std::io::Result<IpcHandle>; // binds socket, spawns listener thread
```
- Thread: `UnixListener::bind` (unlink stale first); per accepted stream, spawn a handler thread: read one JSON line → `Request`; make a `mpsc::channel::<Reply>()`; send `Incoming{req, reply:tx}` to app; block on `rx.recv()` (bounded by a hard 5-min read-timeout via `set_read_timeout` guard) → write reply JSON line → close.

- [ ] Step 1: Test `socket_round_trips_a_panes_request`: `spawn()`, connect a client `UnixStream` to `socket_path()`, write `{"op":"Panes","v":1}\n`, then on the app side `handle.rx.recv()` → assert it's `Panes`, send back `Reply::Roster{panes:vec![]}` via `incoming.reply`, client reads the line → asserts it parses to `Roster`.
- [ ] Step 2: Run → FAIL.
- [ ] Step 3: Implement. `socket_path`: `std::env::var("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|_| dirs_config_crew()).join("crew-ipc.sock")`. Handler threads keep it simple; JSON is newline-delimited.
- [ ] Step 4: Run → PASS.
- [ ] Step 5: Commit `feat(ask): unix-socket ipc thread`.

---

### Task 6: Client subcommands `crew ask` / `crew panes` (`askclient.rs` + main.rs)

**Files:** Create `crates/crew-app/src/askclient.rs`; Modify `crates/crew-app/src/main.rs` (short-circuit before GUI, like `--list-fonts` at main.rs:154).

**Interfaces — Produces:** `pub(crate) fn run_ask(to: &str, question: &str) -> anyhow::Result<i32>;` and `pub(crate) fn run_panes() -> anyhow::Result<i32>;` — connect to `ipc::socket_path()`, send the `Request`, read the `Reply`, print human text, return an exit code (0 answered/roster, 2 no_answer, 3 unreachable/no socket).

- [ ] Step 1: Test `format_reply_renders_verdicts` (pure formatter `fn render(r:&Reply)->(String,i32)`): `Answered{"v2"}`→("ANSWERED: v2",0); `NoAnswer{IdleNoEngage,None}`→ contains "NO_ANSWER" & "idle", code 2; `Roster` → a table string, code 0.
- [ ] Step 2: Run → FAIL.
- [ ] Step 3: Implement `render` + `run_ask`/`run_panes` (connect/send/recv; on connect error → print "NO_ANSWER unreachable (no crew running)", code 3). In main.rs, before GUI init: parse `args`: `["ask", to, q]` → `return run_ask(...)`-mapped; `["panes"]` → `run_panes`. Use `exit(code)`.
- [ ] Step 4: Run → PASS.
- [ ] Step 5: Commit `feat(ask): crew ask / crew panes client subcommands`.

---

### Task 7: Wire into the app (poll loop + startup)

**Files:** Modify `crates/crew-app/src/app.rs` (fields), `crates/crew-app/src/handler.rs` (spawn ipc in `run()`), `crates/crew-app/src/poll.rs` (drain + inject + tick + resolve).

**Interfaces — Consumes:** everything above.

- [ ] Step 1: Test (integration, `chat`-style) `poll_resolves_unreachable_and_busy` — construct a `CrewApp` with one idle terminal pane; push an `Incoming{Ask{to:"nope"…}, reply}`; run one `poll_panes` equivalent drain; assert `reply.recv()` is `NoAnswer{Unreachable}`. Second case: target busy → `NoAnswer{BusyElsewhere}`. (Use a helper that calls the new `drain_asks` method directly to avoid a full winit loop.)
- [ ] Step 2: Run → FAIL.
- [ ] Step 3: Implement:
  - app.rs: add `pub(crate) ipc: Option<crate::ipc::IpcHandle>`, `pub(crate) pending_asks: Vec<(crate::askwait::PendingAsk, std::sync::mpsc::Sender<crate::ipc_types::Reply>)>` (defaults: None / empty).
  - New `crate::askpump` method `drain_asks(&mut self, now_ms)`: for each `Incoming`: if `Panes` → `reply.send(Roster{roster(...)})`. If `Ask` → `resolve`; None → `NoAnswer{Unreachable}`; busy (roster card busy for that pane) → `NoAnswer{BusyElsewhere}`; else inject `wrap(...)` via a new `inject_terminal(index, text)` (body = send_to_label's PTY write, by index) and push a `PendingAsk`.
  - Advance pending each tick: gather each target terminal's new output (extend `poll_panes`'s existing per-pane read to expose bytes; simplest: `PtyTerm` already appends to its grid — instead capture via a small tail buffer. For v1, read the pane's newly-read bytes by having `try_read` return/`take` the decoded delta — add `pty.take_recent()` returning the last-read chunk). Feed `observe`; on non-Wait → `reply.send(verdict)`, drop the pending.
  - handler.rs `run()`: `let ipc = crate::ipc::spawn().ok();` store on app.
  - poll.rs `poll_panes`: call `self.drain_asks(now_ms)` and `self.tick_asks(now_ms)`.
- [ ] Step 4: Run → PASS; full `cargo test -p crew-app`.
- [ ] Step 5: Commit `feat(ask): wire ipc into poll loop + startup`.

---

### Task 8: End-to-end integration, docs, release

- [ ] Step 1: Harness integration test (ignored by default, run manually): launch isolated crew with two terminal panes; from pane A run `crew ask p1 "reply OK"`; a cooperating stub in pane B echoes `<CREW-ANS …>OK</CREW-ANS …>`; assert `ANSWERED: OK`. Plain-shell pane B → `NO_ANSWER idle`.
- [ ] Step 2: Add a short `crew ask` / `crew panes` section to `docs/CREW.md`.
- [ ] Step 3: Capability bootstrap (v1c seed): when spawning a terminal pane for a known agent CLI, no change needed for v1 (documented commands); leave a TODO-free note in the spec that MCP-relay exposure is the follow-up. (No code.)
- [ ] Step 4: Full gates (`cargo test --workspace`, clippy, fmt); all touched files ≤200.
- [ ] Step 5: Merge `--no-ff`, bump version, build, install, tag, push.
