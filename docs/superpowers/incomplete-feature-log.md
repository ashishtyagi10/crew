# Incomplete-feature loop — log

Playbook: `incomplete-feature-loop.md`. Baseline at loop start: **v0.6.93** on
`main`. Each iteration appends one section below; iteration numbering starts at 1.

The loop finishes features Crew already started — half-wired surfaces,
documented-but-missing behaviour, unreachable working code. A clean marker sweep
is not an empty hunt; see the playbook's six lenses.

---

## Iteration 2 — 2026-07-27 16:52 EDT — RELEASED v0.6.95
- Gap: the composer's slash palette described `/goal` as "set the crew's shared
  goal" — a feature that exists in no engine. The broker runs relay rounds until
  a judge rules `MET:`. Missing leg: **inverse-docs**. Membership of the three
  construct lists is pinned in every direction by four existing tests; their
  DESCRIPTIONS were pinned by nothing, and a fourth hand-written copy of them
  (`chatcomplete::describe`) had drifted.
- Evidence: `describe("/goal")` vs `HELP`'s `/goal` line, side by side. No test
  could have caught it — every membership test passed the whole time.
- Fix: `crew_plugin::construct_summary(name)` exposes the broker's own `/help`
  line; `describe()` returns it for every offered construct except a short,
  declared `PANE_WORDS` list where the pane genuinely changes what is worth
  saying (`/plan`'s enter/esc, `/memory`'s `#<note>`, "this list", and the three
  pane-local constructs). Also documented the `/goal` name collision — composer
  = judged relay, command bar = swarm task graph — in `docs/CREW.md` and the
  command-bar palette. [broker/commands.rs, broker/mod.rs, lib.rs,
  chatcomplete.rs, cmddefs.rs, docs/CREW.md]
- Class closed: the fourth copy is gone. A hint is now the broker's sentence
  unless someone deliberately overrode it, and `derived_hints_are_the_brokers_own_words`
  asserts exactly that; `pane_words_only_override_offered_constructs` keeps the
  override list from accumulating dead rows.
- Docs: `docs/CREW.md` `/goal` entry (collision note), `CHANGELOG.md` 0.6.94 + 0.6.95.
- Gate: fmt ok · clippy clean · tests 2027 pass, 0 failures
- Release: v0.6.95
- **Process bug found and fixed in the playbook:** iteration 1 gated BEFORE
  bumping the version, which turned `changelog_covers_the_current_version` red
  after the gate had already passed — and `release.yml` only builds, so v0.6.94
  shipped with a red test and no CHANGELOG entry. Both entries written
  retroactively; the playbook now says bump-then-gate.
- Candidates found, not fixed: carried forward from iteration 1 (dead-code
  allows, transcript litter, undocumented env vars), plus:
  - **`docs/CREW.md` internal anchors are unchecked.** Writing `#swarm` instead
    of `#swarm-orchestration-crew-hive` produced a silently broken link that only
    a manual grep caught. Nothing validates in-document anchor targets.

## Iteration 1 — 2026-07-27 16:31 EDT — RELEASED v0.6.94
- Gap: the three `DIRECT` providers — `openai`, `gemini`, `deepseek` — ship fully
  wired (key var, endpoint, native model chain, catalog vendor, model-picker
  routing) and are named nowhere in `README.md` or `docs/CREW.md`. Missing legs:
  **docs** and **reachability** — they probe last by design, so `CREW_PROVIDER=gemini`
  is the only way to reach one when another key is set, and that pin was itself
  undocumented (`pick_provider`'s own doc comment listed only the original three).
- Evidence: `every_direct_provider_is_documented` RED on all three rows before
  the fix; `grep -i "gemini\|deepseek" docs/CREW.md README.md` returned one hit,
  an unrelated OpenRouter slug.
- Fix: documented all three in `docs/CREW.md` as a "Direct vendor keys" table
  (key, default chain, chain override, endpoint override) plus the last-probe
  ordering and why; widened the `CREW_PROVIDER` line to list all six; corrected
  `pick_provider`'s doc comment; pointed README's provider-stack line at it.
  [docs/CREW.md, README.md, broker/discover.rs, broker/discover_tests.rs]
- Class closed: `every_direct_provider_is_documented` reads `docs/CREW.md` via
  `include_str!` and asserts every `DIRECT` row's name, key var, chain env and
  base-url env appear in it. A new provider row cannot land undocumented — the
  same mechanism `broker_constructs()` uses, which is the only thing that has
  ever kept two lists together here. Second test pins `CREW_PROVIDER=<direct>`.
- Docs: `docs/CREW.md` "Models & rate-limits" (+ new table), `README.md:184`.
- Gate: fmt ok · clippy clean · tests 2024 pass (26 binaries, 0 failures)
- Release: v0.6.94
- Candidates found, not fixed:
  - **`/goal` means two different things.** `README.md:365` documents `/goal <text>`
    as an app-side swarm pane that plans a task graph (`LlmPlanner` + `ApiAgent`);
    `broker/constructs.rs:69` implements `/goal <text>` as relay rounds judged by
    a critic agent. Same word, two engines. Which one a user gets is undocumented.
  - **The app litters the repo with transcripts.** `crew-transcript-<ts>.md` files
    are written into the CWD (4 accumulated in `crates/crew-app/` during this
    iteration alone) and `.gitignore` does not cover them.
  - **`#[allow(dead_code)]` outside platform `cfg_attr`** — `osc7.rs:170`,
    `broker/mod.rs:103`, `chatpulse.rs:58,67`, `chatflow.rs:13`.
  - **`session.rs` is 248 lines** and 130 `.rs` files exceed the stated 200-line
    cap; the guardrail is aspirational. Not a per-iteration fix — flagged only so
    a future iteration does not mistake it for one.
  - **Undocumented env vars** beyond the provider set: `CREW_HTTP_TIMEOUT_MS`,
    `CREW_PROJECT_DIR`, `CREW_STREAM_TEXT`, `CREW_CHAT_PLUGIN`,
    `CREW_ORCHESTRATOR_PLUGIN`, `CREW_CREDENTIALS_PATH`. Triage which are
    user-facing vs. internal/test-only before documenting.
