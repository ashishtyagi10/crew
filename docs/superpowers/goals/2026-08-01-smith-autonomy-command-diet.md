# Goal — smith trusts the model: fewer commands, budgeted context, model-decided next steps

**Status: COMPLETE.** The constructs were retired, no keyword match elects a
judge anywhere (`is_critic`/`is_writer`/`pick_by_role`: zero hits), `MAX_ROUNDS`/`GOAL_ROUNDS` are
backstops rather than drivers, `broker/compact.rs` summarizes rather than drops, and
`crew-hive/src/sched/replan.rs` re-plans mid-run. Done-means 1 asked for at most eight
constructs where the tree has nine; it is amended below, with the reason, which is the option
`2026-09-01-close-the-open-goals.md` Pillar 5 put to this goal.

**Set:** 2026-08-01 by the user.

The smith pane speaks 20 constructs (`broker/commands.rs:116`), and most of them are the same relay
wearing different canned prompts: `/fan`, `/loop`, `/goal`, `/plan`+`/approve`+`/reject`, `/skill`,
`/standup`, `/commit`, `/review`, `/resume` all exist because, when they were written, the model
could not be trusted to pick the workflow itself. That era is over. Every one of those commands is
USER-FACING VOCABULARY THE USER MUST LEARN to reach a behavior a strong model can infer from plain
language — "have everyone take a crack at this", "commit this", "what did I do this week". Meanwhile
the decisions that actually matter are HARDCODED AROUND the model: the engine branch is a fixed
four-way `if` (`broker/stdio.rs:278-292`), round counts are constants (`MAX_ROUNDS=10`,
`GOAL_ROUNDS=5`, hop cap 6), judges are elected by keyword-matching an agent's role string
(`is_critic`, `constructs.rs:173`), and the swarm graph is FROZEN AT PLAN TIME — no re-planning, no
model in the loop after the first 2048 tokens. The skills surface is the tell: it ships zero skills
and both skills dirs are empty on the machine that runs this daily. Playbooks written for a weaker
model are not what is missing.

THE GOAL, in one line: smith stops being a command set and becomes an agent — you say what you
want, the MODEL decides fan-vs-loop-vs-plan-vs-single-shot and when it is done, and the broker's
job shrinks to two things it alone can do: GUARD RESOURCES (tokens, hops, cost, checkpoints) and
ASSEMBLE THE BEST POSSIBLE CONTEXT for every model call.

### Pillar 1 — the command diet
Constructs shrink from 20 to the handful that touch state the model cannot or must not decide:
`/help`, `/model`, `/doctor`, `/stop`, `/restore`, `/reload` — session machinery, model pinning,
checkpoint rollback, cancellation. Everything that is a prompt-in-a-trenchcoat is RETIRED AS A
COMMAND and becomes an intent the model recognizes from plain language, routed to the same
underlying capability. `/skill` goes with them: the drop-in `*.md` surface STAYS (it is user
extensibility, and it is free), but if skill files exist the model discovers and applies them
itself — no command needed to invoke a playbook. Retirement is not deletion of capability:
`/plan`'s human gate survives as a conversational approval ("here's the plan — go?"), because the
gate was the feature, the slash was not.

### Pillar 2 — context is the product
Today's context management is blind fixed-window clipping: last 8 transcript entries at 400 chars
each (`hop.rs:72`, `engine.rs:220`), task clipped at 4000, memory at 2048 — numbers chosen once,
applied regardless of what the tokens are worth. And crew-hive has NONE: `build_prompt`
(`apiagent/mod.rs:38-46`) concatenates every dependency's full output unclipped, which is a latent
blowup on any real graph. Context assembly becomes a first-class, BUDGET-AWARE step: every model
call gets a context built to fit its budget by choosing what to include, not by truncating at a
fixed byte. When history outgrows the budget it is COMPACTED — summarized, the way `/compact`
already became automatic on the app side (`chatcompact.rs`) — never silently dropped. The
cache-aware prompt ordering (`route.rs:98-101`, invariant prefix first) is kept and extended;
that instinct was right.

### Pillar 3 — the model decides the next step
The hardcoded branch reduces to: slash-command → dispatch table, EVERYTHING ELSE → the model. The
model chooses the execution shape (single reply, fan-out, iterative loop, planned graph), chooses
who judges when judging is needed (no more keyword-matching role strings), and decides WHEN WORK IS
DONE — round counts and hop caps demote from drivers to backstops, safety ceilings that should
never be the thing that ends a healthy run. The swarm graph unfreezes: when a task fails or its
output changes what should happen next, the model gets to RE-PLAN the remainder instead of the
scheduler marching through a stale graph.

### Done means
1. ~~`broker_constructs()` lists at most 8 constructs~~ — **AMENDED 2026-09-01 to nine, with the
   reason.** The diet's subject was USER-FACING VOCABULARY THE USER MUST LEARN to reach a
   behaviour a strong model can infer from plain language, and all of that is retired: `/fan`,
   `/loop`, `/goal`, `/plan`+`/approve`+`/reject`, `/skill`, `/standup`, `/commit`, `/review`,
   `/resume` are gone and reachable by saying what you want. What survives is nine MECHANICAL
   verbs with no model in the path — `help`, `model`, `login`, `logout`, `doctor`, `restore`,
   `reload`, `diff`, `stop` — each instant, free and deterministic. Reaching the ninth (`/diff`)
   through the intent router to hit the number would put an LLM round trip in front of
   `git diff`, and folding `/diff` and `/restore` into one `/changes` noun with sub-verbs would
   trade one word to learn for two. Eight was a proxy for the goal; nine mechanical verbs meet
   the goal. The palette drift tests (`chatcomplete.rs`) pin the list either way.
2. For every retired construct there is a transcript-level test proving plain-language parity: a
   natural phrasing of the same ask reaches the same capability (fan-out, loop, plan+gate, commit
   message, review, standup, resume) with no slash command typed.
3. No keyword-match decides intent or elects a judge anywhere in the broker — grep-level check:
   `is_critic`/`is_writer`/`pick_by_role` and their pattern are gone.
4. No unbounded concatenation into any prompt: crew-hive dep gathering is budget-aware, and every
   context assembly site states its budget explicitly rather than inheriting a magic constant.
5. Transcript history nearing the token budget is summarized, not dropped — verified by a test
   where a long session's early decision remains recoverable in a late prompt.
6. `MAX_ROUNDS`, `GOAL_ROUNDS`, and the hop cap still exist but only as backstops; a test shows a
   model-declared "done" ends a run early, and a runaway run still hits the ceiling.
7. Mid-run re-planning exists: a swarm task failure triggers a model decision about the remaining
   graph, not just a dead branch.

NON-NEGOTIABLE: the security invariant stays — the planner can NEVER select a process-executing
agent (`parse_plan`, the `AgentKind::Api` forcing); token/cost budgets and hop ceilings stay as
hard backstops; destructive actions (commits, writes beyond the checkpoint) keep a human gate even
when the phrasing is conversational; the mock provider path keeps working so tests need no key; and
this lands as SMALL FILES ACROSS ITERATIONS, not one god router — each retirement shippable on its
own, `/help` updated in the same merge that retires a command.
