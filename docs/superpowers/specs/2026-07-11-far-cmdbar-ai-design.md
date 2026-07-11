# Far Command Bar: AI Suggestions (Phase 2) — Design

**Date:** 2026-07-11
**Status:** Draft (implement after Phase 1:
`2026-07-11-far-cmdbar-complete-design.md`)

## Goal

Type `! <what you want>` in the Far command bar and get a shell command
back as an editable suggestion. Zero cost and zero latency unless invoked;
never auto-runs.

## Trigger & UX

- A cmdline starting with `!` (followed by a space or word) is an AI ask;
  Enter submits the description instead of running a shell command.
- While the request runs: status line shows `thinking…` with the elapsed
  seconds; the bar stays editable (typing cancels the in-flight ask).
- On reply: the suggested command REPLACES the bar content, selected-style
  highlighted, with a status hint `Enter run · Esc discard · keep typing to
  edit`. Enter executes it through the normal `run_cmdline` path (history
  records the final command, not the `!` ask). Esc restores the original
  `!` text.
- Errors (no provider, timeout, refusal) land in the status line; the `!`
  text stays so the user can retry.

## Provider access (no broker child)

crew-app already links crew-plugin and crew-hive. The bar reuses:

- `crew_plugin::broker::discover` provider discovery (same env/key rules
  as the broker — DASHSCOPE/OPENROUTER/etc, `CREW_PROVIDER` pin honored).
  Discovery is currently crate-internal; the plan will expose the minimal
  needed surface (a `pub fn` returning a boxed `Provider`) rather than
  opening the whole broker module.
- One `Provider::complete` call with a fixed system prompt: "Reply with
  exactly one POSIX shell command for the user's request. No prose, no
  code fences. The command runs in <cwd> on <os>." — `max_tokens` small
  (128). Response post-processing strips fences/whitespace to one line.

Threading: the call runs on a spawned thread; the result posts back via
the pane's existing poll loop (a `mpsc::Receiver<Result<String, String>>`
on `FarPane`, mirroring how `running` command output already flows).
Timeout 20s. `CREW_BROKER_MOCK_REPLY` short-circuits to the mock provider
exactly like broker discovery does, so tests and the GUI harness work
without keys.

## Testing

- Prompt post-processing: fence/prose stripping to one line (pure fn).
- Trigger parsing: `!` detection, `!` with empty description → status nag.
- Lifecycle with the mock provider: submit → suggestion replaces bar →
  Enter runs via run_cmdline; Esc restores the ask; typing cancels.
- No-provider path: discovery returns none → status message, bar intact.

## Out of scope (YAGNI)

Multi-turn refinement, command explanations, auto-run, streaming the
suggestion, using the chat pane's live broker session.
