# Incomplete-feature loop — playbook

**Goal:** every feature Crew claims — in `README.md`, `docs/CREW.md`, a shipped
key binding, a construct the router answers, a config field, a CLI flag — is
**fully reachable and fully wired**, or it is removed. No half-landed surface,
no code path only a test can reach, no documented behaviour that does nothing.

**Owner:** autonomous (Claude Code). **Repo:** `/Users/atyagi/code/crew`.
This file is the single source of truth for an iteration. Each firing reads it
plus the tail of `incomplete-feature-log.md` and runs ONE iteration end-to-end,
so it survives context compaction.

---

## What counts as "incomplete"

A feature is incomplete when **any leg of it is missing**, not when it is absent
entirely. Missing features are the `/crew` loop's job; this loop finishes what
was already started. The legs:

| Leg | Incomplete when |
|---|---|
| Implementation | the path exists but bails early, returns a stub value, or handles only the happy case |
| Reachability | the code works but no key, construct, palette entry, or menu reaches it |
| Docs | it works and is reachable but `README.md` / `docs/CREW.md` never says so |
| Inverse-docs | docs promise it but the code doesn't do it (or does something else) |
| Tests | it works but nothing pins the behaviour, so the next edit can silently delete it |
| Platform | it works on macOS only, and the other arms are `cfg`'d-out shells |

**The canonical case** is recorded in the source itself
(`crates/crew-plugin/src/broker/commands.rs:100`):

> `/reload` shipped for eleven releases as a working command no palette offered,
> because two lists existed and nothing compared them.

Working code. Zero users could reach it. That is the bug class.

## Hunt — how to find them

**Do not start with grep for `TODO`.** As of 2026-07-27 the workspace has zero
`TODO`, `FIXME`, `XXX`, `HACK`, `todo!()` and `unimplemented!()` markers. A
marker sweep returns clean and would let the iteration declare "nothing found"
while real gaps sit in plain sight. Run these lenses instead, in this order:

1. **Two-list drift.** Any place the codebase keeps a second copy of a set.
   Compare them mechanically, and where possible leave behind a test that
   compares them forever (`broker_constructs()` exists for exactly this).
   - broker `CONSTRUCTS` vs. the host palette vs. `/help` text vs. `README.md`
   - key bindings in the app vs. `/keys` output vs. documented shortcuts
   - `Theme` fields vs. what the settings pane can actually edit
   - env vars read (`CREW_*`) vs. env vars documented
2. **Doc-claim diff.** Read `README.md` and `docs/CREW.md` as a spec. For each
   concrete claim (a flag, a key, a construct, a config key), find the code that
   honours it. A claim with no code is a finding; code with no claim is a finding.
3. **Half-wired surface.** A field that is written but never read, a variant
   never constructed, an event emitted that nothing consumes, a `pub fn` with a
   single caller in tests. `#[allow(dead_code)]` outside a genuine
   `cfg_attr(target_os = …)` arm is a strong smell — the platform ones in
   `appregister.rs` / `reglinux.rs` / `regwin.rs` are legitimate; others are not.
4. **`cfg` asymmetry.** A feature implemented under `target_os = "macos"` with
   Linux/Windows arms that return `Ok(())`. Either finish the arm or make the
   docs say macOS-only.
5. **Spec leftovers.** `docs/superpowers/specs/*.md` "Open questions" and
   "Deferred" sections, and `.superpowers/sdd/progress.md` unchecked tasks /
   "Minor findings (for final review triage)". Something deliberately deferred
   and never revisited is fair game if it is now cheap.
6. **Error-path stubs.** A `match` whose failure arm swallows the error, a
   timeout with no user-visible message, an empty state that renders nothing.
   The feature "works" until it doesn't, and then it is silent.

Record every candidate — including ones you don't fix — in the ledger. The
unfixed list is the next iteration's starting point.

## Rank and pick

Score each candidate: **user-visible impact** × **evidence it is really broken**
÷ **effort**. Prefer a gap you can prove with a failing test.

Fix **one** gap per iteration, end-to-end and completely. Two half-fixed gaps is
the failure mode this loop exists to delete. If the chosen gap turns out to be
larger than one iteration, log it with a scoped plan and pick a smaller one.

## Per-iteration deliverables

Branch off current `main`: `auto/finish-<N>-<slug>`.

1. **Prove it.** Write the failing test first (RED) — the test that would have
   caught the gap. For reachability gaps, that is usually an assertion comparing
   the two lists, not a UI test. For a gap no test can express (a rendering or
   platform gap), state in the ledger exactly how you verified it by hand.
2. **Finish it.** GREEN. Complete every leg from the table above, not just the
   one you noticed — if you wire the palette, also fix `/help` and the README.
3. **Close the class, not just the case.** If the gap came from two lists, make
   one of them derive from the other, or add the comparison test. A fix that
   leaves the next drift possible is half a fix.
4. **Docs.** Update `README.md` / `docs/CREW.md` in the existing style. Add
   only; do not rewrite surrounding content.
5. **Ledger.** Append the iteration section (format below).

## Gate

**Bump the version FIRST, then gate.** `changelog_covers_the_current_version`
asserts `CHANGELOG.md`'s top entry names the workspace version, so bumping after
a green gate turns it red and ships anyway — `release.yml` only builds, it never
runs tests. Iteration 1 did exactly that. Order: bump `Cargo.toml` → write the
`CHANGELOG.md` entry → `cargo check` (regenerates `Cargo.lock`) → gate.

From the branch, run ALL of:

- `cargo fmt --all` (then confirm no diff)
- `cargo clippy --workspace --all-targets -- -D warnings` — the **whole**
  workspace warning-free, not just the diff. Delete dead code; never suppress it.
- `cargo test --workspace`
- **Review the diff** for the guardrails below — especially the 200-line cap,
  which a "finish the feature" edit is unusually likely to push a file past.

**GREEN** = fmt clean AND clippy clean AND all tests pass AND no guardrail broken.

### If GREEN → merge and release
1. Bump the workspace version by one patch in `Cargo.toml`; run `cargo check` so
   `Cargo.lock` regenerates, and `git add Cargo.toml Cargo.lock` **together** —
   a dirty lock file aborts the later checkout.
2. `git checkout main && git merge --no-ff auto/finish-<N>-<slug>`, then delete
   the branch.
3. `git tag vX.Y.Z && git push origin main --follow-tags`. The tag push triggers
   `.github/workflows/release.yml`. **Never build a release locally** — disk runs
   low; CI owns release binaries and `/update` delivers them.
4. Verify it published: `gh run list --workflow=release.yml`. All five build
   targets must pass — a Windows failure skips the `release` job and no assets
   ship. If the release job was skipped or failed, log the iteration as NOT
   released.

### If NOT GREEN → WIP, no release
Commit the work on the branch, do not merge, do not tag, and log the exact
blocker. A gap left visibly unfinished beats a broken release.

## Ledger

Append one section to `docs/superpowers/incomplete-feature-log.md`:

```
## Iteration <N> — <local timestamp> — <RELEASED vX.Y.Z | WIP (blocked: …)>
- Gap: <what was incomplete, and which leg was missing>
- Evidence: <the failing test, or how it was verified by hand>
- Fix: <what landed> [<files>]
- Class closed: <what now prevents the same drift> | none (why)
- Docs: <README/CREW.md lines touched>
- Gate: fmt <ok> · clippy <ok> · tests <n pass>
- Release: <tag | skipped: reason>
- Candidates found, not fixed: <list — the next iteration's queue>
```

## Guardrails (never violated, whatever the gap)

- **Don't grow files needlessly.** The `/crew` guardrails state a hard 200-line
  cap per `.rs` file; as of 2026-07-27, 130 files already exceed it, so treat it
  as direction, not a gate. Prefer splitting along responsibility boundaries
  when a file you touch is already large — but never let a mass re-split become
  the iteration. Finishing the feature is the iteration.
- **No overlay UI.** In-pane UI is laid into a `ratatui` `Buffer` and converted
  to GPU cells. Panels are rounded cards with a fieldset legend on the top border.
- **Auto-tiling grid only** (`cols = ceil(sqrt(n))`, LRU demotion to the strip).
  No layout-switching system.
- **Terminal keys pass through** — inside a focused terminal pane every key
  except Crew's own chords reaches the running program.
- **No new dependencies** without first confirming the capability isn't already
  in the current dep tree.
- **Never break working behaviour to finish an unfinished one.** If completing a
  feature requires changing something that already works, log it and stop.
- **Removal is a valid completion.** If a half-landed feature is not worth
  finishing, delete it and its docs. Say so explicitly in the ledger — an
  intentional removal is a finished iteration, not a skipped one.

## Stop condition

The loop ends when an iteration's hunt runs all six lenses and produces **zero**
candidates, and the previous iteration's "candidates found, not fixed" list is
empty. Log that verdict explicitly — an empty hunt is a result, not a no-op.

---

## Seed backlog (2026-07-27, unverified — confirm before fixing)

Starting candidates from the first sweep. Each needs verification; a grep is a
hypothesis, not a finding.

- **Broker constructs missing from `README.md`.** `CONSTRUCTS`
  (`commands.rs:95`) lists 20; a README sweep for single-backtick command names
  did not surface `/fan`, `/loop`, `/plan`, `/approve`, `/reject`, `/standup`,
  `/stop`, `/help`. Verify against the full README text (the sweep matched only
  one form of mention) before treating it as drift. This is the `/reload` class
  exactly — and `broker_constructs()` already exists to assert against.
- **`docs/CREW.md` documents no slash commands at all** by the same sweep, while
  README documents ~37. Confirm whether CREW.md is meant to carry them; if so
  that is a whole documented surface missing.
- **`#[allow(dead_code)]` outside platform `cfg_attr`** — `osc7.rs:170`,
  `broker/mod.rs:103`, `chatpulse.rs:58,67`, `chatflow.rs:13`. Each is either a
  feature never wired up or a suppression that should be a deletion.
- **Spec "Deferred"/"Open questions" sections** in
  `2026-06-20-crew-terminal-design.md:311`, `2026-06-27-crew-agent-swarm-design.md:239`,
  `2026-06-21-crew-chat-plugins-design.md:219`, `2026-05-16-update-command-design.md:133`
  — triage for anything now cheap.
- **`.superpowers/sdd/progress.md` "Minor findings (for final review triage)"** —
  five specific nits recorded for a triage pass that appears never to have run.
