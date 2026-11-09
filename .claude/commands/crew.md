# Crew loop — one change per round, merged; one release per day

Autonomous enhancement loop for **crew**, the native GPU terminal (Rust,
`winit` + `wgpu` + `glyphon`) with agent smith built in. Each round ships
**one** improvement end to end and ends **committed and merged into main**.
Rounds never release: crew cuts at most ONE release per calendar day (see
"Releases").

Mode: $ARGUMENTS — `[lanes] [loop N | loop Nh | one]`
- **lanes** (comma-separated, default all three, rotated round by round):
  - `ai` — the harness, models, routing, memory (was `/improve`). Charter:
    `.superpowers/sdd/ai-loop-2026-09-14.md`.
  - `smith` — agent smith's depth and its pane (was `/smith-loop`). Charter:
    `.superpowers/sdd/smith-loop-2026-10-16.md`.
  - `ui` — the terminal, panes, rendering, themes, terminal emulation (was
    the old `/crew`). Charter: the newest `.superpowers/sdd/ui-loop-*.md`, or
    start a new one.
- `loop N` = N rounds (default 3, cap 12); `loop Nh` ≈ N hours; `one` = a
  single round; empty = propose the top three rows per chosen lane and ask.

Read `MEMORY.md` and the linked notes first: they outrank this file. Then
read the lane's charter — it is the loop's memory, it holds the ranked
backlog and the shipped table, and each round writes its row there.

## Guardrails (these outrank any idea)

- Crew is a native GPU terminal, not a TUI. In-pane UI only — laid into a
  `ratatui` `Buffer` and drawn as GPU cells; no overlays.
- Panes auto-tile into a near-square grid; no layout switcher (tried, failed).
- Panels are rounded cards with a fieldset legend, never a title bar.
- Keys pass through inside a focused terminal pane, except crew's own chords.
- Every `.rs` file ≤ 200 lines, total. Split along responsibility before you
  cross it. A file in `crates/crew-app/line-cap-debt.txt` may not GROW.
- No new dependency without checking the workspace lacks it.
- New capability arrives as behaviour, not a new slash command, unless no
  other door exists. Fold spellings; never break one (`/look` is the pattern).
- Durable learning goes into the recall graph
  (`crew-plugin/src/broker/recall/`), not a second store.
- Bring-your-own-provider; default to the latest, most capable models.
- Never force-push. Never commit a red gate. If the same idea fails the gate
  three times, drop it and take the next row.

## What every round must serve

1. **Intuitive, responsive UI** — nothing new on screen without a shot; motion
   off costs no frames and is a still frame that says the same thing; it
   holds in every palette, including glass over a light desktop (bright white
   words on glass is the user's taste).
2. **Autonomous, agentic** — smith decides and acts; the model drives tools,
   plan, verification and memory; constants are backstops. Asking is the
   exception, and every ask is one keystroke.
3. **Depth, not breadth** (smith) — what would a staff engineer do here that
   smith doesn't? Understand before changing, reproduce before fixing, root
   cause, smallest change, evidence, self-review, name the uncertainty.
4. **Feedback that earns trust** — outcome first, then what changed, why,
   how it was verified (the command and its result), what is left or risky.
5. **Everything needed, nothing more** — a round that adds rows to a turn
   removes or folds others; no walls of tool calls; numbers beat adjectives.
   A round may RETIRE or FOLD commands.

Steal shamelessly from Claude Code, Codex, Cursor, Amp, Gemini CLI, Kiro,
Cline, Grok Build, Ghostty, WezTerm, Kitty, Warp, zellij — and say in the
changelog line which tool the idea came from.


## What is current (refreshed 2026-10-20 — re-read the sources, this rots)

Refresh this section at the start of every AI loop: a web sweep of the tools'
changelogs (code.claude.com/docs/en/changelog — the weekly what's-new digest
lags by a month — learn.chatgpt.com/docs/changelog, cursor.com/changelog,
ampcode.com/news, geminicli.com/docs/changelogs, kiro.dev/changelog,
github.com/cline/cline/releases) and the vendors' model/pricing/deprecation
pages (platform.claude.com release notes + model-deprecations,
developers.openai.com/api/docs/models + /deprecations, ai.google.dev changelog
+ pricing, Model Studio newly-released-models + model-pricing +
model-depreciation, api-docs.deepseek.com pricing), plus the bundled
`claude-api` skill for Anthropic ids and request rules. Mark what could not be
read from a primary source as unverified and do not ship on it.

**Models crew should default to.** Anthropic: `claude-opus-5-5` ($4/$20, the
default), `claude-sonnet-5-5` ($2/$10), **`claude-haiku-5-5`** (released
2026-10-07; $0.10/$0.50 for prompts ≤100K tokens, $0.50/$2.50 above — keep
cheap-tier prompts under 100K; replaces `claude-haiku-4-5` as the Cheap tier),
`claude-fable-5-1` ($10/$50) — all 1M context, 128K output. `claude-haiku-4-5`
is Legacy: still active, no deprecation notice yet (≥60 days' notice, so no
retirement before mid-December 2026); `claude-sonnet-4-5-20250929` retires
2026-11-30. Cache reads are 5% of input on Opus 5.5 / Sonnet 5.5, 2.5% on
Fable 5.1, 10% elsewhere. OpenAI: `gpt-6.1-sol` ($2/$10, the default — it
replaced `gpt-6-sol` on 2026-09-29), `gpt-6-astra` ($10/$50), `gpt-6-luna`
($0.10/$0.50, the only one with effort `none`); GPT-6 takes
`max_completion_tokens`. Google: `gemini-3.8-flash` (free tier; $0.75/$3.75,
doubling 2027-01-01) as executor, **`gemini-4-argon`** (GA 2026-10-13, $2/$10
until 2026-11-30 then $4/$20, no free tier) as the hard-task model in place of
`gemini-3.1-pro-preview`; 2.5 is closed to new users, 3.7-flash and 3.5-flash
auto-route to 3.8 / 3.6. Qwen (DashScope intl): `qwen3.8-max` ($2/$6, 1M) /
`qwen3.8-max-0902`, `qwen3.7-plus` (there is no 3.8-plus), `qwen3.8-flash`
($0.15/$0.47, 1M), `qwen-flash` for one-line Cheap calls; `qwen-max` is 32K;
`qwen-turbo` was decommissioned 2026-10-10 (live call 2026-10-20: HTTP 429 on
every try; dropped from crew's chain) and `qwen3-max` on 2026-09-08. DeepSeek: `deepseek-flash`
(now V4.1-Flash-1017, since 2026-10-17; off-peak half price) and
`deepseek-v4-pro`. NVIDIA: `nemotron-3.5-lightning` as the free executor (no
newer Nemotron found — unverified).

**Request rules the Claude 5.x family enforces** (a 400 otherwise): no
`budget_tokens`, no `temperature`/`top_p`/`top_k`, no assistant prefill (Haiku
5.5 included), no forced `tool_choice` (`any`/`tool`) on Opus 5.5 / Sonnet 5.5
/ Fable 5.1 — use `auto` with strict tool use. Opus 5.5 and Fable 5.1 think
always: `thinking: {type: "disabled"}` and `{type: "enabled"}` are both a 400;
depth is `output_config.effort` (`low`…`max`; defaults Opus 5.5 `medium`,
Sonnet 5.5 `high`, Haiku 5.5 `medium`, Fable 5.1 `high`). Sonnet 5.5 rejects
`disabled` but takes `thinking: {type: "between_tools"}` (no up-front thinking,
at effort `high` or below). Do not hard-code these: the Models API reports
`capabilities.thinking.types.disabled|between_tools`, `server_tools` and
`line` per model. Thinking counts against `max_tokens`, its text is empty
unless `display: "summarized"`, and a thinking block is bound to the exact
prefix, model, conversation and account that produced it — a harness that
edits earlier turns, `system` or `tools` must strip them (crew does) rather
than replay them edited. Prompt-cache misses are explained by the GA
`diagnostics` request object; tools can be added mid-conversation without
busting the cache via `tool_addition` blocks (beta `inline-tools-2026-09-15`).
DashScope Qwen: `enable_thinking` (+ `thinking_budget`) ride the streaming
body only.

**Harness techniques that shipped this year, not yet in crew** (the backlog's
source; strike a row when it ships):
- ~~prompt caching on the tool loop~~ (shipped v0.25.x)
- ~~long tool output written to a file the agent can read back~~ (shipped)
- ~~steer, don't queue~~ (shipped; Codex made it the default 2026-10-15)
- a completion judge after each turn that starts another turn until a stated
  condition holds, with a no-progress stop (Claude Code `/goal` + LLM-judged
  `Stop` hooks, Codex goals' no-activity limit, Grok Build `/goal`); crew's
  judge covers swarms only (`swarmverify.rs`)
- a read-only advisor model consulted before committing to an approach, when
  stuck, and before declaring done — advice text only, ~2–3 calls a task
  (Anthropic `advisor_20260301`, Claude Code `/advisor`, Amp oracle, Codex
  Guardian, Claude Code's "You should know" side agent)
- hooks with a JSON contract — PreToolUse / PostToolUse / Stop, a block
  reason fed back without ending the turn, fail-closed when the hook can't run
  (Claude Code `continueOnBlock`, `onFailure: "block"`; Codex; Kiro)
- continue a reply that died mid-stream from its partial text, retry
  thinking-only / empty replies, cap 3 (Claude Code 2.1.288, Cline 4.1.25)
- an ordered fallback model list on 429 / 529 / refusal / retired id, with a
  visible "fell back to X" line (Claude Code `fallbackModel`, Kiro V3 notices)
- effort / thinking per route and per subagent, not one global budget (Claude
  Code Agent-tool `effort`, `output_config.effort`, Sonnet `between_tools`,
  DashScope `enable_thinking`, DeepSeek `reasoning_effort`)
- review in levels (Quick / Standard / Deep), each finding verified before it
  is reported, with the blast radius (callers of a changed pub item) in front
  of the reviewer (Claude Code Code Review levels, ultrareview, Cursor
  Security Review)
- memory of corrections and RULED-OUT approaches, proposed for the user to
  accept (Kiro 1.4.0 Memory, Codex, Gemini CLI)
- compaction that keeps tool names and finished work in the summary (no
  re-doing pre-compaction work), per-subagent compaction windows, idle
  compaction before the 1-hour cache expires (Claude Code 2.1.293–2.1.304);
  crew's relay keeps 8 entries verbatim already
- MCP 2026-07-28 — per-request `_meta` protocol version, `server/discover`,
  `ttlMs` / `resultType` / `cacheScope` (Claude Code defaults stdio servers to
  it since 2.1.292; crew's client still speaks 2024-11-05)
- tool search / deferred tools once the tool list grows (Claude Code default
  behind gateways, Codex ranked tool search, Kiro on-demand MCP tools)
- parallel read-only tool calls in every loop (Codex default 2026-10-15; crew's
  relay batches ≤4 reads already — check the native tool-use loops)
- ~~a budget reminder ("3 calls left") in the last rounds~~ (`toolround::next_step`)

**Measure before and after.** An AI iteration is judged on a live probe, not
on green tests alone: drive `crew --broker-plugin` over stdio JSON lines in a
scratch clone (never `~/code/crew/.crew` — probes pollute the recall graph)
and time routing, first token and the answer's grounding. DashScope is the API
engine for probes; the Anthropic API org has no credit (Max plans now include
monthly API credits per the 2026-10-07 release note — check the Console before
assuming), so an Anthropic-only feature is checked through `claude -p` or
DashScope's Anthropic-compatible endpoint, or marked unverified.

**Pane and terminal work** is judged on shots: before/after with the chat,
work or tab shot harnesses, composited over black and lavender; dump the
cells before believing your eye. Bloat: count the rows a fixture turn takes
on screen, before and after. The `verify` skill drives the live GUI.

## The round

1. **Pick one** — the top unshipped row of the lane's backlog, re-ranked
   against what the tree does today (`ui`: research Ghostty / Alacritty /
   WezTerm / Kitty / tmux / zellij / Warp when the backlog runs dry). Write a
   five-line design note in the charter: the behaviour, what the user will
   see, how it is measured, rows added and removed. Say what it is before
   building it.
2. **Branch** `feat/<slug>` in the main checkout. If another session holds
   the checkout, use a worktree with an APFS-cloned target: `cp -cR
   target/debug target-<br>/debug` and `CARGO_TARGET_DIR=target-<br>`; remove
   the worktree and `target-<br>` together once the branch merges. A
   multi-round loop may run TWO lanes — main checkout plus one worktree, an
   implementer subagent each — merging strictly one at a time.
3. **Build the smallest honest version.** Module boundaries: window/pane/
   input in `crew-app`, GPU in `crew-render`, PTY/grid in `crew-term`, agents
   and the broker in `crew-plugin` / `crew-hive`. Tests in a sibling
   `*_tests.rs`; mutation-check them (break the code, watch them fail). A
   test that reads the theme takes `theme_test_guard()`. Doc comments say WHY.
4. **Gate, in the foreground, all green:**
   - `cargo fmt --all --check`
   - `cargo check --workspace --all-targets` — ZERO warnings (the Windows job
     builds with `-D warnings`)
   - `cargo test --workspace --no-fail-fast` — 0 failed (the broker's e2e
     tests are not in `--lib`); the `linecap` ratchet runs inside them
5. **Verify** with the probe and/or shot, and write the numbers into the
   charter row.
6. **Docs** — `README.md` and `docs/CREW.md` when a command, key or `CREW_*`
   knob changes (doc guards fail the build otherwise). Add, don't rewrite.
7. **Changelog line** — do NOT edit `CHANGELOG.md` or bump `Cargo.toml` (a
   test asserts the top entry names the current version). Write the round's
   changelog paragraph into the charter's "Pending release" list: what was
   wrong, what it does now, what it costs when it has nothing to say, which
   tool the idea came from.
8. **Commit and merge** — commit on the branch, merge `--no-ff` into main,
   `git push origin main`. No version bump, no tag. Delete the branch. Read
   `gh run list` for the main push: a red CI job stops the loop.
9. **Record the row** (version: "unreleased", lane, pillars, before → after)
   and print `--- Round N/M (lane): merged <sha> — <title> ---`. Keep going.

## Releases — at most one per day

Rounds never tag. At the end of the day's loop (or when the user asks), and
only if nothing was released today, cut ONE release with
`scripts/release.sh <branch> entry.md msg.txt "subject"`, where `entry.md`
folds every lane's "Pending release" list into one CHANGELOG entry (the bold
headline is UI text the welcome card shows — under ~70 chars, project voice).
The script picks the version (patch 0..99, then the minor rolls) and fills
`{V}` in the message; set `EXTRA="crew-plugin crew-hive"` for the crates
touched since the last release. The tag triggers
`.github/workflows/release.yml`; never `gh release create`. Watch the Release
and Windows runs until the release is published (not a draft, all assets),
then fill the version into the rows it covers and empty the pending lists.
If a release already went out today, leave the lists pending for tomorrow
unless the user explicitly asks for another.

## Unattended operation

- If `git push` fails, keep the commit local, report it, and stop.
- If a whole round produces nothing that passes the gate, stop the loop and
  say what happened.
- Final summary: a table of rounds (lane, merge sha, title, before → after)
  and whether they were released (version) or are pending.
