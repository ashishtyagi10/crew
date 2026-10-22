# Goal — a meeting invite in the inbox becomes a calendar entry

**Status: IN PROGRESS.** Set 2026-10-22; the questions answered the same day (the user: "go with
your recommendations for the calendar goal"): the system calendar app first, then Google
Calendar; answers, each pressed twice; and `m` for maybe only on the card.

**Set:** 2026-10-22 by the user: *"write a goal to create calendar entries for the meeting
invite is that possible ?"*

**Yes.** The inbox already has the invite in hand, and crew already sends mail. What it lacks
is a reader for the whole invite and somewhere to write the entry.

## Where it stands today

- **crew has the invite.** A mail's `text/calendar` part is kept as written
  (`api.rs`, `Mail::calendar`). This is the part Google Calendar, Outlook, Apple Calendar and
  Zoom all send.
- **Only the todo card reads it,** and only for one line: `DTSTART` becomes the card's due date,
  unless the invite says `METHOD:CANCEL` (`tododue.rs`, `invite`). Nothing shows the invite's
  title, end, place, organizer or who else is asked, and nothing can answer it.
- **Times in a named zone are read as written.** `DTSTART;TZID=America/New_York:…T100000` is
  taken as 10:00 here. That is close enough for a due date and wrong for a calendar entry.
  crew has `chrono` and no timezone database.
- **crew sends mail** under the one permission it asks Google for (`gmail.modify`,
  `composesend.rs`, `outgoing.rs`). An RSVP is a mail (an iCalendar `METHOD:REPLY`), so
  answering an invite needs no new permission.
- **Writing to Google Calendar would need one.** The Calendar API's `calendar.events` scope is
  "sensitive", not "restricted" like Gmail's read scopes. You would turn on the Calendar API in
  the Cloud project you already made, then sign in once more to grant the new scope.
- **Gmail often puts an invite on your calendar by itself** (Google Calendar's "add
  invitations" setting). For those invites the real job is the answer, not the entry.

## Things the design must not break

1. **Nothing leaves the computer until you press a key for it.** Reading the invite happens on
   your computer. Adding it, or answering, happens only on a key, and the note says which
   calendar or which people it reached.
2. **An answer is a mail to other people.** Accept, decline and maybe tell the organizer, so
   each is pressed twice (the first press names who will be told), like Esc on a reply with
   words in it.
3. **A wrong time is worse than no entry.** An invite whose time crew cannot place in your
   zone shows the time as written, says so, and offers no add.
4. **The privacy policy changes in the release that changes what crew sends.** It currently
   says "Crew asks for one scope, `gmail.modify`."

## The shape: an invite card at the top of the mail

A mail with an invite opens with a small card above its words, drawn in the reader's own
style:

```
 invite · Q4 planning                                       ✓ on your calendar
 Thu Oct 30 · 10:00–10:45 your time (7:00–7:45 in Los Angeles)
 Zoom: zoom.us/j/…   organizer: Ann Lee   you + 6 others
 c add to calendar   y accept   n decline   m maybe
```

Its keys are `c`, `y`, `n` and `m`. The inbox already uses `m` (mark read or unread), so the
card takes it only while its line is selected; that is decided in phase 1. An update to the
invite (a higher `SEQUENCE`) replaces the card's lines, and a cancellation shows "cancelled"
with a remove key.

## Done means

**Phase 1: crew reads the whole invite (on your computer).**
1. An iCalendar reader for `VEVENT` covering `SUMMARY`, `DTSTART`/`DTEND` (a date, UTC, or a
   named zone), `DURATION`, `LOCATION`, the meeting link (`X-GOOGLE-CONFERENCE`, a Zoom or
   Teams URL in `DESCRIPTION`), `ORGANIZER`, the `ATTENDEE` list with each `PARTSTAT`, `UID`,
   `SEQUENCE`, `RRULE` (shown in words: "every Tuesday") and `METHOD`. It handles folded
   lines and escaped text.
2. **Times land in your zone.** IANA zone names go through a timezone database
   (`chrono-tz`, or `jiff`). Outlook's Windows zone names ("Pacific Standard Time") are mapped
   by CLDR's table. Any other name falls back to the `VTIMEZONE` block the invite carries. If
   none of those works, the time shows as written, marked, with no add (rule 3). The todo
   card's due date reads its time through the same reader.
3. The invite card draws at the top of the mail, as above, and fits the reader's width rules
   and the 200-line cap.
4. **Measured on a corpus** of about 20 invites written for the test. It covers Google, Outlook
   with a Windows zone name, Apple, Zoom, Teams, an all-day event, a weekly repeat, an update,
   a cancellation, an airline's `METHOD:PUBLISH` booking, and a folded multi-line description.
   Every time must land right in two different local zones.

**Phase 2: add to calendar, without a new permission.**
5. `c` writes the invite to a temporary `.ics` file and opens it with the system's calendar
   app: Calendar on a Mac, Outlook or Calendar on Windows, `xdg-open` on Linux. That app shows
   its own add-or-answer page and writes to whichever account you keep there (iCloud, Google,
   Exchange). The note says "opened in Calendar". Nothing goes to Google from crew.

**Phase 3: your Google Calendar, and answers the organizer sees.**
6. The setup page gets a sixth step ("turn on the Calendar API"). Once signed in again with
   `calendar.events`, `c` adds the event straight to your primary Google Calendar with
   `events.import`. It keeps the invite's `UID`, so an invite Gmail already added is never
   added twice. The card's corner says "✓ on your calendar" when the event is already there,
   read with `events.list` by `iCalUID`.
7. **`y`, `n` and `m` answer.** For an event already on your Google Calendar, the answer
   sets your response with `sendUpdates=all`, so the organizer is told as Google tells them.
   For any other invite, crew sends a `METHOD:REPLY` mail to the organizer through the send
   path it already has, as Outlook and Apple Mail do. Either way the first press names who
   will be told (rule 2), and the card shows your answer afterwards.
8. Signed in without the Calendar scope, `c` and the answer keys keep working as in phase 2,
   and the note says what one more sign-in would add.
9. The privacy policy and terms name the new scope and what it is used for, with a new
   effective date, in the release that adds it.

## Not in this goal

- Making an event from words in a mail ("let's meet Tuesday at 3"). That is agent smith's kind
  of guess, and a fit for a goal of its own once this card exists.
- A calendar pane: seeing your day or week, moving or editing events.
- Proposing a new time, forwarding an invite, or sending invites of your own.
- Calendars other than Google's by API. Phase 2's `.ics` route reaches them through the
  system's calendar app.

## Decided (2026-10-22, the user took all three recommendations)

1. **Which calendar do entries go to?**
   - **(a) Google Calendar through its API** (phase 3). It needs the Calendar API turned on and
     one more sign-in. The entry shows on your phone at once, and the card can say "already
     on your calendar".
   - **(b) The system calendar app through `.ics`** (phase 2). It needs no new permission, and
     you confirm the add in that app.
   - *Recommended:* build (b) first, since it ships with no new permission, then (a), keeping
     (b) as the route whenever the Calendar scope has not been granted.
2. **Should crew answer invites, or only add them?** Answering mails other people.
   *Recommended:* yes, with the press-twice rule above.
3. **Which key means "maybe"?** The inbox already uses `m` for mark read or unread, and `t`
   ("tentative") opens the todo card. *Recommended:* `m` means maybe only while the invite
   card is selected, and keeps its old meaning everywhere else.

## Where it stands

**Phase 1: built (not yet released).** `ics.rs` reads the invite (`icsline.rs` its lines and
values; the series, not a moved occurrence; an alarm's lines kept apart; the join link from
`X-GOOGLE-CONFERENCE`, else the first Zoom, Teams, Meet, Webex, Whereby, GoTo or Chime link in
the place or the words). `icstz.rs` places a time: IANA names through `chrono-tz` (a
Thunderbird name by its tail), Windows names through CLDR's table (`windowszones.txt`), then
the invite's own `VTIMEZONE` rules; a skipped hour reads an hour on, and a zone nothing places
stays as written. `icswhen.rs` says it (`Tue Nov 3 · 12:00–12:30 · 09:00–09:30 in Los
Angeles`, `every week on Tuesday, until Dec 15`). `invitecard.rs` draws the card under the
sender. The todo card's due date now reads through the same reader, so a named zone's time is
right there too. The corpus is 20 invites (`invite_corpus.toml`), every time right in New York
and in Kolkata.
