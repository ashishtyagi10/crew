# `docs/superpowers/` — where crew's intent is written down

Four kinds of document live here, and only one of them is about the future.

| directory | what it is | is it live? |
|---|---|---|
| `goals/` | What the user asked for, why it matters, and what "done" means. One file per goal, each carrying a **Status** line kept true against the tree. | **Yes.** Start here. |
| `specs/` | Design rationale written before a feature was built — the reasoning, alternatives and trade-offs behind a shape crew still has. | Historical, but still cited (`README.md`, `CONTRIBUTING.md`, `docs/CREW.md`). |
| `*-loop.md` | Playbooks for the autonomous loops: the lenses each one looks through and the rules it runs under. | Yes. |
| `*-log.md` | What those loops actually did, iteration by iteration. | Append-only record. |

**There is no `plans/` directory any more.** Sixty-two implementation plans were written between
2026-06-19 and 2026-08-01, one per feature, and every one of them shipped; they were deleted on
2026-09-01 because a directory of finished instruction sets is a place to go looking for open work
and find none. Git has them, `CHANGELOG.md` says what each became, and the code is the version that
is true.

**Where to look for what:**

* *What is crew supposed to become?* → `goals/`, newest first. `2026-09-01-close-the-open-goals.md`
  is the current one and audits every other goal's state in a table.
* *Why is this built this way?* → `specs/`, then the module doc comment, which is usually more
  current.
* *What changed in release N?* → `CHANGELOG.md` at the repo root.
* *How does it fit together?* → `docs/ARCHITECTURE.md`; the user-facing manual is `docs/CREW.md`.

A goal document is only worth keeping if its **Status** line is true. Update it in the merge that
changes the state, not later.
