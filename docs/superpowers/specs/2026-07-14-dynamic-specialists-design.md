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
- **Bounding:** cap 24, LRU eviction by last use. Same name = same specialist
  (merged, not suffixed). The cap was raised from 12 after the spike measured
  ~5 new specialists per run with little name reuse — see "Bounding, revised".
- **Roster display:** the roster is row-budgeted (≤ 5 rows, active first, then
  `+N more`); the dial-able set (24) and the displayed set (5) are different
  numbers on purpose.
- **Name guard:** `^[a-z0-9-]{2,28}$`, enforced at the parse boundary.
  Unsalvageable names fall back to a derived `specialist-{id}`. The 28 is
  empirical — see "Prompt spike" below.
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
hint and its place in the LRU order (`last_used`). It does **not** gain a
private conversation history: none exists today. The session log
(`sessionlog.rs:18`) is a single shared prose file (`"{sender}: {text}"`), not
a per-agent transcript. Per-agent memory is out of scope.

## Prompt spike (run 2026-07-14, DashScope `qwen-max`)

Specialty quality is the one risk no unit test catches: the failure isn't a
crash, it's a roster of bland or absurd names. It was also testable before any
implementation, since the only variables are the prompt and the model. Five
prompt revisions were run against six deliberately non-coding goals
(stakeholder comms, CVE audit, trip planning, blog post, conversion-drop
analysis, billing schema). Findings, in the order they were learned:

**The title-echo failure never occurred.** Across ~150 generated names, not one
specialty was merely its task title slugged (`"Gather Project Details"` →
`gather-details`). A `specialty == slug(title)` rejection was considered for
`parse_plan` and **dropped as dead code** — it guards a failure the model does
not commit.

**Bare topic nouns did occur** (v1: `security`, `risk`). Fixed by the "must be
a person, not a subject" rule, which is why that clause exists.

**Few-shot examples backfired badly** (v2). Naming flavourful examples
(`archivist`, `synthesist`, `cryptographer`) turned them into a word bank: the
model produced `synthesist` 13 times and assigned `epidemiologist` to "Check
Weather and Pack" for a holiday. Specific-but-wrong is worse than
generic-but-right. This is why the prompt carries an explicit anti-anchoring
clause instead of an example gallery, and why examples appear only as
contrast pairs ("X, not Y") rather than as a list to choose from.

**Length cannot be enforced by prompt.** v4 stated "20 characters maximum" and
still returned `quality-assurance-engineer` (26), `communication-strategist`
(24), `database-administrator` (22). A word-count rule doesn't bound length
either — `communication-strategist` is two words and 24 chars. Length is
enforced in code, and the char-count claim was removed from the prompt rather
than left in as an instruction the model demonstrably ignores.

**The clamp is 28 because 26 was observed.** ~1 name in 6 exceeds 20 chars, so
the originally-specified 20 would have mangled real output routinely.

**Diversity is good, so no anti-generic machinery is needed.** The final
prompt yields 28 distinct names across 32 tasks. The v1 concern — `analyst`
recurring in 5 of 6 plans and merge-by-name collapsing the roster into one
catch-all — does not survive the fix.

### Bounding, revised

That diversity number cuts both ways, and it invalidated a decision made
before the spike ran. Cap 12 was chosen assuming names would recur and merge.
At 28 distinct / 32 tasks they barely recur: roughly **5 new specialists
arrive per run**, so a 12-cap turns the roster over every ~2.5 runs and
merge-by-name almost never fires. That is a churn window, not an accumulating
network.

The cap is therefore **24** — about five runs of history before eviction
starts, so the LRU trims a tail instead of thrashing.

One honest caveat on the measurement: the six spike goals were deliberately
unrelated (trip planning, CVE audit, blog post). A real project's goals
cluster, so name reuse in practice will be higher than 28/32 and the effective
cap pressure lower. 28/32 is the worst case, and 24 is sized against it.

The harness is kept as an `#[ignore]`d, network-gated test (it needs a real
provider). It asserts the mechanical properties — slug-legality without
mangling, distinctness within a plan, no title echo — and prints the roster
for eyeballing. It exists to re-validate the prompt when the planner model
changes, which matters because the provider chain is DashScope → OpenRouter →
Anthropic and this prompt is tuned on only the first.

## Architecture

### 1. Name normalization — `crates/crew-hive/src/agentname.rs` (new)

One shared answer to "what is a legal agent name", because names are about to
become LLM-authored.

```rust
/// Normalize `raw` to `^[a-z0-9-]{2,28}$`, or None if nothing survives.
pub fn slug(raw: &str) -> Option<String>
/// `slug`, falling back to a derived `specialist-{id}`.
pub fn slug_or(raw: &str, id: u64) -> String
```

Rules: trim, lowercase, whitespace → `-`, drop every char outside
`[a-z0-9-]`, collapse repeated `-`, trim leading/trailing `-`, clamp to 28,
require ≥ 2 chars.

**Truncation is a hard cut, deliberately not at a hyphen boundary.** Boundary
truncation looks tidier and is wrong: `accommodation-specialist` →
`accommodation` converts an agent noun back into a bare topic word, which is
the precise failure the prompt rule exists to prevent. A hard cut leaves an
obviously-mangled name; a boundary cut leaves a plausible-looking wrong one.
At 28 chars this path is nearly unreachable anyway (see the spike).

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
`expertise: String`. `PLANNER_SYSTEM` becomes the spike-validated text:

> You are a task planner. Decompose the user's goal into a JSON array of
> tasks. Each task is an object with integer `id` (0-based), short `title`, a
> `prompt` describing the work, `deps` (array of task ids that must finish
> first), `specialty`, and `expertise`.
>
> `specialty` names the specialist the task needs. Rules:
> - At most TWO words, joined by a hyphen. Never three.
> - It must be a person, not a subject: an agent noun. Write
>   "security-auditor", not "security"; "risk-assessor", not "risk".
> - Name them for their craft, not for the task: a task titled "Gather Project
>   Details" needs an "archivist", not a "gatherer".
> - Use the word a real practitioner of that work would call themselves. Do
>   not borrow vocabulary from these instructions — an example word that does
>   not genuinely fit the goal at hand is worse than a plain one.
>
> `expertise` is a short comma-separated phrase naming that specialist's
> craft, e.g. "records, retrieval, provenance".
>
> Return ONLY the JSON array, no prose.

Every clause is load-bearing and was earned; see the prompt spike above before
editing this prompt.

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
   "last_used": 1784066194783 }]
```

`name` is the planner's slugged `specialty`; `role` is its clamped
`expertise`. There is deliberately no `runs` counter: nothing reads it, and a
field written but never read is a maintenance cost with no consumer.

```rust
pub fn load() -> Vec<Specialist>            // absent/corrupt → empty
pub fn record(seen: &[(String, String)])    // (name, role): merge, bump last_used, evict LRU
pub fn touch(name: &str)                    // bump last_used on @-dial
pub const CAP: usize = 24;
```

`record` takes `(name, role)` pairs, not `Specialist`s: `last_used` is the
store's business, and a caller should not be able to invent it.

**`touch` exists so that use, not just creation, defers eviction.** Without
it, `last_used` only moves when a run happens to re-invent a name — so a
specialist you `@`-dial every day would be evicted by churn from unrelated
runs, which is precisely backwards. `relay_counting` calls it on the resolved
target.

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

### 9. App side — the roster UI must be row-budgeted

The roster UI assumes a ~3-agent roster and breaks quietly at 24. Two
concrete defects, both found in spec review rather than at runtime:

**`chatchips::layout` (`chatchips.rs:77`) spends one pane row per agent,
unbudgeted:** `shown = views.len().min(avail_rows)`, `rows = shown`. With
`avail` being nearly the whole pane (`chatview.rs:137`), a full roster on a
24-row pane claims 12+ rows and leaves ~8 for the conversation. The message
area is the point of the pane; the roster is furniture.

**`chips_on_border` (`chatinput.rs:178`) silently drops overflow:** it
`break`s on the first chip that doesn't fit. Three short names always fit;
`@user-experience-specialist @accommodation-specialist …` will not, so most of
the legend vanishes with no indication it was truncated.

Both get the same treatment:

- `layout` takes a display cap (`ROSTER_MAX_ROWS = 5`) alongside `avail_rows`,
  and `Layout` carries `hidden: usize`. When `views.len() > shown`, the last
  row renders `+N more` in the muted style.
- `agent_views` orders **active first, then by recency**, so a run's working
  specialists are always the ones on screen and the tail is what gets hidden.
  Ordering lives there (not in `layout`) because it needs `active` and
  `last_used`, which are pane state.
- `chips_on_border` renders a `+N` chip in the border style when it runs out
  of room, and returns the same "first free column" contract it does today.

The dial-able set (24) and the displayed set (5) are different numbers on
purpose: `@`-completion (`chatcomplete.rs`) and the palette keep reaching the
full roster, so a hidden specialist is still one keystroke away. Only the
always-on display is budgeted.

`chatroster::agent_color` hashes the agent name into a signature hue, so
invented specialists get distinct colours for free — this closes the
"signature accent hue" backlog item (`progress.md:893`), which was explicitly
filed to be folded into this work.

The slug charset pays off here too: `chips_on_border` measures with
`chip.len()` (bytes), which is only correct because `[a-z0-9-]` is ASCII, so
byte length equals display width. A non-ASCII agent name would already
mis-measure that legend today.

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

**Prompt regression (`#[ignore]`d, network-gated).** The spike harness,
promoted into the repo: runs `PLANNER_SYSTEM` against the six fixed goals on
the live provider and asserts every specialty is slug-legal without mangling,
distinct-within-plan, and not a slugged task title; prints the cast for
eyeballing. Ignored by default — it costs API calls and needs a key.

**crew-hive.** `slug` unit tests: whitespace, `@`/`+`/`/`, punctuation, unicode,
empty, over-length (hard cut, not at a hyphen boundary), repeated/leading/
trailing `-`, ≥2-char floor, the 28-char ceiling.
`parse_plan`: specialty slugged; missing → derived; garbage → derived;
expertise clamped (whitespace collapsed, control chars dropped, over-length
truncated); missing expertise → `""`; the existing Pty-forcing security tests
retained plus a valid-slug assertion.

**crew-plugin.** Store: merge by name bumps `last_used` and preserves role;
LRU evicts the right one at cap 24; `touch` defers eviction for a dialed but
un-re-invented specialist; atomic write; corrupt file → empty.
`ApiAdapter`: `role()` returns its own field. `roster_with`: composes stored
specialists + manifest agents. `stdio::roster`: the no-key vs no-specialists
split. `relay::split_target`: unknown name reports instead of defaulting.

**crew-app.** `layout` caps at `ROSTER_MAX_ROWS` and reports `hidden`;
`agent_views` orders active-then-recent so a working specialist is never the
one hidden; `+N more` renders only when something is hidden; `chips_on_border`
emits a `+N` chip instead of silently dropping overflow, and still returns the
first free column. A 24-agent roster on a 24-row pane must leave the message
area usable — the regression the row budget exists to prevent.

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
- **Specialty quality on other providers.** Closed for DashScope `qwen-max` by
  the spike above (28 distinct names / 32 tasks, no title echoes, no topic
  nouns). It is *not* closed for OpenRouter or Anthropic, the other two links
  in the provider chain — the prompt is tuned on one model. The `#[ignore]`d
  harness is how that gets checked; running it against the other two is
  follow-up work, not a blocker, since a weak name degrades the roster rather
  than breaking a run.
- **Prompt edits can silently regress.** Every clause in `PLANNER_SYSTEM` was
  earned against a specific observed failure, and the v2 result shows a
  well-intentioned edit (adding examples) can make output dramatically worse
  while still looking reasonable in review. The spike section documents which
  clause defends against what, so a future editor knows what they'd be
  removing.
- **Project-scoped store.** Specialists do not follow the user across
  projects. Chosen deliberately: a project's experts are project-shaped.
- **Concurrent writers lose updates.** Two crew instances in one project write
  the same store; the atomic tmp+rename keeps the file from corrupting, but
  the later writer clobbers the earlier one's additions. Accepted: the loss is
  a specialist that has to be re-invented, and per-instance collision is
  already a known open issue (`progress.md:868`, per-instance socket path).
  Not worth a lockfile at this size.
- **Prior-decision reversal.** Documented above.
