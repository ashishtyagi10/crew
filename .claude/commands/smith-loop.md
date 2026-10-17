# Agent Smith loop — one deeper smith, one release, every iteration

Autonomous loop that makes **agent smith** (crew's built-in agent and its
pane) better at the work and better at showing it. Each iteration ships ONE
improvement end to end and ends with a released version. The charter, the
ranked backlog and the shipped table live in
`.superpowers/sdd/smith-loop-2026-10-16.md`. Read it first: it is the loop's
memory, and each iteration writes its row there.

Mode: $ARGUMENTS — `loop N` (N iterations, default 5), `one` (a single
iteration), or empty (propose the top three rows and ask).

## The bar: every iteration names which of these it moves

1. **Depth, not breadth.** Work the way a senior engineer works, and don't add
   surface. Before shipping, answer in the charter: *what would a staff
   engineer do here that smith doesn't?* A staff engineer understands the
   code before changing it (callers, tests, the history of the lines),
   reproduces before fixing, finds the root cause rather than the symptom,
   makes the smallest change that fixes it, proves it with evidence, reviews
   their own diff, names what is uncertain, and asks only when the decision
   belongs to someone else. Model-driven: constants are backstops, not
   drivers.
2. **Feedback that earns trust.** The reply leads with the outcome, then what
   changed (files and line counts), why, how it was verified (the command
   and its result, never just "tests pass"), and what is left or risky.
   While working, one line says what smith is doing now and why. On a
   failure, stop or cancel, it says what was done, what state the tree is in,
   and how to undo it.
3. **Graphics that pull the eye.** The smith pane is drawn, not printed: the
   Matrix code line, glass cards, meters, diff bars and drawn marks. Every
   visual carries information, and no motion is decorative only. With motion
   off the pane is a still frame that says the same thing. It must hold in
   all 8 palettes, including glass over a light desktop (bright white words on
   glass is the user's taste; don't argue contrast, build it legibly).
4. **Everything needed, nothing more.** There is a bloat budget: an iteration
   that adds rows to a turn removes or folds others. No walls of tool calls
   (the user, 2026-10-15: no tool-call list; smith looks busy Matrix-style
   instead). Long things fold into a count you can open. Numbers beat
   adjectives.

## Measure before and after (not "tests pass")

- **Behaviour:** run a live probe. Drive `crew --broker-plugin` over stdio
  JSON lines in a SCRATCH CLONE, never in `~/code/crew/.crew`, because probes
  pollute the recall graph; reset recall between runs. Use the same prompt
  before and after. Record rounds, time to first token, whether the answer is
  grounded, and the reply's line count. A behaviour change that no probe can
  reach is not shipped on faith; say it is unverified.
- **Pane:** take a before/after shot with the chat or work shot harnesses,
  composited over black and lavender. Dump the cells before believing your
  eye.
- **Bloat:** count the rows a fixture turn takes on screen, before and after.

## The iteration

1. **Pick one.** Take the top unshipped backlog row, re-ranked against what
   the tree does today. Write a five-line design note in the charter: the
   senior behaviour, what the user will see, how it is measured, and the rows
   added and removed.
2. **Branch** `feat/<slug>` in the main checkout. If another session holds the
   checkout, use a worktree with an APFS-cloned target: `cp -cR target/debug
   target-<br>/debug` and `CARGO_TARGET_DIR=target-<br>`.
3. **Build the smallest honest version.** Every `.rs` file stays ≤ 200 lines
   (the linecap ratchet). Tests go in a sibling `*_tests.rs`. Mutation-check
   new tests: break the code and watch them fail. A test that reads the theme
   takes `theme_test_guard()`. Doc comments say WHY.
4. **Gate, in the foreground:** `cargo fmt --all --check`, `cargo check
   --workspace --all-targets` with zero warnings, and tests for every crate
   touched. The broker lives in `crew-plugin`, and its e2e tests are not in
   `--lib`.
5. **Verify** with the probe and/or shot above, and write the numbers into
   the charter row.
6. **Docs:** update `README.md` and `docs/CREW.md` when a command or a
   `CREW_*` knob changes (doc guards fail the build otherwise).
7. **Release:** `scripts/release.sh feat/<slug> entry.md msg.txt "subject"`.
   It picks the version (the patch runs 0..99, then the minor rolls) and
   fills in `{V}` in the message. Set `EXTRA="crew-plugin crew-hive"` for the
   crates touched. The CHANGELOG headline is UI text the welcome card shows,
   so keep it under ~70 chars and in the project's voice. Read `gh run list`
   before the next tag: a red Release or Windows job stops the loop.
8. **Record** the row (version, pillars, before → after), then keep going.

When another loop runs in the same session (the UI loop), alternate releases,
and run only one release at a time.

## Guardrails (these outrank any idea)

- In-pane UI only. Panels are rounded cards with a fieldset legend, never a
  title bar. Panes auto-tile. Keys pass through inside a focused terminal
  pane.
- No new dependency without checking that the workspace lacks it.
- New capability arrives as behaviour, not a new slash command, unless no
  other door exists. Fold spellings; never break one.
- Durable learning goes into the recall graph (`crew-plugin/src/broker/recall/`),
  not a second store.
- Claude 5.x request rules apply. There is no `temperature`, prefill, forced
  `tool_choice` or `budget_tokens`. Depth is `output_config.effort`. See
  `/improve`'s "What is current" for the model table; refresh it from primary
  sources when it is more than a month old.
- Never force-push. Never commit a red gate. If the same idea fails the gate
  three times, drop it and take the next row.
