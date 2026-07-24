# /smith `@` attach popup — design

**Date:** 2026-07-24
**Goal:** Typing `@` in the smith composer opens a picker popup — like other
coding CLIs — from which the user can attach **agents**, **skills**, or
**files** from the current directory.

## Today

- Leading `@` (first token) opens the agents palette (`chatpalette`,
  routing selector) — agents only.
- Mid-line `@` opens the file-mention popup (`chatmention` + `fileindex`) —
  files only.
- Skills exist only broker-side (`~/.config/crew/skills/` +
  `./.crew/skills/`, project overrides user; `crew-plugin`'s
  `broker/skills.rs`); the pane never sees them.
- `chatmention::expand` skips token 0 unconditionally, so a leading `@path`
  is never attached.

## Decision

One conceptual picker, surfaced in both positions, over three sources:

| Source | Rows | Accept inserts | On send |
|---|---|---|---|
| Agents | roster (`AgentInfo`) | `@name ` | routing (leading) / plain-text hint (mid-line) |
| Skills | skill dirs via new `crew-plugin` API | `@skill:name ` | playbook body appended as `--- skill: name ---` block |
| Files | `fileindex::scan` (cwd) | `@path ` | contents appended (existing `--- file ---` block) |

### 1. Skills listing API (`crew-plugin`)

`broker/skills.rs` gains `pub fn list(project_root: &Path) -> Vec<Skill>`:
the existing user-dir + `<project_root>/.crew/skills` load + merge, rooted
explicitly instead of relying on process cwd. `Skill` and `list` are
re-exported from `crew_plugin` (as `skills::Skill`, `skills::list`). The
broker's internal `load()` delegates to `list(".")`. No behavior change
broker-side.

### 2. Mid-line mention popup (`chatmention`)

`MentionState.matches` becomes `Vec<MentionEntry>`:

```rust
pub(crate) enum MentionEntry {
    Agent { name: String, role: String },
    Skill { name: String, desc: String },
    File(String),
}
```

Sources struct (`MentionSources { agents, skills, files }`) is scanned once
when the popup opens (files via the existing bounded scan; skills via
`crew_plugin::skills::list(cwd)`; agents from the pane's roster). Filtering
reuses the existing `rank` (prefix > substring > subsequence) per entry
label; section order **Agents, Skills, Files**, each internally ranked,
total still capped at `MAX_MATCHES`. Accept splices per the table above.
The popup card title changes from "files" to "attach"; rows show a dim
kind marker in `desc` ("agent · role", "skill · description", file rows
keep an empty desc).

### 3. Leading `@` palette (`chatpalette`)

`agent_items` becomes sectioned the same way: Agents (existing rows),
then Skills, then Files (both filtered by the same query; files capped so
the card stays shallow — reuse `MAX_MATCHES` before `menu_rows` clamps).
Accept fills `@skill:name` / `@path` through the existing leading-token
`accept`. Card title for `Kind::Agent` changes from "agents" to "attach".

### 4. Send-time expansion (`chatmention::expand`)

- New signature: `expand(text, cwd, agents: &[AgentInfo], skills)` — the
  chat pane passes its roster and a lazily-loaded skills list.
- Token 0 is skipped ONLY when it names a rostered agent (exact name
  match, or a `+`-joined multi-target whose every segment is rostered) —
  otherwise it expands like any token. This makes leading `@file`/`@skill:`
  attachments work, and is the safety condition for §3.
- `@skill:name` tokens resolve against the skills list; matches append
  `\n\n--- skill: name ---\n<body>\n--- end skill ---`, deduped, size-capped
  by the existing `MAX_FILE_BYTES` rule (oversize → skip note). Unresolved
  `@skill:` tokens are left alone like unresolvable paths.
- File tokens behave exactly as before.

### Non-goals

- No new protocol events (skills are read pane-side from disk; hot-reload
  freshness comes from re-scanning each time the popup opens).
- No folder attachments, no MCP resources, no multi-select.
- The `!`-ask, run-pane, and /far surfaces are untouched.

## Error handling

Missing/unreadable skill dirs → empty section (never an error). Binary or
oversize bodies → skip notes (existing rule). Empty roster (mock provider)
→ popup simply lacks the Agents section.

## Testing

Pure-logic tests beside each module, following the existing patterns:
1. `crew-plugin`: `list(root)` finds flat + directory skills under
   `<root>/.crew/skills`, project overrides user (existing merge tests
   already cover precedence; new test covers explicit rooting).
2. `chatmention`: sectioned filter ordering (agents before skills before
   files; rank preserved within a section); accept splices `@name` /
   `@skill:name` / `@path`; popup key-flow unchanged.
3. `expand`: leading rostered agent still skipped; leading non-agent path
   now attached; `@skill:name` appends the playbook block, dedupes, and
   leaves unknown skills alone; multi-target `@a+b` leading token skipped
   when all segments rostered.
4. `chatpalette`: leading `@` lists agents + skills + files; slash palette
   unaffected.
5. render: mention card title "attach"; agent/skill rows carry their dim
   kind marker through to `MenuItem.desc`.
