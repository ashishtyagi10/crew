# Nightly autonomous improvement loop — iteration playbook

**Created:** 2026-07-02 23:xx EDT. **Window:** hourly until **2026-07-03 08:00 EDT**.
**Owner:** autonomous (Claude Code, session-only cron). **Repo:** `/Users/atyagi/code/crew`.

This file is the single source of truth for each loop iteration. The cron prompt
points here so every firing is self-contained (survives context compaction).

## Hard stop
At the START of every iteration, get the local time (`date`). **If it is
2026-07-03 08:00 EDT or later: do NOTHING except delete the cron
(`CronList` → `CronDelete <id>`) and write a final summary line to the log.**
Do not start new work past 08:00.

## Preconditions (each iteration)
1. `main` must be clean and releasable. `git checkout main && git pull` (or ensure up to date).
   If a previous iteration left an un-merged `auto/nightly-*` branch, ignore it — start fresh from `main`.
2. Read this playbook and the tail of `docs/superpowers/nightly-loop-log.md` to see
   what prior iterations already did (avoid repeating a feature/source).

## The six deliverables (one branch per iteration: `auto/nightly-<N>-<slug>`)
Branch off the latest `main`. Do all six, then gate (below).

1. **Port one feature** from **opencode**, **claude-code**, or **codex** — rotate the
   source by iteration number: `N % 3 == 0` → claude-code, `== 1` → codex, `== 2` → opencode.
   Pick a SMALL, self-contained, genuinely useful feature not already present. Candidate menu
   (pick an unused one; cross it off in the log):
   - claude-code: slash-command aliases; `/resume` last session; output-style presets;
     `/compact` transcript summarizer for the crew pane; per-agent `/model` shortcut chips.
   - codex: approval/sandbox profiles for `sys:run` (read-only vs full); `/diff` pane showing
     working-tree changes; `--cd` working-dir switch; token-budget `--max-tokens` guard surfaced in UI.
   - opencode: theme presets/switcher; multi-model routing hints; share/export link;
     LSP-style file hover in far panel; fuzzy command palette.
   Keep it to 1–3 files where possible; TDD (RED→GREEN); match surrounding code style.

2. **UI feature** — add/update/improve ONE UI element in crew-app (crew pane, far panel,
   status area, composer, or terminal). Small and polished. Cell-render tests where applicable.

3. **Test + security review** — run the FULL suite and a security pass (see Gate).

4. **Token-usage optimization** — one concrete improvement that reduces tokens the broker/agents
   spend or surfaces/controls usage: e.g. tighter relay prompts, transcript trimming before a hop,
   a context-window guard, dedup of repeated system text, a `/status` token-efficiency hint, or
   caching a repeated hint. Measure or reason about the saving in the report.

5. **UX improvement** — one change that makes the app nicer to use (perf, clarity, a papercut fix,
   a helpful default, better copy). Can overlap with #2 but must be a distinct change.

6. **Commit / push / release** — gated (below).

## Gate (decides release vs WIP)
Run, from the loop branch, ALL of:
- `cargo fmt --all -- --check`  (or `cargo fmt --all` then ensure no diff)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- **Security review** (Workflow, ultracode): fan out reviewers over the iteration diff
  (`git diff main...HEAD`) — lenses: injection/`sys:run` shell-safety, path traversal in file ops,
  unbounded resource use / DoS, secret handling, unsafe/`unwrap` panics on untrusted input,
  dependency/version changes. Adversarially verify each finding (skeptics). Classify survivors
  Critical / High / Medium / Low.

**GREEN** = fmt clean AND clippy clean AND all tests pass AND **no Critical or High** security finding.

### If GREEN → release
1. Bump the workspace version by one **patch** in `Cargo.toml` (`version = "0.5.X"` → `0.5.X+1`).
   (Match the existing release convention: a `Bump version to 0.5.X` commit is standard.)
2. Commit the feature work + version bump on the branch (`cargo fmt` first; pre-commit runs fmt+check).
3. `git checkout main && git merge --ff-only auto/nightly-<N>-<slug>` (fast-forward; if it can't
   ff, `git merge --no-ff` is acceptable). 
4. `git tag v0.5.X+1 && git push origin main --follow-tags` (pushing the `v*` tag triggers
   `.github/workflows/release.yml`).
5. **Verify the release actually published**: `gh run watch` / `gh run list --workflow=release.yml`
   — confirm the `build` matrix (all 5 targets incl. **Windows** — a Windows failure SKIPS the
   `release` job, so no assets publish) AND the `release` job succeeded and uploaded assets.
   If the release job was skipped/failed, treat the iteration as NOT released and log it loudly.

### If NOT GREEN → WIP, no release
1. `cargo fmt` + commit the work-in-progress on the loop branch.
2. `git push -u origin auto/nightly-<N>-<slug>` (push the branch; do NOT merge to main, do NOT tag).
3. Log the exact blocker (which check failed / which Critical|High finding) in the loop log.
   Do not release broken binaries unattended.

## Log (every iteration, GREEN or not)
Append one section to `docs/superpowers/nightly-loop-log.md`:
```
## Iteration <N> — <local timestamp> — <RELEASED v0.5.X | WIP (blocked: …)>
- Feature (<source>): <what> [<files>]
- UI: <what>
- Token opt: <what> (~saving)
- UX: <what>
- Gate: fmt <ok> · clippy <ok> · tests <n pass> · security <Crit/High count>
- Release: <tag + run URL | skipped: reason>
- Crossed off menu: <items>
```

## Then re-check the clock
Iterations are driven by the hourly cron; each firing runs ONE iteration end-to-end.
After finishing, if the next hour would be past 08:00, delete the cron. Otherwise wait for
the next fire. Keep each iteration within its hour where possible; if it runs long, the next
cron fire will simply wait until the REPL is idle.

## Ground rules
- Never leave `main` un-releasable. If anything goes wrong mid-merge, `git reset` main back to
  the last good commit and fall back to the WIP path.
- Ultracode is on: use Workflows with adversarial verification for the feature build, the review,
  and the security pass. Be exhaustive; correctness over speed.
- Prefer small, real, tested changes over large speculative ones. A tested papercut fix that ships
  green beats an ambitious feature that blocks the release.
