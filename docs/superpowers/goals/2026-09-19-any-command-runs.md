# Goal — any command typed in the input box opens a shell and runs it

**Status: SHIPPED v0.22.78. Set 2026-09-19, built the same day.** Every
done-means below holds in the tree except 7, which found nothing to change:
no key legend or `/help` line ever said "not a command" — only the two hint
strings did, and they are gone. Builtins and unresolved words open the
interactive shell (`runpane.rs`, `run_in_shell`); a resolved binary keeps
the wrapper; `route.rs` carries the shape. Not GUI-verified: the evidence is
the routing tests, the preview-row tests and two real-shell marker tests.

## The ask

> write a goal to execute any command from input box, it should open a shell
> and run it

## What it means

The input bar already routes a bare line (the 2026-07-07 smart input bar
spec): an idle focused shell receives it, and otherwise crew looks the first
word up on PATH and spawns a pane when it finds a binary. What the bar does
NOT do is run everything. Three kinds of line end in a grey status hint and
nothing happens:

- a shell builtin — `export FOO=1`, `source .env`, `alias ll='ls -l'`,
  `set -o vi`, `eval "$(direnv hook zsh)"`;
- shell syntax whose first word is not a binary — `for f in *.md; do …`,
  `if …`, `(cd x && make)`, `{ a; b; }`, `time cargo test`;
- anything crew's PATH check cannot see — a function or alias from the
  user's rc, a binary the hydrated PATH has not arrived for yet, a typo.

The spec called these hints "instead of a junk pane". The premise was wrong
twice over. First, the pane a bare line spawns is not junk: the wrapper
`exec`s the user's shell after the command, so a typo leaves a live prompt
showing `command not found`, which is exactly what a terminal does. Second,
crew's PATH check is a worse judge than the shell. It does not know aliases,
functions, keywords, or the PATH the rc file sets, and every line it turns
away is one the user then has to retype with a `!` in front.

So the rule becomes one sentence: **a bare line in the input bar is a shell
command, and the shell is the judge.** Crew's own check stops being a gate
and becomes advice in the preview row.

One reading to make explicit: "open a shell" does not mean "open a second
shell". An idle shell in front of you IS the shell. It keeps receiving the
line as keystrokes, because it is where the user's aliases, functions and
history live and because a new pane for every `ls` would be the worse
experience. A new shell opens when there is no idle shell to type into: no
panes, a busy terminal, a chat, markdown, todo or settings pane focused.

## Done means

1. **No bare line ends in a hint.** Every line that is not a construct
   (`/cmd`, `!cmd`, `*text`, `?ask`, `??explain`, `cd`) runs in a shell:
   the focused idle shell if there is one, else a new pane. `route.rs`
   loses its two hint arms; `route_bare` returns `TypeInto` or `Spawn`,
   nothing else, and the routing tests say so.
2. **Builtins, keywords, pipelines and env prefixes run.** Each of these
   lines, submitted with no idle shell focused, opens a pane, runs, and
   leaves a prompt: `export FOO=1`, `for i in 1 2; do echo $i; done`,
   `FOO=1 env | grep FOO`, `ls | head`, `(echo a; echo b)`. A real-shell
   test with a marker echo, in the style of `runpane_tests.rs`, covers
   each shape.
3. **State survives into the prompt that follows.** After `export FOO=1`
   the pane's shell has `FOO` set; after `alias ll='ls -l'` the alias
   works at the prompt; after `source file` what the file defined is
   there. This is what "run it in a shell" means for a builtin, and it is
   the reason the persistent wrapper is not enough on its own (see below).
4. **The preview row stays honest.** With no idle shell the palette shows
   `↵ run — new pane`, and when crew's check misses the first word it
   appends a dim `· not on PATH` so a typo is still visible before Enter.
   Enter does exactly what the row says, in every case; the existing
   "the preview mirrors submit" invariant holds.
5. **A typo does not litter.** The pane a mistyped line opens is a live
   shell, focused, labelled by the first word; the corrected line typed
   next goes into that same pane (the idle-shell rule), not a third one.
6. **`!` and `/run` keep working** as synonyms for the bare line, for
   muscle memory and scripts. Their palette rows and hints stop claiming
   they do something the bare line does not.
7. **The key legend and `/help`** describe the bar's prefixes without
   the phrase "not a command": `/` app command · `!`/bare = run in a
   shell · `*` broadcast · `?` ask.

## How it is built

- `route.rs` — `BareRoute` shrinks to `TypeInto(usize)` and `Spawn`; the
  `Verdict` still flows in, but only `input_preview` reads it (for the dim
  `not on PATH` suffix). `submit_input` in `appsubmit.rs` has one arm per
  variant and no `set_status` in the bare path.
- `cmdcheck.rs` — unchanged as a checker; its `BUILTINS` list becomes the
  list of first words that need the interactive path below, and its
  `Verdict::No` becomes preview advice. The hydrated-PATH thread stays.
- `runpane.rs` — the persistent wrapper (`bash -c 'set -m; body; exec
  $SHELL'`) serves every line whose effect is output: binaries, pipelines,
  keywords, env prefixes. It cannot serve a builtin, because the state
  a builtin sets dies when `exec` replaces bash with the user's shell
  (an `export` survives exec, an `alias` or a `source`d function does
  not). For those, and for a first word crew cannot resolve (it may be an
  alias or a function), the pane opens the user's interactive shell and
  types the line as its first input, the same `submit_bytes` path the
  idle-shell rule uses. The shell echoes it at the prompt and runs it with
  every rc-defined name in scope. `run_parts` grows a second constructor
  for this shape; the label is still the first word.
- Type-ahead into a shell that has not reached its prompt is ordinary
  tty typeahead and works in every terminal, but the test that proves it
  waits for the marker, not for a timer.
- `cwd.rs` — `cd` stays crew's own: it moves the app's working directory,
  which every later spawn inherits. It is not a command run in a shell
  and the doc says so, so the one exception is deliberate.
- Windows: the wrapper is PowerShell already; the same rule applies and
  the builtin list is empty there until someone reads it against
  PowerShell's cmdlets. `foreground_pid()` is always `None` on Windows,
  so a focused terminal always counts as idle.

## What it must not break

- The busy-terminal rule: the bar never types into a running program.
  A line with a busy focused terminal opens a new pane, as today.
- The `cd` construct, the `*` broadcast and the `?`/`??` asks, which are
  checked before the bare route and stay where they are.
- The line cap: `route.rs` and `appsubmit.rs` shrink; `runpane.rs` at
  133 lines takes the second constructor, and if the interactive path
  needs more than a screen it gets its own file.

## Not done

- No GUI verification until the loop runs it: the evidence is the routing
  tests, the real-shell marker tests and the preview-row tests.
- Nothing here changes what the smith pane does with plain English; that
  pane has its own composer and the bar has never fed it.
