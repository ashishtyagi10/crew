# Composer palettes Implementation Plan

> **For agentic workers:** implement task-by-task with TDD; each step is a failing test → minimal code → green → commit.

**Goal:** Add a `/` command palette and a leading-`@agent` picker to the crew chat composer, reusing the `chatmention` popup pattern and `cmdmenu` card rendering.

**Architecture:** New `chatpalette.rs` (pure logic + popup state) handles the LEADING token only; `chat.rs` routes its keys before the file mention; `render.rs` draws its card. At most one popup is ever open (leading-token palette vs mid-line file mention are mutually exclusive by construction).

**Spec:** docs/superpowers/specs/2026-07-02-composer-palettes-design.md

## Global Constraints
- crew-app files stay focused; tests in `#[cfg(test)] mod tests` within the file (or a sibling per existing pattern).
- Reuse `crate::suggest::MenuItem { label, desc, fill, submit }`, `crate::chatkeys::ChatInput`, `crew_plugin::AgentInfo` (`.name`, `.role`), `crate::chatcomplete::CONSTRUCTS`.
- Commit messages end with: `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- Pre-commit runs cargo fmt + check; `cargo fmt` before each commit.

---

### Task 1: `chatpalette.rs` — pure logic + popup state

**Files:** Create `crates/crew-app/src/farpane`… no — `crates/crew-app/src/chatpalette.rs`; register `mod chatpalette;` in `crates/crew-app/src/main.rs` (alongside `mod chatmention;`). Add a `describe` table to `crates/crew-app/src/chatcomplete.rs`.

**Interfaces produced (used by Tasks 2–3):**
- `enum Kind { Slash, Agent }` (derive Clone, Copy, PartialEq, Eq, Debug)
- `struct PaletteState { pub kind: Kind, pub items: Vec<MenuItem>, pub sel: usize }`
- `enum PaletteKey { Consumed, Forward }`
- `fn pending_palette(input: &str) -> Option<(Kind, &str)>`
- `fn after_edit(palette: &mut Option<PaletteState>, input: &str, agents: &[AgentInfo])`
- `fn popup_key(palette: &mut Option<PaletteState>, input: &mut String, key: &ChatInput) -> PaletteKey`
- `fn accept(input: &str, kind: Kind, fill: &str) -> String`
- In chatcomplete: `pub(crate) fn describe(construct: &str) -> &'static str`

- [ ] **Step 1 — failing tests** (in `chatpalette.rs` `#[cfg(test)] mod tests`):

```rust
use super::*;
use crate::chatkeys::ChatInput;
use crew_plugin::AgentInfo;

fn agents() -> Vec<AgentInfo> {
    // AgentInfo does NOT derive Default — construct all three fields.
    ["planner", "coder"].iter().map(|n| AgentInfo {
        name: n.to_string(), role: "role".into(), model: String::new()
    }).collect()
}

#[test]
fn pending_palette_detects_leading_slash_and_agent() {
    assert_eq!(pending_palette("/mod"), Some((Kind::Slash, "mod")));
    assert_eq!(pending_palette("@co"), Some((Kind::Agent, "co")));
    assert_eq!(pending_palette("@a+co"), Some((Kind::Agent, "co"))); // segment after '+'
    assert_eq!(pending_palette("@planner"), Some((Kind::Agent, "planner")));
    assert_eq!(pending_palette("hey @co"), None); // non-leading → file mention's job
    assert_eq!(pending_palette("/model x"), None); // token ended
    assert_eq!(pending_palette("plain"), None);
    assert_eq!(pending_palette(""), None);
}

#[test]
fn accept_replaces_leading_token_preserving_multi_target() {
    assert_eq!(accept("/mod", Kind::Slash, "/model"), "/model ");
    assert_eq!(accept("@co", Kind::Agent, "coder"), "@coder ");
    assert_eq!(accept("@a+co", Kind::Agent, "coder"), "@a+coder ");
}

#[test]
fn after_edit_opens_refilters_and_closes() {
    let a = agents();
    let mut p = None;
    after_edit(&mut p, "@", &a);
    assert_eq!(p.as_ref().unwrap().items.len(), 2);
    assert_eq!(p.as_ref().unwrap().kind, Kind::Agent);
    after_edit(&mut p, "@co", &a);
    assert_eq!(p.as_ref().unwrap().items.len(), 1); // only coder
    after_edit(&mut p, "@zzz", &a);
    assert!(p.is_none()); // no match closes
    after_edit(&mut p, "/mo", &a);
    assert_eq!(p.as_ref().unwrap().kind, Kind::Slash);
    assert!(p.as_ref().unwrap().items.iter().any(|i| i.fill == "/model"));
    after_edit(&mut p, "hey", &a);
    assert!(p.is_none()); // no leading selector
}

#[test]
fn popup_key_navigates_accepts_and_closes() {
    let a = agents();
    let mut p = None;
    after_edit(&mut p, "@", &a);
    let mut input = "@".to_string();
    assert!(matches!(popup_key(&mut p, &mut input, &ChatInput::Down), PaletteKey::Consumed));
    assert!(matches!(popup_key(&mut p, &mut input, &ChatInput::Enter), PaletteKey::Consumed));
    assert!(input.starts_with('@') && input.ends_with(' '));
    assert!(p.is_none());
    // Esc closes the popup, not the pane.
    after_edit(&mut p, "/", &a);
    assert!(matches!(popup_key(&mut p, &mut input, &ChatInput::Close), PaletteKey::Consumed));
    assert!(p.is_none());
    // Closed popup forwards.
    assert!(matches!(popup_key(&mut None.as_mut().map(|_:&mut PaletteState| unreachable!()).into(), &mut input, &ChatInput::Enter), PaletteKey::Forward));
}
```
(If the last `popup_key` forward-case is awkward to express, replace with: `let mut none: Option<PaletteState> = None; assert!(matches!(popup_key(&mut none, &mut input, &ChatInput::Enter), PaletteKey::Forward));`)

- [ ] **Step 2 — run, expect fail** (`cargo test -p crew-app chatpalette`): compile error, module missing.

- [ ] **Step 3 — implement `chatpalette.rs`:**

```rust
//! Leading-token pop-ups in the crew composer: a `/` command palette and a
//! leading `@agent` picker. Distinct from the mid-line `@file` mention
//! (chatmention): this handles ONLY the leading token, that only non-leading
//! ones, so at most one is open. Pure string logic + popup state.
use crew_plugin::AgentInfo;

use crate::chatcomplete::{describe, CONSTRUCTS};
use crate::chatkeys::ChatInput;
use crate::suggest::MenuItem;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Slash,
    Agent,
}

/// The open leading-token palette: already-filtered rows + selection.
pub(crate) struct PaletteState {
    pub kind: Kind,
    pub items: Vec<MenuItem>,
    pub sel: usize,
}

pub(crate) enum PaletteKey {
    Consumed,
    Forward,
}

/// The leading token being typed, if it's a `/command` or `@agent` selector
/// (nothing before it — no whitespace yet). For a multi-target `@a+b`, the
/// query is the segment after the last `+` (matching chatcomplete's Tab).
pub(crate) fn pending_palette(input: &str) -> Option<(Kind, &str)> {
    if input.contains(char::is_whitespace) {
        return None;
    }
    if let Some(rest) = input.strip_prefix('/') {
        return Some((Kind::Slash, rest));
    }
    if let Some(rest) = input.strip_prefix('@') {
        return Some((Kind::Agent, rest.rsplit('+').next().unwrap_or(rest)));
    }
    None
}

/// Sync the palette to the input after an edit: open on a leading `/`/`@`
/// token, refilter as it narrows, close when it ends or nothing matches.
pub(crate) fn after_edit(
    palette: &mut Option<PaletteState>,
    input: &str,
    agents: &[AgentInfo],
) {
    let Some((kind, query)) = pending_palette(input) else {
        *palette = None;
        return;
    };
    let items = match kind {
        Kind::Slash => slash_items(query),
        Kind::Agent => agent_items(query, agents),
    };
    if items.is_empty() {
        *palette = None;
        return;
    }
    match palette {
        Some(p) if p.kind == kind => {
            p.sel = p.sel.min(items.len() - 1);
            p.items = items;
        }
        _ => *palette = Some(PaletteState { kind, items, sel: 0 }),
    }
}

fn slash_items(query: &str) -> Vec<MenuItem> {
    CONSTRUCTS
        .iter()
        .filter(|c| c[1..].starts_with(query))
        .map(|c| MenuItem {
            label: c.to_string(),
            desc: describe(c).to_string(),
            fill: c.to_string(),
            submit: false,
        })
        .collect()
}

fn agent_items(query: &str, agents: &[AgentInfo]) -> Vec<MenuItem> {
    let q = query.to_lowercase();
    agents
        .iter()
        .filter(|a| a.name.to_lowercase().starts_with(&q))
        .map(|a| MenuItem {
            label: format!("@{}", a.name),
            desc: a.role.clone(),
            fill: a.name.clone(),
            submit: false,
        })
        .collect()
}

/// Popup-first key routing: arrows move, Tab/Enter accept, Esc closes the
/// popup (not the pane).
pub(crate) fn popup_key(
    palette: &mut Option<PaletteState>,
    input: &mut String,
    key: &ChatInput,
) -> PaletteKey {
    let Some(p) = palette else {
        return PaletteKey::Forward;
    };
    match key {
        ChatInput::Up => p.sel = p.sel.saturating_sub(1),
        ChatInput::Down => p.sel = (p.sel + 1).min(p.items.len().saturating_sub(1)),
        ChatInput::Complete | ChatInput::Enter => {
            if let Some(item) = p.items.get(p.sel) {
                *input = accept(input, p.kind, &item.fill);
            }
            *palette = None;
        }
        ChatInput::Close => *palette = None,
        _ => return PaletteKey::Forward,
    }
    PaletteKey::Consumed
}

/// Replace the leading token's active segment with `fill`: a slash construct
/// becomes `/cmd `; an agent becomes `@name `, preserving any `@a+` prefix.
pub(crate) fn accept(input: &str, kind: Kind, fill: &str) -> String {
    match kind {
        Kind::Slash => format!("{fill} "),
        Kind::Agent => match input.rfind('+') {
            Some(plus) => format!("{}{fill} ", &input[..=plus]),
            None => format!("@{fill} "),
        },
    }
}
```

Add to `chatcomplete.rs` (after `CONSTRUCTS`):

```rust
/// One-line description for each construct, shown as the dim hint in the
/// composer's slash palette. Falls back to "" for anything unlisted.
pub(crate) fn describe(construct: &str) -> &'static str {
    match construct {
        "/help" => "list the constructs",
        "/agents" => "show the crew roster",
        "/model" => "set an agent's model",
        "/fan" => "fan a task out to every agent",
        "/loop" => "run a task on a loop",
        "/goal" => "set the crew's shared goal",
        "/plan" => "draft a plan for approval",
        "/approve" => "approve the drafted plan",
        "/reject" => "reject the drafted plan",
        "/checkpoint" => "snapshot the session",
        "/checkpoints" => "list checkpoints",
        "/restore" => "restore a checkpoint",
        "/skills" => "list available skills",
        "/skill" => "run a skill",
        "/mcp" => "list MCP servers and tools",
        "/stop" => "stop the running task",
        "/status" => "show session status",
        "/export" => "export the transcript",
        _ => "",
    }
}
```

- [ ] **Step 4 — run, expect green** (`cargo test -p crew-app chatpalette`).
- [ ] **Step 5 — commit** (`feat(crew-app): leading-token palette logic for the composer`).

---

### Task 2: wire the palette into `chat.rs`

**Files:** Modify `crates/crew-app/src/chat.rs` (the `ChatPane` struct + `on_key`), tests in `crates/crew-app/src/chat_tests.rs`.

**Consumes:** Task 1's `chatpalette::{PaletteState, popup_key, after_edit, PaletteKey}`.

- [ ] **Step 1 — failing test** in `chat_tests.rs` (mirror an existing `on_key` test for shape; a ChatPane constructor + a synthetic KeyEvent helper already exist there — reuse them):
  - Typing `/` opens the palette (`pane.palette.is_some()`), then a `Close` key returns `None` (NOT `Some(ChatAction::Close)`) and clears the palette.
  - If constructing `KeyEvent`s in a unit test is heavy, instead test at the `chatpalette` boundary here and cover the on_key routing with a focused test that calls the same `popup_key`/`after_edit` the handler calls; note in the report which was used.

- [ ] **Step 2 — run, expect fail.**
- [ ] **Step 3 — implement:**
  - Add field `pub(crate) palette: Option<crate::chatpalette::PaletteState>` to `ChatPane`, init `None` in its constructor(s).
  - In `on_key`, BEFORE the existing `chatmention::popup_key` block, add:
    ```rust
    if matches!(
        crate::chatpalette::popup_key(&mut self.palette, &mut self.input, &k),
        crate::chatpalette::PaletteKey::Consumed
    ) {
        return None;
    }
    ```
  - In the edit branch (where `chatmention::after_edit` is called), also call:
    ```rust
    crate::chatpalette::after_edit(&mut self.palette, &self.input, &self.agents);
    ```
- [ ] **Step 4 — run green** (`cargo test -p crew-app chat`).
- [ ] **Step 5 — commit** (`feat(crew-app): route composer palette keys in the chat pane`).

---

### Task 3: render the palette card + full verification

**Files:** Modify `crates/crew-app/src/render.rs` (near the existing `c.mention` "files" card block, ~lines 187-227), tests in `crates/crew-app/src/render_tests.rs` if one exists for this file (else a focused assertion in an existing render test module).

**Consumes:** Task 1 `Kind`/`PaletteState`; the existing `cmdmenu::menu_card`/`menu_rows` and placement math used for the "files" card.

- [ ] **Step 1 — failing test:** with a `ChatPane` whose `palette` is `Some(Slash …)`, the render output contains a card titled `commands` and a row label like `/model`; for `Agent`, titled `agents`. (Follow the existing mention-card render test's harness; if none, assert on the produced cells' characters.)
- [ ] **Step 2 — run, expect fail.**
- [ ] **Step 3 — implement:** mirror the `c.mention` render block: when `c.palette` is `Some(p)`, build items from `p.items` (label + desc), title `match p.kind { Slash => "commands", Agent => "agents" }`, and place the card above the composer with the same `menu_rows`/anchor math. Selected row = `p.sel`.
- [ ] **Step 4 — verify:** `cargo test -p crew-app` green; `cargo test --workspace` green; `cargo clippy -p crew-app -- -D warnings` clean; `cargo fmt`.
- [ ] **Step 5 — commit** (`feat(crew-app): render the composer slash/@agent palette card`).
