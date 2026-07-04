# Directory Skills & Progressive Disclosure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Skills can be directories with bundled resources, and oversized playbooks frame as a pointer (description + outline + path) the agent follows via `@tool sys:read_file` instead of being inlined whole.

**Architecture:** `broker/skills.rs` learns directory discovery and two new `Skill` fields (`path`, `dir`); a new `broker/skillframe.rs` owns all presentation (inline frame, pointer frame, `/skills` listing); `broker/systools.rs` `read_file` gains an optional byte `offset` so >64 KB files are readable in chunks. Spec: `docs/superpowers/specs/2026-07-03-skill-dirs-progressive-disclosure-design.md`.

**Tech Stack:** Rust, std only. Tests are unit tests in sibling `*_tests.rs` files included via `#[cfg(test)] #[path = "..."] mod tests;`.

## Global Constraints

- Files stay ≤200 lines (repo guardrail). `skills.rs` is at 186, `systools.rs` at 181 — that's why presentation moves to `skillframe.rs`.
- No new external dependencies.
- TDD every task: write the failing test, watch it fail, implement, watch it pass.
- Pre-commit hook runs `cargo fmt` check + `cargo check`; run `cargo fmt` before committing.
- Test command for this crate: `cargo test -p crew-plugin <filter>`.
- Frame copy uses the existing typography: `\u{201c}`/`\u{201d}` curly quotes around skill names, `\u{2014}` em-dash, `\u{2026}` ellipsis, `\u{25aa}` list bullet.

---

### Task 1: `Skill.path`/`Skill.dir` + directory discovery

**Files:**
- Modify: `crates/crew-plugin/src/broker/skills.rs` (struct at :16, `parse` at :27, `load_dir` at :69)
- Test: `crates/crew-plugin/src/broker/skills_tests.rs`

**Interfaces:**
- Consumes: existing `parse(text, stem, origin) -> Skill`, `load_dir(dir, origin) -> Vec<Skill>`.
- Produces: `Skill { name, description, body, origin, path: PathBuf, dir: Option<PathBuf> }`. `parse` keeps its signature and sets `path: PathBuf::new(), dir: None`; `load_dir` fills both. Later tasks rely on `skill.path` (markdown source: the flat file, or the dir's `SKILL.md`) and `skill.dir` (`Some(root)` only for directory skills).

- [ ] **Step 1: Write the failing tests**

Append to `skills_tests.rs` (the `tmpdir` helper already exists at the top of the file):

```rust
#[test]
fn load_dir_reads_a_directory_skill() {
    let d = tmpdir("dirskill");
    std::fs::create_dir_all(d.join("My Skill")).unwrap();
    std::fs::write(d.join("My Skill").join("SKILL.md"), "Do the thing.").unwrap();
    let skills = load_dir(&d, "user");
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "my-skill");
    assert_eq!(skills[0].body, "Do the thing.");
    assert_eq!(skills[0].path, d.join("My Skill").join("SKILL.md"));
    assert_eq!(skills[0].dir.as_deref(), Some(d.join("My Skill").as_path()));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn directory_skill_frontmatter_name_wins_over_dir_name() {
    let d = tmpdir("dirname");
    std::fs::create_dir_all(d.join("foo")).unwrap();
    std::fs::write(
        d.join("foo").join("SKILL.md"),
        "---\nname: Real Name\n---\nbody",
    )
    .unwrap();
    let skills = load_dir(&d, "project");
    assert_eq!(skills[0].name, "real-name");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn subdir_without_skill_md_is_skipped_and_flat_files_get_path() {
    let d = tmpdir("mixed");
    std::fs::create_dir_all(d.join("not-a-skill")).unwrap();
    std::fs::write(d.join("flat.md"), "flat body").unwrap();
    let skills = load_dir(&d, "user");
    let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["flat"]);
    assert_eq!(skills[0].path, d.join("flat.md"));
    assert_eq!(skills[0].dir, None);
    let _ = std::fs::remove_dir_all(&d);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-plugin skills_tests 2>&1 | tail -20` — wait, the module filter is `skills::tests`. Use: `cargo test -p crew-plugin skills::tests`
Expected: compile error — `Skill` has no field `path`/`dir`. A compile failure in the test is the correct RED here.

- [ ] **Step 3: Implement**

In `skills.rs`, extend the struct (keep the doc comments):

```rust
/// One loaded playbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,
    /// Where it came from: `"user"` or `"project"`.
    pub origin: &'static str,
    /// The markdown source: the flat file, or the directory's `SKILL.md`.
    pub path: PathBuf,
    /// The skill's root directory — `Some` only for directory skills.
    pub dir: Option<PathBuf>,
}
```

In `parse`, extend the final constructor:

```rust
    Skill {
        name,
        description,
        body: body.to_string(),
        origin,
        path: PathBuf::new(),
        dir: None,
    }
```

Replace `load_dir` (currently :69–:87). Doc comment change: “All `.md` skills in `dir`, plus subdirectories containing a `SKILL.md` (empty when the dir doesn't exist), sorted by name so loading order is stable.”

```rust
pub(crate) fn load_dir(dir: &Path, origin: &'static str) -> Vec<Skill> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    // (markdown source, name stem, skill root for directory skills)
    let mut sources: Vec<(PathBuf, String, Option<PathBuf>)> = Vec::new();
    for p in entries.flatten().map(|e| e.path()) {
        if p.extension().is_some_and(|e| e == "md") {
            let Some(stem) = p.file_stem() else { continue };
            sources.push((p.clone(), stem.to_string_lossy().into_owned(), None));
        } else if p.is_dir() && p.join("SKILL.md").is_file() {
            let Some(name) = p.file_name() else { continue };
            let name = name.to_string_lossy().into_owned();
            sources.push((p.join("SKILL.md"), name, Some(p)));
        }
    }
    sources.sort_by(|a, b| a.1.cmp(&b.1));
    sources
        .into_iter()
        .filter_map(|(path, stem, root)| {
            let text = std::fs::read_to_string(&path).ok()?;
            let mut s = parse(&text, &stem, origin);
            s.path = path;
            s.dir = root;
            Some(s)
        })
        .collect()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-plugin skills::tests`
Expected: all pass, including the pre-existing parse/merge tests (they build `Skill`s via `parse`, which now defaults the new fields).

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-plugin/src/broker/skills.rs crates/crew-plugin/src/broker/skills_tests.rs
git commit -m "feat(crew): skills can be directories with a SKILL.md"
```

---

### Task 2: `skillframe.rs` — inline frame moves out, dir skills get a supporting-files line

**Files:**
- Create: `crates/crew-plugin/src/broker/skillframe.rs`
- Create: `crates/crew-plugin/src/broker/skillframe_tests.rs`
- Modify: `crates/crew-plugin/src/broker/skills.rs` (delete `framed` at :133–:139; update the call in `skill_cmd` at :181)
- Modify: `crates/crew-plugin/src/broker/mod.rs` (add `mod skillframe;` next to `mod skills;`)

**Interfaces:**
- Consumes: `skills::Skill` (Task 1 shape), `systools::enabled() -> bool`.
- Produces: `skillframe::framed(skill: &Skill, task: &str, sys_on: bool) -> String`. With `sys_on` either way, a small body must render byte-identical to the old `skills::framed` output when `skill.dir` is `None`. (`sys_on` is plumbed now, used by Task 3.)

- [ ] **Step 1: Write the failing tests**

`skillframe_tests.rs`:

```rust
use super::*;
use crate::broker::skills::parse;

fn skill_with(body: &str, dir: Option<&str>) -> crate::broker::skills::Skill {
    let mut s = parse(body, "demo", "user");
    s.path = std::path::PathBuf::from("/skills/demo/SKILL.md");
    s.dir = dir.map(std::path::PathBuf::from);
    s
}

#[test]
fn small_flat_skill_frames_exactly_as_before() {
    let s = skill_with("Check unsafe blocks.", None);
    assert_eq!(
        framed(&s, "review foo.rs", true),
        "SKILL \u{201c}demo\u{201d} \u{2014} follow this playbook:\n\
         Check unsafe blocks.\n\nTASK:\nreview foo.rs"
    );
}

#[test]
fn directory_skill_frame_points_at_its_supporting_files() {
    let s = skill_with("Check unsafe blocks.", Some("/skills/demo"));
    let f = framed(&s, "review foo.rs", true);
    assert!(f.contains("Supporting files: /skills/demo"), "got: {f}");
    assert!(f.contains("@tool sys:read_file"), "got: {f}");
    assert!(f.contains("@tool sys:run"), "got: {f}");
    assert!(f.ends_with("TASK:\nreview foo.rs"), "got: {f}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-plugin skillframe`
Expected: compile error — module `skillframe` does not exist.

- [ ] **Step 3: Implement**

`skillframe.rs`:

```rust
//! Skill framing: how a playbook is presented to the relay. Small bodies are
//! inlined whole (as `/skill` always did); directory skills add a pointer to
//! their bundled files, readable through the `@tool sys` surface.
use super::skills::Skill;

/// The relay body for a skill run: playbook first, then the task.
pub(crate) fn framed(skill: &Skill, task: &str, _sys_on: bool) -> String {
    inline(skill, task)
}

/// The full-body frame (identical to the historical format for flat skills).
fn inline(skill: &Skill, task: &str) -> String {
    format!(
        "SKILL \u{201c}{}\u{201d} \u{2014} follow this playbook:\n{}\n{}\nTASK:\n{task}",
        skill.name,
        skill.body,
        support(skill)
    )
}

/// One line pointing a directory skill's agent at its bundled files.
fn support(skill: &Skill) -> String {
    match &skill.dir {
        Some(root) => format!(
            "\nSupporting files: {} \u{2014} read with @tool sys:read_file \
             {{\"path\": \u{2026}}}; run scripts with @tool sys:run.\n",
            root.display()
        ),
        None => String::new(),
    }
}

#[cfg(test)]
#[path = "skillframe_tests.rs"]
mod tests;
```

In `broker/mod.rs`, add `mod skillframe;` alongside the existing `mod skills;`.

In `skills.rs`: delete `framed` (:133–:139) and change the last line of `skill_cmd` to:

```rust
    let sys_on = super::systools::enabled();
    relay_turn(
        &broker,
        &start,
        &super::skillframe::framed(skill, &task, sys_on),
        "skill-1",
        emit,
    )
    .map(|_| ())
```

Move the two existing `framed`-related tests out of `skills_tests.rs` if present (grep for `framed` there; the current file has none — the doc block test coverage lived in `skills.rs` history, so nothing to move).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-plugin skillframe && cargo test -p crew-plugin skills::tests`
Expected: PASS both.

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-plugin/src/broker/skillframe.rs crates/crew-plugin/src/broker/skillframe_tests.rs crates/crew-plugin/src/broker/skills.rs crates/crew-plugin/src/broker/mod.rs
git commit -m "refactor(crew): skill framing moves to skillframe.rs; dir skills point at their files"
```

---

### Task 3: Pointer framing for oversized bodies

**Files:**
- Modify: `crates/crew-plugin/src/broker/skillframe.rs`
- Test: `crates/crew-plugin/src/broker/skillframe_tests.rs`

**Interfaces:**
- Consumes: Task 2's `framed`/`inline`/`support`.
- Produces: `pub(crate) const INLINE_CAP: usize = 8 * 1024;` (Task 5 reads it for the `/skills` suffix). `framed` behavior: body > `INLINE_CAP` **and** `sys_on` → pointer frame (description, intro, `Outline:` of `##`/`###` headings, `Full playbook: <path> …`, support line, task); otherwise inline.

- [ ] **Step 1: Write the failing tests**

Append to `skillframe_tests.rs`:

```rust
fn big_body() -> String {
    // > 8 KB, with headings and an intro paragraph.
    let mut b = String::from("This skill explains everything.\n\n## Setup\n");
    b.push_str(&"filler line for bulk\n".repeat(500));
    b.push_str("### Details\nmore\n## Usage\nfinal\n");
    b
}

#[test]
fn oversized_body_frames_as_outline_and_path() {
    let s = skill_with(&big_body(), None);
    let f = framed(&s, "do it", true);
    assert!(f.len() < 2048, "pointer frame stays small, got {} bytes", f.len());
    assert!(f.contains("SKILL \u{201c}demo\u{201d} \u{2014} This skill explains everything."));
    assert!(f.contains("Outline:\n## Setup\n### Details\n## Usage"), "got: {f}");
    assert!(f.contains("Full playbook: /skills/demo/SKILL.md"), "got: {f}");
    assert!(f.contains("@tool sys:read_file"), "got: {f}");
    assert!(f.ends_with("TASK:\ndo it"), "got: {f}");
}

#[test]
fn oversized_body_with_no_headings_frames_as_intro_and_path() {
    let body = "line\n".repeat(2000);
    let s = skill_with(&body, None);
    let f = framed(&s, "do it", true);
    assert!(!f.contains("Outline:"), "got: {f}");
    assert!(f.contains("Full playbook:"), "got: {f}");
    // Intro is byte-clipped ~1 KB, so the frame stays small.
    assert!(f.len() < 2048, "got {} bytes", f.len());
}

#[test]
fn sys_tools_off_falls_back_to_full_inline() {
    let s = skill_with(&big_body(), None);
    let f = framed(&s, "do it", false);
    assert!(f.contains("follow this playbook"), "got: {f}");
    assert!(f.contains("filler line for bulk"), "got: {f}");
    assert!(!f.contains("Full playbook:"), "got: {f}");
}

#[test]
fn small_body_still_inlines_even_with_sys_on() {
    let s = skill_with("tiny", None);
    assert!(framed(&s, "t", true).contains("follow this playbook"));
}
```

Note on the first test: the skill's `description` falls back to the body's first non-empty line (`parse` behavior), which is why the header reads `demo — This skill explains everything.`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-plugin skillframe`
Expected: `oversized_body_frames_as_outline_and_path`, `oversized_body_with_no_headings_frames_as_intro_and_path` FAIL (frame contains the whole body, no `Full playbook:`); the other two PASS (they pin current behavior).

- [ ] **Step 3: Implement**

In `skillframe.rs`, add the cap and replace `framed`:

```rust
/// Bodies over this many bytes are pointer-framed instead of inlined (~2k tokens).
pub(crate) const INLINE_CAP: usize = 8 * 1024;

/// The relay body for a skill run: playbook first, then the task. Oversized
/// playbooks become a pointer frame — description, intro, heading outline, and
/// the path to read on demand — when the agent has `sys` tools to follow it;
/// without them a pointer would be a dead end, so everything inlines.
pub(crate) fn framed(skill: &Skill, task: &str, sys_on: bool) -> String {
    if !sys_on || skill.body.len() <= INLINE_CAP {
        return inline(skill, task);
    }
    let heads: Vec<&str> = skill
        .body
        .lines()
        .filter(|l| l.starts_with("## ") || l.starts_with("### "))
        .collect();
    let outline = if heads.is_empty() {
        String::new()
    } else {
        format!("Outline:\n{}\n", heads.join("\n"))
    };
    format!(
        "SKILL \u{201c}{}\u{201d} \u{2014} {}\n{}\n{outline}Full playbook: {} \u{2014} \
         read the sections you need with @tool sys:read_file \
         {{\"path\": \u{2026}, \"offset\": \u{2026}}} before starting.\n{}\nTASK:\n{task}",
        skill.name,
        skill.description,
        intro(&skill.body),
        skill.path.display(),
        support(skill)
    )
}

/// Body text before the first `##` heading, byte-clipped at 1 KB on a char
/// boundary — the playbook's own preamble, kept as scene-setting.
fn intro(body: &str) -> &str {
    let head = if body.starts_with("## ") {
        ""
    } else {
        body.find("\n## ").map_or(body, |i| &body[..i])
    };
    if head.len() <= 1024 {
        return head.trim_end();
    }
    let mut cut = 1024;
    while !head.is_char_boundary(cut) {
        cut -= 1;
    }
    head[..cut].trim_end()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-plugin skillframe`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-plugin/src/broker/skillframe.rs crates/crew-plugin/src/broker/skillframe_tests.rs
git commit -m "feat(crew): oversized skill bodies frame as outline + path, not full inline"
```

---

### Task 4: `sys:read_file` offset

**Files:**
- Modify: `crates/crew-plugin/src/broker/systools.rs` (dispatch at :95, `read_file` at :120)
- Test: `crates/crew-plugin/src/broker/systools_tests.rs`

**Interfaces:**
- Consumes: existing `CAP` (64 KB), `is_utf8_boundary(bytes, idx)`.
- Produces: `read_file(path: &str, offset: usize)`; JSON surface `@tool sys:read_file {"path": …, "offset": …}` where `offset` is an optional byte position (default 0). Truncated reads end with `… (truncated at 64 KB — file is <total> bytes; continue with {"offset": <next>})`. Tool description string updated to mention `offset`.

- [ ] **Step 1: Write the failing tests**

Append to `systools_tests.rs` (check its top for an existing temp-file helper and reuse it; otherwise write files under `std::env::temp_dir()` with pid+thread in the name, as `skills_tests.rs` does):

```rust
#[test]
fn read_file_offset_resumes_mid_file() {
    let p = std::env::temp_dir().join(format!("crew-sys-off-{}.txt", std::process::id()));
    std::fs::write(&p, "abcdefghij").unwrap();
    let out = call("read_file", &format!("{{\"path\": \"{}\", \"offset\": 4}}", p.display())).unwrap();
    assert_eq!(out, "efghij");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_file_truncation_notice_names_the_next_offset() {
    let p = std::env::temp_dir().join(format!("crew-sys-big-{}.txt", std::process::id()));
    std::fs::write(&p, "x".repeat(CAP + 100)).unwrap();
    let out = call("read_file", &format!("{{\"path\": \"{}\"}}", p.display())).unwrap();
    assert!(out.contains("truncated at 64 KB"), "got tail: {}", &out[out.len() - 120..]);
    assert!(
        out.contains(&format!("file is {} bytes", CAP + 100)),
        "got tail: {}",
        &out[out.len() - 120..]
    );
    assert!(
        out.contains(&format!("continue with {{\"offset\": {CAP}}}")),
        "got tail: {}",
        &out[out.len() - 120..]
    );
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_file_offset_past_eof_says_so() {
    let p = std::env::temp_dir().join(format!("crew-sys-eof-{}.txt", std::process::id()));
    std::fs::write(&p, "short").unwrap();
    let out = call("read_file", &format!("{{\"path\": \"{}\", \"offset\": 99}}", p.display())).unwrap();
    assert!(out.contains("offset 99"), "got: {out}");
    assert!(out.contains("5 bytes"), "got: {out}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_file_offset_mid_codepoint_skips_to_a_boundary() {
    let p = std::env::temp_dir().join(format!("crew-sys-utf8-{}.txt", std::process::id()));
    std::fs::write(&p, "é-tail").unwrap(); // 'é' is 2 bytes; offset 1 lands mid-char
    let out = call("read_file", &format!("{{\"path\": \"{}\", \"offset\": 1}}", p.display())).unwrap();
    assert_eq!(out, "-tail");
    let _ = std::fs::remove_file(&p);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-plugin systools`
Expected: the four new tests FAIL — offset is ignored (`read_file` takes no offset), notices absent. If an existing truncation test asserts the exact old string `"… (truncated at 64 KB)"`, it will keep passing now and fail after Step 3 — update its assertion to `contains("truncated at 64 KB")` as part of Step 3.

- [ ] **Step 3: Implement**

Dispatch (:95) becomes:

```rust
        "read_file" => read_file(str_arg(&v, "path")?, offset_arg(&v)),
```

Below `str_arg`, add:

```rust
/// The optional `"offset"` byte argument, defaulting to 0.
fn offset_arg(v: &serde_json::Value) -> usize {
    v.get("offset").and_then(|n| n.as_u64()).unwrap_or(0) as usize
}
```

Tool description (:68–:69) becomes:

```rust
        mk(
            "read_file",
            "read a UTF-8 text file, 64 KB per call: {\"path\": \"README.md\", \"offset\": 0}",
        ),
```

Replace `read_file`:

```rust
fn read_file(path: &str, offset: usize) -> Result<String, String> {
    use std::io::{Read, Seek, SeekFrom};
    // Bound the I/O itself: at most CAP+1 bytes, so a huge or never-EOF file
    // (e.g. /dev/zero) can't blow up memory or hang before the cap applies.
    let mut f = std::fs::File::open(path).map_err(|e| format!("read {path}: {e}"))?;
    let total = f.metadata().map_err(|e| format!("read {path}: {e}"))?.len() as usize;
    if offset > 0 {
        f.seek(SeekFrom::Start(offset as u64))
            .map_err(|e| format!("read {path}: {e}"))?;
    }
    let mut buf = Vec::new();
    f.take(CAP as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|e| format!("read {path}: {e}"))?;
    if buf.is_empty() && offset > 0 {
        return Ok(format!(
            "\u{2026} (offset {offset} is at or past the end \u{2014} file is {total} bytes)"
        ));
    }
    // An offset that lands mid-codepoint skips forward to the next boundary
    // (at most 3 continuation bytes).
    let start = (0..=3.min(buf.len()))
        .find(|&i| is_utf8_boundary(&buf, i))
        .unwrap_or(0);
    let buf = &buf[start..];
    if buf.len() > CAP {
        // A valid UTF-8 boundary must occur within 3 bytes of any index, so
        // bound the walk-back to at most 3 steps. If none of them is a
        // boundary, the content isn't valid UTF-8 at all — report that.
        let floor = CAP.saturating_sub(3);
        let cut = (floor..=CAP).rev().find(|&i| is_utf8_boundary(buf, i));
        let cut = cut.ok_or_else(|| {
            format!("read {path}: not valid UTF-8: no character boundary near the 64 KB cap")
        })?;
        let text = std::str::from_utf8(&buf[..cut])
            .map_err(|e| format!("read {path}: not valid UTF-8: {e}"))?;
        return Ok(format!(
            "{text}\n\u{2026} (truncated at 64 KB \u{2014} file is {total} bytes; \
             continue with {{\"offset\": {}}})",
            offset + start + cut
        ));
    }
    std::str::from_utf8(buf)
        .map(str::to_owned)
        .map_err(|e| format!("read {path}: not valid UTF-8: {e}"))
}
```

`systools.rs` grows past 200 lines with this (~181 → ~205): trim by moving nothing — instead compact the two error-notice comments into one line each; if still over 200, move `is_utf8_boundary` + `read_file` into a new `broker/sysread.rs` (`pub(super)` items, `use super::systools::CAP;`) and re-export. Check with `wc -l` and prefer the split only if needed.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-plugin systools && cargo test -p crew-plugin toolcall`
Expected: all PASS (toolcall e2e tests exercise `sys:` dispatch and must not regress).

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-plugin/src/broker/systools.rs crates/crew-plugin/src/broker/systools_tests.rs
git commit -m "feat(crew): sys:read_file takes a byte offset for chunked reads"
```

---

### Task 5: `/skills` listing suffixes (and `list_report` moves to skillframe)

**Files:**
- Modify: `crates/crew-plugin/src/broker/skillframe.rs` (gains `list_report`)
- Modify: `crates/crew-plugin/src/broker/skills.rs` (delete `list_report` at :112–:131)
- Modify: `crates/crew-plugin/src/broker/commands.rs:190` (call site)
- Test: `crates/crew-plugin/src/broker/skillframe_tests.rs`

**Interfaces:**
- Consumes: `INLINE_CAP` (Task 3), `Skill.dir`/`Skill.body` (Task 1).
- Produces: `skillframe::list_report(skills: &[Skill]) -> String`. Line shape: `▪ <name> — <description> (<origin>[, dir][, <n> KB → outline])`. Empty-list copy unchanged from today's `skills::list_report`.

- [ ] **Step 1: Write the failing tests**

Append to `skillframe_tests.rs`:

```rust
#[test]
fn list_report_marks_dir_skills_and_outline_framing() {
    let mut small = skill_with("tiny", Some("/skills/small"));
    small.name = "small".into();
    let mut big = skill_with(&big_body(), None);
    big.name = "big".into();
    let report = list_report(&[small, big]);
    assert!(report.contains("small \u{2014} tiny (user, dir)"), "got: {report}");
    assert!(report.contains("(user, 10 KB \u{2192} outline)"), "got: {report}");
}

#[test]
fn list_report_plain_flat_skill_line_is_unchanged() {
    let s = skill_with("tiny", None);
    let report = list_report(&[s]);
    assert!(report.contains("demo \u{2014} tiny (user)"), "got: {report}");
}
```

Note: `big_body()` is ~10.5 KB (500 × 21-byte lines + headings), so integer division shows `10 KB`. If the assertion fails on the number, print the report and match the actual `len / 1024`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-plugin skillframe`
Expected: compile error — no `list_report` in scope.

- [ ] **Step 3: Implement**

Move `list_report` from `skills.rs` into `skillframe.rs`, with the suffix logic:

```rust
/// The `/skills` listing: one line per skill, or where to put files.
pub(crate) fn list_report(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return "No skills found. Drop markdown playbooks into \
                ~/.config/crew/skills/ or ./.crew/skills/ \
                (optional `---` frontmatter: name, description), then \
                run one with /skill <name> <task>."
            .into();
    }
    let lines: Vec<String> = skills
        .iter()
        .map(|s| {
            let mut tag = s.origin.to_string();
            if s.dir.is_some() {
                tag.push_str(", dir");
            }
            if s.body.len() > INLINE_CAP {
                tag.push_str(&format!(", {} KB \u{2192} outline", s.body.len() / 1024));
            }
            format!("\u{25aa} {} \u{2014} {} ({tag})", s.name, s.description)
        })
        .collect();
    lines.join("\n")
}
```

Delete `list_report` from `skills.rs`. Update `commands.rs:190`:

```rust
            super::skillframe::list_report(&super::skills::load()),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-plugin skillframe && cargo test -p crew-plugin commands`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt && git add crates/crew-plugin/src/broker/skillframe.rs crates/crew-plugin/src/broker/skillframe_tests.rs crates/crew-plugin/src/broker/skills.rs crates/crew-plugin/src/broker/commands.rs
git commit -m "feat(crew): /skills marks directory skills and outline-framed sizes"
```

---

### Task 6: E2E — an agent follows the pointer frame through the tool loop

**Files:**
- Test: `crates/crew-plugin/src/broker/toolcall_tests.rs` (append; mirrors `relay_runs_a_real_sys_command_and_logs_hops` at :192, which defines the `SysOnly` runner at :180)

**Interfaces:**
- Consumes: `skillframe::framed` (Task 3), `sys:read_file` offset surface (Task 4), existing `SysOnly` tool runner, `Broker::run_tools`, `Scripted`, `RunStats`, `env()` — all already in this test file.

- [ ] **Step 1: Write the test**

```rust
#[test]
fn pointer_framed_skill_lets_the_agent_read_the_playbook() {
    // A >8 KB skill on disk: the frame must carry a pointer, and the loop
    // must resolve a sys:read_file for that pointer's path.
    let p = std::env::temp_dir().join(format!("crew-skillframe-e2e-{}.md", std::process::id()));
    let body = format!("Intro.\n## Only Section\n{}", "needle-content\n".repeat(700));
    std::fs::write(&p, &body).unwrap();
    let mut skill = crate::broker::skills::parse(&body, "big-skill", "user");
    skill.path = p.clone();
    let frame = crate::broker::skillframe::framed(&skill, "use the playbook", true);
    assert!(frame.contains("Full playbook:"), "got: {frame}");
    assert!(frame.contains(&p.display().to_string()), "got: {frame}");

    let broker = Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5))
        .with_tools(std::sync::Arc::new(SysOnly));
    let agent = Scripted::new(&["read it, proceeding"]);
    let mut hops = Vec::new();
    let mut stats = RunStats::default();
    let reply = broker.run_tools(
        &agent,
        &frame,
        format!("checking\n@tool sys:read_file {{\"path\": \"{}\"}}", p.display()),
        &mut stats,
        &env(),
        &mut |h| hops.push(h),
    );
    assert_eq!(reply, "read it, proceeding");
    assert!(
        hops.iter().any(|h| h.text.contains("needle-content")),
        "tool result hop carries the playbook text"
    );
    let _ = std::fs::remove_file(&p);
}
```

- [ ] **Step 2: Run it — this test should pass immediately**

Run: `cargo test -p crew-plugin pointer_framed_skill`
Expected: PASS. This is a deliberate integration check of Tasks 3+4 together, not a RED step; if it fails, one of the earlier tasks is wrong — debug there, do not adjust this test to fit.

- [ ] **Step 3: Run the whole crate**

Run: `cargo test -p crew-plugin`
Expected: everything green.

- [ ] **Step 4: Commit**

```bash
cargo fmt && git add crates/crew-plugin/src/broker/toolcall_tests.rs
git commit -m "test(crew): e2e — pointer-framed skill resolves through the sys tool loop"
```

---

### Task 7: Reinstall the official skills as directories (ops, no crew code)

**Files:** none in-repo; operates on `~/Library/Application Support/crew/skills/` and the scratchpad clone of `anthropics/skills`.

- [ ] **Step 1: Replace flat conversions with full skill folders**

The clone lives at `<scratchpad>/anthropic-skills` (re-clone `https://github.com/anthropics/skills` shallow if the scratchpad was cleaned):

```bash
SRC=<scratchpad>/anthropic-skills/skills
DEST="$HOME/Library/Application Support/crew/skills"
for d in "$SRC"/*/; do
  name=$(basename "$d")
  rm -f "$DEST/$name.md"        # the flat conversion from 2026-07-03
  rm -rf "$DEST/$name"
  cp -R "$d" "$DEST/$name"
done
ls "$DEST"
```

Expected: 17 directories, no stray `<name>.md` flats for those names.

- [ ] **Step 2: Verify discovery end-to-end**

Skills hot-reload (fresh `load()` per call), but the *broker binary* must contain Tasks 1–5 — the running app spawns the installed `~/.local/bin/crew`. Rebuild and reinstall it, then restart the app:

```bash
cargo build --release && cp target/release/crew ~/.local/bin/crew
```

In the app's `/crew` pane run `/skills`. Expected: all 17 listed with `(user, dir)`; `claude-api` also shows `, 72 KB → outline` (its body is ~74 KB on disk; the parsed body is a little smaller). Then a live probe: `/skill frontend-design say READY and stop` — the reply should show the agent got the playbook (small skill, inlined; its frame now also names the supporting-files root).

- [ ] **Step 3: Nothing to commit** — this task changes only the user machine.

---

## Self-Review Notes

- Spec §1 → Task 1; §2 → Tasks 2–3; §3 → Task 4; §4 → Task 5 (errors are load-and-skip, preserved by Task 1's `filter_map`); §5 → per-task tests + Task 6; §6 → Task 7.
- Type check: `framed(&Skill, &str, bool) -> String` consistent across Tasks 2, 3, 6; `INLINE_CAP` defined Task 3, consumed Task 5; `read_file(&str, usize)` defined Task 4, exercised via `call()` JSON surface in Tasks 4 and 6.
- Line-count risk called out where it exists (`systools.rs`, Task 4 Step 3) with the split fallback named.
