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
- Measured: redraws per toast at Motion off ≈5 (the 340 ms exit window at the
  16 ms poll, throttled by `BUSY_ANIM_DIV` 4) → 1. Mutation (Off follows the motion rule) fails the test.

### R4 (ai) — provider freshness
- Vendor sweep (agent, primary pages read 2026-11-22): `gpt-5` shuts down
  2026-12-11; `gemini-2.5-pro` routes to 3.8 Flash from 2026-12-01; Gemini 4
  Argon goes $2/$10 → $4/$20 on 2026-12-01; new `claude-fable-5-2` ($8/$40)
  and `qwen4-max` ($1.5/$4.5). Anthropic unaffected by the API-version
  `temperature` removal (crew pins `2023-06-01`).
- Live probe NOT run: the DashScope account answers `AccessDenied.Unpurchased`
  (403) for qwen-flash, qwen-max and qwen4-max alike — so qwen4-max is
  catalogued and priced, not made the default.
- Changes: chains drop `gpt-5` and `gemini-2.5-pro` (CREW.md table too);
  catalog + pricing rows for fable-5-2, qwen4-max, Argon's December rate;
  a new hint row: that 403 (and OpenAI's "does not have access to model")
  now ends "this account may not call that model (not activated, or its free
  quota is spent); enable it with the provider, or /model picks another".
  Test `no_chain_names_a_retiring_model` guards every chain.
- Measured: the captured body, before → after: bare sentence → sentence +
  fix. Unverified: fable-5-2 cache rate (10% assumed, errs high), its and
  qwen4-max's context windows (0 = unknown), Argon tools on the compat wire.

### R5 (nav) — the Explorer from the keyboard
- Behaviour: Cmd+Shift+E (VS Code) opens the nav on the Explorer and hands
  the tree the keys: ↑/↓ or j/k move, →/Enter/Space open a folder (→ again
  steps in) or preview a file in the tree's one viewer and keep the keys,
  ← closes a folder or goes to the parent, Home/End, Esc or the chord again
  gives them back. Any other key ends the mode and reaches the pane.
- Whose keys: `TreeKeys { row, owner }` — the tree holds them only while
  the pane focused when it took them (or the viewer it opened) is still
  focused and the bar is not, so a click, Cmd+I or a new pane ends it with
  no path having to know. The focused pane draws unringed meanwhile.
- Cursor = the far pane's cursor bar (bold white on the inked accent),
  whole row. Shots `navbar_shot_tree_keys` on TokyoNight + Porcelain.
- Measured: 4 tests (chord, walk/open/preview, give-back, bar); mutation
  (no owner hand-over after a preview) fails the walk test once the test
  starts with focus elsewhere — it first survived, the preview had landed
  on the owner's index. Hook rides `keys.rs`'s hint line (at 200, no growth).
- Live GUI not driven: osascript has no assistive access this session.

### R6 (perf) — a caret crossing blocks stops re-doing the document
- Cost found: each ↓ that crossed into another block re-parsed the whole
  document to find the block (`block_at`, 1.2 ms on the 327 KB manual) and
  re-rendered all of it to splice the reveal in (~6 ms), on the frame thread.
- `md/memo.rs`: the block list and the unrevealed render are kept for the
  last text seen (hash-keyed; render also by width); `block_at` picks from
  the list, `render_revealed` splices onto a copy. `md::render` is pure in
  (text, width) — checked: no theme/config reads.
- Measured (release, `time_downs_through_the_manual`, 200 ↓): 7.12 ms →
  2.96 ms each. Correction (measured later): R6's split of the old 7.1 ms
  was wrong — most of the 200 downs never cross a block; one full render of
  the manual is ~27 ms (see Close).

### R7 (ai) — an account's failure ends the chain walk
- Gap: the OpenAI-compatible chain (OpenAI, Gemini, DeepSeek, DashScope,
  NVIDIA, OpenRouter) short-circuited only on a MISSING key. A refused key
  or an unpaid account asked every rung in turn — each a full request —
  and then reported the last one's error.
- Now `fellback::account_wide` ends the walk for a refused key (the hint's
  Key row, so the same words) and for OpenAI `insufficient_quota`, DeepSeek
  "insufficient balance", DashScope `Arrearage`. Deliberately NOT: OpenRouter
  "insufficient credits" (a `:free` rung answers it), a plain "quota"
  (per model on DashScope and Gemini), `AccessDenied.Unpurchased` (per model).
- Measured (fake server counting connections, 3-rung chain): refused key
  3 requests → 1, with the key hint; a 402 still walks all 3. Mutation (stop
  removed from `complete`) fails the refused-key test. Not probed live
  (DashScope 403s every model; no other key on this machine was tried).
- `hint.rs` had grown past 200 with R4's row: `hintmatch.rs` split out.

### R8 (nav) — git's marks in the Explorer
- Behaviour: in a repository, a changed file's name in its change's ink
  with M/A/U/D/! at the row's end; a folder holding changes amber with a
  dot; an untracked folder U; the root none (VS Code's tree).
- Cost: none extra. The fleet's `git status --porcelain --branch` already
  ran every few seconds per pane folder and kept only the line count;
  `gitfiles::query` keeps the lines (repo root found by walking up to
  `.git`; porcelain paths are root-relative — checked live with `-C sub`).
  `GitInfo` untouched (in debt at 210, built literally by 8 tests).
- Bug found by the shot: the frame's fleet poll evicted every folder that
  was not a pane's, so a tree rooted on the remembered root (or the far
  pane's) never had marks. While the Explorer shows, its root is polled too.
- Measured: 8 tests (porcelain kinds incl. rename/conflict/untracked
  folder, a real `git init` asked from a subfolder, the drawn letter and
  dot); shots `navbar_shot_tree_git` (TokyoNight, Porcelain) — before the
  poll fix no marks, after: `src •`, `explorerdraw.rs M`, `explorergit.rs U`.

### R9 (ui) — the tab you leave folds away
- Gap (motion loop R3's open item): the arriving tab came in from its side
  of the list, the outgoing one vanished in the same frame.
- Behaviour: `ghost::Exit::Departed(dir)` — each visible card of the tab
  being left retracts (the close animation) while drifting the opposite way
  to the arrival, `tabentry::TRAVEL` of its height. Drawn UNDER the grid
  (ghosts were all on top), without its legend. Motion off: the ghost is
  born settled and dropped, as every ghost is.
- Shots `entry_shot` (paper-dark, glass-night, 0/60 ms) caught the old
  card's legend letter showing through the still-assembling arrival's text
  — departed ghosts now carry no legend.
- Measured: tests for the drift (both ways, end position, x fixed) and for
  the switch (two ghosts going down the list, none on a no-op switch, none
  from an empty tab).

### Fix-ups after the second review (R3, R5, R6, R8, R9, R2)
- **Toast hold (R3 fix regression):** a held card's age freezes only on a
  drawn frame and a held stack asks for none, so the poll's `expire` timed
  out a card under a resting pointer; the draw path pruned BEFORE holding
  (pre-existing, same gap). `expire` skips a held stack; `push_toasts`
  holds then prunes; a prune drops the stale `rects` (they index `items`).
  Test `a_held_card_outlives_an_idle_gap`; mutation (no hold guard) fails it.
- **Tree keys re-armed (R5):** `tree_has_keys` recomputed owner==focused,
  so Cmd+I→Esc, Cmd+]→back, Cmd+G×2 or a click on the focused pane handed
  the keys back to the tree. A lost hold is now dropped the first frame it
  is lost, any click off a tree row drops it, and an editing viewer no
  longer outranks it (`view_edit_key`). Test `a_lost_hold_does_not_come_back`;
  mutation (no per-frame drop) fails it.
- **Off a Mac (R2, R5):** plain Ctrl+P is Cmd+P unless `p` is in `TYPED` —
  it now is, so a terminal keeps its previous-line key. Ctrl+Shift+E was
  already hints' chord there, so Cmd+Shift+E is Mac-only; `/nav explorer`
  now hands the tree the keys on every platform.
- **R9:** ghosts only for cards drawn (the zoomed one alone; never off-grid).
- **R6:** a reader (no reveal) renders directly — no hash, copy or pin.
  Typing cost measured: 200 inserts on the manual 27.7 ms each with the memo
  vs 26.6 without (+1.1 ms, the miss's hash and copy). The ~27 ms per
  keystroke itself is the next perf row.
- **R8:** `core.quotePath=false`, so a non-ASCII file name gets its mark.
- `toast.rs` (debt 334) had grown to 340: lifetime methods moved to
  `toastlife.rs` (child module) — 308 now.

### Fix-ups after review (R2, R3)
- **R2:** Enter takes the highlighted row, so `/dump read` + Enter became
  `/dump readme.md` — an overwrite — already via the folder's prefix rows,
  and the tree rows made it likelier (and `/batch` would RUN a fuzzy pick).
  Tree rows now only for `/view`, `/md`, `/doc`; `/dump` lists folders
  only. Cmd+P starts the walk on the chord (prewarm). Test
  `writing_and_running_commands_offer_no_file_to_write_over`.
- **R3:** "past its TTL" stayed true until a DRAWN frame pruned — unbounded
  with an occluded window. The poll now prunes (`Toasts::expire`, one
  repaint); at Motion off `wants_frame` is always false. Test
  `expire_owes_one_repaint_then_nothing`.
- CI: clippy 1.100 (CI's stable; local is 1.96) found 1 + 1 + 15 more lints
  across crew-term, crew-render, crew-app (merges 6b1db5f7, 9d4791ca,
  9b941e6f). crew-app can't be linted on 1.100 locally (objc2 0.4.1 breaks
  on macOS; ring blocks a Linux-target check), so CI is its oracle.

## Pending release
- **CI checks again** (R1): the CI workflow had failed on every push for
  weeks — clippy lints and 16 unused workspace dependencies — so it caught
  nothing. Clippy is clean and the unused dependencies are gone; a red CI run
  now means something. No change to the app.

- **`/dump` no longer offers a file to overwrite** (R2 fix): its picker
  listed existing files, and Enter took the highlighted one — so a new name
  you were typing could become an existing file's. It lists folders only.
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
  about five redraws, and then stopped asking one frame early — leaving an
  empty card on screen until something else repainted. It now holds still
  and whole, and costs one frame: the one that takes it away. (loop-30's
  open list; the invariant every reduced-motion setting promises.)

- **Provider chains refreshed; "access denied" says what to do** (R4): the
  OpenAI chain still fell back to `gpt-5`, which OpenAI shuts down on 11
  December, and the Gemini chain to `gemini-2.5-pro`, which Google reroutes
  from 1 December; both are gone. Claude Fable 5.2 and Qwen4 Max can be
  picked in `/model`, with their prices, and Gemini 4 Argon is costed at its
  December rate. A DashScope account that may not call a model
  (`AccessDenied.Unpurchased`) got the bare sentence; it now says to enable
  the model with the provider or pick another with `/model`.

- **The Explorer from the keyboard** (R5): the file tree was mouse-only.
  **Cmd+Shift+E** opens it and gives it the keys — arrows (or j/k) to move,
  → or Enter to open a folder or preview a file, ← to close or go up, Esc
  to hand the keys back. The row you are on is a bar like the far pane's
  cursor, and the pane behind loses its focus ring while the tree has the
  keys. (From VS Code's Explorer.)

- **Editing a long markdown file is quicker** (R6): moving the caret from
  one block to the next in the live preview re-read and re-drew the whole
  document each time — ~7 ms a keystroke in crew's 327 KB manual. The
  document's blocks and its render are now kept until the text changes:
  ~3 ms. (Obsidian's and VS Code's incremental preview.)

- **A refused key stops at once** (R7): with a wrong or revoked key, or an
  unpaid OpenAI, DeepSeek or DashScope account, crew asked every model in
  the provider's chain in turn before saying so — three requests, three
  waits, for an answer none of them could give. It now stops at the first
  and says the key was refused. A failure that belongs to one model (a
  per-model quota, OpenRouter credits with a free model behind) still tries
  the next. (Claude Code's and Cline's fallback rules: only a retryable
  failure falls back — attribution not read from their docs; unverified.)

- **The Explorer shows what git sees** (R8): changed files are coloured
  with a letter at the end of their row — M, A, U, D, or ! for a conflict —
  and folders holding changes carry a dot, as in VS Code. It reads the git
  status the nav already asks for, so it costs nothing extra. (VS Code.)

- **The tab you leave folds away** (R9): switching tabs brought the new
  tab's cards in from its side of the list while the old tab's vanished in
  the same frame. The old cards now fold away underneath, drifting the other
  way, so a switch reads as one movement. Motion off still cuts. (macOS
  Spaces, iOS push transitions.)

- **Fixes to tonight's rounds**: a notification under a resting pointer
  could still time out; the Explorer could take the keyboard back after you
  had left it; on Windows and Linux Ctrl+P is a terminal's again, and
  `/nav explorer` gives the tree the keys there; only cards on screen fold
  away on a tab switch; non-ASCII file names get their git mark.

## Log
| # | lane | merge | title | before → after |
|---|------|-------|-------|----------------|
| R1 | perf | fe0bf52d | CI green | clippy 25 errors → 0, 16 unused deps → 0 |
| R2 | nav | 7f237aa9 | Cmd+P go to file | 1 folder by prefix → whole tree fuzzy, 2/2 real rows for `exhit` |
| R3 | perf | bd923d7d | toasts free at Motion off | ≈20 frames + stuck empty card → 1 frame |
| R4 | ai | 44b9ca9c | provider freshness + access hint | 2 retiring ids in chains → 0; 403 bare → with fix |
| R5 | nav | aaf4da63 | Explorer from the keyboard | mouse-only → Cmd+Shift+E + arrows, 4 tests, 2 shots |
| R6 | perf | 94b5471c | md caret crossings memoized | 7.12 → 2.96 ms per ↓ (release, 327 KB) |
| R7 | ai | 3f435ee6 | account failure ends the chain | refused key: 3 requests → 1 |
| R8 | nav | d797f2bb | git marks in the Explorer | none → M/A/U/D/! + folder dots, 0 extra git calls |
| R9 | ui | b96b9266 | tab you leave folds away | vanish → departing ghosts under the grid |
| F2 | fix | (this) | second-review fix-ups | toast hold, tree re-arm, Ctrl+P off-Mac, ghosts, reader path, quotePath |

## Close (2026-11-22, ~03:50)
Nine rounds plus two review fix-up merges, all merged untagged; CI green
from 9b941e6f on (Windows and Coverage green too). Released by the 8 PM job.

Open, ranked:
1. **perf — typing into a long markdown file:** one keystroke = one full
   render, ~27 ms on the 327 KB manual (release; counter showed exactly one
   `rung_lines` per insert). Needs block-incremental rendering (render per
   top-level block, keyed by its source and width; list numbering and
   footnotes cross blocks).
2. **ai — no live probe possible:** the DashScope account answers
   `AccessDenied.Unpurchased` to every model; R4/R7 are proven by replay and
   a fake server only. qwen4-max as the DashScope head is unprobed.
3. **nav — Explorer:** type-to-find in the tree; reveal the focused file;
   Ctrl+N in a terminal off a Mac is still Cmd+N (same shape as Ctrl+P was).
4. **ui:** cursor blink; OSC 52 read; IME and dead keys (carried over).
5. **process:** nothing tonight was driven in the live GUI — osascript has
   no assistive access in this session. crew-app can only be linted on
   CI's clippy (1.100) by CI itself.
