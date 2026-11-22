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

### R2 (nav) — Cmd+P, go to file
- Behaviour: Cmd+P opens the bar at `/view `; under the folder's own rows
  the picker lists up to 30 files from anywhere in the tree, ranked in four
  tiers (name start, substring, letters in order inside the name, then
  across the path — the last only when nothing better matched), then the
  tighter span, then the shorter path.
- The walk (`fileindex::scan_max`, 20,000 cap) runs on a worker, cached per
  root for 30 s (`pathtree.rs`); ranked rows memoized per query, so a focused
  bar does not re-rank every frame. Folded in: `render.rs` built the whole
  picker (a `read_dir` for a path command) every frame even when the bar was
  unfocused — now only while it has focus.
- Measured: crew's tree 3,127 files, walk 18 ms (release, off-thread), rank
  1.7 ms per new query. Shot `go_to_file_shot` (`/view exhit`): 2 rows, both
  real; before the tier split, 12 of 14 rows were letters smeared across
  `inbox/html/corpus/*`. Mutations caught: no tree rows, no de-dupe.
- Rows: + `pathtree.rs`; Cmd+P in the chord table, help, README, CREW.md.

### R3 (perf/ui) — toasts cost no frames at Motion off
- Behaviour: at Motion off a toast holds still and whole to the end of its
  life; the only frame it asks for is the one that removes it.
- Bug found on the way: Motion off still dissolved the text over the last
  340 ms (≈20 frames), then `any_live` stopped asking at TTL — BEFORE the
  frame that pruned it, so an emptied card box stayed on screen until
  something else repainted.
- `toastexit.rs` (`leave`, `wants_frame`) takes the exit rules out of
  `toast.rs` (debt file: 334 → 323 lines).
- Measured: frames asked for per toast at Motion off ≈20 (exit window at the
  16 ms poll) → 1. Mutation (Off follows the motion rule) fails the test.

## Pending release
- **CI checks again** (R1): the CI workflow had failed on every push for
  weeks — clippy lints and 16 unused workspace dependencies — so it caught
  nothing. Clippy is clean and the unused dependencies are gone; a red CI run
  now means something. No change to the app.

- **Go to file** (R2): `/view`'s picker listed one folder by prefix, so a
  file you knew by name but not by folder was a walk. **Cmd+P** now opens it,
  and under the folder's rows it lists files from anywhere in the tree that
  match — the name's start first, then anywhere in the path, then the
  letters in order (`exhit` finds `explorerhit.rs`). The tree is walked off
  the frame thread (crew's 3,127 files in 18 ms) and each query ranked once.
  The bar also stopped building its picker every frame while unfocused.
  (From VS Code's, Zed's and Cursor's Cmd+P.)

- **Toasts are free at Motion off** (R3): with Motion off a notification
  card still dissolved its text over its last third of a second, asking for
  about twenty frames, and then stopped asking one frame early — leaving an
  empty card on screen until something else repainted. It now holds still
  and whole, and costs one frame: the one that takes it away. (loop-30's
  open list; the invariant every reduced-motion setting promises.)

## Log
| # | lane | merge | title | before → after |
|---|------|-------|-------|----------------|
| R1 | perf | fe0bf52d | CI green | clippy 25 errors → 0, 16 unused deps → 0 |
| R2 | nav | 7f237aa9 | Cmd+P go to file | 1 folder by prefix → whole tree fuzzy, 2/2 real rows for `exhit` |
| R3 | perf | (this) | toasts free at Motion off | ≈20 frames + stuck empty card → 1 frame |
