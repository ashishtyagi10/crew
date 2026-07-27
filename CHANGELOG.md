# Changelog

Notable changes per release, newest first. Versions are the workspace version
in `Cargo.toml`; every tag builds a release the app picks up through
`/update`.

The top entry must always name the current version — `changelog_covers_the_
current_version` in `crew-app` asserts it, so a release cannot ship without a
line saying what it was.

## 0.6.96

- The input bar says which pane it is talking to. The focused pane's name now
  rides the bottom border as a right-aligned legend, mirroring the working
  directory on the top one — typing there has always acted on the selected
  pane, and the bar never said which one that was. A transient status message
  borrows the slot while it flashes and gives it back.

## 0.6.95

- The slash palette stopped disagreeing with the commands it labels. `/goal`
  was described as "set the crew's shared goal" — a feature that exists
  nowhere; it runs relay rounds until a judge agent rules the goal met. Every
  hint the pane does not deliberately own is now the broker's own `/help`
  sentence, so the two cannot drift apart again.
- `/goal` means two different things and now says so. In the agent smith
  composer it is the judged relay; in the command bar it plans a task graph and
  runs it as a swarm. Same word, two engines — the docs and the command-bar
  palette both name the split.
- The shell's deadline fits a real build. `sys:run` had 30 seconds, which
  `cargo test` cannot finish inside on any real project, so the shell's most
  valuable use was its broken one. It is 120 seconds now, and the timeout
  message names `CREW_SYS_TIMEOUT_MS` instead of leaving you to guess a knob
  exists.

## 0.6.94

- Three providers that shipped fully wired and entirely unmentioned are
  documented: **OpenAI**, **Gemini** and **DeepSeek** each have a key variable,
  an endpoint and a native model chain, and neither the README nor the manual
  ever said so. `docs/CREW.md` now carries the table.
- They probe **last** in discovery on purpose, so adding one never changes
  which provider an existing install resolves to — which means
  `CREW_PROVIDER=gemini` is the only way to reach one while another key is set.
  That pin worked all along and was documented nowhere, including in the
  function's own comment.

## 0.6.93

- An agent that runs out of tool calls in a turn says so. There is a budget of
  four per turn; spending it just ended the turn, leaving the agent's unrun
  `@tool` line standing as its answer with nothing anywhere explaining why it
  stopped halfway through what it was doing.
- The agent is also told how many it has left, and told plainly when the next
  one is its last. A budget it cannot see is one it plans straight past.

## 0.6.92

- `@src/main.rs:120-180` attaches just those lines. `@file` attached the whole
  file and anything past 64 KB was skipped outright — so the file you most
  want to point an agent at one function of was the file you could not point
  it at at all. `:120` is a single line; the size limit now applies to what is
  attached rather than to the file on disk, and the "too large" note names the
  idiom instead of only saying no.
- A file genuinely named `odd:10` still wins over reading that as a range, and
  a colon anywhere else in a name is left alone.

## 0.6.91

- A tool call reads as what it is. Every call an agent makes is logged in the
  pane so you can see what is happening to your machine, but it was shown in
  wire form — `[tool] sys:write_file {"path": "src/lib.rs", "content": "use
  std::fmt;\n…` — where the one thing worth reading is buried in punctuation
  and, for a write, the line is mostly the first 150 bytes of the file. It now
  reads `sys:write_file  src/lib.rs`. A tool whose arguments say nothing
  identifiable still shows them, so nothing goes missing.

## 0.6.90

- The built-in `sys` tool surface is decided once, when a session's broker is
  built, instead of being re-read from the environment on every hint and every
  tool call. It is meant to be stable for the life of a session, and one of
  the variables it read is set and cleared by the test harness while other
  tests are running — which made two tests fail about one run in six.

## 0.6.89

- A test no longer switches text streaming off for the whole process. Two
  tests set `CREW_STREAM_TEXT` and were serialised against each other but not
  against anything else, so any test building a text streamer at that instant
  saw streaming disabled. The switch is handed in now; nothing mutates the
  environment.
- Typing `/stop` drops the queue too. Esc has since the last release; the
  typed form is the same intention and sends the same string, and it was
  still stopping one run and starting everything behind it. `/stop #2` is
  left alone — calling off one of several parallel tasks says nothing about
  the rest of the session.

## 0.6.88

- Interrupting also drops what you queued behind the run. Esc stopped the
  agent, which made the pane idle, which is exactly what flushes the queue —
  so a stop immediately started every follow-up waiting behind it, each one
  written on the premise that the interrupted work was going fine. It says how
  many it dropped.
- Three tests that assert something is absent from a rendered pane now check
  the pane rendered at all first. None of them were passing vacuously; the
  guard is so that they cannot start to.

## 0.6.87

- A pane opened in a project you have worked in before says so, and names
  `/resume`. The previous conversation has been rotated into a resumable file
  since long before this and announced nowhere, so the only way to find out it
  was there was to guess the construct and try it. It offers rather than
  resumes: folding yesterday's conversation into today's first task spends
  context nobody asked for, and yesterday's task is as often unrelated as not.
- A first run in a fresh project still says nothing.

## 0.6.86

- The agent composer suggests the rest of a prompt you have sent before, dim
  after the caret, taken with Tab or Right. The docked input bar has done this
  since long before; the composer — where the long prompts are actually typed
  — did not. Tab keeps its existing meaning and gains this only where it used
  to do nothing.
- A leading `/` is left to the palette, which is already answering the same
  question as a popup.

## 0.6.85

- `/restore` removes the files the task created, instead of putting the edits
  back and leaving the new ones for you to find and delete by hand. An undo
  that covers half a change is not an undo, and the half it skipped was the
  one most likely to be unwanted — the half-written module a run was abandoned
  over. It names every file it deletes.
- Nothing that predates the snapshot is touched, and ignored files — build
  output, an untracked `.env` — are never candidates.

## 0.6.84

- `/review` reviews the files the agent created, and `/commit` describes them.
  Both read one diff, and it came from `git diff` — which cannot see a path git
  has never been told about. A review of a change whose main file is a new
  module was a review of everything except the code, reported as a clean pass.
- `/commit apply` commits what it described. It ran `git commit -am`, which
  stages tracked modifications and nothing else, so the new file stayed behind
  while the pane reported a successful commit. Staging a subset by hand still
  commits exactly that subset — that is a statement about which change you
  mean, and it is still respected.

## 0.6.83

- `/diff` shows files the agent created. It ran `git diff --stat`, which
  compares the working tree to the index — so a new file was untracked and
  invisible, and anything staged was excluded too. Those are the two states an
  agent most often leaves a repository in, and both printed "working tree
  clean — no changes". It now compares everything that exists against the last
  commit, and works in a repository with no commits at all.
- It excludes `.crew/` for the same reason the end-of-task note does, so the
  note and the command it names cannot disagree.

## 0.6.82

- A finished task says which of your files it changed. A clean run printed its
  reply and nothing else, so an agent that edited four files and answered
  "done" left no record anywhere of which four — you had to know about `/diff`
  and remember to run it. No new bookkeeping: the checkpoint taken before
  every task already pins the tree to compare against.
- It stays quiet when nothing of yours changed, which needed care — the broker
  rewrites `.crew/session-live.md` as every reply streams, so the answer to
  "what changed?" would otherwise have been "the transcript" after every
  question ever asked, in every repo but crew's own.

## 0.6.81

- Up and Down in an agent pane recall what you already sent, filtered by what
  you have typed — the docked input bar's rule, which the composer did not
  share. The arrows were classified for the popups and then dropped when no
  popup was open, so changing one word of a long prompt meant retyping it.
- `/keys` shows each description in full. The overlay was a fixed 58 columns
  wide, which left 30 for a description, and eight rows had outgrown it: `Esc`
  read "Discard a pending plan · inter", losing both the interrupt and the
  close, and `@a+b` lost "in parallel".

## 0.6.80

- The provider-key tests assert the same thing on every machine. They read the
  process environment and asserted only when the key was absent, so on any
  machine that had one they passed without testing anything — and they never
  covered an exported but blank key, which is the case that matters.

## 0.6.79

- The `@` picker says that `@a+b` fans a task out to both agents in parallel,
  and `/keys` lists it. The idiom worked and was documented in exactly one
  place the app never shows.

## 0.6.78

- Cmd+click a code block in an agent pane to copy it. Reading an answer and
  using it are different acts, and the second had no support beyond selecting
  text with the mouse. The fence chrome and language tag do not come with it,
  so there is nothing to strip after pasting.

## 0.6.77

- The first automatic snapshot of a session says so, once, and names
  `/restore`. Checkpoints have been automatic and entirely silent since
  0.6.44 — an undo nobody knows about is an undo nobody uses.

## 0.6.76

- The welcome screen names the agent pane. It said "new shell" and "commands"
  and nothing else, so the reason crew is not just a terminal went unmentioned
  on the one screen a first run is guaranteed to see. It also picks a form
  that fits the window instead of vanishing on a narrow one.
- The installer says that agents work with no API key at all when you are
  already signed in to claude, codex or opencode.

## 0.6.75

- Grouped the entries below by what the work was for. Thirteen consecutive
  per-release sections trace history well and read badly; the run was one body
  of work and now says so.

## 0.6.62 – 0.6.74

The second half of the same run. Mostly the consequences of the first half —
six providers where there had been three, ten fewer constructs — arriving as
messages that named the wrong thing, state that outlived what owned it, and
documents describing commands that no longer existed.

### Failures that now say something

- A provider's API error is a sentence, not a JSON envelope pasted into the
  chat and truncated mid-structure. A rejected key names the fix; it is the
  most common provider error and the only one a user can always fix. (0.6.68)
- A missing key names the variable that is actually missing. It said
  `ANTHROPIC_API_KEY` whatever had failed, and the OpenAI-wire client backs
  six providers through six variables. (0.6.69)
- An unknown `/command` says so and guesses, rather than doing nothing.
  (0.6.53)
- A CLI that is installed but not signed in says so. Empty output from it was
  being treated as a successful empty reply. (0.6.60)

### Things that stopped outliving what owned them

- Closing an agent pane stops the agents it was running. The broker died; the
  `claude` or `codex` it had spawned survived, reparented and still working.
  (0.6.72)
- A lost broker no longer leaves phantom running tasks in the footer, or a
  pending plan that still answers to enter. (0.6.67)
- The transcript fold marker counts everything it ever folded, and folds in
  batches rather than copying the whole transcript on every message past the
  cap. (0.6.66)

### The pane fits

- All three footer lines budget themselves and drop the least important
  segment rather than being clipped from the right. A 40-column pane was
  showing "enter runs it" and cutting "esc discards it" — half an
  instruction. (0.6.57, 0.6.58)
- `/keys` documents the agent pane, and fits a default window; it was 58 rows
  and silently truncated. (0.6.65)

### Knowing what you are running

- `crew --version` prints a version instead of launching the window;
  `crew --help` lists the CLI modes. (0.6.71)
- `/about` opens the changelog that shipped with the binary, and a build that
  is not the one that ran last says so on startup. (0.6.63, 0.6.64)
- This changelog exists, bound by a test to the version it ships with.
  (0.6.62)

### Guards, so this does not happen again

- The docs stop describing constructs that no longer exist, and a test fails
  the build if they ever do. (0.6.73)
- An end-to-end test sends every construct the router advertises to a real
  broker. (0.6.74)
- The test harness clears every provider key rather than the three it was
  written against — with an `OPENAI_API_KEY` exported, nobody could run the
  suite green. (0.6.70)

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
