# Active Agents on the Footer Mode Line — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the always-on `@agent` roster strip from the composer's top border and instead show the names of actively-working agents on footer line 3 (the `▶▶` mode line), only while they work.

**Architecture:** Two pure render modules change. `chatinput.rs` loses `chips_on_border` (the composer border becomes a plain card border; the `agents` param stays for `@mention` colouring). `chatsummary.rs` gains a `FooterCtx.active` field fed from `pane.active_names()` — `Activity` events already carry real agent names for both relay hops and swarm runs, so no broker/protocol changes. Both modules are pure and fully unit-testable.

**Tech Stack:** Rust workspace, crate `crew-app`. Tests are plain `#[test]` fns in sibling `*_tests.rs` files. Spec: `docs/superpowers/specs/2026-08-01-active-agents-footer-design.md`.

## Global Constraints

- Work on a feature branch off `main` in the main checkout (no worktree): `feat/active-agents-footer`.
- Every test must be seen RED before its implementation lands (capture the failing run's output in the task report — the assertion message, not just "it failed").
- Cap rule everywhere: past 3 names, show a count (matches `roster_seg` / `running_seg`).
- Idle line 3 must be byte-identical to today's output: `▶▶ swarm mode · / for constructs · @ to relay to an agent`.
- The `agents` parameter of `composer_cells` is NOT removed — it still drives mention colouring and `relay_target`.
- Commit style: repo-conventional `feat:`/`test:` one-liners; pre-commit runs fmt + check.

---

### Task 1: Composer — remove the roster strip from the border

**Files:**
- Modify: `crates/crew-app/src/chatinput.rs` (delete `chips_on_border` at ~line 189, simplify `badge_on_border` ~line 238, update `composer_cells` ~line 256, rewrite module doc lines 1–5)
- Test: `crates/crew-app/src/chatinput_tests.rs`

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces: `composer_cells(input: &str, ghost: Option<&str>, agents: &[AgentInfo], cols: u16, rows: u16) -> Vec<CellView>` — signature UNCHANGED (caller `chatview.rs:197` untouched). `badge_on_border(cells: &mut Vec<CellView>, badge: &str, cols: u16, row: u16)` — `chips_end` parameter removed.

- [ ] **Step 1: Make the border test assert the strip is gone**

In `chatinput_tests.rs`, edit `tall_pane_gets_a_bordered_card` (line 29): replace the chips assertion with its negation, and assert the border is otherwise intact:

```rust
#[test]
fn tall_pane_gets_a_bordered_card() {
    let cells = composer_cells("hi", None, &agents(&["planner", "coder"]), 80, 10);
    // Top border (row 7): a plain rounded border — the roster strip is gone
    // (the footer's mode line now names who is WORKING; the full roster
    // lives in the @ picker and footer line 1).
    let top = row_text(&cells, 7);
    assert!(top.starts_with('\u{256d}'), "top: {top}"); // ╭
    assert!(top.ends_with('\u{256e}'), "top: {top}"); // ╮
    assert!(
        !top.contains('@'),
        "roster chips must not ride the border: {top}"
    );
    // Interior (row 8): side borders around the prompt.
    let mid = row_text(&cells, 8);
    assert!(mid.starts_with("\u{2502} \u{276f} hi"), "mid: {mid}"); // │ ❯ hi
    assert!(mid.ends_with('\u{2502}'), "mid: {mid}"); // │
    // Bottom border (row 9): a plain fieldset edge, no key hints.
    let bot = row_text(&cells, 9);
    assert!(bot.starts_with('\u{2570}'), "bot: {bot}"); // ╰
    assert!(bot.ends_with('\u{256f}'), "bot: {bot}"); // ╯
    assert!(!bot.contains("Esc close"), "bot: {bot}");
}
```

Note the border row must still be a CONTIGUOUS `─` run between the corners once chips are gone — `row_text` preserves gaps, so also add this line after the corner asserts:

```rust
    assert!(
        top.chars().skip(1).take(top.chars().count() - 2).all(|c| c == '\u{2500}'),
        "border must be solid without the chips: {top}"
    );
```

- [ ] **Step 2: Run it — must FAIL on the chips**

Run: `cargo test -p crew-app tall_pane_gets_a_bordered_card`
Expected: FAIL with `roster chips must not ride the border: ╭─ @planner @coder ─...`. Capture this output.

- [ ] **Step 3: Remove the strip**

In `chatinput.rs`:

1. Delete the whole `chips_on_border` function (lines ~185–228) including its doc comment.
2. Change `badge_on_border` to lose the `chips_end` parameter — the only left-hand obstacle now is the corner:

```rust
/// Right-align the char-count badge on the card's top border, clear of the
/// left corner and the right corner; skipped entirely if the row is too
/// narrow to fit it without touching either.
fn badge_on_border(cells: &mut Vec<CellView>, badge: &str, cols: u16, row: u16) {
    let muted = crew_theme::theme().text_muted;
    let w = badge.chars().count() as u16;
    if cols < 3 + w {
        return;
    }
    let start = cols - 2 - w;
    if start <= 1 {
        return;
    }
    for (x, c) in (start..).zip(badge.chars()) {
        cells.push(cell(x, row, c, muted, false));
    }
}
```

3. In `composer_cells`, replace

```rust
    let chips_end = chips_on_border(&mut cells, agents, cols, top);
    if let Some(badge) = char_count_badge(input.chars().count()) {
        badge_on_border(&mut cells, &badge, chips_end, cols, top);
    }
```

with

```rust
    if let Some(badge) = char_count_badge(input.chars().count()) {
        badge_on_border(&mut cells, &badge, cols, top);
    }
```

4. Rewrite the module doc (lines 1–5) to describe the new shape:

```rust
//! The crew pane's input composer. Tall panes get a bordered fieldset card
//! with the `❯` prompt on the interior row (a valid leading `@agent` mention
//! takes that agent's roster colour); who is WORKING is the footer mode
//! line's job (`chatsummary`), not the border's. Short panes fall back to a
//! single bare prompt row.
```

- [ ] **Step 4: Run the module's tests — all green**

Run: `cargo test -p crew-app chatinput`
Expected: PASS, including `long_input_shows_a_muted_char_count_badge_on_the_top_border` (badge still right-aligned) and `valid_mention_is_highlighted_in_agent_colour` (mention colouring untouched).

- [ ] **Step 5: Commit**

```bash
git checkout -b feat/active-agents-footer
git add crates/crew-app/src/chatinput.rs crates/crew-app/src/chatinput_tests.rs
git commit -m "feat(app): drop the roster strip from the composer border"
```

---

### Task 2: Footer — active agent names on line 3

**Files:**
- Modify: `crates/crew-app/src/chatsummary.rs` (`FooterCtx` at ~line 78, `footer_lines` line-3 block at ~lines 287–316, `footer_ctx` at ~line 328, module doc lines 1–8)
- Test: `crates/crew-app/src/chatsummary_tests.rs` (helper `fc` at line 27 + new tests)

**Interfaces:**
- Consumes: `crate::chatroster::agent_color(name: &str) -> (u8, u8, u8)` (exists); `ChatPane::active_names(&self) -> Vec<&str>` (exists, `chatflow.rs:175`).
- Produces: `FooterCtx` gains field `pub active: Vec<&'a str>` — names of agents currently thinking, render-ordered. Task 3 relies on nothing new beyond this.

- [ ] **Step 1: Extend the test helper and write the failing tests**

In `chatsummary_tests.rs`, add `active: Vec::new(),` to the `FooterCtx` literal in the `fc` helper (after `plan_pending: false,`). Then append these tests at the end of the file:

```rust
/// Who is working right now, by name, in the same roster colour that names
/// them everywhere else. This is the information the composer's border strip
/// used to gesture at with the ENTIRE roster.
#[test]
fn active_agents_show_on_line3_in_their_roster_colours() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.active = vec!["analyst", "coder"];
    f.running_tasks = &[3];
    let l3 = &footer_lines(&f, 120)[2];
    let s = text(l3);
    assert!(
        s.contains("@analyst \u{00b7} @coder"),
        "names missing or misordered: {s}"
    );
    assert!(s.contains("running #3"), "{s}");
    // Mode leads, names follow, work ids after.
    let (m, a) = (s.find("swarm mode").unwrap(), s.find("@analyst").unwrap());
    assert!(m < a && a < s.find("running #3").unwrap(), "{s}");
    // Each name renders in ITS agent colour — the same hash-picked colour
    // the chip grid and message cards use.
    let chars: Vec<char> = l3.iter().map(|(c, _)| *c).collect();
    for name in ["analyst", "coder"] {
        let chip: Vec<char> = format!("@{name}").chars().collect();
        let at = (0..chars.len())
            .find(|&i| chars[i..].starts_with(&chip))
            .unwrap();
        let want = crate::chatroster::agent_color(name);
        for j in at..at + chip.len() {
            assert_eq!(l3[j].1, want, "@{name} char {j} off-colour in: {s}");
        }
    }
}

/// Past a few, names stop fitting and stop informing — same rule as the
/// roster segment and the running-task ids.
#[test]
fn a_crowd_of_active_agents_collapses_to_a_count() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.active = vec!["a", "b", "c", "d"];
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert!(l3.contains("4 agents working"), "{l3}");
    assert!(!l3.contains('@'), "names leaked past the cap: {l3}");
}

/// Nobody working → the line is BYTE-IDENTICAL to today's idle line. The
/// segment must be absent, not empty.
#[test]
fn an_idle_line3_is_unchanged_by_the_active_segment() {
    let empty_ctx = HashMap::new();
    let f = fc(&[], &empty_ctx);
    assert_eq!(
        text(&footer_lines(&f, 120)[2]),
        "\u{25b6}\u{25b6} swarm mode \u{00b7} / for constructs \u{00b7} @ to relay to an agent"
    );
}

/// On a narrow pane the names go and the line keeps its identity and the
/// work ids — and it always FITS.
#[test]
fn a_narrow_pane_drops_the_names_before_the_mode() {
    let empty_ctx = HashMap::new();
    for cols in [24usize, 30, 40, 60, 80, 120] {
        let mut f = fc(&[], &empty_ctx);
        f.active = vec!["analyst", "coder"];
        f.running_tasks = &[3];
        let l3 = text(&footer_lines(&f, cols)[2]);
        assert!(l3.chars().count() <= cols, "{cols}: {l3}");
        // The work ids are the last thing standing; the mode outlasts the
        // names but yields to the ids on the tightest panes (24 cols fits
        // `running #3` alone, not `▶▶ swarm mode · running #3` at 26 wide).
        assert!(l3.contains("#3"), "work ids lost at {cols}: {l3}");
        if cols >= 30 {
            assert!(l3.contains("swarm mode"), "mode lost at {cols}: {l3}");
        }
        if cols >= 120 {
            assert!(l3.contains("@analyst"), "{cols}: {l3}");
        }
        if cols <= 30 {
            assert!(!l3.contains("@analyst"), "names survived {cols}: {l3}");
        }
    }
}

/// A pending plan still owns the line; the names ride along when they fit.
#[test]
fn active_names_coexist_with_a_pending_plan() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.active = vec!["coder"];
    f.plan_pending = true;
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert!(l3.contains("plan ready"), "{l3}");
    assert!(l3.contains("@coder"), "{l3}");
}
```

- [ ] **Step 2: Run them — compile error first, then RED**

Run: `cargo test -p crew-app chatsummary`
Expected: compile FAILS — `FooterCtx` has no field `active`. This is the RED for the struct change. (After Step 3's struct-only change, the four new behaviour tests must FAIL on assertions — e.g. `names missing or misordered: ▶▶ swarm mode · running #3 …`. Capture that output too.)

- [ ] **Step 3: Add the field and the wiring**

In `chatsummary.rs`:

1. `FooterCtx` (after `plan_pending`):

```rust
    /// Names of agents thinking RIGHT NOW (`ChatPane::active_names`) —
    /// `Activity` events name agents for both relay hops and swarm tasks,
    /// so this one field covers both. Empty when idle.
    pub active: Vec<&'a str>,
```

2. `footer_ctx` (the builder at ~line 328): add `active: pane.active_names(),`.

Run `cargo test -p crew-app chatsummary` again: everything compiles; the four new behaviour tests FAIL (the idle-identity test passes — it asserts today's output). Capture the failing assertions.

- [ ] **Step 4: Render the segment**

In `footer_lines`, immediately after `let mut l3s: Vec<(Seg, u8)> = vec![((mode, yellow), 1)];` (~line 295), insert:

```rust
    // Who is working right now, each name in its roster colour so it matches
    // the chip grid and message cards; past three names the count is the
    // information. Priority 2: the trailing hints and the `/stop` how drop
    // first (ties break toward the right), then the names — the mode (1) and
    // the plan/running segments (0) always outlast them.
    match fc.active.as_slice() {
        [] => {}
        names if names.len() > 3 => {
            l3s.push(((format!("{} agents working", names.len()), green), 2));
        }
        names => {
            for n in names {
                l3s.push(((format!("@{n}"), crate::chatroster::agent_color(n)), 2));
            }
        }
    }
```

Update the module doc's line-3 sentence (line 5) from "the live routing mode (swarm vs. `@agent` relay) followed by the hints" to "the live routing mode (swarm vs. `@agent` relay), the agents working right now, and the running-task/hint tail".

- [ ] **Step 5: Run the whole crate's tests — all green**

Run: `cargo test -p crew-app`
Expected: PASS — the five new tests plus all existing ones, especially `line3_fits_and_never_teaches_half_an_answer`, `a_pending_plan_owns_line3`, `running_tasks_replace_the_hints_on_line3` (which must be untouched by an empty `active`).

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/chatsummary.rs crates/crew-app/src/chatsummary_tests.rs
git commit -m "feat(app): footer mode line names the agents working right now"
```

---

### Task 3: Live verification and merge

**Files:**
- No source changes expected; fixes discovered here loop back into Tasks 1–2's files.

**Interfaces:**
- Consumes: the built app with Tasks 1–2 merged on the branch.
- Produces: a verified branch ready for the no-ff merge to `main`.

- [ ] **Step 1: Full workspace check**

Run: `cargo test -p crew-app && cargo fmt --all -- --check && cargo clippy -p crew-app -- -D warnings`
Expected: all clean. (Pre-commit runs fmt/check, but clippy is not in the hook.)

- [ ] **Step 2: Live verify**

Use the repo `verify` skill (isolated-HOME dev instance, frontmost-PID guard — recipe in `.claude/skills/verify`). Confirm with screenshots:
1. Composer top border is a plain `╭───╮` — no `@agent` chips (the circled strip in the user's report is gone).
2. Submit a task (mock broker `CREW_BROKER_MOCK_REPLY` is fine for relay; a real swarm run if the environment allows): while agents work, footer line 3 reads `▶▶ swarm mode · @<name> …` with the name coloured; after settle, line 3 returns to the idle hints.
3. `@mention` typed in the composer still colours the mention (roster colouring path intact).

- [ ] **Step 3: Merge per repo flow**

Local no-ff merge to `main` and delete the branch (repo convention), then offer the user a push:

```bash
git checkout main
git merge --no-ff feat/active-agents-footer -m "Merge feat/active-agents-footer: footer names the working agents, composer border sheds the roster strip"
git branch -d feat/active-agents-footer
```

---

## Self-Review Notes

- **Spec coverage:** composer strip removal → Task 1; footer segment, source, colours, cap, idle identity, budget priority → Task 2; header untouched → no task touches `chathdr`/`chatflow`; live verify → Task 3. The spec's "badge collision guard collapses to the corner" → Task 1 Step 3.2.
- **RED transcripts required:** Steps 1–2 of each coding task produce a captured failing run before implementation (per standing feedback that plan-written tests are often vacuous — reviewers should ask for the failing output, not a verdict).
- **Type consistency:** `active: Vec<&'a str>` matches `active_names() -> Vec<&str>` borrowed from the pane; `agent_color(&str) -> (u8, u8, u8)` matches `Fg`.
