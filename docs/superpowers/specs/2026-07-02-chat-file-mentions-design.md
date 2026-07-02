# Chat file mentions (`@path`) — design

Date: 2026-07-02
Status: approved (autonomous session; decisions taken with recommended defaults, revisit freely)

## Goal

Let the user reference a file in the crew chat composer by typing `@`, picking the
file from a fuzzy popup, and have the file's contents delivered to the agents when
the message is sent. Today the composer only understands a *leading* `@agent`
selector; agents have no way to see local files.

## Non-goals

- No broker/protocol changes: `PluginCommand::Send { channel, text }` stays as is.
- No per-pane cwd (the app-global cwd is the base).
- No images/binary attachment, no live file watching or re-indexing.
- The leading `@agent` selector and its Tab-completion are untouched.

## UX

- Typing `@` at the start of any token *after* the first word position opens a
  file-suggestion popup anchored to the chat pane (message-leading `@` remains the
  agent selector, as parsed broker-side).
- The popup is a fieldset card mirroring `cmdmenu::menu_card`: max 10 rows,
  fuzzy-filtered as the user keeps typing (`@src/mn` → `src/main.rs`), highlighted
  selection.
- Keys while the popup is open: `Up`/`Down` move, `Tab`/`Enter` accept (inserting
  `@<relative-path>` plus a trailing space), `Esc` closes the popup only. All other
  keys behave as normal composer input and refilter.
- Accepted mentions render as colored chips like agent mentions (extend
  `mention_len` to recognize resolvable paths anywhere in the line).

## Candidate index

- Built with `walkdir` (already a workspace dependency; `ignore` is not in the
  tree and gitignore-awareness is YAGNI for v1): skip hidden entries and
  `target`/`node_modules`/`.git`, rooted at the app cwd.
- Bounded: depth ≤ 8, ≤ 2,000 files collected; truncation is fine (fuzzy filter
  still works over what was collected). The walk runs when the popup opens and is
  cached per (cwd, generation); this keeps the winit thread stall bounded to a few
  milliseconds even on big repos.
- Ranking reuses the subsequence matcher in `suggest.rs` (`is_subsequence`/`rank`
  style) against the relative path.

## Send-time expansion

- On Enter, before `chatexport::intercept` / `plugin.send` in `chat.rs`, scan the
  message body (everything after the optional leading `@agent`/`@a+b` selector) for
  `@path` tokens.
- Each token that resolves (relative to cwd) to an existing regular file is kept
  in place, and a block is appended to the outgoing text:

  ```
  --- file: src/main.rs ---
  <contents>
  --- end file ---
  ```

- Caps: skip files > 64 KiB or non-UTF-8, appending a one-line note instead
  (`--- file: X skipped: too large/binary ---`). Unresolvable tokens are left
  alone (they may be genuine prose like an email handle).
- The broker renders hops as transcript messages and never echoes the user's
  outgoing text, so the expansion is not visible in the pane by construction.

## Architecture / seams

- `crates/crew-app/src/chatmention.rs` (new): pure helpers — trailing-`@token`
  detection, fuzzy filtering over the index, token→path resolution, message
  expansion. Unit-tested.
- `crates/crew-app/src/fileindex.rs` (new): bounded gitignore-aware walk + cache.
- `ChatPane` gains popup state (`mention: Option<MentionState>`) and a `cwd`
  synced from the app (set at pane creation and on `set_cwd`).
- `chatkeys.rs` gains `Up`/`Down` classification; `chat.rs::on_key` routes keys to
  the popup first when it is open.
- Rendering mirrors `cmdmenu::menu_card` above the chat input row.

## Error handling

- Walk errors (permissions etc.) are silently skipped by the walker.
- Read failures at send time degrade to the skip-note, never block sending.

## Testing

- Pure unit tests for: token detection (leading selector excluded, mid-message
  included), fuzzy filter ordering, expansion (happy path, oversize, binary,
  unresolvable token untouched), cache invalidation on cwd change.
- Reducer-level tests for popup open/close/accept key flows.
