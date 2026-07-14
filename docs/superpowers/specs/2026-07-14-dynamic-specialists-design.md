# Dynamic specialists replace the fixed planner/coder/reviewer roster

**Date:** 2026-07-14
**Status:** Approved

## Problem

The broker ships a hard-coded roster of three API agents — `planner`,
`coder`, `reviewer` (`apiadapter.rs::inbuilt_agents`) — and that trio is what
the `/crew` roster rows and the input-bar `@`-legend render. Meanwhile a swarm
run's actual work is done by agents the planner invents per task, named after
their task title (`swarmmsg.rs::translate`).

These two populations never meet. The roster UI reads `ChatPane.agents`
(`chatview.rs::agent_views`), which only the `Roster` event writes
(`chat.rs:152`), and `Roster` is emitted exactly once, from `hello()`
(`stdio.rs:75`). A run's agents arrive on a different channel entirely —
`Activity` → `chatflow::absorb_activity` → `ChatPane.active` — and are
structurally incapable of appearing in the roster. So during a swarm run the
user watches three irrelevant agents sit at `idle` while unnamed specialists
do the work off-screen.

The names are also wrong in kind. `planner`/`coder`/`reviewer` frames the
product as a coding crew; the goal is a network of diverse specialists, where
the work decides who is needed. `agents.rs:15-21` already anticipates this —
roles are documented as general archetypes (plan / build / critique) and
`constructs::is_critic` already elects the judge by capability rather than by
the literal name `reviewer` (commit `1fafa93`). The names themselves are the
last hard-coded piece.

**Goal:** the planner invents a named specialist per task; those specialists
appear in the roster, light up while working, persist after the run, and are
`@`-dial-able. The fixed trio is deleted.

## Decisions (user-approved)

- **Identity source:** the planning LLM names each specialist via new
  `specialty` (the name) and `expertise` (its capability hint) fields in the
  plan schema. Not derived from task titles, not drawn from a fixed pool.
- **Defaults:** none. The inbuilt trio is deleted; the roster starts empty and
  is populated by runs.
- **Persistence:** a run's specialists survive it and accumulate as dial-able
  agents.
- **Bounding:** cap 12, LRU eviction by last use. Same name = same specialist
  (merged, not suffixed).
- **Name guard:** `^[a-z0-9-]{2,20}$`, enforced at the parse boundary.
  Unsalvageable names fall back to a derived `specialist-{id}`.
- **Store scope:** project-local (`./.crew/`), not global.
- **Scope:** one spec covering the whole change, not split into additive and
  subtractive phases.

### Explicitly reverses a prior decision

The 2026-07-12 crew-hive design doc states: *"The trio hop-relay is retired as
the default path; planner/coder/reviewer remain in the roster for direct
dials."* This spec removes them. That doc predates the "coding is solved"
framing (`progress.md:871`); keeping a coding-shaped trio purely as dial
targets preserves the exact thing this change exists to remove. The `@`-dial
path itself is preserved — it now dials invented specialists instead.

### "Keeps its history" means role and usage, not a transcript

When a second run re-invents `analyst`, the merged specialist retains its role
hint and accumulated usage (`last_used`, `runs`). It does **not** gain a
private conversation history: none exists today. The session log
(`sessionlog.rs:18`) is a single shared prose file (`"{sender}: {text}"`), not
a per-agent transcript. Per-agent memory is out of scope.

## Architecture

### 1. Name normalization — `crates/crew-hive/src/agentname.rs` (new)

One shared answer to "what is a legal agent name", because names are about to
become LLM-authored.

```rust
/// Normalize `raw` to `^[a-z0-9-]{2,20}$`, or None if nothing survives.
pub fn slug(raw: &str) -> Option<String>
/// `slug`, falling back to a derived `specialist-{id}`.
pub fn slug_or(raw: &str, id: u64) -> String
```

Rules: trim, lowercase, whitespace → `-`, drop every char outside
`[a-z0-9-]`, collapse repeated `-`, trim leading/trailing `-`, clamp to 20,
require ≥ 2 chars.

The charset is the point: `@`, `+`, `/`, and whitespace become
unrepresentable. Every consumer already assumes this and none enforce it —
`relay.rs:209` terminates a name at whitespace, `relay.rs:192` reserves `+` as
the multi-target separator, `chatcomplete.rs:86` bails on whitespace entirely,
and `stdio.rs:229` routes on a leading `/`.

Lives in crew-hive (not crew-plugin) because `parse_plan` is the enforcement
point and crew-plugin already depends on crew-hive.

`plugins.rs::from_manifest` (`:52-63`) currently does a weaker version of this
(lowercase + whitespace → `-`, no charset filter) and switches to `slug`.
This tightens manifest names: a manifest with `+` or `@` in its name is now
normalized rather than registered verbatim. That is a behaviour change for an
edge case that was already broken — such a name could never be `@`-dialled.

### 2. Specialty in the plan schema — `crates/crew-hive/src/planner/mod.rs`

`PlanNode` gains `specialty: Option<String>` and `expertise:
Option<String>`; `TaskSpec` (`graph/spec.rs`) gains `specialty: String` and
`expertise: String`. `PLANNER_SYSTEM` asks for both:

> Each task is an object with integer `id`, short `title`, a `prompt`, `deps`,
> `specialty` — a one-or-two-word name for the kind of specialist the task
> needs (e.g. "archivist", "analyst", "skeptic") — and `expertise`, a short
> comma-separated phrase naming that specialist's craft (e.g. "records,
> retrieval, provenance"). Name the specialist for its craft, not for the task.

Two fields, because they serve different consumers and normalize differently.
`specialty` becomes an `@`-handle and must be a strict slug. `expertise` is
the role hint — it fills `ApiAdapter.role`, the peer list (`registry.rs:72`),
the palette description (`chatpalette.rs:96`), and the `AgentInfo.role` wire
field, and it shapes the specialist's system prompt. It stays prose, so it is
clamped rather than slugged: trim, collapse whitespace, drop control
characters, clamp to 60 chars. An empty or unsalvageable `expertise` degrades
to `""` — exactly what `role_for` returns for unknown agents today, so every
consumer already handles it.

`parse_plan` runs every specialty through `slug_or(s, n.id)` and every
expertise through the clamp.

**The security invariant is unchanged and its doc comment must say why.**
`AgentKind::Api` is still forced for every task; `specialty` is inert data — a
display label, an `@`-handle, and a role hint fed into the specialist's system
prompt. It never selects an executor, so it cannot reach the `Pty`
command-injection sink the invariant exists to guard. It is normalized anyway
because it becomes an addressable handle. The existing `debug_assert!` gains a
companion asserting every specialty is a valid slug, so a regression fails
loudly in the same place.

### 3. Specialist store — `crates/crew-plugin/src/broker/specialists.rs` (new)

`./.crew/specialists.json`, alongside the existing `session-live.md`.

```json
[{ "name": "archivist", "role": "records, retrieval, provenance",
   "last_used": 1784066194783, "runs": 3 }]
```

`name` is the planner's slugged `specialty`; `role` is its clamped
`expertise`.

```rust
pub fn load() -> Vec<Specialist>          // absent/corrupt → empty
pub fn record(seen: &[Specialist])        // merge by name, bump last_used, evict LRU
pub const CAP: usize = 12;
```

No system prompt and no model id are stored: the prompt is derived from
`name` + `role` at construction, and the model comes from live provider
discovery. This keeps a stored specialist from pinning a retired model slug.

**Why a file is the right mechanism, not merely persistence.**
`Session::registry()` (`session.rs:91`) calls `Registry::discover_with` on
every `hello`, every send, every construct — the registry is rebuilt from
scratch each time and there is no long-lived instance to hold an accumulating
roster. A file makes `roster_with` re-read the store on each rebuild, so
accumulation and durability are one mechanism instead of two, and no new
mutable state has to be threaded through `Session`. It is also the seam the
e2e suites seed through.

Disk I/O per rebuild is acceptable: this runs in the broker process, not on
the app's winit thread. Writes are atomic (tmp + rename). A write failure is
logged and ignored — specialists are a convenience, not the run's product, and
must never fail a run.

### 4. Runtime specialists — `crates/crew-plugin/src/broker/apiadapter.rs`

- `inbuilt_agents()` and its `[(&str, ModelTier, &str); 3]` array: **deleted**.
- `ApiAdapter` gains a `role: String` field and overrides `Adapter::role()`.
  Today it does not, so `role()` falls through to `agents::role_for(name)`
  (`adapter.rs:49`), a hard-coded match on the six known literal names — an
  invented specialist would render with an empty role in the palette
  (`chatpalette.rs:96`), the peer list (`registry.rs:72`), and the
  `AgentInfo.role` wire field.
- New constructor building an adapter from a stored specialist. Its `role` is
  the stored `role` (the planner's `expertise`), and its system prompt is
  derived: *"You are the {name}. Your specialty is {role}. …"*, following the
  shape of the deleted inbuilt prompts.
- `agents::role_for` keeps its CLI-agent arms (`claude`/`codex`/`opencode`)
  and drops the trio arms.

`discover::roster_with` composes: stored specialists (as `ApiAdapter`s on the
discovered provider) + manifest plugin agents. `provider_and_model()`
(`discover.rs:155`) already hands back a provider and a concrete model id and
is already used by `swarm::backend()`, so the planner and the roster share one
provider handle.

### 5. Registration and re-emission — `crates/crew-plugin/src/broker/swarm.rs`

After a plan lands and before the scheduler runs: collect the distinct
`(specialty, expertise)` pairs from the `TaskSpec`s, call
`specialists::record`, then emit `PluginEvent::Roster { agents: reg.infos() }`.
Where two tasks in one plan share a `specialty` but give different
`expertise`, the first task's wins — one name is one specialist, and the
alternative (suffixing) was rejected in the bounding decision.

The re-emission is load-bearing. `Roster` is currently emitted only from
`hello()`, so without this the UI's `agents` vec never learns about a
specialist invented mid-session and the new names would never appear.

### 6. Activity keyed by specialty — `crates/crew-plugin/src/broker/swarmmsg.rs`

`translate` currently names agents by task title (`title_of(task)`). It
switches to the task's specialty. Roster rows then light up, because
`chatview::agent_views` sets `active` by matching `active_names()` against
roster names — with titles, that match could never hit.

Task **titles** stay as-is in the swarm block's task rows
(`chatswarmview::block_cells`) and the header status line: the block answers
"what work is happening", the roster answers "who is doing it".

### 7. Messaging — `crates/crew-plugin/src/broker/stdio.rs`

`roster()` (`:248`) currently conflates two states under one message: *"No
inbuilt agents available. Set OPENROUTER_API_KEY, …"*. With an empty starting
roster this would tell a correctly-keyed user to set a key they already have.
Split on `provider_and_model().is_some()`:

- no provider → the existing set-a-key message.
- provider, no specialists → *"No specialists yet — type a task and the
  planner will invent the ones it needs."*

`relay_counting`'s empty-registry short-circuit (`:270`) uses the same split.

### 8. Unknown `@name` — `crates/crew-plugin/src/broker/relay.rs`

A latent bug, in code this change touches and makes more likely to fire.
`split_target` (`:206`) resolves an unknown name by falling back to the *first*
registered agent and leaving `@typo` in the prompt body. Nothing reports it —
the comment at `:184` claiming the single-target path "reports it" is simply
wrong. With LLM-authored names, typos get likelier and the fallback target
gets arbitrary.

`split_target` returns an explicit unknown-agent outcome; `relay_counting`
reports *"No agent named @typo. Known: archivist, analyst."* The `@a+b`
multi-target path (`:185-202`) already returns `None` on any miss and then
falls into this same path, so it is fixed by the same change.

### 9. App side

No structural change. The roster renders `self.agents`; re-emitted `Roster`
events update it; `Activity` keyed by specialty lights the rows.

`chatroster::agent_color` hashes the agent name into a signature hue, so
invented specialists get distinct colours for free — this closes the
"signature accent hue" backlog item (`progress.md:893`), which was explicitly
filed to be folded into this work.

## Error handling

| Condition | Behaviour |
|---|---|
| Store absent / corrupt / bad JSON | Empty roster; no crash |
| Store write fails | Logged, run continues |
| `specialty` missing or unsalvageable | Derived `specialist-{id}` |
| `expertise` missing or unsalvageable | `""` — same as `role_for`'s unknown-agent arm today |
| Two tasks share a specialty, differing expertise | One specialist; first task's expertise wins |
| Plan omits both fields (old model) | Every task derives a name, empty role; run proceeds |
| Cap exceeded | Evict least-recently-used |
| Provider absent | Empty roster + set-a-key message |

## Testing

**crew-hive.** `slug` unit tests: whitespace, `@`/`+`/`/`, punctuation, unicode,
empty, over-length, repeated/leading/trailing `-`, ≥2-char floor.
`parse_plan`: specialty slugged; missing → derived; garbage → derived;
expertise clamped (whitespace collapsed, control chars dropped, over-length
truncated); missing expertise → `""`; the existing Pty-forcing security tests
retained plus a valid-slug assertion.

**crew-plugin.** Store: merge by name bumps `last_used` and preserves role;
LRU evicts the right one at cap 12; atomic write; corrupt file → empty.
`ApiAdapter`: `role()` returns its own field. `roster_with`: composes stored
specialists + manifest agents. `stdio::roster`: the no-key vs no-specialists
split. `relay::split_target`: unknown name reports instead of defaulting.

**e2e.** The store file is the seam.
- `e2e_discovery::discovery_lists_the_inbuilt_roster` → replaced by
  `discovery_reports_empty_roster_until_a_run_invents_specialists`: a keyed
  broker reports 0 agents and the hint. This test's subject was the trio; it
  inverts rather than adapts.
- `shell_env_probe_recovers_missing_provider_key` asserts `"3 agent(s)"` only
  incidentally — it should assert key recovery directly, not a roster count.
- `at_selector_starts_with_chosen_agent`, `e2e_relay`, `e2e_tasks`: seed the
  store in a temp project dir and swap `@planner` for the seeded name.

## Risks

- **Cold start.** A keyed user with no history sees an empty roster until
  their first run. Accepted: plain messages already route to the swarm
  (`stdio.rs:229`), so nothing is blocked — only `@`-dial is unavailable, and
  the message says why.
- **Specialty quality depends on the model.** A weak planner may invent bland
  or repetitive names. The prompt steers ("name for its craft, not the task")
  and merge-by-name means repetition converges rather than proliferates.
- **Project-scoped store.** Specialists do not follow the user across
  projects. Chosen deliberately: a project's experts are project-shaped.
- **Prior-decision reversal.** Documented above.
