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
