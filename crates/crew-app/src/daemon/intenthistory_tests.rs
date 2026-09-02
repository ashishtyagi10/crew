use super::*;
use crate::daemon::intent::Repeat;

const DAY: u64 = 86_400_000;

fn list(tag: &str) -> Watchlist {
    let p = std::env::temp_dir().join(format!("crew-history-{}-{tag}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&p);
    Watchlist::at(p)
}

/// Two firings, one of them after three missed days, fold to a count, a last time and a
/// missed total; an add and a cancel are not firings and count for nothing.
#[test]
fn firings_fold_and_nothing_else_counts() {
    let w = list("fold");
    let it = w
        .add(
            "briefing",
            "telegram:42",
            DAY,
            Repeat::Every { secs: 86_400 },
            0,
        )
        .unwrap();
    let once = w.add("noon", "", DAY, Repeat::Once, 0).unwrap();
    assert!(w.history().is_empty(), "nothing has fired");
    w.record_fire(&it, DAY).unwrap();
    // The laptop was shut for three days, then it fired again.
    let rolled = w.live().into_iter().find(|i| i.id == it.id).unwrap();
    assert_eq!(w.record_fire(&rolled, 5 * DAY).unwrap(), 3);
    w.cancel(&once.id, 5 * DAY).unwrap();
    let h = w.history();
    assert_eq!(h.len(), 1, "{h:?}");
    assert_eq!(
        h[&it.id],
        Fired {
            count: 2,
            last_ms: 5 * DAY,
            missed: 3
        }
    );
    assert!(!h.contains_key(&once.id), "a cancel is not a firing");
}

#[test]
fn the_note_counts_and_says_missed_only_when_there_is_one() {
    let now = 10 * DAY;
    let f = Fired {
        count: 1,
        last_ms: now - 2 * 3_600_000,
        missed: 0,
    };
    assert_eq!(note(&f, now), "fired once \u{b7} last 2h ago");
    let f = Fired {
        count: 40,
        last_ms: now - DAY,
        missed: 3,
    };
    assert_eq!(
        note(&f, now),
        "fired 40\u{d7} \u{b7} last 1d ago \u{b7} 3 missed"
    );
}
