//! The watchlist is the only reason a standing intent survives a restart, so these tests are
//! about the fold — what a log of adds, fires and cancels says is still standing — and about the
//! ways a log can be damaged without taking the rest of the watchlist with it.
use super::*;

const DAY: u64 = 86_400_000;

fn tmp(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("crew-watchlist-{}-{tag}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}

fn list(tag: &str) -> Watchlist {
    Watchlist::at(tmp(tag))
}

#[test]
fn what_was_added_is_what_comes_back_after_a_restart() {
    let w = list("add");
    let it = w
        .add("the forecast", "telegram:42", 5_000, Repeat::Once, 1_000)
        .unwrap();
    assert_eq!(it.id, "w1");
    // A second Watchlist over the same file is exactly the restart case.
    let after = Watchlist::at(w.path()).live();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].text, "the forecast");
    assert_eq!(after[0].to, "telegram:42");
    assert_eq!(after[0].fire_ms, 5_000);
    assert_eq!(after[0].repeat, Repeat::Once);
    assert_eq!(after[0].created_ms, 1_000);
}

#[test]
fn a_one_shot_that_fired_is_no_longer_standing() {
    let w = list("oneshot");
    let it = w
        .add("wake me", "telegram:42", 5_000, Repeat::Once, 0)
        .unwrap();
    assert_eq!(w.record_fire(&it, 5_000).unwrap(), 0);
    assert!(w.live().is_empty(), "a fired one-shot is done");
}

#[test]
fn a_repeat_that_fired_comes_back_at_its_next_time() {
    let w = list("repeat");
    let it = w
        .add(
            "briefing",
            "telegram:42",
            DAY,
            Repeat::Every { secs: 86_400 },
            0,
        )
        .unwrap();
    let skipped = w.record_fire(&it, DAY).unwrap();
    assert_eq!(skipped, 0);
    let live = w.live();
    assert_eq!(live.len(), 1, "a repeat stays on the watchlist");
    assert_eq!(live[0].fire_ms, 2 * DAY, "and moved on by one day");
}

#[test]
fn a_repeat_fired_late_reports_what_it_skipped_and_lands_in_the_future() {
    let w = list("late");
    let it = w
        .add(
            "briefing",
            "telegram:42",
            0,
            Repeat::Every { secs: 86_400 },
            0,
        )
        .unwrap();
    // The laptop was shut for three days.
    let skipped = w.record_fire(&it, 3 * DAY).unwrap();
    assert_eq!(skipped, 3);
    assert_eq!(w.live()[0].fire_ms, 4 * DAY);
}

#[test]
fn a_cancel_is_a_tombstone_and_the_log_still_holds_the_add() {
    let w = list("cancel");
    let it = w.add("noon", "telegram:42", DAY, Repeat::Once, 0).unwrap();
    assert!(w.cancel(&it.id, 10).unwrap());
    assert!(w.live().is_empty());
    let (entries, _) = w.entries();
    assert_eq!(entries.len(), 2, "the add is still there to audit");
    assert!(matches!(entries[0], Entry::Added { .. }));
    assert!(matches!(entries[1], Entry::Cancelled { .. }));
}

#[test]
fn cancelling_nothing_says_so_and_writes_nothing() {
    let w = list("nocancel");
    w.add("noon", "telegram:42", DAY, Repeat::Once, 0).unwrap();
    assert!(!w.cancel("w9", 10).unwrap(), "no such intent");
    assert_eq!(w.entries().0.len(), 1, "a miss appends no tombstone");
}

#[test]
fn ids_are_never_reused_once_an_intent_is_gone() {
    // `w3` in an old log line has to keep meaning the thing it meant. Handing the id to a new
    // alarm would make the history read as a lie.
    let w = list("ids");
    let a = w.add("one", "telegram:42", DAY, Repeat::Once, 0).unwrap();
    let b = w.add("two", "telegram:42", DAY, Repeat::Once, 0).unwrap();
    assert_eq!((a.id.as_str(), b.id.as_str()), ("w1", "w2"));
    w.cancel("w2", 1).unwrap();
    assert_eq!(w.next_id(), "w3", "the cancelled id is spent");
    let c = w.add("three", "telegram:42", DAY, Repeat::Once, 0).unwrap();
    assert_eq!(c.id, "w3");
}

#[test]
fn the_listing_is_soonest_first() {
    let w = list("order");
    w.add("later", "telegram:42", 3 * DAY, Repeat::Once, 0)
        .unwrap();
    w.add("sooner", "telegram:42", DAY, Repeat::Once, 0)
        .unwrap();
    w.add("middle", "telegram:42", 2 * DAY, Repeat::Once, 0)
        .unwrap();
    let live = w.live();
    let texts: Vec<&str> = live.iter().map(|i| i.text.as_str()).collect();
    assert_eq!(texts, ["sooner", "middle", "later"]);
}

#[test]
fn a_truncated_last_line_costs_one_entry_not_the_watchlist() {
    let w = list("torn");
    w.add("keep me", "telegram:42", DAY, Repeat::Once, 0)
        .unwrap();
    // A crash mid-append: half a JSON object with no newline after it.
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(w.path())
        .unwrap();
    f.write_all(b"{\"op\":\"Added\",\"intent\":{\"id\":\"w2\"")
        .unwrap();
    drop(f);
    let (entries, bad) = w.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(bad, 1, "the torn line is counted, not swallowed");
    assert_eq!(w.live().len(), 1, "the good intent is still standing");
}

#[test]
fn an_empty_or_missing_watchlist_is_an_empty_one_not_an_error() {
    let w = Watchlist::at(std::env::temp_dir().join("crew-watchlist-does-not-exist.jsonl"));
    assert!(w.live().is_empty());
    assert_eq!(w.next_id(), "w1");
}

#[test]
fn a_fire_recorded_for_an_id_that_is_gone_changes_nothing() {
    // The daemon and a CLI cancel can race: a firing written just after a tombstone must not
    // resurrect the intent it fired.
    let w = list("race");
    let it = w
        .add(
            "briefing",
            "telegram:42",
            0,
            Repeat::Every { secs: 3_600 },
            0,
        )
        .unwrap();
    w.cancel(&it.id, 1).unwrap();
    w.record_fire(&it, 2).unwrap();
    assert!(w.live().is_empty(), "a cancelled intent stays cancelled");
}
