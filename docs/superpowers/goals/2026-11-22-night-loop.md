# Night loop — UI · nav · AI · perf — started 2026-11-22 (from v0.30.22)

Ask: "keep improving crew in each aspect for rest of the night .. UI, navigation,
ai capabilities and overall performance." v0.30.22 went out at 00:35 today, so
every round merges to main untagged; the 8 PM job releases.

Lanes rotate: perf/health → nav → ai → ui. One change per round, gated, merged.

## Backlog (ranked, re-ranked each round against the tree)
- perf: CI's "CI" workflow red since before 2026-11-18 (clippy + 16 unused
  workspace deps) — nothing on main was checked by clippy.
- perf: toasts ask for frames for 340 ms at Motion off (loop-30 U8).
- perf: ~8 ms per block-crossing caret move on the 324 KB manual (md reveal).
- nav: Explorer from the keyboard; git status in the tree (VS Code).
- nav: the outgoing tab still vanishes on a switch (motion loop R3).
- ai: corrections can't be seen or forgotten one by one (loop-30 S10).
- ai: the advisor's two calls a turn are shared by failures and stalls (S9).
- ui: cursor blink; OSC 52 read; IME and dead keys.

## Rounds

### R1 (perf) — CI green
- Behaviour: `cargo clippy --workspace --all-targets -D warnings` passes, so
  the CI workflow can fail on a real regression instead of always.
- Changes: 25 lints fixed (`clippy --fix` + 13 by hand: `from_ref`, struct
  update over field reassignment, `strip_prefix`, `const` assert, renamed
  `into_receipt` → `file_in_receipt` and `from_smith` → `take_smith`, which
  take `&mut self`); `clippy.toml` raises `type-complexity-threshold` (test
  fixtures and frame scratch tuples read at one site); 16 workspace deps
  nothing used removed from the root `Cargo.toml`.
- Measured: clippy 25 errors → 0; manifest warnings 16 → 0; gate 41 suites,
  8052 passed.

## Pending release
- **CI checks again** (R1): the CI workflow had failed on every push for
  weeks — clippy lints and 16 unused workspace dependencies — so it caught
  nothing. Clippy is clean and the unused dependencies are gone; a red CI run
  now means something. No change to the app.

## Log
| # | lane | merge | title | before → after |
|---|------|-------|-------|----------------|
| R1 | perf | (this) | CI green | clippy 25 errors → 0, 16 unused deps → 0 |
