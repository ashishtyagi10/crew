# Footer Model Segment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Footer line 1 of the agent smith pane leads with the model actually serving the swarm (`qwen3-coder-plus · 5 agents`) instead of hiding it whenever CLI agents sit on the roster.

**Architecture:** GUI-only change in `crates/crew-app/src/chatsummary.rs`: `roster_seg` adopts the `/model` picker's consensus semantics by calling the existing `chatpalette::shared_model` (ignores empty models, fails only on real disagreement) instead of its own all-agents-must-report loop. No broker, protocol, header, or legend changes — the data already rides the `Roster` event.

**Tech Stack:** Rust, cargo test. Spec: `docs/superpowers/specs/2026-07-31-footer-model-segment-design.md`.

## Global Constraints

- Work on feature branch `feat/footer-model-segment` in the main checkout (no worktree), merged back with a local no-ff merge, branch deleted after.
- TDD with a RED transcript: the new test must be shown failing (actual cargo output, with the wrong string it produced) before the implementation lands.
- `·` is U+00B7. The model+count text is ONE segment string so `budget()` drops it atomically.
- No new colors, no priority changes (`P_ROSTER = 0` stays), no changes to footer lines 2/3.
- Release flow (user-authorized): bump root `Cargo.toml` version 0.10.0 → 0.10.1, commit on main, tag `v0.10.1`, push main + tag. CI builds the release; NEVER build a release locally (disk).

---

### Task 1: `roster_seg` leads with the shared model

**Files:**
- Modify: `crates/crew-app/src/chatsummary.rs:48-79` (`roster_seg` + its doc comment)
- Test: `crates/crew-app/src/chatsummary_tests.rs`

**Interfaces:**
- Consumes: `crate::chatpalette::shared_model(&[AgentInfo]) -> Option<String>` (exists, `pub(crate)`, chatpalette.rs:200); `short_model(&str) -> &str` (chatsummary.rs:24, unchanged).
- Produces: `roster_seg(&[AgentInfo]) -> Option<String>` (private; only `footer_lines` calls it — signature unchanged).

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-app/src/chatsummary_tests.rs` (after `a_crowded_roster_falls_back_to_a_count`, matching the file's `agent(name, model)` / `text(&footer_lines(...)[0])` helpers):

```rust
/// API specialists share one model; CLI agents (empty model) ride along. The
/// model is the identity the user asked the line for — the count still says
/// how many hands are on deck.
#[test]
fn a_mixed_cli_and_api_roster_leads_with_the_shared_model() {
    let agents = [
        agent("planner", "qwen/qwen3-coder-plus"),
        agent("coder", "qwen/qwen3-coder-plus"),
        agent("claude", ""),
    ];
    let line = text(&footer_lines(&fc(&agents, &HashMap::new()), 120)[0]);
    assert!(
        line.starts_with("qwen3-coder-plus \u{00b7} 3 agents | "),
        "{line}"
    );
}

/// A large all-API roster also shows the model, not just a count.
#[test]
fn a_crowded_agreeing_roster_keeps_the_model_with_its_count() {
    let agents = [
        agent("a", "m1"),
        agent("b", "m1"),
        agent("c", "m1"),
        agent("d", "m1"),
    ];
    let line = text(&footer_lines(&fc(&agents, &HashMap::new()), 120)[0]);
    assert!(line.starts_with("m1 \u{00b7} 4 agents | "), "{line}");
}
```

- [ ] **Step 2: Run tests to verify they fail (capture RED transcript)**

Run: `cargo test -p crew-app a_mixed_cli_and_api_roster_leads_with_the_shared_model a_crowded_agreeing_roster_keeps_the_model_with_its_count 2>&1 | tail -20` — actually run them separately or with a shared substring filter: `cargo test -p crew-app roster -- --nocapture`.
Expected: BOTH new tests FAIL, each assertion message printing the actual line produced today (`planner·coder·claude | …` and `4 agents | …`). Record the printed wrong strings — numbers/strings, not verdicts.

- [ ] **Step 3: Implement**

Replace `roster_seg` (chatsummary.rs:48-79) — doc comment and body — with:

```rust
/// Who is on the roster, as line 1's leading segment. This is the whole of
/// what `/agents` used to report, minus having to ask for it — which is why
/// that construct no longer exists.
///
/// The model leads whenever one model is the answer: `shared_model` ignores
/// CLI agents (`claude`, `codex`, `opencode` report no model — the CLI
/// chooses), so specialists agreeing on `qwen3-coder-plus` show that model
/// even with CLIs on the roster, plus the count that says how many hands are
/// on deck. Only when models genuinely disagree — or nobody reports one — are
/// names the honest answer, and past three even names stop fitting and stop
/// informing; the count does both.
fn roster_seg(agents: &[AgentInfo]) -> Option<String> {
    if agents.is_empty() {
        return None;
    }
    if let Some(m) = crate::chatpalette::shared_model(agents) {
        let m = short_model(&m);
        return Some(if agents.len() == 1 {
            m.to_string()
        } else {
            format!("{m} \u{00b7} {} agents", agents.len())
        });
    }
    let names: Vec<&str> = agents.iter().map(|a| a.name.as_str()).collect();
    if names.len() > 3 {
        return Some(format!("{} agents", names.len()));
    }
    Some(names.join("\u{00b7}"))
}
```

(The old consensus loop and the `models` Vec are deleted; `short_model` keeps its other caller-free position above.)

- [ ] **Step 4: Run the full crate's tests to verify green**

Run: `cargo test -p crew-app 2>&1 | tail -5`
Expected: PASS, 0 failed. Pre-existing roster tests (`a_mixed_roster_names_its_agents`, `a_modelless_cli_roster_shows_names_not_a_gap`, `a_single_modelled_agent_still_shows_its_model`, `a_crowded_roster_falls_back_to_a_count`, `an_empty_roster_contributes_nothing`, `the_roster_is_the_last_thing_to_go`) all still pass unchanged — they use single-agent or no-consensus rosters.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/chatsummary.rs crates/crew-app/src/chatsummary_tests.rs
git commit -m "feat(footer): lead line 1 with the shared model even when CLI agents ride along"
```

---

### Task 2: Merge and release v0.10.1

**Files:**
- Modify: `Cargo.toml` (root; `version = "0.10.0"` → `"0.10.1"`)

**Interfaces:**
- Consumes: Task 1 merged commits on `feat/footer-model-segment`.
- Produces: tag `v0.10.1` pushed; CI builds the release; user updates in-app via `/update`.

- [ ] **Step 1: Merge the branch (no-ff) and delete it**

```bash
git checkout main
git merge --no-ff feat/footer-model-segment -m "Merge feat/footer-model-segment: footer line 1 shows the serving model"
git branch -d feat/footer-model-segment
```

- [ ] **Step 2: Verify main is green**

Run: `cargo test -p crew-app 2>&1 | tail -3`
Expected: PASS, 0 failed.

- [ ] **Step 3: Bump version and tag**

Edit root `Cargo.toml`: `version = "0.10.1"`. Then:

```bash
cargo check --quiet   # refresh Cargo.lock with the new version
git add Cargo.toml Cargo.lock
git commit -m "v0.10.1: footer shows the serving model"
git tag v0.10.1
```

- [ ] **Step 4: Push main and the tag**

```bash
git push origin main v0.10.1
```

Expected: both refs accepted; CI release workflow starts. Do NOT run a local release build.

- [ ] **Step 5: Update the goal memory**

Mark `project-smith-model-display-goal` complete (goal file + MEMORY.md index line): shipped in v0.10.1 via `roster_seg` → `shared_model`.
