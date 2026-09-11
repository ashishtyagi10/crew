# Todo assignees: one list a team is run from

**Set 2026-09-11. Shipped v0.22.17.**

## The ask

> Can we also add like '#' support in todo for assignment and group todo by
> '#' when looking, I actually wants to run my team and get daily status.

## What it means

The `/todo` pane already knew one thing about a task besides its words and
its date: which `@project` it belongs to. Running a team needs the other
one — **who it is for** — and needs the list to be readable *by person*, not
only by deadline. A flat list sorted by due date answers "what is next".
A lead standing up a team every morning is asking a different question:
"where is everyone", and that one is per-person.

## Done means

1. `#name` in the composer becomes the item's assignee, the same way
   `@project` becomes its project: free-form, created on first use,
   completed from names already in use, tinted live, stripped from the
   title, and carried back by `e`.
2. The two axes are independent and AND-ed. A lone `#name` filters to one
   person; a bare `#` clears that axis and leaves `@project` alone.
3. `#123` is not a person. An all-digit token stays in the title.
4. `g` on the list (and `/todo by who`) bands the rows under each assignee,
   unassigned last and named, each band carrying `N open · N overdue ·
   N done today` counted over the **whole store** — "done today" is the half
   of a standup a list with done items hidden otherwise cannot show.
5. The history view keeps its day bands and ignores the person bands; it is
   already a per-day log. `/todo done #name` filters it to one person.

## How it is built

- `todopane/parse.rs` — both sigils, one tokeniser: the first `@token` and
  the first `#token` leave the title, the rest is prose.
- `todopane/group.rs` — `Bands`, the banded display order, which display row
  opens a band, and the roll-up. `group::starts` is THE band truth: the
  draw, the scroll math and the hit-test all sum it, so a header can never
  be counted by one and missed by another.
- `todopane/headrow.rs` — every row of the pane that is not an item: the
  filter/info row, the band headers, the empty-list hint.
- `todopane/legend.rs` — what the composer's legend says, split from the box
  it is drawn on.
- `todopane/listkeys.rs` — the list's keys, split from the composer's.
- `item::Filters` — the two filters as one `Copy` value, so every ordering
  function takes the same thing and cannot honour one axis and forget the
  other.

## What it dragged in

Longer rows made two pre-existing weaknesses visible, both fixed here:
every due label now names the calendar date as well as the relative word,
and a stacked (narrow-pane) row lays its chips from the left of the line the
title vacated rather than right-aligning them behind the due, where they
were being silently dropped.

## Not done

- No person-filter cycle key to match `]`/`[` for projects; the bands are
  the browse affordance.
- Bands are one level deep — no `@project` inside `#person`.
- Nothing here is GUI-verified: the evidence is the unit suite and the
  off-screen shot sweep (`todo-by-who-*.png`).
