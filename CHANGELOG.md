# Changelog

Notable changes per release, newest first. Versions are the workspace version
in `Cargo.toml`; every tag builds a release the app picks up through
`/update`.

The top entry must always name the current version — `changelog_covers_the_
current_version` in `crew-app` asserts it, so a release cannot ship without a
line saying what it was.

## 0.6.64

- The "updated to" note waits for the first rendered frame instead of being
  flashed during startup. Status messages expire after three seconds and a
  cold launch takes far longer than that to draw anything, so the note was
  being lost on exactly the launch it exists for.

## 0.6.63

- `/about` opens the changelog that shipped with this binary, in the markdown
  viewer, rather than flashing a version number. The number is only useful as
  a way to find out what changed.
- A build that is not the one that ran last says so on startup and points at
  `/about`. Auto-update lands quietly and applies on the next restart, so
  without this a user can find themselves in a different app with no idea why.

## 0.6.62

- Added this changelog, covering the run of releases below, plus a test
  binding its newest entry to the workspace version.

## 0.6.38 – 0.6.61

One night's work, grouped by what it was for rather than by release. Every
version in the range is a real release with its own tag.

### Fewer commands, because the pane says it instead

Thirty-three constructs became twenty-one. Each deletion moved information
somewhere it could be seen rather than asked for.

- `/agents` — the roster is in the footer, and bare `/model` already printed
  the same report. (0.6.40)
- `/tasks` — running tasks are in the footer, with their ids, because
  `/stop #n` needs one to name. The broker had no way to report a task
  ending until this release: `Tasks::reap` runs lazily on the next command,
  so a task finishing was not an observable moment anywhere. (0.6.41)
- `/status` — its turn count, token total and budget moved to `/doctor`,
  which is already the "state of this stack" report. Nothing was dropped
  without a home. (0.6.42)
- `/cwd` — the working directory leads the footer; the sandbox mode it also
  reported had moved to `/doctor` the release before. (0.6.43)
- `/checkpoint` — checkpoints are automatic now (below). (0.6.44)
- `/approve`, `/reject` — a drafted plan is answered with enter or esc. Both
  still route, for hosts driving the broker without a keyboard. (0.6.46)
- `/checkpoints`, `/skills` — folded into `/restore` and `/skill`, which list
  when given no argument. (0.6.54)
- `/compact` — the transcript folds itself and says so. (0.6.55)

### Things that now happen without being asked

- **A checkpoint before every task that can touch files.** Predicting which
  task is worth protecting is asking the user to be right in advance. Silent
  when the tree has not changed, capped at 25 snapshots, and ordered by a
  sequence in the ref name because git timestamps are second-resolution and
  automatic snapshots land inside one second. (0.6.44)
- **A signed-in CLI is a working crew.** `claude`, `codex` and `opencode`
  join the roster on PATH, with no key at all — the adapters had existed
  since the beginning with nothing calling them. A plain message relays to
  one instead of running the offline stub swarm. (0.6.39)
- **Choosing a model chooses the provider.** Holding two vendors' keys used
  to mean picking a model from the wrong one silently answered from the
  other. (0.6.51)
- **The transcript folds itself**, announcing what it folded, where it used
  to drop messages in silence at 500. (0.6.55)

### Providers

- Providers are a table: an endpoint, a key variable, a model chain and the
  vendor it serves. OpenAI, Gemini and DeepSeek joined DashScope, OpenRouter
  and Anthropic. `OPENROUTER_API_KEY` had been the answer for every vendor
  because OpenRouter was the only multi-vendor route; a row now asks for its
  own vendor's key. (0.6.47, 0.6.48)
- Every provider key is probed from the login shell, not just the original
  three — a key crew never imports is indistinguishable from no key at all.
  (0.6.49)
- One copy of the "no provider" advice, shared by the broker's roster line,
  `/doctor`, the Far pane's ask and the pane's empty state. Four wordings
  existed and two had gone stale. (0.6.50)
- A CLI that is installed but not signed in says so, instead of returning an
  empty reply. Empty stdout was treated as success. (0.6.60)

### Reading code in the pane

- **Syntax highlighting** in fenced blocks — comment, string, keyword — from
  a small hand-rolled lexer. Tokenized before wrapping, so a string crossing
  a wrap boundary keeps its colour. (0.6.45)
- Semantic colours clear a floor against body text, not just against the
  page. On the CRT themes code had measured 1.04:1 against prose, which is
  the same colour. (0.6.38)
- The syntax classes sit on a lightness ladder and keywords are marked by
  weight, so highlighting survives a single-phosphor tube where hue cannot
  vary. (0.6.45)

### The pane

- Grouped command palette while browsing, flat while filtering. (0.6.52)
- The footer fits: segments carry a priority and the least important go
  first, rather than the line being clipped from the right. (0.6.57, 0.6.58)
- An unknown `/command` says so and guesses, instead of doing nothing.
  (0.6.53)
