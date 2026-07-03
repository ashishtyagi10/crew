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

## Iteration 4 — 2026-07-03 03:52 EDT — RELEASED v0.5.49
- Feature (codex): `sys` read-only sandbox — `CREW_SYS_MODE=readonly` blocks `sys:run` and `sys:write_file` (mutating tools) while keeping `read_file`/`list_dir`; gated at the single `systools::call` entry point (no bypass). [crew-plugin/src/broker/systools.rs]
- UI: chat cards use a lighter dotted gutter (`┆`) for the system/broker voice so agent replies stand out. [crew-app/src/chatmsgs.rs]
- Token opt: consecutive byte-identical relay transcript entries are de-duplicated before storage (budget enforcement already existed, so this was the chosen fallback). [crew-plugin/src/broker/engine.rs]
- UX: `/status` reports the sys sandbox mode (`sys: full` / `sys: read-only`). [crew-plugin/src/broker/commands.rs]
- Gate: fmt ok · clippy clean · tests 921 pass · security review CLEAN (sandbox-bypass focus; 0 confirmed).
- Release: v0.5.49.
- Crossed off menu: codex approval/sandbox profiles for sys:run.

## Iteration 5 — 2026-07-03 04:52 EDT — RELEASED v0.5.50
- Feature (opencode): `/theme` pane command — lists the 5 built-in themes (current marked) and switches by name (`/theme paper-light`), reusing crew-theme's `from_name`/`set_theme`; new `chattheme.rs` module. [crew-app/src/chattheme.rs, chat.rs, chatcomplete.rs]
- UI/UX: far-panel legend shows `· empty` for a directory with no entries (instead of `· 0 · 0 B`). [crew-app/src/farpane/render.rs] (The crew-pane empty-state hint from the plan already existed pre-branch, so no dup was added.)
- Token opt: the relay frame caps very long task text at 4 KB (context-window guard) so a huge task doesn't cost its full length on every hop. [crew-plugin/src/broker/route.rs]
- Gate: fmt ok · clippy clean · tests 928 pass · security review CLEAN (0 confirmed; /theme global-state + panic focus).
- Release: v0.5.50.
- Crossed off menu: opencode theme presets/switcher.
- Note: crew-pane empty-state hint & `/export` path feedback were already implemented, so this iteration shipped 3 net-new changes rather than 4.

## Iteration 6 — 2026-07-03 05:52 EDT — RELEASED v0.5.51
- Feature (claude-code): `/compact` pane command — folds older crew-pane messages into a `(compacted N earlier messages)` marker, keeping the last 20 (or `/compact <n>`); new `chatcompact.rs`. [crew-app/src/chatcompact.rs, chat.rs, chatcomplete.rs]
- Token visibility: `/status` reports the relay token budget (`budget: unlimited` / `~N tok` from CREW_BROKER_TOKEN_BUDGET). [crew-plugin/src/broker/commands.rs]
- UX: `/export` confirmation now includes the message count (`transcript exported (N messages) → path`). [crew-app/src/chatexport.rs]
- Gate: fmt ok · clippy clean · tests 935 pass · security review CLEAN (0 confirmed; message-Vec panic focus).
- Release: v0.5.51.
- Crossed off menu: claude-code /compact transcript summarizer.
- Note: the unread "N new" indicator (planned UI change) already existed (chatscroll::new_pill_cells), so 3 net-new changes shipped.

## Iteration 7 — 2026-07-03 06:52 EDT — RELEASED v0.5.52
- Feature (codex): `/cwd` construct — shows the broker's working directory (where sys tools operate) and the sandbox mode. [crew-plugin/src/broker/commands.rs, chatcomplete.rs]
- UI: composer shows a dim right-aligned char-count badge (`Nc`) when input exceeds 120 chars (bordered-pane variant). [crew-app/src/chatinput.rs]
- Token opt: the relay frame now trims surrounding whitespace from the task text (paid on every hop) before the 4 KB cap. [crew-plugin/src/broker/route.rs]
- UX: `/status` pluralizes correctly (`1 task running` / `N tasks running`, no more `task(s)`). [crew-plugin/src/broker/commands.rs]
- Gate: fmt ok · clippy clean · tests 946 pass · security review CLEAN (0 confirmed).
- Release: v0.5.52.
- Crossed off menu: codex working-directory surfacing (`/cwd`).

## LOOP COMPLETE — 2026-07-03 07:56 EDT
The final cron fire (the 07:26 slot, delayed to 07:56 by jitter + idle) landed
with only ~4 minutes to the 08:00 hard stop. A full iteration (implement →
security review → gated release) takes ~35–40 min and would have auto-released
well past 08:00, overrunning the "until 8:00 AM" window — so iteration 8 was NOT
started. The cron (`dcfee9b6`) was deleted; the loop is stopped.

**Summary: 7 iterations, 7 green gated releases, 0 blocked, 0 rollbacks.**
Baseline v0.5.45 (long-running-agents, merged before the loop) → v0.5.46 … v0.5.52.
Every release passed fmt + clippy(-D warnings) + full `cargo test --workspace` +
an adversarial security review (0 Critical/High across all 7) and was verified
published (all 5 targets incl. Windows + assets). main is releasable at v0.5.52.

| Iter | Source      | Version | Highlights |
|------|-------------|---------|------------|
| 1 | codex       | v0.5.46 | /diff, far-panel count, prompt compaction, /help tip |
| 2 | opencode    | v0.5.47 | fuzzy completion, far-panel size, /status per-turn, task capacity |
| 3 | claude-code | v0.5.48 | slash aliases, composer placeholder, transcript skip-empty, did-you-mean |
| 4 | codex       | v0.5.49 | sys read-only sandbox, system gutter, transcript dedup, /status sys mode |
| 5 | opencode    | v0.5.50 | /theme switcher, far-panel · empty, task-length cap |
| 6 | claude-code | v0.5.51 | /compact, /status budget, /export count |
| 7 | codex       | v0.5.52 | /cwd, composer char badge, task-trim, /status pluralization |
