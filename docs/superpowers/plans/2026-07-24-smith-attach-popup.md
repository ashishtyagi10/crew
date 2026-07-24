# /smith `@` Attach Popup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Typing `@` in the smith composer opens a sectioned picker (agents, skills, files from the cwd) in both the leading and mid-line positions, and send-time expansion attaches skill playbooks and leading-position files.

**Architecture:** `crew-plugin` exposes a rooted skills-listing API. `chatmention` grows a typed `MentionEntry` (agent/skill/file) with section-ordered filtering; `chatpalette`'s leading-`@` picker gains skill and file rows; `chatmention::expand` becomes roster-aware (token 0 skipped only for rostered agents) and expands `@skill:name` tokens by appending playbook blocks.

**Tech Stack:** Rust workspace; crates `crew-plugin` (broker + protocol) and `crew-app` (GUI); plain `cargo test`.

**Spec:** `docs/superpowers/specs/2026-07-24-smith-attach-popup-design.md`

## Global Constraints

- Workspace root `/Users/atyagi/code/crew`; paths below relative to it.
- Skill token format is exactly `@skill:<name>` (names are already normalized lowercase-hyphen by `crew-plugin`'s `normalize_name`).
- Section order everywhere: **Agents, Skills, Files**; total rows capped by the existing `MAX_MATCHES` (50).
- Attachment blocks: `\n\n--- skill: <name> ---\n<body>\n--- end skill ---`; oversize (> `MAX_FILE_BYTES` = 64 KiB) → `\n\n--- skill: <name> skipped: too large ---`.
- Popup card titles: mention card `"attach"` (was `"files"`); leading `@` palette card `"attach"` (was `"agents"`); slash palette stays `"commands"`.
- Pre-commit runs `cargo fmt --check` + `cargo check`; run `cargo fmt` before every commit.
- Tests: `cargo test -p crew-plugin skills` (Task 1), `cargo test -p crew-app chatmention` / `chatpalette` / `chat_tests` as named per task.

---

### Task 1: `crew_plugin::skills` public listing API

**Files:**
- Modify: `crates/crew-plugin/src/broker/skills.rs`
- Modify: `crates/crew-plugin/src/broker/mod.rs` (re-export)
- Modify: `crates/crew-plugin/src/lib.rs` (re-export)
- Test: tests module inside `skills.rs` (follow its existing test style if one exists; otherwise add `#[cfg(test)] mod tests` at the bottom)

**Interfaces:**
- Produces: `crew_plugin::skills_list(project_root: &Path) -> Vec<crew_plugin::Skill>` — user-dir skills (`~/.config/crew/skills`) merged with `<project_root>/.crew/skills` (project wins on name clash), sorted by name. `Skill` is the existing struct (fields `name`, `description`, `body`, `origin`, `path`, `dir`) made `pub`.
- Consumes: existing `load_dir` / `merge` helpers (unchanged).

- [ ] **Step 1: Write the failing test**

In `skills.rs`'s tests module add:

```rust
#[test]
fn list_is_rooted_at_the_given_project_dir_not_the_cwd() {
    let root = std::env::temp_dir().join(format!("crew-skills-list-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join(".crew/skills");
    std::fs::create_dir_all(dir.join("deploy")).unwrap();
    std::fs::write(dir.join("review.md"), "---\ndescription: review playbook\n---\nsteps").unwrap();
    std::fs::write(dir.join("deploy").join("SKILL.md"), "deploy steps").unwrap();
    let got = list(&root);
    let names: Vec<&str> = got.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"review"), "flat skill missing: {names:?}");
    assert!(names.contains(&"deploy"), "dir skill missing: {names:?}");
    let review = got.iter().find(|s| s.name == "review").unwrap();
    assert_eq!(review.description, "review playbook");
    assert_eq!(review.origin, "project");
    let _ = std::fs::remove_dir_all(&root);
}
```

(User-dir skills may leak into `got` on a dev machine — assert with `contains`, never exact equality.)

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-plugin list_is_rooted`
Expected: FAIL to compile — `list` not found.

- [ ] **Step 3: Implement**

In `skills.rs`: change `pub(crate) struct Skill` to `pub struct Skill` (doc comment stays). Replace `load()` with:

```rust
/// Load every skill visible from the broker's cwd.
pub(crate) fn load() -> Vec<Skill> {
    list(Path::new("."))
}

/// User + project skills, the project dir rooted explicitly at
/// `project_root` — the GUI's attach picker lists skills for a pane whose
/// cwd is not the process cwd.
pub fn list(project_root: &Path) -> Vec<Skill> {
    let user = dirs::config_dir()
        .map(|d| load_dir(&d.join("crew").join("skills"), "user"))
        .unwrap_or_default();
    let project = load_dir(&project_root.join(".crew/skills"), "project");
    merge(user, project)
}
```

In `broker/mod.rs`, alongside the existing `pub use agents::known_adapters;` add:

```rust
pub use skills::{list as skills_list, Skill};
```

(If the `mod skills;` declaration is not already visible enough for this re-export, leave the mod declaration as-is — `pub use` through a private module path is fine within the crate, and the re-export chain makes the items public.)

In `lib.rs`, extend the existing `pub use broker::{...}` list with `skills_list, Skill`.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-plugin skills`
Expected: ALL PASS (new test + existing skills tests).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-plugin/src/broker/skills.rs crates/crew-plugin/src/broker/mod.rs crates/crew-plugin/src/lib.rs
git commit -m "feat(plugin): public rooted skills listing for the GUI attach picker"
```

---

### Task 2: Sectioned mid-line mention popup

**Files:**
- Modify: `crates/crew-app/src/chatmention.rs`
- Modify: `crates/crew-app/src/chat.rs` (scan call site, ~line 424)
- Modify: `crates/crew-app/src/render.rs` (mention card mapping, lines 146–188)
- Test: `chatmention.rs` tests module

**Interfaces:**
- Consumes: `crew_plugin::skills_list` (Task 1), `crate::fileindex::scan`, `ChatPane.agents: Vec<AgentInfo>`.
- Produces: `pub(crate) enum MentionEntry { Agent { name: String, role: String }, Skill { name: String, desc: String }, File(String) }` with methods `label(&self) -> &str` (name / name / path), `token(&self) -> String` (`name` / `skill:name` / `path`), `desc(&self) -> String` (`"agent · {role}"` / `"skill · {desc}"` / `""`); `MentionState { entries: Vec<MentionEntry>, matches: Vec<MentionEntry>, sel: usize }`; `pub(crate) fn scan_entries(cwd: &Path, agents: &[crew_plugin::AgentInfo]) -> Vec<MentionEntry>`; `filter(entries: &[MentionEntry], query: &str) -> Vec<MentionEntry>`; `accept(input, token)` unchanged signature (takes the token string). Task 4 relies on `MentionEntry` and `scan_entries` being reusable; render relies on `label()`/`desc()`.

- [ ] **Step 1: Write the failing tests**

Rewrite the affected tests in `chatmention.rs` (keep every test not named here):

```rust
fn entries(paths: &[&str]) -> Vec<MentionEntry> {
    paths.iter().map(|p| MentionEntry::File(p.to_string())).collect()
}

#[test]
fn filter_sections_agents_then_skills_then_files() {
    let mut e = entries(&["review-checklist.md"]);
    e.push(MentionEntry::Agent { name: "reviewer".into(), role: "reviews".into() });
    e.push(MentionEntry::Skill { name: "review".into(), desc: "playbook".into() });
    let got = filter(&e, "rev");
    let labels: Vec<&str> = got.iter().map(|m| m.label()).collect();
    assert_eq!(labels, vec!["reviewer", "review", "review-checklist.md"]);
}

#[test]
fn tokens_by_kind() {
    assert_eq!(MentionEntry::Agent { name: "coder".into(), role: String::new() }.token(), "coder");
    assert_eq!(MentionEntry::Skill { name: "deploy".into(), desc: String::new() }.token(), "skill:deploy");
    assert_eq!(MentionEntry::File("src/main.rs".into()).token(), "src/main.rs");
}
```

Adapt the existing `filter_ranks_name_prefix_over_substring_over_subsequence`, `filter_empty_query_lists_everything_and_misses_are_dropped`, `popup_navigates_accepts_and_closes`, `after_edit_opens_refilters_and_closes`, and the `open(...)` helper to the `MentionEntry` types: build `MentionEntry::File`s via `entries(...)`, compare via `label()`, and popup-accept assertions expect `see @b.rs ` exactly as before (File token = path). `accept` keeps its `(input: &str, token: &str) -> String` shape, so `accept_replaces_the_trailing_token` needs no change.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app chatmention`
Expected: compile FAIL — `MentionEntry` undefined.

- [ ] **Step 3: Implement**

In `chatmention.rs`:

```rust
/// One row of the attach picker: a rostered agent, a skill playbook, or a
/// file from the pane's cwd index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MentionEntry {
    Agent { name: String, role: String },
    Skill { name: String, desc: String },
    File(String),
}

impl MentionEntry {
    /// The text the query filters on (and the row label's body).
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Agent { name, .. } | Self::Skill { name, .. } => name,
            Self::File(p) => p,
        }
    }
    /// What `accept` splices after the `@`.
    pub(crate) fn token(&self) -> String {
        match self {
            Self::Agent { name, .. } => name.clone(),
            Self::Skill { name, .. } => format!("skill:{name}"),
            Self::File(p) => p.clone(),
        }
    }
    /// Dim hint after the label — the row's kind (files stay unadorned,
    /// matching the old files-only popup).
    pub(crate) fn desc(&self) -> String {
        match self {
            Self::Agent { role, .. } => format!("agent \u{b7} {role}"),
            Self::Skill { desc, .. } => format!("skill \u{b7} {desc}"),
            Self::File(_) => String::new(),
        }
    }
    /// Section rank: agents, then skills, then files.
    fn section(&self) -> u8 {
        match self {
            Self::Agent { .. } => 0,
            Self::Skill { .. } => 1,
            Self::File(_) => 2,
        }
    }
}

/// Everything the picker offers, scanned once when the popup opens:
/// roster agents, skills (user + project via crew-plugin), cwd files.
pub(crate) fn scan_entries(
    cwd: &std::path::Path,
    agents: &[crew_plugin::AgentInfo],
) -> Vec<MentionEntry> {
    let mut out: Vec<MentionEntry> = agents
        .iter()
        .map(|a| MentionEntry::Agent { name: a.name.clone(), role: a.role.clone() })
        .collect();
    out.extend(crew_plugin::skills_list(cwd).into_iter().map(|s| {
        MentionEntry::Skill { name: s.name, desc: s.description }
    }));
    out.extend(crate::fileindex::scan(cwd).into_iter().map(MentionEntry::File));
    out
}
```

`filter` becomes entry-typed — same rank logic on `label()`, section-major order:

```rust
pub(crate) fn filter(entries: &[MentionEntry], query: &str) -> Vec<MentionEntry> {
    let q = query.to_lowercase();
    let mut scored: Vec<(u8, u8, &MentionEntry)> = entries
        .iter()
        .filter_map(|e| rank(e.label(), &q).map(|r| (e.section(), r, e)))
        .collect();
    scored.sort_by(|(sa, ra, ea), (sb, rb, eb)| {
        (sa, ra, ea.label().len(), ea.label()).cmp(&(sb, rb, eb.label().len(), eb.label()))
    });
    scored.truncate(MAX_MATCHES);
    scored.into_iter().map(|(_, _, e)| e.clone()).collect()
}
```

`MentionState` fields rename `files: Vec<String>` → `entries: Vec<MentionEntry>`, `matches: Vec<MentionEntry>`; `popup_key`'s accept arm uses `m.matches.get(m.sel)` → `accept(input, &entry.token())`; `after_edit`'s `scan` closure returns `Vec<MentionEntry>`. `accept`, `pending_mention`, `spans`, `expand` are untouched in this task.

In `chat.rs` (~line 424), the roster must reach the scan closure without borrowing `self` inside it — clone it first:

```rust
            // A Char/Backspace edit: sync the mention popup to the new input.
            let agents = self.agents.clone();
            crate::chatmention::after_edit(&mut self.mention, &self.input, || {
                crate::chatmention::scan_entries(cwd, &agents)
            });
```

In `render.rs` lines 152–175, map entries through the new accessors and retitle the card:

```rust
                    if let Some(m) = &c.mention {
                        if !m.matches.is_empty() {
                            let items: Vec<crate::suggest::MenuItem> = m
                                .matches
                                .iter()
                                .map(|e| crate::suggest::MenuItem {
                                    label: format!("@{}", e.token()),
                                    desc: e.desc(),
                                    fill: String::new(),
                                    submit: false,
                                })
                                .collect();
```

and `menu_card("files", ...)` → `menu_card("attach", ...)`.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-app chatmention && cargo test -p crew-app chat`
Expected: ALL PASS (chat_tests exercise on_input paths over the new closure).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chatmention.rs crates/crew-app/src/chat.rs crates/crew-app/src/render.rs
git commit -m "feat(smith): mid-line @ popup offers agents, skills, and files"
```

---

### Task 3: Roster-aware expansion + `@skill:name` attachments

**Files:**
- Modify: `crates/crew-app/src/chatmention.rs` (`expand`, `attachment` sibling)
- Modify: `crates/crew-app/src/chat.rs` (expand call site, ~line 412; also the `/stop` comment at ~283 stays valid — no change there)
- Test: `chatmention.rs` tests module

**Interfaces:**
- Produces: `pub(crate) fn expand(text: &str, cwd: &Path, agent_names: &[String]) -> String`. Token 0 is skipped only when every `+`-separated segment of its `@`-stripped form equals a name in `agent_names`; `@skill:<name>` tokens (any position that is a mention) append the skill block; skills are loaded lazily via `crew_plugin::skills_list(cwd)` only when a `@skill:` token exists.
- Consumes: Task 1's `skills_list`; Task 2's module state (no structural overlap — this task touches only `expand` and its tests).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn expand_attaches_a_leading_non_agent_mention() {
    let dir = tmp("leadfile");
    std::fs::write(dir.join("a.txt"), "A").unwrap();
    // roster contains planner only: leading @a.txt is a mention, not routing.
    let out = expand("@a.txt summarize", &dir, &["planner".to_string()]);
    assert!(out.contains("--- file: a.txt ---"), "{out}");
    // rostered leading selector still skipped, including multi-target
    let out = expand("@planner do it @a.txt", &dir, &["planner".to_string()]);
    assert!(out.starts_with("@planner do it @a.txt"));
    assert_eq!(out.matches("--- file: a.txt ---").count(), 1);
    let out = expand("@planner+coder go", &dir, &["planner".to_string(), "coder".to_string()]);
    assert_eq!(out, "@planner+coder go");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn expand_attaches_skill_playbooks_and_leaves_unknown_skills_alone() {
    let dir = tmp("skilltok");
    let sk = dir.join(".crew/skills");
    std::fs::create_dir_all(&sk).unwrap();
    std::fs::write(sk.join("deploy.md"), "---\ndescription: d\n---\nship it").unwrap();
    let out = expand("use @skill:deploy now", &dir, &[]);
    assert!(out.contains("--- skill: deploy ---\nship it\n--- end skill ---"), "{out}");
    // dedup + unknown left alone
    let out = expand("x @skill:deploy @skill:deploy @skill:ghost", &dir, &[]);
    assert_eq!(out.matches("--- skill: deploy ---").count(), 1);
    assert!(!out.contains("--- skill: ghost"));
    let _ = std::fs::remove_dir_all(&dir);
}
```

Update the existing expand tests' call sites: `expand(text, &dir)` → `expand(text, &dir, &[])`, EXCEPT `expand_ignores_the_leading_selector_and_dedups`, which must now pass a roster that contains the leading token's name to keep asserting the skip (`expand("@a.txt do it", &dir, &["a.txt".to_string()])`) — its second assertion (mid-line dedup) keeps `&[]`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app chatmention::tests::expand`
Expected: compile FAIL — wrong arity.

- [ ] **Step 3: Implement**

Replace `expand` (keep `attachment` as-is, add `skill_attachment`):

```rust
/// Expand mentions in an outgoing message: every mention token gets its
/// referent appended — file contents as a `--- file ---` block, `@skill:`
/// playbooks as a `--- skill ---` block. Token 0 is the routing selector
/// only while it names rostered agents (every `+` segment); otherwise it
/// expands like any other token, so attachments picked at the leading
/// position aren't silently dropped. Never blocks sending.
pub(crate) fn expand(text: &str, cwd: &std::path::Path, agent_names: &[String]) -> String {
    let mut out = text.to_string();
    let mut seen: Vec<&str> = Vec::new();
    let mut skills: Option<Vec<crew_plugin::Skill>> = None;
    for (i, tok) in text.split_whitespace().enumerate() {
        let Some(rel) = tok.strip_prefix('@') else {
            continue;
        };
        if i == 0 && !rel.is_empty() && rel.split('+').all(|s| agent_names.iter().any(|a| a == s)) {
            continue; // the @agent routing selector
        }
        if rel.is_empty() || seen.contains(&rel) {
            continue;
        }
        if let Some(name) = rel.strip_prefix("skill:") {
            let list = skills.get_or_insert_with(|| crew_plugin::skills_list(cwd));
            if let Some(s) = list.iter().find(|s| s.name == name) {
                seen.push(rel);
                out.push_str(&skill_attachment(s));
            }
            continue;
        }
        let path = cwd.join(rel);
        if !path.is_file() {
            continue;
        }
        seen.push(rel);
        out.push_str(&attachment(rel, &path));
    }
    out
}

/// One skill mention's appended block: the playbook body, or a skip note.
fn skill_attachment(s: &crew_plugin::Skill) -> String {
    if s.body.len() > MAX_FILE_BYTES {
        return format!("\n\n--- skill: {} skipped: too large ---", s.name);
    }
    format!("\n\n--- skill: {} ---\n{}\n--- end skill ---", s.name, s.body)
}
```

In `chat.rs` (~line 412):

```rust
                let agent_names: Vec<String> =
                    self.agents.iter().map(|a| a.name.clone()).collect();
                let expanded = crate::chatmention::expand(&text, cwd, &agent_names);
```

Check for other `expand(` callers (`grep -rn "chatmention::expand" crates/crew-app/src`) — `askroute.rs` showed a mention hit earlier; update any caller to pass its roster or `&[]` when it has none, preserving its current behavior (if a caller relied on token-0 always being skipped, pass a roster that keeps that true and note it in your report).

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-app chatmention && cargo test -p crew-app chat && cargo test -p crew-app askroute`
Expected: ALL PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chatmention.rs crates/crew-app/src/chat.rs $(git diff --name-only | grep askroute || true)
git commit -m "feat(smith): expansion attaches skills and leading non-agent mentions"
```

---

### Task 4: Leading `@` palette lists skills and files

**Files:**
- Modify: `crates/crew-app/src/chatpalette.rs`
- Modify: `crates/crew-app/src/chat.rs` (palette after_edit call, ~line 427)
- Modify: `crates/crew-app/src/render.rs` (`palette_card_title`, ~line 269)
- Test: `chatpalette.rs` tests module

**Interfaces:**
- Consumes: Task 2's `MentionEntry` + `scan_entries` + `filter` (reused for the skill/file rows so ranking is identical in both popups).
- Produces: `after_edit(palette, input, scan: impl FnOnce() -> Vec<MentionEntry>)` — the closure replaces the `agents: &[AgentInfo]` parameter and supplies ALL sources (Task 2's `scan_entries` output, which already includes agents). `PaletteState` gains `entries: Vec<MentionEntry>` (scanned once per open, like `MentionState`). `Kind::Agent` items are built from the entries: agent rows keep their current label/desc/fill shape; skill rows fill `skill:<name>`; file rows fill the path. When the leading token contains `+`, only Agent entries are offered (multi-target routing).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn leading_at_offers_agents_skills_and_files_in_order() {
    let mut p = None;
    let mut entries: Vec<crate::chatmention::MentionEntry> = vec![
        crate::chatmention::MentionEntry::Agent { name: "reviewer".into(), role: "r".into() },
        crate::chatmention::MentionEntry::Skill { name: "review".into(), desc: "d".into() },
        crate::chatmention::MentionEntry::File("review.md".into()),
    ];
    after_edit(&mut p, "@rev", || entries.clone());
    let items = &p.as_ref().unwrap().items;
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["@reviewer", "@skill:review", "@review.md"]);
    assert_eq!(items[1].fill, "skill:review");
    assert_eq!(items[2].fill, "review.md");
    // narrowing refilters without rescanning
    entries.clear();
    after_edit(&mut p, "@revi", || unreachable!("no rescan while open"));
    assert!(p.is_some());
}

#[test]
fn multi_target_plus_offers_agents_only() {
    let mut p = None;
    let entries = vec![
        crate::chatmention::MentionEntry::Agent { name: "coder".into(), role: "c".into() },
        crate::chatmention::MentionEntry::File("coder.md".into()),
    ];
    after_edit(&mut p, "@planner+co", || entries.clone());
    let labels: Vec<&str> = p.as_ref().unwrap().items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["@coder"]);
}
```

Existing chatpalette tests calling `after_edit(&mut p, input, &agents())` change to pass a closure building `MentionEntry::Agent`s from the same fixture: `|| agents().iter().map(|a| crate::chatmention::MentionEntry::Agent { name: a.name.clone(), role: a.role.clone() }).collect()`. Slash-palette tests (`Kind::Slash`) keep passing a closure too (it must not be called for slash input — use `|| unreachable!()` there only if the current implementation doesn't scan for slash; otherwise return `Vec::new()`).

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app chatpalette`
Expected: compile FAIL — `after_edit` arity.

- [ ] **Step 3: Implement**

In `chatpalette.rs`: add `entries: Vec<crate::chatmention::MentionEntry>` to `PaletteState`. Rework `after_edit`:

```rust
pub(crate) fn after_edit(
    palette: &mut Option<PaletteState>,
    input: &str,
    scan: impl FnOnce() -> Vec<crate::chatmention::MentionEntry>,
) {
    let Some((kind, query)) = pending_palette(input) else {
        *palette = None;
        return;
    };
    // Reuse the open palette's scan; (re)scan when opening or kind changed.
    let entries = match palette {
        Some(p) if p.kind == kind => std::mem::take(&mut p.entries),
        _ => match kind {
            Kind::Agent => scan(),
            Kind::Slash => Vec::new(),
        },
    };
    let items = match kind {
        Kind::Slash => slash_items(query),
        Kind::Agent => attach_items(query, &entries, input.contains('+')),
    };
    if items.is_empty() {
        *palette = None;
        return;
    }
    match palette {
        Some(p) if p.kind == kind => {
            p.sel = p.sel.min(items.len() - 1);
            p.items = items;
            p.entries = entries;
        }
        _ => *palette = Some(PaletteState { kind, items, sel: 0, entries }),
    }
}

/// Rows for the leading `@`: the full attach picker (agents, skills, files
/// — `chatmention::filter`'s section order), agents only once the token has
/// a `+` (multi-target selectors route, they don't attach).
fn attach_items(query: &str, entries: &[crate::chatmention::MentionEntry], multi: bool) -> Vec<MenuItem> {
    use crate::chatmention::MentionEntry;
    crate::chatmention::filter(entries, query)
        .into_iter()
        .filter(|e| !multi || matches!(e, MentionEntry::Agent { .. }))
        .map(|e| MenuItem {
            label: format!("@{}", e.token()),
            desc: e.desc(),
            fill: e.token(),
            submit: false,
        })
        .collect()
}
```

Delete `agent_items` (replaced by `attach_items`; agent rows' `desc` changes from bare role to `"agent · role"` — consistent with the mention popup). Note `attach_items` filters by the query segment already extracted by `pending_palette` (after the last `+`), and `accept`'s existing `Kind::Agent` arm (`@a+` prefix preserved, `@{fill} `) works unchanged for all three kinds because `fill` is the full token.

NOTE (intended behavior change): `chatmention::filter` ranks by prefix/substring/subsequence, while the old `agent_items` matched by `starts_with` only — agent rows now also surface on substring/subsequence matches, consistent with the mention popup. State this in your report; do not "fix" it back.

In `chat.rs` (~line 427):

```rust
            let agents = self.agents.clone();
            crate::chatpalette::after_edit(&mut self.palette, &self.input, || {
                crate::chatmention::scan_entries(cwd, &agents)
            });
```

(The two clones in this function — this one and Task 2's — can share one `let agents = self.agents.clone();` placed before both `after_edit` calls; do that.)

In `render.rs` `palette_card_title`: `Kind::Agent => "attach"`.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-app chatpalette && cargo test -p crew-app chat && cargo test -p crew-app render`
Expected: ALL PASS. Then `cargo test -p crew-app` once (full crate) before committing.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chatpalette.rs crates/crew-app/src/chat.rs crates/crew-app/src/render.rs
git commit -m "feat(smith): leading @ palette becomes the full attach picker"
```
