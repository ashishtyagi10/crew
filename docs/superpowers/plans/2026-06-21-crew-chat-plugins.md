# Crew Chat Panes + Plugin System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add human team chat to Crew as a new pane type backed by subprocess plugins: a `crew-plugin` host + JSON line protocol, a reference echo plugin, and a `ChatPane` that renders messages + input as cells and tiles in the agent grid (`Cmd+J`).

**Architecture:** A pane becomes polymorphic — `PaneContent = Terminal | Chat`. Both emit `Vec<CellView>`, so `crew-render` stays terminal-agnostic. `crew-plugin` (new crate) holds the protocol types + a host that spawns a provider subprocess and exchanges newline-delimited JSON over stdio (reader-thread→channel, the PtyTerm pattern). The reference `crew-echo-plugin` ships as a bin inside `crew-plugin` (so integration tests can locate it via `CARGO_BIN_EXE_*`).

**Tech Stack:** Rust 2021, `serde` + `serde_json` (new dep — justified: standard JSON, not already in deps), existing `winit`/`wgpu`/`glyphon`/`alacritty_terminal`/`portable-pty`.

## Global Constraints

- **`crew-render` must NEVER import `crew-term` or `crew-plugin`** — chat crosses to the renderer as `CellView`, built in `crew-app`.
- **Every `.rs` file ≤ 200 lines (HARD).**
- **Reserved keys stay Super(Cmd)-chords only**; `Cmd+J` joins the set (spawn chat pane). Non-Super keys go to the focused pane (Terminal → PTY bytes; Chat → input edit).
- New deps limited to `serde`/`serde_json` (check they aren't already transitively usable first; add to workspace deps pinned).
- `cargo fmt` + `cargo clippy --workspace --all-targets` clean; no dead code / no `#[allow(dead_code)]`.
- GPU/integration tasks: gate = compile + clippy clean + `cargo test` + timeboxed non-panic launch (`timeout 6 cargo run -p crew-app` → exit 124). Reconcile any from-memory API against the real pinned crate.

---

### Task 1: `crew-plugin` crate — protocol types (TDD)

**Files:**
- Create: `crates/crew-plugin/Cargo.toml`, `crates/crew-plugin/src/lib.rs`, `crates/crew-plugin/src/protocol.rs`
- Modify: root `Cargo.toml` (add member + `serde`/`serde_json` to `[workspace.dependencies]`)

**Interfaces:**
- Produces:
  - `PluginCommand` (`#[serde(tag="type", rename_all="snake_case")]`): `Hello { v: u32 }`, `Subscribe { channel: String }`, `Send { channel: String, text: String }`.
  - `PluginEvent` (same tagging): `Ready { v: u32, provider: String, channels: Vec<String> }`, `Message { channel: String, sender: String, text: String, ts: String }`, `Error { message: String }`.

- [ ] **Step 1: Workspace + crate setup.** Root `Cargo.toml`: add `"crates/crew-plugin"` to members; add `serde = { version = "1", features = ["derive"] }` and `serde_json = "1"` to `[workspace.dependencies]`. `crew-plugin/Cargo.toml`: depends on `serde.workspace = true`, `serde_json.workspace = true`, `anyhow.workspace = true`. `lib.rs`: `mod protocol; pub use protocol::{PluginCommand, PluginEvent};`.

- [ ] **Step 2: Write the failing round-trip test** (`protocol.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hello_serializes_tagged() {
        let s = serde_json::to_string(&PluginCommand::Hello { v: 1 }).unwrap();
        assert_eq!(s, r#"{"type":"hello","v":1}"#);
    }
    #[test]
    fn message_event_roundtrips() {
        let line = r#"{"type":"message","channel":"general","sender":"bob","text":"hi","ts":"t"}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::Message { channel, sender, text, ts } => {
                assert_eq!((channel.as_str(), sender.as_str(), text.as_str(), ts.as_str()),
                           ("general", "bob", "hi", "t"));
            }
            _ => panic!("wrong variant"),
        }
    }
}
```

- [ ] **Step 3: Run → fail** (`cargo test -p crew-plugin`).
- [ ] **Step 4: Implement the enums** with `#[derive(Debug, Clone, Serialize, Deserialize)]` + `#[serde(tag = "type", rename_all = "snake_case")]`.
- [ ] **Step 5: Run → pass.** `cargo clippy -p crew-plugin --all-targets` clean.
- [ ] **Step 6: Commit** — `feat(crew-plugin): chat plugin protocol types`.

---

### Task 2: `crew-echo-plugin` reference plugin (TDD)

**Files:**
- Create: `crates/crew-plugin/src/bin/crew-echo-plugin.rs`, `crates/crew-plugin/src/echo.rs`
- Modify: `crates/crew-plugin/src/lib.rs` (`mod echo; pub use echo::respond;`)

**Interfaces:**
- Produces: `pub fn respond(cmd: &PluginCommand) -> Vec<PluginEvent>` — pure mapping: `Hello` → `[Ready { v:1, provider:"echo", channels:["general"] }]`; `Send { channel, text }` → `[Message { channel, sender:"echo", text, ts:"".into() }]`; `Subscribe` → `[]`.

- [ ] **Step 1: Failing test** (`echo.rs`): `respond(&Hello{v:1})` yields one `Ready` with provider `"echo"` and channels `["general"]`; `respond(&Send{channel:"general",text:"hi"})` yields one `Message` with text `"hi"`, sender `"echo"`.
- [ ] **Step 2: Run → fail.**
- [ ] **Step 3: Implement `respond`.**
- [ ] **Step 4: The bin** (`src/bin/crew-echo-plugin.rs`): read stdin lines; for each, `serde_json::from_str::<PluginCommand>(&line)` (skip parse errors); for each event in `respond(&cmd)`, `println!("{}", serde_json::to_string(&ev)?)` and flush stdout. Exit on EOF.

```rust
use std::io::{BufRead, Write};
use crew_plugin::{respond, PluginCommand};

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(cmd) = serde_json::from_str::<PluginCommand>(&line) else { continue };
        for ev in respond(&cmd) {
            writeln!(out, "{}", serde_json::to_string(&ev)?)?;
        }
        out.flush()?;
    }
    Ok(())
}
```

- [ ] **Step 5: Run → pass** (`cargo test -p crew-plugin`); `cargo build -p crew-plugin` (builds the bin); clippy clean.
- [ ] **Step 6: Commit** — `feat(crew-plugin): reference echo plugin`.

---

### Task 3: `crew-plugin` host — spawn + JSON lines (TDD)

**Files:**
- Create: `crates/crew-plugin/src/host.rs`
- Modify: `crates/crew-plugin/src/lib.rs` (`mod host; pub use host::Plugin;`)

**Interfaces:**
- Produces: `struct Plugin { child, stdin, rx: Receiver<PluginEvent> }` with
  - `Plugin::spawn(cmd: &str, args: &[String]) -> anyhow::Result<Plugin>`
  - `send(&mut self, cmd: &PluginCommand) -> anyhow::Result<()>` (writes one JSON line + `\n` + flush to child stdin)
  - `try_recv(&self) -> Vec<PluginEvent>` (non-blocking drain of the channel)

- [ ] **Step 1: Failing integration test** (`host.rs`): spawn the echo bin via its cargo path; send `Hello{v:1}`, poll `try_recv` up to ~3s until a `Ready` arrives (assert provider `"echo"`); send `Send{channel:"general",text:"ping"}`, poll until a `Message` with text `"ping"` arrives. The bin path: `env!("CARGO_BIN_EXE_crew-echo-plugin")`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginCommand;
    use std::time::{Duration, Instant};
    fn drain_until<F: Fn(&PluginEvent)->bool>(p:&Plugin, pred:F)->bool {
        let end = Instant::now()+Duration::from_secs(3);
        while Instant::now()<end {
            if p.try_recv().iter().any(&pred) { return true; }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }
    #[test]
    fn echo_roundtrip() {
        let mut p = Plugin::spawn(env!("CARGO_BIN_EXE_crew-echo-plugin"), &[]).unwrap();
        p.send(&PluginCommand::Hello { v: 1 }).unwrap();
        assert!(drain_until(&p, |e| matches!(e, PluginEvent::Ready{provider,..} if provider=="echo")));
        p.send(&PluginCommand::Send{channel:"general".into(), text:"ping".into()}).unwrap();
        assert!(drain_until(&p, |e| matches!(e, PluginEvent::Message{text,..} if text=="ping")));
    }
}
```

- [ ] **Step 2: Run → fail.**
- [ ] **Step 3: Implement `Plugin`** — `Command::new(cmd).args(args).stdin(piped).stdout(piped).spawn()`; take stdout, wrap in `BufReader`, spawn a thread: `for line in reader.lines() { if let Ok(ev)=serde_json::from_str(&line?) { tx.send(ev) } }` (drop unparseable lines; exit on EOF/err). Keep child stdin handle for `send`. `try_recv` drains `rx.try_recv()` into a Vec.

> Malformed-line resilience and broken-pipe handling are part of this: unparseable lines are skipped; when the child exits, the reader thread ends and `try_recv` simply returns empty (a synthetic disconnect Error can be added later — keep v1 simple but never panic).

- [ ] **Step 4: Run → pass** (`cargo test -p crew-plugin`); clippy clean.
- [ ] **Step 5: Commit** — `feat(crew-plugin): subprocess host with JSON line protocol`.

---

### Task 4: `crew-app` — `ChatPane` render + input (TDD)

**Files:**
- Create: `crates/crew-app/src/chat.rs`
- Modify: `crates/crew-app/Cargo.toml` (depend on `crew-plugin`), `crates/crew-app/src/main.rs` (`mod chat;`)

**Interfaces:**
- Produces:
  - `struct Message { sender: String, text: String }`
  - `struct ChatPane { plugin: Plugin, channel: String, messages: Vec<Message>, input: String, connected: bool }`
  - `ChatPane::new(plugin, channel) -> ChatPane`
  - `poll(&mut self)` — drain `plugin.try_recv()`: `Ready` → `connected=true` (+ set channel if empty); `Message` → push to `messages` (cap to last 500); `Error` → `connected=false`.
  - `cells(&self, cols: u16, rows: u16) -> Vec<CellView>` — render the most recent messages top-down wrapped to `cols`, reserve the bottom row for `"> " + input`, sender drawn in the neon accent fg `(0,255,160)`, text in `(200,200,200)`; cursor block at input end.
  - `on_key(&mut self, key: &KeyEvent)` — printable char → push to `input`; Backspace → pop; Enter → `plugin.send(Send{channel, text:input})` then clear `input`.

- [ ] **Step 1: Failing tests** (`chat.rs`) — pure layout/reducer, NO real plugin (construct `ChatPane` with an injected plugin is hard; instead split the PURE helpers out and test those):
  - `fn layout_cells(messages: &[Message], input: &str, cols: u16, rows: u16) -> Vec<CellView>` and `fn input_reduce(input: &mut String, ch: Option<char>, enter: bool, backspace: bool) -> Option<String>`.
  - Test: `layout_cells(&[Message{sender:"a".into(),text:"hi".into()}], "xy", 20, 5)` places `>` + `xy` on the last row (row 4), and `a` somewhere above; assert a cell with `c=='>'` at `row==4, col==0` and a cell `c=='x'` on row 4.
  - Test: `input_reduce(&mut s, Some('z'), false, false)` pushes `z`; `input_reduce(&mut s, None, true, false)` returns `Some(prev)` and clears `s`; backspace pops.

- [ ] **Step 2: Run → fail.**
- [ ] **Step 3: Implement** `layout_cells` + `input_reduce` (pure), then `ChatPane` methods delegating to them (`cells` → `layout_cells`; `on_key` → translate the winit key to (char/enter/backspace) then `input_reduce`, and on `Some(text)` call `plugin.send`). `poll` drains events. Split into `chat.rs` (+ a `chatlayout.rs` if over 200 lines).

> CellView import: `crew_render::CellView`. Reconcile winit `KeyEvent`→char as in `session.rs::key_to_bytes` (logical_key Character / NamedKey).

- [ ] **Step 4: Run → pass**; clippy clean; files ≤200.
- [ ] **Step 5: Commit** — `feat(crew-app): ChatPane render + input reducer`.

---

### Task 5: `crew-app` — pane polymorphism + Cmd+J (build-run-observe)

**Files:**
- Modify: `crates/crew-app/src/pane.rs`, `crates/crew-app/src/app.rs`, `crates/crew-app/src/handler.rs`, `crates/crew-app/src/session.rs`

**Interfaces:**
- Produces: `enum PaneContent { Terminal(TermPane), Chat(ChatPane) }` where `TermPane { pty: PtyTerm, input: Box<dyn Write+Send> }`; `Pane { content: PaneContent, grid: GridSize, rect: Rect }`. `Pane::cells(&self) -> Vec<CellView>` dispatches (Terminal → `to_cellviews(pty.cells())`; Chat → `chat.cells(grid.cols, grid.rows)`).

- [ ] **Step 1: Refactor `Pane`** to hold `PaneContent`. `spawn_pane` (terminal) wraps a `TermPane` in `PaneContent::Terminal`. `relayout` resizes only Terminal panes' PTYs (Chat panes just take the rect/grid). `build_scenes` calls `pane.cells()`.
- [ ] **Step 2: `spawn_chat_pane(&mut self)`** — resolve the plugin command: env `CREW_CHAT_PLUGIN` else `current_exe`-dir + `crew-echo-plugin`; `Plugin::spawn`; send `Hello{v:1}`; build `ChatPane::new`, wrap in `PaneContent::Chat`, push as a pane, focus it, relayout. eprintln on spawn failure.
- [ ] **Step 3: `about_to_wait`** — for each pane: Terminal → `pty.try_read()`; Chat → `chat.poll()`; redraw if any produced new output (chat: track if messages grew or events seen).
- [ ] **Step 4: keyboard routing** (`handler.rs`) — Super-chords unchanged + add `Cmd+J` → `spawn_chat_pane()`. Non-Super key → focused pane: `match content { Terminal(t) => key_to_bytes→t.input.write, Chat(c) => c.on_key(&event) + redraw }`.
- [ ] **Step 5: Gate** — `cargo build -p crew-app` + `cargo clippy --workspace --all-targets` clean; `cargo test -p crew-app -p crew-plugin -p crew-term` green; every `.rs` ≤200 (split if needed); `timeout 6 cargo run -p crew-app` exit 124.
- [ ] **Step 6: Commit** — `feat(crew-app): polymorphic panes + Cmd+J chat pane`.

---

### Task 6: Cleanup + milestone verification

- [ ] **Step 1:** `cargo fmt --all`; `cargo clippy --workspace --all-targets` (fix all; no `#[allow]`).
- [ ] **Step 2:** `cargo test --workspace` — all green (protocol, echo, host roundtrip, chat layout/reducer, terminal, layout, pane_at).
- [ ] **Step 3:** Every `.rs` in `crates/crew-*/src/` ≤200.
- [ ] **Step 4:** Manual smoke (record in commit body): `Cmd+J` opens a chat pane (tiles in); type + Enter echoes your message back as a `message`; `Cmd+T` still opens a terminal; click focuses; typing into a terminal pane still runs commands; `Cmd+W` closes either kind.
- [ ] **Step 5:** Commit milestone — `chore: Crew chat panes + plugin system milestone`.

---

## Notes for the next phase

- Real provider plugins (Slack first: token auth + socket-mode streaming) as separate programs speaking the protocol — no Crew changes needed beyond config.
- Channel switcher UI (a chord or in-pane list); multiple channels per pane.
- Synthetic disconnect `Error` event + a visible "disconnected" state in the chat pane.
- The agent orchestrator box (the separate agent surface) — its own design/plan.
