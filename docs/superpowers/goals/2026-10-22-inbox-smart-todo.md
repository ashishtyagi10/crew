# Goal — `t` in the inbox makes the todo the mail asks for

**Status: IN PROGRESS.** Set 2026-10-22; the open questions answered the same day (the user: "go
ahead with the goal, use your recommendations"): a card first, smith only on Tab, and no
`#assignee` for a task asked of you.

**Set:** 2026-10-22 by the user: *"write a goal to enhance todo action in inbox, it should be
smart enough to identify if it is related with some projects, should be able to find the due
date and assignee, you can read the content of email."*

## Where it stands today

`t` on an open thread (or on a row, which opens the thread first) adds one `/todo` row at once
(`handoff.rs`, `to_todo`):

- **The title** is always "Reply to {sender}: {subject}" (`smith.rs`, `todo_title`), even when
  the mail asks for something else ("can you send the Q4 numbers").
- **The due date** is a date within the four words after "by", "before", "until", "due" or
  "no later than", in the newest mail only (`smith.rs`, `deadline`). It is read against today,
  not the day the mail was sent, so "by Friday" in last week's mail lands on next Friday.
- **No project and no assignee.** The row has both fields (`item.rs`: `@project`, `#assignee`),
  and `t` fills neither.

What crew already has to work with:

- **The whole thread:** every mail's sender, To and Cc, date, words, and the names of its
  files (`api.rs`, `Mail`), so the people on the thread and when each mail was sent are known.
- **The projects you have:** the `@project` tags already in `todos.toml` (`store.rs`), the
  bindings in `projects.toml`, and the folders crew infers projects from (`projectdir.rs`).
- **The people you assign to:** the `#assignee` tags already in `todos.toml`.
- **A date reader:** `duedate.rs` (`find`), the one `/todo` uses for typed dates. It takes the
  "now" to read against, so it can read a mail as of the day it was sent.
- **Agent smith on a thread:** `a` and Tab in the reply box already hand the thread to smith,
  fenced and labelled untrusted (`smith.rs`).

## Two things the design must not break

1. **A wrong `@project` is worse than none.** A todo row can run itself: `r` hands it to an
   agent in the project's directory, and `projectdir.rs` is built on "crew must never guess one
   silently — a coding agent let loose in the wrong checkout is the worst outcome". So a
   project inferred from a stranger's mail is shown and confirmed before it lands, and a guess
   crew cannot defend is left blank.
2. **`t` sends nothing to an AI provider today**, and the privacy policy says so
   (`docs/privacy.html`: "making a todo with t (its deadline is read on your computer) send
   nothing to it"). The smart part runs on your computer. Smith reads the thread only when you
   ask, with a key, and the policy is updated in the same release that adds it.

## The shape: a todo card, as the reply box is a card

`t` opens a small card under the mail, as `r` opens the reply box. It has four lines, filled
in by crew: the **title**, **@project**, **#assignee** and **due**. Beside each guess it says
why, in a few words (`@crew · the subject's [crew]`, `#bo · "Bo, can you…"`,
`due Fri Oct 24 · "by Friday" in Ann's mail of Oct 20`). Up and Down move between the lines,
typing changes one, Enter adds the row, and Esc closes the card (twice, once you have changed
something, as the reply box does). Tab asks agent smith to read the thread and fill the card
again (phase 3).

Why a card rather than adding the row at once: the guesses are only worth having if a wrong
one is seen before it is saved (point 1 above), and the reply box already taught the keys.
One card at a time: with a reply open, `t` says so, as a click on another thread does.

## Done means

**Phase 1 — the card, with today's guesses.**
1. `t` opens the card under the open mail. It follows the reply box's rules: the mail's card
   stays taller than it, every file stays under the 200-line cap, and the foot shows the
   card's keys. `t` pressed in the list opens the thread, then the card.
2. Enter adds the row as the card shows it. Esc leaves nothing behind. The note says what was
   added ("added to /todo: … @crew #bo due Fri").
3. **The due date reads each mail as of the day it was sent.** "By Friday" in a mail of
   Monday Oct 20 is Friday Oct 24, whenever `t` is pressed. A deadline that has already passed
   is kept and shown as passed.

**Phase 2 — crew reads the thread (on your computer).**
4. **Due date:** the whole thread, newest mail first, and the subject. Phrases `/todo` already
   reads, plus "EOD", "end of day", "COB", "end of (the) week", "this/next {weekday}",
   "tomorrow morning", "in N days", and month-name dates ("Oct 24", "24 October"). A calendar
   invite (a `text/calendar` file in the thread) gives its start as the due date: it is the
   strongest due a mail can carry. A numeric date that reads two ways ("10/11") is left blank
   rather than guessed.
5. **Project:** only a project you already have (a tag in `todos.toml`, a binding in
   `projects.toml`, or a folder crew can see), matched in this order: a bracketed tag in the
   subject (`[crew]`), a repository link in the mail (`github.com/…/crew`), the sender's list
   (`List-Id`, a GitHub or issue-tracker sender), and then the project's name as a whole word
   in the subject or words. Two projects matching equally means none. A project not yet in
   the list is never made up from a mail; you type it.
6. **Assignee:** only a person on the thread (sender, To or Cc). Picked from a direct ask
   ("Bo, can you…", "@bo", "assigned to Bo", "owner: Bo", an action-item line "Bo — the
   deck") and written with the spelling of an `#assignee` tag you already use for them. Asked
   of you (you are on To, nobody else is named), the row has no `#assignee`.
7. **Title:** the ask, when a mail makes one plainly ("can you send the Q4 numbers" becomes
   "Send the Q4 numbers"); else "Reply to {sender}: {subject}" as today.
8. **Measured on a corpus.** About 25 mails written for the test (newsletters with no task,
   GitHub notifications, a meeting invite, a delegation to someone on Cc, a deadline in last
   week's mail, a two-way numeric date, an ask in the third mail of a thread), each with the
   card it should give. The test asserts: **no wrong project and no assignee off the thread**
   on any of them; no due date on the mails that state none; and the right due date on at
   least nine in ten of those that do.

**Phase 3 — agent smith, when you ask (Tab on the card).**
9. Tab sends the thread to smith (fenced and untrusted, as `a` does) and asks for the card's
   four lines as JSON, each with a reason. Crew checks every answer against the same rules as
   phase 2 before showing it: a project from your list, an assignee from the thread, and a due
   date that `duedate.rs` reads from smith's phrase. Anything else is dropped, never trusted.
   No smith provider set up: Tab says so, and the card keeps crew's own guesses.
10. The privacy policy's `t` sentence says what Tab on the card sends, with a new effective
    date, in the release that adds it.

## Not in this goal

- More than one row per `t` (a mail with three action items). The card makes one; a second
  `t` can make another.
- Creating projects, binding folders or naming people from a mail without you typing it.
- Anything written back to Gmail or Google Tasks, reminders by mail, or recurring todos.
- Learning from your corrections (this sender's mail is always `@crew`). It may be worth a
  goal of its own once the card shows which guesses get changed.

## Decided

The user took the three recommendations: **a card** before the row lands; **smith only on
Tab**; and **no `#assignee`** for a task asked of you.

## Where it stands

**Phase 1: shipped in v0.27.95.** `t` opens the card (`todocard.rs`, its keys; `todocarddraw.rs`;
`todoadd.rs`, Enter; `todoguess.rs`, crew's guesses), in the reply box's place and with its
rules: another thread waits while it is open, a paste lands on its line, a click moves to a
line. The due date reads the newest mail as of the day it was sent (`days.rs`, `sent`), with
the words it came from and the day beside it; a due line must read whole as a date.

Phase 2: not started. Phase 3: not started.
