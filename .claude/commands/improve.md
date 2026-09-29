# Self-improvement loop — one feature, one release, every iteration

Autonomous enhancement loop for **crew / agent smith**. Each iteration ships
**exactly one feature** and ends with a **released version**. Charter, backlog
and the shipped table live in `.superpowers/sdd/ai-loop-2026-09-14.md` — read
it first; it is the loop's memory and it is where the next row is written.

Mode: $ARGUMENTS — `loop N` (N iterations, default 3, cap 12), `loop Nh`
(about N hours), `one` (a single iteration), empty (propose the next feature
and ask).

## What every feature must serve

1. **Intuitive, responsive UI** — nothing new on screen without a shot; motion
   off costs no frames; the pane says what crew knows without being asked.
2. **Autonomous execution** — smith decides and acts. Asking is the exception,
   and every ask is one keystroke to answer.
3. **Agentic** — the model drives the tools, the plan, the verification and
   the memory; constants are backstops, not drivers.
4. **Only the needed command** — an iteration may RETIRE or FOLD commands. New
   capability arrives as behaviour, not as a new slash command, unless there
   is no other door. `/look` (v0.22.25) is the pattern: fold a family into one
   verb with a picker, keep the old spellings answering.
5. **Memory** — the recall graph (`crew-plugin/src/broker/recall/`) is how
   "remember everything" is real: turns, topics and files on disk, walked two
   hops in front of the next task. A feature that learns something durable
   writes it there rather than inventing a second store.

Steal shamelessly from Claude Code, Codex, Cursor, Amp, Gemini CLI, Kiro,
Cline, Grok Build and the rest — and say in the changelog which tool the idea
came from.

## What is current (refreshed 2026-09-29 — re-read the sources, this rots)

Refresh this section at the start of every AI loop: a web sweep of the tools'
changelogs (code.claude.com/docs/en/whats-new, learn.chatgpt.com/docs,
cursor.com/changelog, ampcode.com/news, geminicli.com/docs/changelogs,
kiro.dev/changelog) and the vendors' model/pricing pages, plus the bundled
`claude-api` skill for Anthropic ids and request rules. Mark what could not be
read from a primary source as unverified and do not ship on it.

**Models crew should default to.** Anthropic: `claude-opus-5-5` ($4/$20, the
default), `claude-sonnet-5-5` ($2/$10), `claude-haiku-4-5` ($1/$5, retirement
not before 2026-10-15), `claude-fable-5-1` ($10/$50) — all 1M context but
Haiku (200K). OpenAI: the `gpt-6-*` family (sol/luna/astra). Google:
`gemini-3.8-flash`, `gemini-3.1-pro-preview`; 2.5 is closed to new users since
2026-09-18. Qwen: `qwen3.8-max` / `qwen3.8-flash` (1M context, where
`qwen-max` has 32K). NVIDIA: `nemotron-3.5-lightning` as the free executor.

**Request rules the Claude 5.x family enforces** (a 400 otherwise): no
`budget_tokens`, no `temperature`/`top_p`, no assistant prefill, no forced
`tool_choice` (`any`/`tool`) on Opus 5.5 / Sonnet 5.5 / Fable 5.1. Thinking is
always on for Opus 5.5 and Fable 5.1 and counts against `max_tokens`; depth is
`output_config.effort` (`low`…`max`, Opus 5.5 defaults to `medium`), and the
thinking text is empty unless `display: "summarized"`. Thinking blocks are
bound to the exact prefix that produced them — a harness that edits earlier
turns must strip them (crew does) rather than replay them edited.

**Harness techniques that shipped this year, not yet in crew** (the backlog's
source; strike a row when it ships):
- prompt caching on the tool loop — top-level `cache_control`, stable prefix
  first (Anthropic; Qwen caches implicitly at 10–20%)
- long tool output written to a file the agent can read back, instead of
  clipped (Cursor dynamic context discovery)
- steer, don't queue — a message typed mid-task joins the next tool round
  (Amp, Cursor, Codex `instant_interrupt`)
- `/goal`-style completion check by a cheap judge after each turn (Claude
  Code, Cursor)
- hooks with a JSON contract — PreToolUse / PostToolUse / Stop, a block reason
  fed back to the model (Codex, Claude Code, Kiro)
- compaction that keeps the last K rounds verbatim (Claude Code rewind,
  Anthropic `compact-2026-09-04`)
- an ordered fallback model list on 429 / 529 / refusal (Claude Code)
- a read-only second-model reviewer before "done" (Amp oracle)
- a memory inbox — proposed memories the user accepts (Codex, Gemini CLI)
- MCP 2026-07-28 — stateless `_meta` requests, `ttlMs` on lists (crew's client
  still speaks 2024-11-05)
- tool search / deferred tools once the tool list grows (Anthropic, OpenAI)

**Measure before and after.** An AI iteration is judged on a live probe, not
on green tests alone: drive `crew --broker-plugin` over stdio JSON lines in a
scratch clone (never `~/code/crew/.crew` — probes pollute the recall graph)
and time routing, first token and the answer's grounding.

## The iteration

1. **Pick one** — the top unshipped row of the backlog, re-ranked against what
   the tree does today. Say what it is before building it.
2. **Branch** `feat/<slug>` in the main checkout (no worktree unless another
   session holds the checkout; then a worktree with its OWN `target-<branch>`).
   A loop of several iterations may run TWO lanes — the main checkout and one
   worktree with its own `target/` — each detached at main, an implementer
   subagent per lane, releases strictly one at a time (squash onto the local
   main, gate, commit-tree merge, tag, push).
3. **Build the smallest honest version.** Every `.rs` file ≤ 200 lines; tests
   in a sibling `*_tests.rs`; doc comments say WHY, not what. A file already
   over the cap (`crates/crew-app/line-cap-debt.txt`) may not GROW.
4. **Gate, in the foreground, all four green:**
   - `cargo fmt --all --check`
   - `cargo check --workspace --all-targets` — ZERO warnings (main is clean
     since 0.25; the Windows job builds with `-D warnings`)
   - `cargo test --workspace --no-fail-fast` — 0 failed
   - the `linecap` ratchet (runs inside the app's tests)
5. **Document it** — `README.md` and `docs/CREW.md`. Doc guards fail the build
   if a command or a `CREW_*` knob is undocumented, and they are right to.
6. **CHANGELOG** — a new top entry naming the new version, in the project's
   voice: what was wrong, what it does now, what it costs when it has nothing
   to say. A test asserts the top entry names the current version. The bold
   headline is UI text (the welcome card shows it): keep it under ~70 chars.
7. **Release** — bump `Cargo.toml`, `cargo check` for the lock, commit, merge
   `--no-ff` into main, `git push origin main`, `git tag vX.Y.Z`, push the tag.
   Then read `gh run list` before the NEXT tag: a red Windows or Release job
   stops the loop.
8. **Record the row** in the loop doc's table, and keep going.

## Guardrails (these outrank any idea)

- Crew is a native GPU terminal. In-pane UI only — no overlays, no layout
  switcher, panes auto-tile.
- Panels are rounded cards with a fieldset legend, never a title bar.
- Keys pass through inside a focused terminal pane.
- No new dependency without first checking the workspace already lacks it.
- Never break an existing spelling to gain a new one: fold, then teach.
- Never force-push. Never commit a red gate. If `cargo check` fails three
  times on the same idea, drop the idea and take the next row.
