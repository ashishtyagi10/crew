//! An audit trail is only worth having if it cannot quietly lose things, so these tests are
//! about durability and about the failure modes that would silently erase history.
use super::*;
use crate::broker::approval::{Outcome, Requester};
use crate::broker::tier::Tier;

fn tmp(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("crew-ledger-{}-{tag}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}

fn rec(tool: &str) -> Record {
    Record::decided(tool, Tier::Irreversible, &Requester::LocalPane, "allow", "")
        .with_outcome("ran")
}

#[test]
fn a_record_survives_a_round_trip_with_every_field() {
    let l = Ledger::at(tmp("rt"));
    let r = Record::decided(
        "gmail:send",
        Tier::Irreversible,
        &Requester::Channel("telegram:me".into()),
        "ask",
        "a1",
    )
    .with_outcome("granted");
    l.append(&r).unwrap();
    let (back, bad) = l.read();
    assert_eq!(bad, 0);
    assert_eq!(back, vec![r]);
    assert_eq!(back[0].requester, "channel:telegram:me");
    assert_eq!(back[0].tier, "irreversible");
    let _ = std::fs::remove_file(l.path());
}

/// Append means append. If a write ever truncated, the ledger would lose exactly the history
/// someone is trying to audit.
#[test]
fn writing_never_replaces_what_is_already_there() {
    let l = Ledger::at(tmp("append"));
    for t in ["a", "b", "c"] {
        l.append(&rec(t)).unwrap();
    }
    let (back, _) = l.read();
    assert_eq!(back.len(), 3);
    assert_eq!(
        back.iter().map(|r| r.tool.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c"],
        "oldest first, in the order they happened"
    );
    let _ = std::fs::remove_file(l.path());
}

/// The whole point of a file rather than memory: a new process reads what the old one wrote.
#[test]
fn the_history_outlives_the_process_that_wrote_it() {
    let path = tmp("restart");
    Ledger::at(&path).append(&rec("before")).unwrap();
    // A completely fresh handle, as a restarted daemon would open.
    let (back, _) = Ledger::at(&path).read();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].tool, "before");
    let _ = std::fs::remove_file(&path);
}

/// A crash mid-append leaves half a line. That must cost one record, not the whole history —
/// the naive implementation returns Err for the file and the audit trail reads as empty.
#[test]
fn a_torn_final_line_does_not_hide_the_records_before_it() {
    let path = tmp("torn");
    let l = Ledger::at(&path);
    l.append(&rec("first")).unwrap();
    l.append(&rec("second")).unwrap();
    // Simulate the crash: a partial JSON object with no newline.
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    f.write_all(br#"{"ts_ms":1,"tool":"half"#).unwrap();
    drop(f);
    let (back, bad) = l.read();
    assert_eq!(back.len(), 2, "both complete records are still readable");
    assert_eq!(bad, 1, "and the torn one is counted, not hidden");
    let _ = std::fs::remove_file(&path);
}

/// Reading a ledger that does not exist yet is an empty history, not an error to handle at
/// every call site.
#[test]
fn an_absent_ledger_reads_as_empty() {
    let (back, bad) = Ledger::at(tmp("absent")).read();
    assert!(back.is_empty());
    assert_eq!(bad, 0);
}

/// The labels are what a human reads months later, so they are pinned.
#[test]
fn requester_and_outcome_labels_are_stable() {
    assert_eq!(requester_label(&Requester::LocalPane), "pane");
    assert_eq!(
        requester_label(&Requester::Trigger("nightly".into())),
        "trigger:nightly"
    );
    assert_eq!(outcome_label(Outcome::Granted), "granted");
    assert_eq!(outcome_label(Outcome::Denied), "denied");
    assert_eq!(
        outcome_label(Outcome::TimedOut),
        "timed_out",
        "a lapse must not read as a refusal"
    );
}

/// Two writers (the daemon and a broker child both logging) must interleave whole lines rather
/// than overwrite each other at a stale offset.
#[test]
fn two_writers_on_one_file_do_not_lose_records() {
    let path = tmp("concurrent");
    let a = Ledger::at(&path);
    let b = Ledger::at(&path);
    for i in 0..25 {
        a.append(&rec(&format!("a{i}"))).unwrap();
        b.append(&rec(&format!("b{i}"))).unwrap();
    }
    let (back, bad) = a.read();
    assert_eq!(bad, 0, "no line was torn by the other writer");
    assert_eq!(back.len(), 50, "every record is present");
    let _ = std::fs::remove_file(&path);
}

/// The ledger only ever grows. A read must never be able to shrink it.
#[test]
fn reading_does_not_modify_the_ledger() {
    let path = tmp("readonly");
    let l = Ledger::at(&path);
    l.append(&rec("x")).unwrap();
    let before = std::fs::metadata(&path).unwrap().len();
    let _ = l.read();
    let _ = l.read();
    assert_eq!(std::fs::metadata(&path).unwrap().len(), before);
    let _ = std::fs::remove_file(&path);
}
