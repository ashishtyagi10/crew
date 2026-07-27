# Incomplete-feature loop — log

Playbook: `incomplete-feature-loop.md`. Baseline at loop start: **v0.6.93** on
`main`. Each iteration appends one section below; iteration numbering starts at 1.

The loop finishes features Crew already started — half-wired surfaces,
documented-but-missing behaviour, unreachable working code. A clean marker sweep
is not an empty hunt; see the playbook's six lenses.

---

## Iteration 5 — 2026-07-27 18:02 EDT — RELEASED v0.6.98
- Gap: six `CREW_*` knobs the shipped source reads appear in no doc —
  `CREW_HTTP_TIMEOUT_MS`, `CREW_STREAM_TEXT`, `CREW_BROKER_PLUGIN`,
  `CREW_CHAT_PLUGIN`, `CREW_ORCHESTRATOR_PLUGIN`, `CREW_PANE`. Missing leg:
  **docs**. Also **inverse-docs**: `CREW_SYS_TIMEOUT_MS` is documented in two
  places with two different defaults, the stale one still claiming 30000 after
  this session's own first commit made it 120000.
- Evidence: every `CREW_*` token in `crates/*/src` diffed against README +
  CREW.md; 15 unmatched, 9 of them genuinely internal.
- Fix: documented the six (the plugin overrides as one paragraph about pointing
  a pane at a debug build while the app stays on the release), corrected the
  stale default. [docs/CREW.md]
- Class closed: `every_env_knob_is_documented_or_declared_internal` walks the
  crates' `src/` trees, extracts every `CREW_*` name, and requires each to be in
  the docs or in a declared `NOT_USER_FACING` list. Verified it has teeth by
  removing one declaration and watching it go red. The declared list is the
  point — it makes "internal seam" a decision rather than an oversight, the same
  idiom as `SENT_BY_THE_PANE`.
- Docs: `docs/CREW.md` tuning paragraph + new plugin-override paragraph,
  `CHANGELOG.md` 0.6.98.
- Gate: fmt ok · clippy clean · tests 2033 pass, 0 failures
- Release: v0.6.98
- Candidates found, not fixed: doc anchor validation; `osc7.rs:170`'s test-only
  helper marked `#[allow(dead_code)]` instead of `#[cfg(test)]`; commands that
  report success on a no-op (the `/export` shape) are not systematically checked.

## Iteration 4 — 2026-07-27 17:41 EDT — RELEASED v0.6.97
- Gap: `/export` wrote a transcript file even with zero messages, and reported
  success. Missing leg: **error-path stub** — the empty case was never
  considered, so the failure was silent and looked like success every time.
- Evidence: 64 `crew-transcript-*.md` files in `crates/crew-app/`, every one
  exactly 68 bytes containing "0 message(s)"; **54 were tracked in git**. New
  ones appeared during this session's own runs.
- Fix: `intercept` answers an empty pane with "nothing to export" and does not
  touch the disk; `exporting_an_empty_pane_writes_no_file` counts stray files in
  the crate root before and after, so a regression is caught by the litter it
  would create rather than by inspection. Deleted all 64 files (verified
  byte-identical and message-free first) and gitignored the pattern so a real
  export made inside a repo cannot be committed by accident.
  [chatexport.rs, .gitignore]
- Class closed: partially. The guard plus the ignore rule stops this specific
  litter. The broader shape — commands that report success on a no-op — is not
  systematically checked anywhere; noted for a future iteration.
- Docs: `CHANGELOG.md` 0.6.97.
- Gate: fmt ok · clippy clean · tests 2032 pass, 0 failures
- Release: v0.6.97
- Note: the trigger that submitted `/export` to empty panes was not identified —
  the litter's timestamps span months and include this session. The fix is at
  the write site, so it holds whatever the caller was.
- Candidates found, not fixed: carried forward — undocumented env vars, doc
  anchor validation, `osc7.rs:170`'s test-only helper marked
  `#[allow(dead_code)]` instead of `#[cfg(test)]` (the other three dead-code
  allows were inspected and carry explicit keep-for-future rationale with
  tested contracts — deliberate, not oversights).

## Iteration 3 — 2026-07-27 17:24 EDT — RELEASED v0.6.96 — user-requested
- Gap (user request, not a hunt finding): the input bar acts on the focused
  pane and never named it. The cwd rides the top border as a legend; the bottom
  border carried only a transient status, so the bar's standing answer to "where
  does this go?" was nowhere on screen. Missing leg: **reachability** — the
  information existed (`pane_rows`, the sidebar `▸` marker) but not where you
  are looking while typing.
- Evidence: rendered-cell dump of `InputBar::cells` before/after; four new tests.
- Fix: `cells()` takes the focused pane's name and draws it right-aligned on the
  bottom border in `legend_off`, clipped with `…` so a long title cannot overrun
  the `╯` corner. A status flash supersedes it and gives the slot back. The name
  is `title_text()` — the same string the PANES list and the pane's own card
  legend show, so one pane is never called two things on one screen.
  [inputbar_render.rs, navcard.rs, render.rs, inputbar_tests.rs]
- Class closed: n/a — new surface, not a drift. The name is sourced from
  `title_text()` rather than re-derived, which is what would have caused drift.
- Docs: `CHANGELOG.md` 0.6.96.
- Gate: fmt ok · clippy clean · tests 2031 pass, 0 failures
- Release: v0.6.96
- **GUI verification unavailable:** osascript is denied both assistive access
  and screen recording in this environment, so keystroke/screencapture
  verification per `.claude/skills/verify` could not run. The isolated dev
  instance built, launched and reached frontmost; visual confirmation came from
  dumping the real `InputBar::cells` grid as text instead. The headless
  `crew-render` screenshot harness cannot cover this — it hand-rolls its own
  mock input bar rather than calling `InputBar::cells`.

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
