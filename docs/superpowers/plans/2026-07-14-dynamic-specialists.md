# Dynamic Specialists Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete the hard-coded `planner`/`coder`/`reviewer` roster; the swarm planner invents a named specialist per task, and those specialists appear in the roster, light up while working, persist in a project-local store, and are `@`-dial-able.

**Architecture:** The planner returns `specialty` (an `@`-handle, strictly slugged) and `expertise` (a prose role hint) per task. A project-local JSON store (`./.crew/specialists.json`) is both the persistence and the accumulation mechanism — `Registry::discover_with` rebuilds from scratch on every call, so re-reading a file per rebuild avoids threading new mutable state through `Session`. The roster UI gets a row budget because it silently assumes ~3 agents today.

**Tech Stack:** Rust 2021, workspace crates `crew-hive` (planner/graph), `crew-plugin` (broker), `crew-app` (winit GUI). serde/serde_json. Tests are inline `#[cfg(test)]` modules or `#[path]`-included `*_tests.rs` siblings.

**Spec:** `docs/superpowers/specs/2026-07-14-dynamic-specialists-design.md` — read it before starting. The prompt text in Task 2 is spike-validated; do not "improve" it (the spec's Prompt spike section explains which clause defends against which observed failure).

## Global Constraints

- Name guard: `^[a-z0-9-]{2,28}$`. Truncation is a **hard cut**, never at a hyphen boundary (boundary truncation turns `accommodation-specialist` into the topic-noun `accommodation`, the exact failure the prompt rule removes).
- Role/expertise clamp: 60 chars, whitespace collapsed, control chars dropped. Empty is a valid outcome (`""`).
- Store cap: `CAP = 24`, LRU by `last_used`. Merge by name — never suffix.
- Roster display cap: `ROSTER_MAX_ROWS = 5`. The dial-able set (24) and displayed set (5) are different numbers on purpose.
- Store path is project-local: `./.crew/specialists.json`. Follow the `sessionlog.rs` house pattern — a `*_at(base: &Path)` function with a thin wrapper passing `Path::new(".")`, so tests use a temp dir.
- Store writes are atomic (tmp + rename) and **best-effort**: a write failure is logged and ignored, never fails a run.
- The security invariant in `parse_plan` (every task forced to `AgentKind::Api`) must survive unchanged, with its `debug_assert!` intact.
- Every file stays under the repo's 200-line-per-module convention where practical; `swarmmsg.rs` exists because `swarm.rs` hit that cap.
- Run `cargo fmt` before every commit — a pre-commit hook enforces fmt + `cargo check` and will reject the commit otherwise.

---

### Task 1: `agentname` — the one legal-name authority

**Files:**
- Create: `crates/crew-hive/src/agentname.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod agentname;` after `pub mod agent;` at line 38, and a `pub use`)

**Interfaces:**
- Consumes: nothing.
- Produces: `crew_hive::agentname::{slug, slug_or, role_clamp}` —
  - `pub fn slug(raw: &str) -> Option<String>`
  - `pub fn slug_or(raw: &str, id: u64) -> String`
  - `pub fn role_clamp(raw: &str) -> String`

Tasks 2, 5 and the `plugins.rs` change in Task 5 all depend on these exact names.

- [ ] **Step 1: Write the failing test**

Create `crates/crew-hive/src/agentname.rs` with only the test module and stub signatures:

```rust
//! The one answer to "what is a legal agent name". Agent names became
//! LLM-authored with dynamic specialists, and every consumer already assumes
//! a strict charset without enforcing it: `relay.rs` terminates a name at
//! whitespace and reserves `+` as its multi-target separator, `chatcomplete`
//! bails on whitespace, and `stdio` routes on a leading `/`. Slugging at the
//! parse boundary makes those assumptions true.

/// Longest name kept. Empirical: a prompt spike on qwen-max produced
/// `user-experience-specialist` (26), and ~1 name in 6 exceeds 20, so a
/// tighter ceiling would mangle ordinary output. See the design doc.
const MAX: usize = 28;
/// Shortest name worth addressing.
const MIN: usize = 2;
/// Longest role hint kept, in chars.
const ROLE_MAX: usize = 60;

pub fn slug(_raw: &str) -> Option<String> {
    unimplemented!()
}

pub fn slug_or(_raw: &str, _id: u64) -> String {
    unimplemented!()
}

pub fn role_clamp(_raw: &str) -> String {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_and_hyphenates_whitespace() {
        assert_eq!(slug("Risk Assessor").as_deref(), Some("risk-assessor"));
        assert_eq!(slug("  Archivist  ").as_deref(), Some("archivist"));
    }

    #[test]
    fn strips_chars_that_break_the_at_tokenizers() {
        // `@` would double-parse, `+` is relay.rs's multi-target separator,
        // `/` collides with construct routing in stdio.rs.
        assert_eq!(slug("@archivist").as_deref(), Some("archivist"));
        assert_eq!(slug("data+ops").as_deref(), Some("data-ops"));
        assert_eq!(slug("sec/ops").as_deref(), Some("sec-ops"));
        assert_eq!(slug("The Skeptic!").as_deref(), Some("the-skeptic"));
    }

    #[test]
    fn collapses_and_trims_hyphens() {
        assert_eq!(slug("a---b").as_deref(), Some("a-b"));
        assert_eq!(slug("--edge--").as_deref(), Some("edge"));
    }

    #[test]
    fn rejects_what_cannot_be_salvaged() {
        assert_eq!(slug(""), None);
        assert_eq!(slug("@#$"), None);
        assert_eq!(slug("x"), None, "one char is below the floor");
        assert_eq!(slug("---"), None);
    }

    #[test]
    fn non_ascii_is_dropped_not_transliterated() {
        // chips_on_border measures with byte length, which is only correct
        // because the charset is ASCII.
        assert_eq!(slug("café-critic").as_deref(), Some("caf-critic"));
        assert_eq!(slug("日本語"), None);
    }

    #[test]
    fn over_length_is_hard_cut_not_boundary_cut() {
        // A boundary cut would yield "accommodation" — a bare topic noun,
        // the exact failure the planner prompt exists to prevent. A hard cut
        // is obviously mangled instead of plausibly wrong.
        let long = "accommodation-specialist-for-travel";
        let got = slug(long).unwrap();
        assert_eq!(got.len(), 28);
        assert_eq!(got, "accommodation-specialist-for");
    }

    #[test]
    fn hard_cut_still_trims_a_trailing_hyphen() {
        let got = slug("abcdefghijklmnopqrstuvwxyz-ab").unwrap();
        assert!(!got.ends_with('-'), "got {got}");
    }

    #[test]
    fn slug_or_derives_from_id_when_unsalvageable() {
        assert_eq!(slug_or("@#$", 3), "specialist-3");
        assert_eq!(slug_or("Archivist", 3), "archivist");
    }

    #[test]
    fn role_clamp_collapses_whitespace_and_drops_controls() {
        assert_eq!(role_clamp("  records,\n retrieval  "), "records, retrieval");
        assert_eq!(role_clamp("a\u{7}b"), "ab");
        assert_eq!(role_clamp(""), "");
    }

    #[test]
    fn role_clamp_truncates_at_sixty_chars() {
        let got = role_clamp(&"x".repeat(100));
        assert_eq!(got.chars().count(), 60);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-hive agentname`
Expected: FAIL — `not implemented` panics from `unimplemented!()`.

- [ ] **Step 3: Implement**

Replace the three stub functions:

```rust
/// Normalize `raw` to `^[a-z0-9-]{2,28}$`, or `None` if nothing survives.
pub fn slug(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    for c in raw.trim().chars() {
        let mapped = if c.is_ascii_alphanumeric() {
            c.to_ascii_lowercase()
        } else if c.is_whitespace() || c == '-' || c == '_' || c == '+' || c == '/' {
            '-'
        } else {
            continue; // drop everything else, including non-ASCII
        };
        // Collapse runs of '-' as we go.
        if mapped == '-' && out.ends_with('-') {
            continue;
        }
        out.push(mapped);
    }
    // Hard cut, deliberately not at a '-' boundary (see the module docs).
    out.truncate(MAX);
    let trimmed = out.trim_matches('-');
    (trimmed.chars().count() >= MIN).then(|| trimmed.to_string())
}

/// [`slug`], falling back to a name derived from the task `id`.
pub fn slug_or(raw: &str, id: u64) -> String {
    slug(raw).unwrap_or_else(|| format!("specialist-{id}"))
}

/// Normalize a prose role hint: collapse whitespace, drop control chars,
/// clamp to 60 chars. `""` is a valid result — it's what `role_for` already
/// returns for unknown agents, so every consumer handles it.
pub fn role_clamp(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    cleaned.chars().take(ROLE_MAX).collect()
}
```

Note `out.truncate(MAX)` is safe because every retained char is single-byte ASCII, so `MAX` is both a byte and a char index.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p crew-hive agentname`
Expected: PASS, 10 tests.

- [ ] **Step 5: Export the module**

In `crates/crew-hive/src/lib.rs`, add after line 38 (`pub mod agent;`):

```rust
pub mod agentname;
```

and next to the other re-exports (near line 81's `pub use planner::{...}`):

```rust
pub use agentname::{role_clamp, slug, slug_or};
```

- [ ] **Step 6: Verify the crate builds**

Run: `cargo check -p crew-hive`
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
cargo fmt
git add crates/crew-hive/src/agentname.rs crates/crew-hive/src/lib.rs
git commit -m "feat(hive): agentname — one legal-name authority for agent slugs"
```

---

### Task 2: Plan schema carries the specialist

**Files:**
- Modify: `crates/crew-hive/src/graph/spec.rs:40-47` (`TaskSpec`)
- Modify: `crates/crew-hive/src/planner/mod.rs` (`StubPlanner::plan`, `PLANNER_SYSTEM`, `PlanNode`, `parse_plan`)
- Test: `crates/crew-hive/src/planner/tests.rs` (existing)

**Interfaces:**
- Consumes: `crew_hive::agentname::{slug_or, role_clamp}` from Task 1.
- Produces: `TaskSpec { id, title, agent, model, deps, prompt, specialty: String, expertise: String }`. Tasks 4, 6 and 8 read `TaskSpec::specialty` / `TaskSpec::expertise`.

- [ ] **Step 1: Add the fields to `TaskSpec`**

In `crates/crew-hive/src/graph/spec.rs`, replace the `TaskSpec` struct:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskSpec {
    pub id: TaskId,
    pub title: String,
    pub agent: AgentKind,
    pub model: ModelTier,
    pub deps: Vec<TaskId>,
    pub prompt: String,
    /// The `@`-handle of the specialist this task needs — always a valid
    /// `agentname::slug`. `#[serde(default)]` because `TaskSpec` crosses the
    /// broker↔app wire in `HivePlan`: a mismatched pair mid-upgrade must
    /// degrade, not fail to parse.
    #[serde(default)]
    pub specialty: String,
    /// That specialist's prose craft hint (`agentname::role_clamp`ed). May be
    /// empty.
    #[serde(default)]
    pub expertise: String,
}
```

- [ ] **Step 2: Run the build to find every constructor**

Run: `cargo check --workspace 2>&1 | grep "missing field"`
Expected: FAIL — errors at each `TaskSpec { .. }` literal. This list is the work for Step 3.

- [ ] **Step 3: Fix `StubPlanner`**

In `crates/crew-hive/src/planner/mod.rs`, in `StubPlanner::plan`, the two `TaskSpec` literals gain the fields. The stub reuses its titles as specialties: it is the deterministic offline path, and a fixed, obviously-synthetic name is clearer here than an invented one.

```rust
            let mut tasks: Vec<TaskSpec> = (0..fanout)
                .map(|i| TaskSpec {
                    id: TaskId(i as u64),
                    title: format!("leaf-{i}"),
                    agent: AgentKind::Api { system: None },
                    model: ModelTier::Standard,
                    deps: vec![],
                    prompt: goal.clone(),
                    specialty: format!("leaf-{i}"),
                    expertise: String::new(),
                })
                .collect();
            let merge = TaskSpec {
                id: TaskId(fanout as u64),
                title: "merge".into(),
                agent: AgentKind::Api { system: None },
                model: ModelTier::Standard,
                deps: (0..fanout).map(|i| TaskId(i as u64)).collect(),
                prompt: goal,
                specialty: "merge".into(),
                expertise: String::new(),
            };
```

Every other `TaskSpec` literal the compiler flagged (test fixtures across `crew-hive`, `crew-plugin`, `crew-app`) gets `specialty: String::new(), expertise: String::new()` unless a test asserts on them.

- [ ] **Step 4: Write the failing `parse_plan` tests**

Add to `crates/crew-hive/src/planner/tests.rs`:

```rust
#[test]
fn parse_plan_slugs_the_specialty() {
    let json = r#"[{"id":0,"title":"Gather Details","prompt":"p","deps":[],
                    "specialty":"Risk Assessor","expertise":"risk,  analysis"}]"#;
    let g = parse_plan(json).expect("valid plan");
    let t = &g.tasks()[0];
    assert_eq!(t.specialty, "risk-assessor");
    assert_eq!(t.expertise, "risk, analysis");
}

#[test]
fn parse_plan_derives_a_name_when_specialty_is_missing_or_garbage() {
    let missing = r#"[{"id":0,"title":"T","prompt":"p","deps":[]}]"#;
    assert_eq!(parse_plan(missing).unwrap().tasks()[0].specialty, "specialist-0");

    let garbage = r#"[{"id":7,"title":"T","prompt":"p","deps":[],"specialty":"@#$"}]"#;
    assert_eq!(parse_plan(garbage).unwrap().tasks()[0].specialty, "specialist-7");
}

#[test]
fn parse_plan_defaults_expertise_to_empty() {
    let json = r#"[{"id":0,"title":"T","prompt":"p","deps":[],"specialty":"analyst"}]"#;
    assert_eq!(parse_plan(json).unwrap().tasks()[0].expertise, "");
}

#[test]
fn parse_plan_every_specialty_is_a_valid_slug() {
    let json = r#"[{"id":0,"title":"T","prompt":"p","deps":[],"specialty":"A B/C+D"},
                   {"id":1,"title":"U","prompt":"p","deps":[],"specialty":""}]"#;
    for t in parse_plan(json).unwrap().tasks() {
        assert_eq!(
            crate::agentname::slug(&t.specialty).as_deref(),
            Some(t.specialty.as_str()),
            "specialty {:?} must be slug-stable",
            t.specialty
        );
    }
}
```

If `TaskGraph` exposes its tasks under a different accessor than `tasks()`, use whatever the existing tests in this file already use.

- [ ] **Step 5: Run to verify they fail**

Run: `cargo test -p crew-hive planner`
Expected: FAIL — `specialty` is empty, not slugged (nothing populates it yet).

- [ ] **Step 6: Wire `PlanNode` and `parse_plan`**

In `crates/crew-hive/src/planner/mod.rs`, replace `PlanNode` and the mapping inside `parse_plan`:

```rust
/// The shape we accept from model output. Deliberately has **no** `agent`,
/// `command`, or `args` field: the model describes *what* work to do and *who*
/// should do it, never *how* to execute it. serde ignores any such extra keys,
/// so an attacker-influenced completion cannot smuggle one in. See the
/// security note below.
#[derive(Deserialize)]
struct PlanNode {
    id: u64,
    title: String,
    prompt: String,
    deps: Vec<u64>,
    #[serde(default)]
    specialty: Option<String>,
    #[serde(default)]
    expertise: Option<String>,
}
```

and in the `.map(|n| TaskSpec { .. })`:

```rust
        .map(|n| TaskSpec {
            id: TaskId(n.id),
            title: n.title,
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: n.deps.into_iter().map(TaskId).collect(),
            prompt: n.prompt,
            specialty: crate::agentname::slug_or(n.specialty.as_deref().unwrap_or(""), n.id),
            expertise: crate::agentname::role_clamp(n.expertise.as_deref().unwrap_or("")),
        })
```

- [ ] **Step 7: Extend the security invariant's doc and assertion**

Still in `parse_plan`, extend the doc comment above it and add the companion assert next to the existing one:

```rust
/// SECURITY INVARIANT: the JSON here is untrusted (LLM output, ultimately
/// influenced by the goal and any tool/context content). Every task it yields
/// is forced to [`AgentKind::Api`] — model output can never select a
/// process-executing [`AgentKind::Pty`] agent. This is the trust boundary that
/// keeps a future Pty executor from becoming a command-injection sink; the
/// `debug_assert!` and `parse_plan_*` tests fail loudly if it ever regresses.
///
/// `specialty` and `expertise` are model-authored but inert: a display label,
/// an `@`-handle, and a role hint fed into the specialist's system prompt.
/// Neither selects an executor, so neither can reach that sink. `specialty` is
/// still slugged, because it becomes an addressable handle and the `@`
/// tokenizers assume `^[a-z0-9-]+$` without enforcing it.
```

```rust
    debug_assert!(
        !tasks.iter().any(|t| t.agent.is_pty()),
        "parse_plan must never yield a process-executing Pty task from model output",
    );
    debug_assert!(
        tasks
            .iter()
            .all(|t| crate::agentname::slug(&t.specialty).as_deref() == Some(t.specialty.as_str())),
        "parse_plan must never yield a specialty that isn't a stable slug",
    );
```

- [ ] **Step 8: Replace `PLANNER_SYSTEM`**

Replace the `PLANNER_SYSTEM` const. **This text is spike-validated — every clause defends against a specific observed failure. Read the design doc's "Prompt spike" section before changing a word of it.**

```rust
/// The planner's system prompt. Every clause here was earned against an
/// observed failure on a real provider (see the design doc's Prompt spike):
/// the agent-noun rule fixes bare topic words ("security"); the
/// anti-anchoring clause exists because naming flavourful examples turned
/// them into a word bank (one revision assigned "epidemiologist" to packing a
/// suitcase). There is deliberately no character limit here — the model
/// ignored one — so length is enforced in `agentname::slug` instead.
const PLANNER_SYSTEM: &str = "\
You are a task planner. Decompose the user's goal into a JSON array of tasks. \
Each task is an object with integer `id` (0-based), short `title`, a `prompt` \
describing the work, `deps` (array of task ids that must finish first), \
`specialty`, and `expertise`.\n\
\n\
`specialty` names the specialist the task needs. Rules:\n\
- At most TWO words, joined by a hyphen. Never three.\n\
- It must be a person, not a subject: an agent noun. Write \"security-auditor\", \
not \"security\"; \"risk-assessor\", not \"risk\".\n\
- Name them for their craft, not for the task: a task titled \"Gather Project \
Details\" needs an \"archivist\", not a \"gatherer\".\n\
- Use the word a real practitioner of that work would call themselves. Do not \
borrow vocabulary from these instructions — an example word that does not \
genuinely fit the goal at hand is worse than a plain one.\n\
\n\
`expertise` is a short comma-separated phrase naming that specialist's craft, \
e.g. \"records, retrieval, provenance\".\n\
\n\
Return ONLY the JSON array, no prose.";
```

- [ ] **Step 9: Run the full workspace suite**

Run: `cargo test --workspace 2>&1 | grep -E "^test result|^error"`
Expected: all PASS. The existing `parse_plan` Pty-forcing tests must still pass untouched.

- [ ] **Step 10: Commit**

```bash
cargo fmt
git add -A
git commit -m "feat(hive): planner invents a named specialist per task

specialty (a strict slug, an @-handle) and expertise (a prose craft hint)
travel with each TaskSpec. Both are serde(default) because TaskSpec crosses
the broker/app wire in HivePlan.

The security invariant is unchanged: AgentKind::Api is still forced, and the
new fields are inert data that cannot select an executor. A companion
debug_assert pins every specialty to a stable slug."
```

---

### Task 3: Prompt regression harness

**Files:**
- Create: `crates/crew-hive/tests/planner_prompt.rs`

**Interfaces:**
- Consumes: `crew_hive::{LlmPlanner, ModelTier, Planner, agentname::slug}`; `crew_hive::OpenRouterProvider` (DashScope speaks the OpenAI-compatible shape on a different endpoint).
- Produces: nothing consumed by later tasks.

This is the spike, promoted. It is `#[ignore]`d: it costs API calls and needs a key. It exists to re-validate the prompt when the planner model changes — the provider chain is DashScope → OpenRouter → Anthropic and the prompt is tuned only on the first.

- [ ] **Step 1: Write the harness**

```rust
//! Prompt regression for `PLANNER_SYSTEM`, against a real provider.
//!
//! `#[ignore]`d: needs `DASHSCOPE_API_KEY` and spends tokens. Run with
//! `cargo test -p crew-hive --test planner_prompt -- --ignored --nocapture`.
//!
//! Asserts only the mechanical properties (slug-legality without mangling,
//! distinctness, no title echo) and prints the cast for eyeballing — "is this
//! a good name" is a human call. See the design doc's Prompt spike section
//! for what each prompt clause defends against.
use crew_hive::agentname::slug;
use crew_hive::{LlmPlanner, ModelTier, OpenRouterProvider, Planner};

const ENDPOINT: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions";

/// Deliberately non-coding: the roster is meant to be a network of diverse
/// specialists, not a coding crew, so the prompt is judged on breadth.
const GOALS: &[&str] = &[
    "explain our project to stakeholders",
    "audit our dependencies for CVEs",
    "plan a 3-day trip to Kyoto in November",
    "write a blog post announcing our new release",
    "figure out why checkout conversion dropped 12% last month",
    "design a schema for a multi-tenant billing system",
];

#[test]
#[ignore = "network + API key + tokens"]
fn planner_invents_craft_shaped_specialists() {
    let key = std::env::var("DASHSCOPE_API_KEY").expect("DASHSCOPE_API_KEY");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut total = 0usize;
    let mut distinct = std::collections::HashSet::new();

    for goal in GOALS {
        let provider = OpenRouterProvider::new(key.clone())
            .with_endpoint(ENDPOINT.to_string())
            .with_fallbacks(vec!["qwen-max".to_string()]);
        let planner = LlmPlanner {
            provider,
            tier: ModelTier::Capable,
            model: Some("qwen-max".to_string()),
        };
        let graph = rt.block_on(planner.plan(goal)).expect("plan");

        println!("\n=== {goal}");
        let mut seen_in_plan = std::collections::HashSet::new();
        for t in graph.tasks() {
            println!("  {:<28} | {:<45} | {}", t.specialty, t.title, t.expertise);
            total += 1;
            distinct.insert(t.specialty.clone());

            assert_eq!(
                slug(&t.specialty).as_deref(),
                Some(t.specialty.as_str()),
                "specialty {:?} is not slug-stable",
                t.specialty
            );
            assert!(
                !t.specialty.starts_with("specialist-"),
                "task {:?} fell back to a derived name — the model omitted or \
                 mangled its specialty",
                t.title
            );
            // The failure signature this guards: the model echoing the task
            // title back instead of naming a craft. Never observed on
            // qwen-max across ~150 names, but a different model might.
            assert_ne!(
                Some(t.specialty.as_str()),
                slug(&t.title).as_deref(),
                "specialty {:?} is just the task title slugged",
                t.specialty
            );
            seen_in_plan.insert(t.specialty.clone());
        }
        assert!(
            seen_in_plan.len() > 1,
            "a whole plan collapsed to one specialist: {seen_in_plan:?}"
        );
    }

    // The spike measured 28 distinct / 32. A hard floor here would be flaky;
    // this catches only a collapse to near-uniformity.
    println!("\n{} distinct / {} total", distinct.len(), total);
    assert!(
        distinct.len() * 2 > total,
        "specialists are barely distinct ({} distinct / {total}) — the prompt \
         has probably regressed toward a catch-all",
        distinct.len()
    );
}
```

- [ ] **Step 2: Verify it compiles and is skipped by default**

Run: `cargo test -p crew-hive --test planner_prompt`
Expected: `0 passed; 1 ignored`.

- [ ] **Step 3: Run it for real once**

Run: `cargo test -p crew-hive --test planner_prompt -- --ignored --nocapture`
Expected: PASS, printing six casts. If `tokio` or `OpenRouterProvider` isn't a dev-dependency of `crew-hive`, add what's missing to `[dev-dependencies]` in `crates/crew-hive/Cargo.toml`.

- [ ] **Step 4: Commit**

```bash
cargo fmt
git add crates/crew-hive/tests/planner_prompt.rs crates/crew-hive/Cargo.toml
git commit -m "test(hive): prompt regression harness for PLANNER_SYSTEM

Ignored by default (network + key + tokens). Asserts the mechanical
properties only; name quality stays a human call. Exists to re-validate the
prompt when the planner model changes."
```

---

### Task 4: The specialist store

**Files:**
- Create: `crates/crew-plugin/src/broker/specialists.rs`
- Modify: `crates/crew-plugin/src/broker/mod.rs` (add `pub(crate) mod specialists;`)

**Interfaces:**
- Consumes: `crew_hive::agentname::slug` (Task 1).
- Produces:
  - `pub(crate) struct Specialist { pub name: String, pub role: String, pub last_used: u64 }` (Clone, Debug, Serialize, Deserialize, PartialEq)
  - `pub(crate) fn load() -> Vec<Specialist>` / `load_at(base: &Path) -> Vec<Specialist>`
  - `pub(crate) fn record(seen: &[(String, String)])` / `record_at(base: &Path, seen: &[(String, String)])`
  - `pub(crate) fn touch(name: &str)` / `touch_at(base: &Path, name: &str)`
  - `pub(crate) const CAP: usize = 24;`

Tasks 5, 6 and 7 consume these.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-plugin/src/broker/specialists.rs`:

```rust
//! The project-local specialist store: `./.crew/specialists.json`, alongside
//! `session-live.md`.
//!
//! This file is not merely persistence — it is the accumulation mechanism.
//! `Session::registry()` calls `Registry::discover_with` on every hello, send
//! and construct, rebuilding from scratch each time, so there is no long-lived
//! registry to hold a growing roster. Re-reading a file per rebuild makes
//! accumulation and durability one thing, and keeps new mutable state out of
//! `Session`.
//!
//! Every write is best-effort: specialists are a convenience, not a run's
//! product, so a failure here must never fail a run.
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Most specialists kept. Sized against the prompt spike's worst case: ~5 new
/// specialists per run with little name reuse (28 distinct / 32 tasks), so 24
/// is about five runs of history before the LRU trims a tail. A tighter cap
/// turns the roster over every couple of runs — churn, not a network.
pub(crate) const CAP: usize = 24;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct Specialist {
    /// The `@`-handle — always a valid `agentname::slug`.
    pub name: String,
    /// Prose craft hint. May be empty.
    pub role: String,
    /// Unix ms of last use, for LRU eviction. Bumped by a run that invents the
    /// name (`record`) and by an `@`-dial (`touch`).
    pub last_used: u64,
}

fn path(base: &Path) -> PathBuf {
    base.join(".crew").join("specialists.json")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub(crate) fn load() -> Vec<Specialist> {
    load_at(Path::new("."))
}

pub(crate) fn load_at(_base: &Path) -> Vec<Specialist> {
    unimplemented!()
}

pub(crate) fn record(seen: &[(String, String)]) {
    record_at(Path::new("."), seen)
}

pub(crate) fn record_at(_base: &Path, _seen: &[(String, String)]) {
    unimplemented!()
}

pub(crate) fn touch(name: &str) {
    touch_at(Path::new("."), name)
}

pub(crate) fn touch_at(_base: &Path, _name: &str) {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);

    /// A fresh project dir per test — these run in parallel against a
    /// process-wide filesystem. Mirrors `tests/common::unique_dir`.
    fn tmp() -> PathBuf {
        let id = SEQ.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!(
            "crew-spec-{}-{id}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn absent_store_loads_empty() {
        assert!(load_at(&tmp()).is_empty());
    }

    #[test]
    fn corrupt_store_loads_empty_instead_of_panicking() {
        let base = tmp();
        std::fs::create_dir_all(base.join(".crew")).unwrap();
        std::fs::write(path(&base), "{not json").unwrap();
        assert!(load_at(&base).is_empty());
    }

    #[test]
    fn record_then_load_roundtrips() {
        let base = tmp();
        record_at(&base, &[("archivist".into(), "records, retrieval".into())]);
        let got = load_at(&base);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "archivist");
        assert_eq!(got[0].role, "records, retrieval");
        assert!(got[0].last_used > 0);
    }

    #[test]
    fn record_merges_by_name_rather_than_suffixing() {
        let base = tmp();
        record_at(&base, &[("analyst".into(), "first".into())]);
        record_at(&base, &[("analyst".into(), "second".into())]);
        let got = load_at(&base);
        assert_eq!(got.len(), 1, "same name is the same specialist: {got:?}");
        assert_eq!(got[0].role, "first", "the original role is kept");
    }

    #[test]
    fn record_skips_names_that_are_not_slugs() {
        let base = tmp();
        record_at(&base, &[("Not A Slug".into(), "x".into()), ("ok-name".into(), "y".into())]);
        let got = load_at(&base);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "ok-name");
    }

    #[test]
    fn evicts_least_recently_used_at_cap() {
        let base = tmp();
        // Fill past the cap, oldest first.
        for i in 0..(CAP + 3) {
            record_at(&base, &[(format!("agent-{i:02}"), String::new())]);
        }
        let got = load_at(&base);
        assert_eq!(got.len(), CAP);
        let names: Vec<&str> = got.iter().map(|s| s.name.as_str()).collect();
        assert!(!names.contains(&"agent-00"), "oldest should be evicted");
        assert!(names.contains(&"agent-26"), "newest should survive");
    }

    #[test]
    fn touch_defers_eviction_for_a_dialed_specialist() {
        // Without touch, last_used only moves when a run re-invents a name, so
        // a specialist you @-dial daily would be evicted by unrelated churn.
        let base = tmp();
        record_at(&base, &[("favourite".into(), String::new())]);
        for i in 0..(CAP - 1) {
            record_at(&base, &[(format!("filler-{i:02}"), String::new())]);
        }
        touch_at(&base, "favourite");
        // Two more push past the cap; `favourite` must outlive the fillers.
        record_at(&base, &[("newcomer-a".into(), String::new())]);
        record_at(&base, &[("newcomer-b".into(), String::new())]);
        let names: Vec<String> = load_at(&base).into_iter().map(|s| s.name).collect();
        assert!(names.contains(&"favourite".to_string()), "got {names:?}");
    }

    #[test]
    fn touch_on_an_unknown_name_is_a_no_op() {
        let base = tmp();
        record_at(&base, &[("archivist".into(), String::new())]);
        touch_at(&base, "nobody");
        assert_eq!(load_at(&base).len(), 1);
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p crew-plugin specialists`
Expected: FAIL — `not implemented`.

- [ ] **Step 3: Implement**

Replace the three `unimplemented!()` bodies:

```rust
/// Read the store. Absent, unreadable or corrupt → empty: a broken file must
/// degrade to "no specialists yet", never break the broker.
pub(crate) fn load_at(base: &Path) -> Vec<Specialist> {
    let Ok(raw) = std::fs::read_to_string(path(base)) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

/// Merge `(name, role)` pairs into the store: a name already present keeps its
/// original role and is bumped to now; a new name is inserted. Over [`CAP`],
/// the least-recently-used are dropped. Best-effort — errors are swallowed.
pub(crate) fn record_at(base: &Path, seen: &[(String, String)]) {
    let mut all = load_at(base);
    let now = now_ms();
    for (name, role) in seen {
        // Defence in depth: parse_plan already slugs, but this store is also
        // the e2e seam and a hand-written file could carry anything.
        let Some(name) = crew_hive::agentname::slug(name) else {
            continue;
        };
        match all.iter_mut().find(|s| s.name == name) {
            Some(existing) => existing.last_used = now,
            None => all.push(Specialist {
                name,
                role: crew_hive::agentname::role_clamp(role),
                last_used: now,
            }),
        }
    }
    save_at(base, all);
}

/// Bump `name`'s recency without inventing it — the `@`-dial path, so that use
/// defers eviction and not only re-invention does.
pub(crate) fn touch_at(base: &Path, name: &str) {
    let mut all = load_at(base);
    let Some(s) = all.iter_mut().find(|s| s.name == name) else {
        return;
    };
    s.last_used = now_ms();
    save_at(base, all);
}

/// Sort newest-first, trim to [`CAP`], write atomically (tmp + rename) so a
/// crash mid-write can't leave a torn file. Every failure is ignored.
fn save_at(base: &Path, mut all: Vec<Specialist>) {
    all.sort_by(|a, b| b.last_used.cmp(&a.last_used));
    all.truncate(CAP);
    let p = path(base);
    let Some(dir) = p.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(json) = serde_json::to_string_pretty(&all) else {
        return;
    };
    let tmp = p.with_extension("json.tmp");
    if std::fs::write(&tmp, json).is_ok() {
        let _ = std::fs::rename(&tmp, &p);
    }
}
```

`record_at` and `touch_at` may write within the same millisecond in tests, making `last_used` ties possible. `sort_by` is stable, so an insertion-ordered tie keeps the earlier entry first (i.e. treated as newer). The eviction test above uses distinct rounds and the `touch` test asserts survival rather than exact order, so neither depends on tie-breaking.

- [ ] **Step 4: Register the module**

In `crates/crew-plugin/src/broker/mod.rs`, alongside the other `mod` lines:

```rust
pub(crate) mod specialists;
```

- [ ] **Step 5: Run to verify they pass**

Run: `cargo test -p crew-plugin specialists`
Expected: PASS, 8 tests.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add crates/crew-plugin/src/broker/specialists.rs crates/crew-plugin/src/broker/mod.rs
git commit -m "feat(broker): project-local specialist store with LRU cap

The file is the accumulation mechanism, not just persistence: the registry is
rebuilt from scratch on every call, so re-reading per rebuild keeps new
mutable state out of Session. touch() makes use (not only re-invention) defer
eviction. Writes are atomic and best-effort — never fail a run."
```

---

### Task 5: Runtime specialists replace the inbuilt trio

**Files:**
- Modify: `crates/crew-plugin/src/broker/apiadapter.rs` (add `role`, delete `inbuilt_agents`, add `specialist_agents`)
- Modify: `crates/crew-plugin/src/broker/agents.rs:22-34` (`role_for` — drop the trio arms)
- Modify: `crates/crew-plugin/src/broker/discover.rs:88-142` (`roster_with`)
- Modify: `crates/crew-plugin/src/broker/plugins.rs:52-63` (`from_manifest` uses `slug`)

**Interfaces:**
- Consumes: `specialists::{load, Specialist}` (Task 4), `crew_hive::agentname::slug` (Task 1).
- Produces: `apiadapter::specialist_agents(provider, model, overrides) -> Vec<Box<dyn Adapter>>`, `ApiAdapter::specialist(name, role, model, provider) -> std::io::Result<Self>`.

- [ ] **Step 1: Write the failing tests**

In `crates/crew-plugin/src/broker/apiadapter.rs`'s `mod tests`, delete `inbuilt_agents_are_planner_coder_reviewer` and add:

```rust
    #[test]
    fn a_specialist_reports_its_own_role() {
        let a = ApiAdapter::specialist("archivist", "records, retrieval", "m", mock("hi"))
            .unwrap();
        assert_eq!(a.name(), "archivist");
        assert_eq!(a.role(), "records, retrieval");
    }

    #[test]
    fn a_specialists_system_prompt_carries_its_name_and_role() {
        let a = ApiAdapter::specialist("archivist", "records, retrieval", "m", mock("hi"))
            .unwrap();
        let sys = a.system.clone().expect("specialists always get a system prompt");
        assert!(sys.contains("archivist"), "got {sys}");
        assert!(sys.contains("records, retrieval"), "got {sys}");
    }

    #[test]
    fn a_roleless_specialist_still_gets_a_usable_prompt() {
        // expertise is allowed to be empty; the prompt must not read as
        // "Your specialty is ." in that case.
        let a = ApiAdapter::specialist("mystery", "", "m", mock("hi")).unwrap();
        let sys = a.system.clone().unwrap();
        assert!(sys.contains("mystery"), "got {sys}");
        assert!(!sys.contains("specialty is ."), "got {sys}");
    }
```

The existing `mock(..)` helper in that test module returns an `Arc<dyn Provider>`; reuse it as-is.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p crew-plugin apiadapter`
Expected: FAIL — no method `specialist`.

- [ ] **Step 3: Add the `role` field and the specialist constructor**

In `crates/crew-plugin/src/broker/apiadapter.rs`, update the struct, add the constructor, and override `role()`:

```rust
pub struct ApiAdapter {
    name: String,
    model: String,
    /// This agent's own capability hint. Held here rather than looked up by
    /// name: `Adapter::role`'s default consults `agents::role_for`, a static
    /// match over the known CLI names, which returns "" for an invented
    /// specialist — blanking the palette, peer list and roster badge.
    role: String,
    system: Option<String>,
    provider: Arc<dyn Provider>,
    /// Current-thread runtime to block the sync broker on the async provider.
    rt: tokio::runtime::Runtime,
}
```

`ApiAdapter::new` gains a `role` parameter of `impl Into<String>` placed after `model`, and sets the field. Then:

```rust
impl ApiAdapter {
    /// A planner-invented specialist: `name` is its `@`-handle (a slug),
    /// `role` its craft hint (possibly empty). The system prompt is derived
    /// here rather than stored, so a persisted specialist never pins stale
    /// prompt text.
    pub fn specialist(
        name: impl Into<String>,
        role: impl Into<String>,
        model: impl Into<String>,
        provider: Arc<dyn Provider>,
    ) -> std::io::Result<Self> {
        let (name, role) = (name.into(), role.into());
        let system = if role.is_empty() {
            format!(
                "You are the {name}. Do the work the task asks for, in your own \
                 specialty. Be concise."
            )
        } else {
            format!(
                "You are the {name}. Your specialty is {role}. Do the work the \
                 task asks for, from that expertise. Be concise."
            )
        };
        Self::new(name, model, role, Some(system), provider)
    }
}
```

and in `impl Adapter for ApiAdapter`:

```rust
    fn role(&self) -> &str {
        &self.role
    }
```

For the tests to read `a.system`, the field must be visible to the test module — it already is, since `mod tests` is a child module of this file.

- [ ] **Step 4: Replace `inbuilt_agents` with `specialist_agents`**

Delete `inbuilt_agents` entirely (the `specs` array and the function) and add:

```rust
/// Build one adapter per stored specialist on `provider`. `overrides` pins a
/// specific model per agent name (the `/model` construct). Adapters whose
/// runtime fails to start are skipped rather than aborting the roster.
///
/// There is no inbuilt roster any more: a fresh project has no specialists
/// until a run invents some. See the design doc.
pub fn specialist_agents(
    provider: Arc<dyn Provider>,
    model: &str,
    overrides: &std::collections::HashMap<String, String>,
) -> Vec<Box<dyn Adapter>> {
    super::specialists::load()
        .into_iter()
        .filter_map(|s| {
            let model = overrides.get(&s.name).cloned().unwrap_or_else(|| model.to_string());
            ApiAdapter::specialist(s.name, s.role, model, provider.clone())
                .ok()
                .map(|a| Box::new(a) as Box<dyn Adapter>)
        })
        .collect()
}
```

- [ ] **Step 5: Trim `role_for`**

In `crates/crew-plugin/src/broker/agents.rs`, delete the three inbuilt arms, keeping the CLI ones, and update the doc comment:

```rust
/// A short capability hint per known *external CLI* agent, surfaced in the peer
/// list so an agent hands the task off to the right one. Empty for anything
/// else — API specialists are invented at runtime and carry their own role
/// (see `ApiAdapter::role`), so there is nothing static to look up.
pub fn role_for(name: &str) -> &'static str {
    match name {
        // External CLI agents (still selectable via the CLI adapters).
        "claude" => "planning, analysis, prose",
        "codex" => "building, implementation",
        "opencode" => "review, second opinion",
        _ => "",
    }
}
```

- [ ] **Step 6: Rewire `roster_with`**

In `crates/crew-plugin/src/broker/discover.rs`, every branch swaps `inbuilt_agents(provider, model_for, overrides)` for `specialist_agents(provider, &model, overrides)`. The tier→model mapping is gone: a specialist has no tier, so each branch uses the same single model id `provider_and_model` already picks.

Replace the body of `roster_with`:

```rust
pub(crate) fn roster_with(
    overrides: &std::collections::HashMap<String, String>,
) -> Vec<Box<dyn Adapter>> {
    let Some((provider, model)) = provider_and_model() else {
        return Vec::new();
    };
    let mut agents = specialist_agents(provider, &model, overrides);
    // The mock roster stays plugin-free so end-to-end tests are deterministic
    // on any machine.
    if !matches!(
        pick_provider(std::env::var("CREW_PROVIDER").ok().as_deref(), |k| {
            std::env::var(k).is_ok_and(|v| !v.is_empty())
        }),
        Some(ProviderKind::Mock)
    ) {
        super::plugins::append(&mut agents);
    }
    agents
}
```

This collapses `roster_with`'s four duplicated provider branches into `provider_and_model`, which the spec notes already mirrors them exactly and is already used by `swarm::backend()`. Verify `provider_and_model`'s Anthropic arm still picks a sensible model — it currently hardcodes `ModelTier::Cheap`, which was chosen for one-shot `!` suggestions; change it to `ModelTier::Standard.model_id()` so specialists get the same tier the old `coder`/`reviewer` had, and update that function's doc comment to say it now serves the roster too.

- [ ] **Step 7: Tighten manifest names**

In `crates/crew-plugin/src/broker/plugins.rs`, `from_manifest`'s hand-rolled normalizer becomes `slug`:

```rust
        // One legal-name authority (`crew_hive::agentname::slug`): the old
        // hand-rolled lowercase+hyphenate let `+`/`@` through, and such a name
        // could never be @-dialled anyway.
        let name = crew_hive::agentname::slug(&raw.name)?;
```

Match the surrounding error style — if `from_manifest` returns `Option`, `?` is right; if it returns `Result`, map `None` to the existing "blank name" error.

- [ ] **Step 8: Fix the fallout and run the suite**

Run: `cargo test -p crew-plugin 2>&1 | grep -E "^error|^test result|FAILED"`

Expected failures to fix:
- `e2e_discovery`, `e2e_relay`, `e2e_tasks` reference the trio — **leave them failing**; Task 8 reworks them. Note which fail.
- Any `ApiAdapter::new` caller needs the new `role` argument.

Unit tests in `crew-plugin` (not the `tests/` dir) must pass.

- [ ] **Step 9: Commit**

```bash
cargo fmt
git add -A
git commit -m "feat(broker): specialists replace the inbuilt planner/coder/reviewer

ApiAdapter carries its own role — Adapter::role's default consults a static
match over CLI names and returns \"\" for an invented specialist, blanking the
palette, peer list and roster badge.

roster_with now composes stored specialists over provider_and_model, which
already mirrored its four provider branches. The e2e suites pinning the trio
fail until the next task."
```

---

### Task 6: Register a run's specialists and light them up

**Files:**
- Modify: `crates/crew-plugin/src/broker/swarm.rs` (`run_with`, after the plan lands)
- Modify: `crates/crew-plugin/src/broker/swarmmsg.rs:12-32` (`translate`)

**Interfaces:**
- Consumes: `specialists::record` (Task 4); `TaskSpec::{specialty, expertise}` (Task 2).
- Produces: `PluginEvent::Roster` re-emission mid-session.

- [ ] **Step 1: Write the failing test for Activity naming**

In `crates/crew-plugin/src/broker/swarm_tests.rs` (or `swarmmsg`'s test module if one exists):

```rust
#[test]
fn activity_names_the_specialist_not_the_task_title() {
    // The roster matches active agents by name (chatview::agent_views), so
    // Activity must carry the specialty — with the title, a roster row could
    // never light up.
    let mut titles = HashMap::new();
    titles.insert(TaskId(0), "Gather Project Details".to_string());
    let mut specialties = HashMap::new();
    specialties.insert(TaskId(0), "archivist".to_string());
    let mut agent_task = HashMap::new();

    let evs = translate(
        &HiveEvent::AgentSpawned { agent: AgentId(1), task: TaskId(0) },
        &titles,
        &specialties,
        &mut agent_task,
    );
    match &evs[0] {
        PluginEvent::Activity { agent, .. } => assert_eq!(agent, "archivist"),
        other => panic!("expected Activity, got {other:?}"),
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-plugin activity_names_the_specialist`
Expected: FAIL to compile — `translate` takes three arguments.

- [ ] **Step 3: Thread specialties into `translate`**

In `crates/crew-plugin/src/broker/swarmmsg.rs`, `translate` gains a `specialties: &HashMap<TaskId, String>` parameter and names agents from it:

```rust
/// Map one HiveEvent to chat-facing events. Raw `Hive` forwarding happens at
/// the call site; this returns only the human-readable translations.
///
/// Agents are named by their task's *specialty*, not its title: the roster
/// lights a row by matching the active name against a roster name, so a title
/// could never match. Titles still name the *work* in the swarm block.
pub(super) fn translate(
    ev: &HiveEvent,
    titles: &HashMap<TaskId, String>,
    specialties: &HashMap<TaskId, String>,
    agent_task: &mut HashMap<u64, TaskId>,
) -> Vec<PluginEvent> {
    let title_of = |t: &TaskId| {
        titles
            .get(t)
            .cloned()
            .unwrap_or_else(|| format!("task-{}", t.0))
    };
    let specialist_of = |t: &TaskId| {
        specialties
            .get(t)
            .cloned()
            .unwrap_or_else(|| format!("specialist-{}", t.0))
    };
```

Replace every `title_of(task)` used as an **agent name** with `specialist_of(task)` — including inside the `agent_name` closure. Leave `title_of` wherever the string names the *work* (task-row text, summaries). Read each call site before switching it; the two are interleaved.

- [ ] **Step 4: Build the map and record in `swarm.rs`**

In `run_with`, where `titles` is built from the plan's `TaskSpec`s, build `specialties` alongside, then record and re-emit:

```rust
    let specialties: HashMap<TaskId, String> =
        graph.tasks().iter().map(|t| (t.id, t.specialty.clone())).collect();

    // Persist this run's cast, then re-emit the roster: `Roster` is otherwise
    // only sent from `hello()`, so without this the app never learns about a
    // specialist invented mid-session and the new names never appear.
    // First-wins on a duplicate name: one name is one specialist.
    let mut seen: Vec<(String, String)> = Vec::new();
    for t in graph.tasks() {
        if !seen.iter().any(|(n, _)| n == &t.specialty) {
            seen.push((t.specialty.clone(), t.expertise.clone()));
        }
    }
    super::specialists::record(&seen);
    emit(PluginEvent::Roster {
        agents: super::Registry::discover().infos(),
    });
```

Match the surrounding code for how events are emitted (`emit`, `tick_emit`, or a channel send) and how the registry is reached — `run_with` receives a session snapshot; if it carries `overrides`, use `discover_with(&snap.overrides)` so `/model` pins survive. Use the same `HashMap`/`TaskId` imports already at the top of the file.

- [ ] **Step 5: Run the suite**

Run: `cargo test -p crew-plugin --lib 2>&1 | grep -E "^test result|^error"`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add -A
git commit -m "feat(broker): record a run's specialists and re-emit the roster

Activity is keyed by specialty, not task title, so roster rows can finally
match an active agent and light up. Titles still name the work in the swarm
block. Roster re-emission is load-bearing: hello() was the only emitter, so
mid-session specialists were invisible to the app."
```

---

### Task 7: Messaging and `@`-dial

**Files:**
- Modify: `crates/crew-plugin/src/broker/stdio.rs:248-275` (`roster`, `relay_counting`)
- Modify: `crates/crew-plugin/src/broker/relay.rs:181-216` (`split_target`)

**Interfaces:**
- Consumes: `specialists::touch` (Task 4), `discover::provider_and_model` (existing).
- Produces: `relay::split_target` returns `Option<(String, String)>` — `None` now means "named an agent that doesn't exist".

- [ ] **Step 1: Write the failing tests**

In `relay.rs`'s test module:

```rust
    #[test]
    fn an_unknown_at_name_is_reported_not_silently_redirected() {
        // Was: fell back to the first registered agent with "@typo" still in
        // the body. Nothing reported it — the fallback target is arbitrary
        // once names are LLM-authored.
        let reg = Registry::new(vec![fake("archivist"), fake("analyst")]);
        assert_eq!(split_target("@typo do the thing", &reg), None);
    }

    #[test]
    fn a_known_at_name_still_splits_target_from_task() {
        let reg = Registry::new(vec![fake("archivist"), fake("analyst")]);
        assert_eq!(
            split_target("@analyst do the thing", &reg),
            Some(("analyst".to_string(), "do the thing".to_string()))
        );
    }

    #[test]
    fn a_bare_task_goes_to_the_first_agent_unchanged() {
        let reg = Registry::new(vec![fake("archivist")]);
        assert_eq!(
            split_target("do the thing", &reg),
            Some(("archivist".to_string(), "do the thing".to_string()))
        );
    }
```

Reuse whatever fake-adapter helper the existing `relay.rs` tests use in place of `fake(..)`.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p crew-plugin relay`
Expected: FAIL — the unknown name currently resolves to the first agent.

- [ ] **Step 3: Fix `split_target`**

Only a message *starting with* `@` claims a target. An unknown one is an error, not a redirect:

```rust
/// Split `@name task` into its target and the task text. `None` when the
/// message names an agent that isn't registered — the caller reports it.
///
/// A message with no leading `@` is not addressed at all: it goes to the first
/// registered agent with its text intact.
pub(crate) fn split_target(task: &str, reg: &Registry) -> Option<(String, String)> {
    let default = || reg.names().first().cloned().unwrap_or_default();
    let Some(rest) = task.strip_prefix('@') else {
        return Some((default(), task.to_string()));
    };
    let (name, body) = match rest.split_once(char::is_whitespace) {
        Some((n, b)) => (n, b.trim_start()),
        None => (rest, ""),
    };
    reg.get(name)
        .map(|a| (a.name().to_string(), body.to_string()))
}
```

Delete the stale comment at `relay.rs:184` claiming a typo "falls through to the normal single-target path, which reports it" — it never did, and now the reporting is real.

- [ ] **Step 4: Report it in `relay_counting`**

At the `split_target` call site in `stdio.rs`, `None` now needs handling:

```rust
    let Some((target, body)) = split_target(trimmed, &reg) else {
        let known = reg.names().join(", ");
        let name = trimmed.split_whitespace().next().unwrap_or(trimmed);
        return format!("No agent named {name}. Known: {known}");
    };
```

Match the function's actual return type and emit path (it may push a `PluginEvent::Message` rather than return a `String`).

- [ ] **Step 5: Split the roster messaging**

In `stdio.rs`'s `roster()`, distinguish "no provider" from "no specialists yet" — with an empty starting roster, the old message tells a correctly-keyed user to set a key they already have:

```rust
    if agents.is_empty() {
        return if super::discover::provider_and_model().is_some() {
            "No specialists yet — type a task and the planner will invent the \
             ones it needs."
                .to_string()
        } else {
            "No provider configured. Set OPENROUTER_API_KEY, DASHSCOPE_API_KEY, \
             or ANTHROPIC_API_KEY and reopen /crew."
                .to_string()
        };
    }
```

Apply the same split at `relay_counting`'s empty-registry short-circuit.

- [ ] **Step 6: Touch the dialed specialist**

After resolving `target` in `relay_counting`, so that use defers eviction:

```rust
    // Use, not only re-invention, keeps a specialist alive in the LRU.
    super::specialists::touch(&target);
```

- [ ] **Step 7: Run to verify they pass**

Run: `cargo test -p crew-plugin --lib 2>&1 | grep -E "^test result|^error"`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
cargo fmt
git add -A
git commit -m "fix(broker): report an unknown @name instead of silently redirecting

split_target fell back to the first registered agent with the bad @name still
in the prompt body, and nothing reported it — the code comment claiming
otherwise was wrong. Arbitrary once names are LLM-authored.

Also split the empty-roster message: with no inbuilt trio, a correctly-keyed
user would have been told to set a key they already have."
```

---

### Task 8: Rework the e2e suites

**Files:**
- Modify: `crates/crew-plugin/tests/common/mod.rs:64-96` (`run_broker` — set CWD; add `seed_specialists`)
- Modify: `crates/crew-plugin/tests/e2e_discovery.rs`
- Modify: `crates/crew-plugin/tests/e2e_relay.rs:12,21,25,63-70`
- Modify: `crates/crew-plugin/tests/e2e_tasks.rs:11-12`

**Interfaces:**
- Consumes: the store file format from Task 4.
- Produces: `common::seed_specialists(dir: &Path, names: &[&str])`.

- [ ] **Step 1: Give the broker a real project directory**

**This is a prerequisite, not a nicety.** `run_broker` (`common/mod.rs:64`) passes `path_dir` only to `PATH` — it never sets `current_dir`, so the spawned broker inherits the *test process's* CWD (the crate root). The store is project-local and CWD-relative, so without this every e2e test would share one `crates/crew-plugin/.crew/specialists.json`, run in parallel against it, and inherit whatever a previous run left behind.

This already happens today — `crates/crew-plugin/.crew/` exists, written by `sessionlog::rotate()`'s `Path::new(".")`. It has been invisible because the directory is gitignored and a session log nobody asserts on can be shared harmlessly. A roster cannot.

In `crates/crew-plugin/tests/common/mod.rs`, add to the `command` builder chain in `run_broker`, right after `.env("PATH", path_dir)`:

```rust
        // The broker resolves its project-local state (.crew/session-live.md,
        // .crew/specialists.json) against its CWD. Each test gets its own
        // `unique_dir`, so this both isolates the tests from each other and
        // matches production, where the broker's CWD *is* the project.
        // Without it these files land in the crate root and are shared by
        // every test in the binary, in parallel.
        .current_dir(path_dir)
```

- [ ] **Step 2: Verify that change alone breaks nothing**

Run: `cargo test -p crew-plugin --test e2e_relay --test e2e_tasks 2>&1 | grep -E "^test result|FAILED"`
Expected: the same results as before the change (some failing from Task 5's trio removal — note which, and confirm the *set* didn't grow). `write_fake` writes agents into `path_dir` and `PATH` points at it absolutely, so nothing depends on the old CWD.

- [ ] **Step 3: Add the seeding helper**

In `crates/crew-plugin/tests/common/mod.rs`:

```rust
/// Seed the specialist store so an `@`-dial has a target. With no inbuilt
/// trio the store is the broker's only roster source, so this is how an e2e
/// test gets a dial-able agent. `run_broker` runs the broker with `dir` as its
/// CWD, which is what makes this the file the broker reads.
pub fn seed_specialists(dir: &Path, names: &[&str]) {
    let items: Vec<String> = names
        .iter()
        .map(|n| format!(r#"{{"name":"{n}","role":"testing","last_used":1700000000000}}"#))
        .collect();
    std::fs::create_dir_all(dir.join(".crew")).unwrap();
    std::fs::write(
        dir.join(".crew").join("specialists.json"),
        format!("[{}]", items.join(",")),
    )
    .unwrap();
}
```

- [ ] **Step 4: Invert the roster-listing test**

In `e2e_discovery.rs`, update the module doc (it describes the trio) and replace `discovery_lists_the_inbuilt_roster` — its whole subject was the trio, so it inverts rather than adapts:

```rust
#[test]
fn discovery_reports_empty_roster_until_a_run_invents_specialists() {
    // There is no inbuilt roster any more: a provider-backed broker with no
    // history has nobody to dial, and must say so rather than naming a key
    // that is already set.
    let dir = unique_dir("disc");
    let r = roster(&run_broker(&dir, &[MOCK], &[HELLO]));
    assert!(r.contains("No specialists yet"), "{r}");
    assert!(!r.contains("ANTHROPIC_API_KEY"), "a keyed broker must not ask for a key: {r}");
}

#[test]
fn discovery_lists_seeded_specialists() {
    let dir = unique_dir("disc-seeded");
    common::seed_specialists(&dir, &["archivist", "analyst"]);
    let r = roster(&run_broker(&dir, &[MOCK], &[HELLO]));
    assert!(r.contains("2 agent(s)"), "{r}");
    assert!(r.contains("archivist") && r.contains("analyst"), "{r}");
}
```

Add `seed_specialists` to the `use common::{...}` list at the top.

`discovery_reports_no_key` needs no change: the new no-provider message still contains `ANTHROPIC_API_KEY`.

- [ ] **Step 5: Decouple the key-recovery test**

`shell_env_probe_recovers_missing_provider_key` asserts `"3 agent(s)"` only incidentally — its subject is that a key was recovered from `$SHELL`, not that a roster has three rows. With an empty store a keyed broker now reports "No specialists yet", which is itself proof the provider resolved:

```rust
    let r = roster(&run_broker(&dir, &env, &[HELLO]));
    // The subject is key recovery, not roster size: with the key recovered the
    // broker must not fall back to the no-provider message.
    assert!(!r.contains("No provider configured"), "{r}");
    assert!(r.contains("No specialists yet"), "{r}");
```

- [ ] **Step 6: Swap the dialed name in the remaining tests**

`at_selector_starts_with_chosen_agent` in `e2e_discovery.rs`:

```rust
#[test]
fn at_selector_starts_with_chosen_agent() {
    let dir = unique_dir("sel");
    common::seed_specialists(&dir, &["archivist", "analyst"]);
    let send = r#"{"type":"send","channel":"crew","text":"@analyst hello there"}"#;
    let ev = run_broker(&dir, &[MOCK], &[send]);
    // analyst (not the default first agent, archivist) handled the task.
    assert!(has_leg(&ev, "analyst → user"), "{:?}", messages(&ev));
    assert!(!has_leg(&ev, "archivist → user"), "{:?}", messages(&ev));
}
```

In `e2e_relay.rs`: `SEND` becomes `"@archivist do it"`, the leg assertions become `"archivist → user"`, the `Activity` assertion becomes `agent: "archivist"`, and each test seeds `common::seed_specialists(&dir, &["archivist"])` before `run_broker`. In `e2e_tasks.rs`, `@planner` is only a vehicle to force relay routing (see its comment at `:8-10`) — same swap, same seeding. Their real subjects (turn summaries, Stats plumbing, the task pool, `/stop`) are untouched.

- [ ] **Step 5: Run the whole workspace**

Run: `cargo test --workspace 2>&1 | grep -E "^test result|FAILED|^error"`
Expected: everything PASS.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add -A
git commit -m "test(broker): rework the e2e suites around the specialist store

The store file is the seam: seed it and the roster is what you seeded.
discovery_lists_the_inbuilt_roster inverts rather than adapts — its subject
was the trio. The key-recovery test asserted a roster count incidentally and
now asserts key recovery directly."
```

---

### Task 9: Row-budget the roster grid

**Files:**
- Modify: `crates/crew-app/src/chatchips.rs` (`Layout`, `layout`, `row_cells`)
- Modify: `crates/crew-app/src/chatview.rs:14-54` (`agent_views` ordering)
- Test: `crates/crew-app/src/chatchips.rs`'s `mod tests`, `crates/crew-app/src/chatview_tests.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks (pure UI).
- Produces: `chatchips::ROSTER_MAX_ROWS`, `Layout::hidden: usize`.

`layout` is the single source of truth for row accounting (`ChatPane::status_rows`) *and* rendering (`chatview::cells`) — the comment at the top of `chatchips.rs` calls this out as the fix for a confirmed overdraw bug. Changing `rows` therefore changes both, automatically and correctly. Do not add a second cap anywhere else.

- [ ] **Step 1: Write the failing tests**

In `chatchips.rs`'s `mod tests`:

```rust
    fn views(n: usize) -> Vec<AgentView> {
        (0..n).map(|i| view(&format!("agent-{i:02}"))).collect()
    }

    #[test]
    fn roster_rows_are_capped_so_the_message_area_survives() {
        // Was: one row per agent, unbudgeted. A 24-specialist roster on a
        // 24-row pane claimed 12+ rows and left ~8 for the conversation.
        let lay = layout(&views(24), 200, 100, true).expect("fits");
        assert_eq!(lay.rows, ROSTER_MAX_ROWS as u16);
        assert_eq!(lay.shown, ROSTER_MAX_ROWS);
        assert_eq!(lay.hidden, 24 - ROSTER_MAX_ROWS);
    }

    #[test]
    fn a_short_roster_hides_nothing() {
        let lay = layout(&views(3), 200, 100, true).expect("fits");
        assert_eq!(lay.rows, 3);
        assert_eq!(lay.hidden, 0);
    }

    #[test]
    fn a_squeezed_pane_still_wins_over_the_display_cap() {
        let lay = layout(&views(24), 200, 2, true).expect("fits");
        assert_eq!(lay.shown, 2, "avail_rows is the harder limit");
        assert_eq!(lay.hidden, 22);
    }

    #[test]
    fn the_overflow_row_is_drawn_only_when_something_is_hidden() {
        let cells = row_cells(&views(24), 200, 0, &layout(&views(24), 200, 100, true).unwrap(), 0);
        let text: String = cells.iter().map(|c| c.c).collect();
        assert!(text.contains("+19 more"), "got {text}");

        let cells = row_cells(&views(3), 200, 0, &layout(&views(3), 200, 100, true).unwrap(), 0);
        let text: String = cells.iter().map(|c| c.c).collect();
        assert!(!text.contains("more"), "got {text}");
    }
```

Reuse the existing `view(..)`/`trio()` helpers already in that module.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p crew-app --bin crew chatchips`
Expected: FAIL — no `ROSTER_MAX_ROWS`, no `hidden`.

- [ ] **Step 3: Cap the layout**

In `chatchips.rs`:

```rust
/// Most agent rows the grid ever draws, however tall the pane. The roster is
/// furniture; the message area is the point. Beyond this the tail collapses
/// into a `+N more` row — the full roster is still @-dial-able and reachable
/// via completion and the palette, so nothing is lost but pixels.
pub(crate) const ROSTER_MAX_ROWS: usize = 5;
```

In `Layout`, add:

```rust
    /// Agents not drawn — the `+N more` count. 0 when everything fits.
    pub hidden: usize,
```

and in `layout`, replace the `shown` computation:

```rust
    // Two independent limits: the pane's height and the display cap. When
    // anything is hidden, the last row is spent on the `+N more` marker.
    let budget = (avail_rows as usize).min(ROSTER_MAX_ROWS);
    let shown = views.len().min(budget);
    if shown == 0 {
        return None;
    }
    let hidden = views.len() - shown;
    let subset = &views[..shown];
    Some(Layout {
        level,
        name_w: name_w(subset) as u16,
        state_w: state_w(subset) as u16,
        tok_w: tok_w(subset) as u16,
        shown,
        rows: shown as u16 + u16::from(hidden > 0),
        hidden,
        show_share,
    })
```

`rows` counts the overflow row so `status_rows` reserves it — that is the whole reason both sides read `layout`.

- [ ] **Step 4: Draw the overflow row**

At the end of `row_cells`, after the per-agent rows, append the marker when `lay.hidden > 0`. Follow the muted style `chatqueue::indicator_cells` uses (`theme.text_muted`, left inset 1):

```rust
    if lay.hidden > 0 {
        let text = format!("  +{} more", lay.hidden);
        let row = top + lay.shown as u16;
        let theme = crew_theme::theme();
        for (col, c) in (0u16..).zip(text.chars()) {
            if col >= cols {
                break;
            }
            v.push(CellView {
                col,
                row,
                c,
                fg: theme.text_muted,
                bg: theme.page_bg,
                bold: false,
                italic: false,
            });
        }
    }
```

Match `row_cells`'s existing accumulator name and `top`/`cols` parameter names.

- [ ] **Step 5: Order active-first in `agent_views`**

In `chatview.rs`, sort before returning. Ordering lives here (not in `layout`) because it needs `active`, which is pane state:

```rust
        let mut views: Vec<crate::chatchips::AgentView> = self
            .agents
            .iter()
            .map(|a| { /* ...existing per-agent mapping, unchanged... */ })
            .collect();
        // Active first, so a working specialist is never the one hidden by the
        // display cap; the roster's tail is what collapses into `+N more`.
        // Stable sort keeps roster order (broker recency) within each group.
        views.sort_by_key(|v| !v.active);
        views
```

- [ ] **Step 6: Write the ordering test**

In `crates/crew-app/src/chatview_tests.rs`:

```rust
#[test]
fn active_specialists_are_never_the_ones_hidden_by_the_row_cap() {
    let mut pane = test_pane(vec![msg("crew", "hi")]);
    pane.agents = (0..12)
        .map(|i| AgentInfo {
            name: format!("agent-{i:02}"),
            role: String::new(),
            model: String::new(),
        })
        .collect();
    // The last agent is the only one working — it must still be on screen.
    pane.absorb_activity("agent-11", "thinking", "hive");

    let views = pane.agent_views();
    assert!(views[0].active, "the active agent must sort first");
    assert_eq!(views[0].name, "agent-11");
}
```

Match `AgentInfo`'s real fields and whatever `absorb_activity`'s real signature is (`chatflow.rs:65`); if it isn't reachable from tests, drive it through the `PluginEvent::Activity` path the pane already consumes.

- [ ] **Step 7: Run to verify they pass**

Run: `cargo test -p crew-app --bin crew 2>&1 | grep -E "^test result|FAILED"`
Expected: PASS. Existing `chatchips` tests asserting `lay.rows == 3` for a trio still hold (3 < 5, nothing hidden).

- [ ] **Step 8: Commit**

```bash
cargo fmt
git add -A
git commit -m "fix(chat): row-budget the roster grid

layout spent one unbudgeted pane row per agent — invisible at 3 agents,
crippling at 24: a full roster claimed 12+ rows of a 24-row pane and left ~8
for the conversation. Cap at 5 rows with a +N more tail, ordered active-first
so a working specialist is never the one hidden.

rows counts the overflow marker, so status_rows reserves it: both row
accounting and rendering read layout, which is why the cap lives there."
```

---

### Task 10: Stop the legend dropping chips silently

**Files:**
- Modify: `crates/crew-app/src/chatinput.rs:167-200` (`chips_on_border`)
- Test: `crates/crew-app/src/chatinput_tests.rs` (or `chatinput.rs`'s test module — match what exists)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: nothing.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_legend_too_narrow_for_every_chip_says_how_many_it_dropped() {
    // Was: break on the first chip that didn't fit, silently. Three short
    // names always fit; two dozen long specialists never will.
    let agents: Vec<AgentInfo> = (0..12)
        .map(|i| AgentInfo {
            name: format!("specialist-{i:02}"),
            role: String::new(),
            model: String::new(),
        })
        .collect();
    let mut cells = Vec::new();
    chips_on_border(&mut cells, &agents, 40, 0);
    let text: String = {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert!(text.contains('+'), "overflow must be marked: {text}");
    assert!(!text.contains("specialist-11"), "the tail should not fit: {text}");
}

#[test]
fn a_legend_with_room_for_everything_marks_nothing() {
    let agents = vec![AgentInfo {
        name: "archivist".into(),
        role: String::new(),
        model: String::new(),
    }];
    let mut cells = Vec::new();
    chips_on_border(&mut cells, &agents, 80, 0);
    let text: String = cells.iter().map(|c| c.c).collect();
    assert!(text.contains("@archivist"), "got {text}");
    assert!(!text.contains('+'), "nothing was dropped: {text}");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-app --bin crew chips_on_border`
Expected: FAIL — no `+` is ever drawn.

- [ ] **Step 3: Mark the overflow**

In `chips_on_border`, count what didn't fit and draw a `+N` chip. Reserve room for the marker *before* laying chips, so the last chip can't consume the space the marker needs:

```rust
    let mut drawn = 0usize;
    for a in agents {
        let chip = format!("@{}", a.name);
        // Chip + its two surrounding spaces must stay clear of the corner, and
        // leave room for a `+N` marker if anything is left over.
        let rest = agents.len() - drawn - 1;
        let reserve = if rest > 0 { 2 + rest.to_string().len() as u16 } else { 0 };
        if x + chip.len() as u16 + 2 + reserve > cols.saturating_sub(2) {
            break;
        }
        // ...existing per-chip cell pushes, unchanged...
        drawn += 1;
    }
    let hidden = agents.len() - drawn;
    if hidden > 0 {
        let marker = format!(" +{hidden}");
        for c in marker.chars() {
            if x + 2 > cols.saturating_sub(2) {
                break;
            }
            cells.push(cell(x, row, c, border, false));
            x += 1;
        }
    }
```

`chip.len()` is a byte count, which is only correct because agent names are ASCII slugs (`agentname::slug`) — the same assumption the existing code already makes. Keep the function's existing return contract: the first free column after the chips.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p crew-app --bin crew 2>&1 | grep -E "^test result|FAILED"`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add -A
git commit -m "fix(chat): mark dropped legend chips instead of hiding them

chips_on_border break()s on the first chip that doesn't fit. Fine for three
short names, wrong for a dozen long specialists — most of the legend vanished
with no indication. Reserve room for the marker before laying chips, so the
last chip can't eat the space the marker needs."
```

---

### Task 11: Full verification

**Files:** none — this task only runs things.

- [ ] **Step 1: Whole suite**

Run: `cargo test --workspace 2>&1 | grep -E "^test result|FAILED|^error"`
Expected: every line `ok`, no failures.

- [ ] **Step 2: Lints**

Run: `cargo clippy --workspace --all-targets 2>&1 | grep -E "^(warning|error)" | head`
Expected: no output. The repo is clippy-clean; keep it that way.

- [ ] **Step 3: Prompt regression against the real provider**

Run: `cargo test -p crew-hive --test planner_prompt -- --ignored --nocapture`
Expected: PASS. Read the printed casts — every specialty should be a plausible craft-noun for its goal. This is the check no unit test can make.

- [ ] **Step 4: Grep for survivors**

Run: `grep -rn "inbuilt_agents\|\"planner\"\|\"coder\"\|\"reviewer\"" --include="*.rs" crates/ | grep -v "^crates/crew-hive/src/planner"`
Expected: no hits outside `crew-hive`'s planner module (where "planner" is the module's own domain word, not an agent name).

- [ ] **Step 5: Report**

Summarize: what shipped, what the prompt regression printed, anything the plan didn't anticipate. Do **not** claim the GUI was verified unless it actually was — the roster row budget and the `+N` legend are visual, and this session's harness could not drive the live app (osascript lacked Accessibility and Screen Recording). Say so plainly if it remains unverified.
