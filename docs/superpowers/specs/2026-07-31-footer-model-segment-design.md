# Footer model segment — show the model actually in use

**Date:** 2026-07-31
**Status:** Approved (design), pending implementation
**Scope:** `crates/crew-app/src/chatsummary.rs`, `crates/crew-app/src/chatpalette.rs` (visibility only), tests

## Problem

The agent smith pane's summary footer (line 1) almost never shows which model
is serving the swarm. `roster_seg` in `chatsummary.rs` shows the model only
when *every* roster agent reports the same non-empty model string. The normal
roster mixes API specialists (which report a model, e.g. `qwen3-coder-plus`)
with CLI-backed plugin agents (`@claude`, `@codex` — model `""`), so the
consensus always fails and the footer falls back to agent names or a bare
`N agents` count. The user sees `~/code/crew | main | 0 in / 0 out` and has no
idea what model they are talking to.

The `/model` picker already solved the semantics: `chatpalette::shared_model`
ignores empty models and fails only on genuine disagreement. The footer just
does not use it.

## Decision

GUI-only change; no broker or protocol work. The model information already
rides the existing `Roster` event (`AgentInfo.model`), which is re-emitted on
broker start, `/model`, and `/reload` — so the segment stays live for free.

Placement is footer line 1, not the fieldset legend or header:
`chathdr.rs` already commits to "session stats (model, context, tokens) live
in the below-input summary footer; the header is identity and liveness only",
and the legend is static identity ("agent smith").

## Behavior

`roster_seg(agents)` in `chatsummary.rs` becomes:

1. Compute the shared model via `chatpalette::shared_model(agents)` —
   the first non-empty `AgentInfo.model`, provided every other agent's model
   is empty or equal.
2. If a shared model exists:
   - roster of exactly 1 agent → `short_model(m)` (no silly `· 1 agent`)
   - roster of N > 1 agents → `short_model(m) · N agents`
     (`·` = U+00B7, matching line-3 style; ONE segment, so `budget` drops it
     atomically and it keeps `P_ROSTER = 0`, last to be dropped)
3. If models genuinely disagree, or all are empty (CLI-only roster):
   unchanged today's behavior — joined names up to 3 agents, `N agents` past
   that. Names remain "the honest answer" when no single model is in use.
4. Empty roster → `None` (segment absent), unchanged. An empty roster means
   nothing is serving; showing a model would be a lie.

Rendered examples (line 1):

```
~/code/crew | qwen3-coder-plus · 5 agents | main | $0.129 | 1.2k in / 950 out
~/code/crew | claude-sonnet-5 | main | 0 in / 0 out            (1-agent roster)
~/code/crew | planner·coder·claude | main | 0 in / 0 out       (mixed models)
```

## Implementation notes

- Reuse, don't duplicate: `roster_seg` calls `chatpalette::shared_model`
  directly (already `pub(crate)`); delete the consensus loop `roster_seg`
  carries today. `short_model` (last-path-segment trim) stays where it is.
- Update `roster_seg`'s doc comment: it currently argues names-over-models for
  mixed rosters; the argument changes to "model when one model is the answer,
  names when it is not".
- No width-priority changes, no new colors (segment stays cyan), no changes to
  lines 2/3, header, legend, broker, or protocol.

## Testing

New/updated cases in `chatsummary_tests.rs`, written test-first with a RED
transcript before the fix (standing rule: plan-authored tests must be shown
failing, with numbers not verdicts):

1. Mixed roster (2 specialists sharing `a/m1` + 1 CLI agent with `""`) →
   segment is `m1 · 3 agents` (today: names — this is the RED case).
2. Single-agent roster (`claude-sonnet-5`) → segment is exactly the short
   slug, no count.
3. Disagreeing models (`m1` vs `m2`) → names, unchanged.
4. All-empty models → names, unchanged; > 3 such agents → `N agents`.
5. Empty roster → segment absent.

Existing tests asserting the old all-agents consensus (if any assert names for
a mixed roster) are updated to the new expectation — that expectation change
is the point of the feature.

## Out of scope

- Per-reply "served model" truth when a provider's fallback chain rolls over
  mid-request (would need `Stats`-level plumbing from `crew-hive`).
- Provider name, price, or context-window badges in the footer.
- Showing a model before the first `Roster` event arrives (`Ready.provider`
  exists but carries no model; the gap is ~seconds and honest).
