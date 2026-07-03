# Nightly autonomous improvement loop — log

Window: 2026-07-03, hourly (cron `26 0-7 3 7 *`, EDT) until the 08:00 hard stop.
Baseline at loop start: **v0.5.45** on `main` (concurrent background tasks / long-running agents).
Cron job id: `dcfee9b6` (session-only). Playbook: `nightly-loop-playbook.md`.

Each iteration appends a section below. Iteration numbering starts at 1.

---

## Iteration 1 — 2026-07-03 00:52 EDT — RELEASED v0.5.46
- Feature (codex): `/diff` construct — shows `git diff --stat` of the working tree in the crew pane, bounded (4 KB cap), "working tree clean" empty state. [crew-plugin/src/broker/diff.rs, commands.rs, chatcomplete.rs]
- UI: far-panel legend now shows the entry count (`name · N`). [crew-app/src/farpane/render.rs]
- Token opt: `route::compact_ws` collapses 3+ newlines→2 and strips trailing whitespace on every relay prompt (`frame`), cutting padding tokens per hop. [crew-plugin/src/broker/route.rs]
- UX: `/help` gained a discoverability tip line for `/tasks` + `/stop #n`. [commands.rs]
- Gate: fmt ok · clippy clean · tests 897 pass · security review CLEAN (0 findings; adversarial 4-lens + skeptics).
- Release: v0.5.46 (tag pushed; see release run below).
- Crossed off menu: codex `/diff` pane.

## Iteration 2 — 2026-07-03 01:52 EDT — RELEASED v0.5.47
- Feature (opencode): fuzzy (subsequence) Tab-completion fallback in the composer — `/gl`→`/goal`, `@pnr`→`@planner`; only fires when prefix matching finds nothing and exactly one candidate fuzzy-matches, so existing behaviour is unchanged. [crew-app/src/chatcomplete.rs]
- UI: far-panel legend now also shows the panel's total size (`name · N · <size>`) via the existing `fmt_size`. [crew-app/src/farpane/render.rs]
- Token opt: `/status` reports average tokens/turn (`~{tok}/turn`) so per-turn cost is visible at a glance. [crew-plugin/src/broker/commands.rs]
- UX: task-start line shows live pool capacity (`▸ task #n started · n/max · label`). [crew-plugin/src/broker/stdio.rs]
- Gate: fmt ok · clippy clean · tests 904 pass · security review CLEAN (0 findings; adversarial 3-lens + skeptics).
- Release: v0.5.47.
- Crossed off menu: opencode fuzzy command palette.

## Iteration 3 — 2026-07-03 02:52 EDT — RELEASED v0.5.48
- Feature (claude-code): single-letter slash aliases expanded at send time (`/s`→`/status`, `/t`→`/tasks`, `/h`→`/help`, `/a`→`/agents`, `/d`→`/diff`, `/m`→`/model`); applied before routing so aliases reach the stdio interceptors too. [crew-plugin/src/broker/commands.rs, stdio.rs]
- UI: composer shows a dim placeholder hint when empty (`type a task · / for constructs · @ to pick an agent`). [crew-app/src/chatinput.rs]
- Token opt: empty/whitespace agent replies are no longer stored in the relay transcript, so they stop costing tokens on every subsequent hop. [crew-plugin/src/broker/engine.rs]
- UX (claude-code "did you mean"): an unknown construct suggests the closest match (`/stauts` → "did you mean /status?"). [crew-plugin/src/broker/commands.rs]
- Gate: fmt ok · clippy clean · tests 915 pass · security review CLEAN (0 confirmed; adversarial scan + skeptics).
- Release: v0.5.48.
- Crossed off menu: claude-code slash-command aliases.
