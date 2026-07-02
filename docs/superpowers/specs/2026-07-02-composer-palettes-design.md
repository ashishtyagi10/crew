# Crew composer pop-ups: slash palette + leading-@agent palette

2026-07-02. Approved direction ("Yes, both").

## Problem

The crew chat-pane composer has a `@file` mention popup (mid-line `@token`,
`chatmention.rs`) but nothing for the two leading-token cases the user hit:
typing `/` shows no command palette, and typing a leading `@` shows no agent
picker — the leading `@agent` selector and `/construct` only Tab-complete
silently (`chatcomplete.rs`). (Note: the *main app input bar* already has a
slash card via `suggest::menu_items`+`cmdmenu`; the *chat pane composer* does
not — this closes that gap.)

## Design

A new leading-token palette mirroring the proven `chatmention` popup pattern,
kept separate from the mid-line file mention so the two never fight:

- **`chatpalette.rs`** (new): pure string logic + popup state.
  - `pending_palette(input) -> Option<(Kind, &str)>` — the LEADING token only
    (nothing before it, cursor still inside it, i.e. no whitespace yet): `/xyz`
    → `(Slash, "xyz")`, `@ab` → `(Agent, "ab")`, else `None`. A leading `@`
    with a `+` (multi-target `@a+b`) queries on the segment after the last `+`,
    matching `chatcomplete`.
  - `enum Kind { Slash, Agent }`.
  - `struct PaletteState { kind: Kind, items: Vec<MenuItem>, sel: usize }` where
    `items` are the already-filtered rows (label + dim desc).
  - `popup_key(&mut Option<PaletteState>, &mut String, &ChatInput) -> PaletteKey`
    — Up/Down move, Tab/Enter accept, Esc closes the popup only; same shape as
    `chatmention::popup_key`.
  - `after_edit(&mut Option<PaletteState>, input, agents: &[AgentInfo])` — opens
    on a leading `/`/`@` token, refilters as it narrows, closes when the token
    ends (first space) or nothing matches. Candidate sources:
    - Slash: `chatcomplete::CONSTRUCTS` paired with one-line descriptions (new
      `describe(construct) -> &str` table in `chatcomplete`, sourced from the
      broker `/help` text), prefix-filtered on the query.
    - Agent: the pane roster (`ChatPane.agents`, `AgentInfo.name`/`role`),
      prefix-filtered.
  - `accept(input, item)` — replace the leading token with the chosen value:
    Slash → `/model ` (construct + trailing space); Agent → the `@name `
    selector, preserving any `@a+` prefix before the edited segment.

- **`chat.rs on_key`**: route `chatpalette::popup_key` FIRST (before the
  `Up/Down`/`Complete`/`Close` early-returns that would otherwise swallow the
  keys), then the existing `chatmention::popup_key`, then normal handling. On a
  Char/Backspace edit, call `chatpalette::after_edit` alongside the existing
  `chatmention::after_edit`. Exactly one popup is ever open: the leading-token
  palette and the mid-line mention are mutually exclusive by construction
  (`pending_palette` needs the leading token; `pending_mention` needs a
  non-leading one).

- **`render.rs`**: when `ChatPane.palette` is Some, render its rows as a
  `cmdmenu::menu_card` titled `"commands"` (Slash) or `"agents"` (Agent),
  anchored above the composer exactly like the existing `"files"` mention card
  (same `menu_rows`/placement math).

## Esc handling

The palette's `popup_key` consumes `Close` to dismiss the popup and returns
`Consumed`, so `on_key` returns before its own `ChatInput::Close =>
ChatAction::Close` — Esc closes the popup, never the pane, while a palette is
open (the same guarantee the file mention already has). Covered by a test.

## Out of scope

- No broker/protocol change (candidates are all app-side).
- No fuzzy matching for the palette (prefix filter matches `chatcomplete`'s Tab
  semantics; the file mention keeps its fuzzy rank).
- The main app input bar's existing slash card is untouched.

## Testing

`chatpalette.rs` unit tests: `pending_palette` (leading `/`, leading `@`,
`@a+b` segment, non-leading `@` → None, plain text → None, ended token → None);
`after_edit` open/refilter/close for both kinds; `popup_key` navigate/accept/
close incl. Esc-closes-popup-not-pane; `accept` for slash, agent, and
`@a+b` multi-target. A `chat_tests.rs` case: typing `/` then Esc leaves the
pane open (no `ChatAction::Close`).
