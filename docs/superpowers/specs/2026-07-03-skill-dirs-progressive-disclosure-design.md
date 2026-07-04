# Directory skills & progressive disclosure — design

2026-07-03. Extends the skills surface from `2026-07-01-skills-plugins-mcp-design.md`
to fix two limits found installing Anthropic's official skills: (1) skills that
bundle supporting files (scripts, references) lose them — crew skills are single
flat markdown files; (2) large playbooks (e.g. `claude-api`, 74 KB) are prepended
whole to every `/skill` turn. One structural fix covers both: skills may be
directories, and oversized bodies frame as a pointer the agent follows with the
existing `@tool sys` surface instead of being inlined.

Guardrails as ever: ≤200-line files, no new external dependencies. `skills.rs`
is at 186 lines, so framing logic lands in a new module.

## 1. Skill model & discovery — `broker/skills.rs`

`Skill` gains two fields:

- `path: PathBuf` — absolute path of the markdown source (the flat file, or the
  directory's `SKILL.md`). Needed so oversized flat skills can be pointer-framed
  too.
- `dir: Option<PathBuf>` — absolute skill root; `Some` only for directory skills.

`load_dir` accepts one new entry shape alongside `*.md` files: a subdirectory
containing `SKILL.md`. Name defaults to the directory name (normalized, as for
file stems); frontmatter `name:`/`description:` override; the description
fallback (first non-empty body line, clipped) is unchanged. A subdirectory
without `SKILL.md` is silently skipped, as is an unreadable one. Load order
stays stable: entries sorted by name, files and dirs together.

Merging is untouched: project (`./.crew/skills/`) overrides user
(`dirs::config_dir()/crew/skills/`) by name, whether either side is a file or a
directory.

Anthropic's official skill folders (`skills/<name>/SKILL.md` + `scripts/`,
`references/`, …) then install with a plain `cp -r` — no conversion step.

## 2. Pointer framing — new `broker/skillframe.rs`

`INLINE_CAP: usize = 8 * 1024` bytes (~2k tokens).

`framed(skill, task, sys_on) -> String` replaces the current `skills::framed`:

- **Body ≤ cap** — today's frame:
  `SKILL "<name>" — follow this playbook:\n<body>\n\nTASK:\n<task>`.
  Directory skills append one line before `TASK:`:
  `Supporting files: <root> — read with @tool sys:read_file {"path": …}; run
  scripts with @tool sys:run.`
- **Body > cap, sys tools on** — pointer frame, top to bottom:
  - `SKILL "<name>" — <description>`
  - intro: body text before the first `##` heading, clipped to 1 KB
  - `Outline:` — the `##`/`###` heading lines, verbatim, one per line
  - `Full playbook: <path> — read the sections you need with
    @tool sys:read_file {"path": …, "offset": …} before starting.`
  - the supporting-files line for directory skills
  - `TASK:\n<task>`
  - A body with no headings frames as intro + path (no `Outline:` block).
- **Sys tools off** (`CREW_SYS_TOOLS=0` or mock provider) — full inline
  regardless of size: a pointer the agent cannot follow is worse than tokens.
  `sys_on` is passed in (from `systools::enabled()`) so the function stays pure
  and testable.

`skill_cmd` calls the new module; the framing text is otherwise unchanged so
existing transcripts stay recognizable.

## 3. `sys:read_file` offset — `broker/systools.rs`

Optional `"offset"` argument (byte position, default 0, clamped to a UTF-8
boundary with the existing helper). Reads up to the existing 64 KB `CAP` from
`offset`; when truncated, the reply's notice states the file's total size and
the next offset to request. Read-only mode (`CREW_SYS_MODE=readonly`) is
unaffected — `read_file` is already permitted there, which is exactly why the
offset lives on `read_file` rather than leaning on `sys:run` + `sed`.

## 4. Surfaces & errors

No new commands. `/skills` lines gain a suffix for the new shapes:
`(user, dir)` for directory skills, plus `, 74 KB → outline` when the body
exceeds `INLINE_CAP`. `/skill <name> <task>` behavior, hot-reload semantics
(fresh `load()` per invocation), and the unknown-skill hint are unchanged.

Error handling stays load-and-skip: missing/unreadable `SKILL.md`, non-UTF-8
content → the skill simply doesn't appear.

## 5. Testing

Unit tests alongside each module, TDD as usual:

- **skills**: directory discovery; dir-name vs frontmatter-name precedence;
  project-dir-overrides-user-flat merge; subdir without `SKILL.md` skipped;
  `path`/`dir` populated correctly.
- **skillframe**: threshold boundary (≤ cap inlines, > cap points); outline
  extraction (`##`/`###` only, verbatim); no-headings fallback; intro clip;
  supporting-files line only for dir skills; sys-off forces full inline.
- **systools**: offset read start/middle/end; truncation notice carries next
  offset; offset clamped to char boundary; default offset 0 preserves current
  behavior.
- **toolcall (e2e-style)**: Scripted agent receives a pointer frame and issues
  `@tool sys:read_file` with the framed path — the loop resolves it.

## 6. Follow-up (no crew code)

Reinstall the official skills as directories (`cp -r` each folder into the user
skills dir, replacing the flat conversions): `docx`/`pptx`/`xlsx`/`pdf` regain
their scripts via `sys:run`, and `claude-api` switches to outline framing
automatically.
