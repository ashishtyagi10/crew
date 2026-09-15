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

Steal shamelessly from Codex, Cline, Claude Code, Grok and the rest — and say
in the changelog which tool the idea came from.

## The iteration

1. **Pick one** — the top unshipped row of the backlog, re-ranked against what
   the tree does today. Say what it is before building it.
2. **Branch** `feat/<slug>` in the main checkout (no worktree unless another
   session holds the checkout; then a worktree with its OWN `target-<branch>`).
3. **Build the smallest honest version.** Every `.rs` file ≤ 200 lines; tests
   in a sibling `*_tests.rs`; doc comments say WHY, not what. A file already
   over the cap (`crates/crew-app/line-cap-debt.txt`) may not GROW.
4. **Gate, in the foreground, all four green:**
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets` — no NEW warnings (main is not
     clean; compare against what was there before)
   - `cargo test --workspace --no-fail-fast` — 0 failed
   - the `linecap` ratchet (runs inside the app's tests)
5. **Document it** — `README.md` and `docs/CREW.md`. Doc guards fail the build
   if a command or a `CREW_*` knob is undocumented, and they are right to.
6. **CHANGELOG** — a new top entry naming the new version, in the project's
   voice: what was wrong, what it does now, what it costs when it has nothing
   to say. A test asserts the top entry names the current version.
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
