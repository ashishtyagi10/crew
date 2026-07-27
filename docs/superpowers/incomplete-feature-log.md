# Incomplete-feature loop — log

Playbook: `incomplete-feature-loop.md`. Baseline at loop start: **v0.6.93** on
`main`. Each iteration appends one section below; iteration numbering starts at 1.

The loop finishes features Crew already started — half-wired surfaces,
documented-but-missing behaviour, unreachable working code. A clean marker sweep
is not an empty hunt; see the playbook's six lenses.

---

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
