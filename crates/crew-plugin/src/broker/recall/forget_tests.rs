use super::super::{store, Recall};

fn scratch(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("crew-forget-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn seeded(base: &std::path::Path) -> Recall {
    let mut r = Recall::open_at(base);
    r.record("edit crates/a/src/lib.rs", "edited crates/a/src/lib.rs");
    r.record_ruled("patching crates/a/src/lib.rs by hand", true);
    r.record_ruled("the serde 2.0 bump", false);
    r
}

#[test]
fn what_is_ruled_out_is_listed_newest_first_and_turns_are_not_in_it() {
    let r = seeded(&scratch("list"));
    let what: Vec<String> = r.ruled().into_iter().map(|(w, _)| w).collect();
    assert_eq!(
        what,
        vec!["the serde 2.0 bump", "patching crates/a/src/lib.rs by hand"]
    );
}

#[test]
fn a_forgotten_correction_no_longer_rides_in_front_of_a_task() {
    let mut r = seeded(&scratch("gone"));
    let before = r.context("tidy crates/a/src/lib.rs", &[]).unwrap();
    assert!(before.contains("ruled out before"), "{before}");
    assert_eq!(
        r.forget_ruled(2).as_deref(),
        Some("patching crates/a/src/lib.rs by hand")
    );
    let after = r.context("tidy crates/a/src/lib.rs", &[]).unwrap();
    assert!(!after.contains("by hand"), "{after}");
    assert!(after.contains("edited crates/a/src/lib.rs"), "{after}");
    assert_eq!(r.ruled().len(), 1);
}

#[test]
fn it_stays_forgotten_when_the_log_is_replayed() {
    let base = scratch("disk");
    let mut r = seeded(&base);
    r.forget_ruled(1).expect("the newest");
    let again = Recall::open_at(&base);
    let what: Vec<String> = again.ruled().into_iter().map(|(w, _)| w).collect();
    assert_eq!(what, vec!["patching crates/a/src/lib.rs by hand"]);
    assert!(store::path_at(&base).exists());
}

#[test]
fn a_number_past_the_list_forgets_nothing() {
    let mut r = seeded(&scratch("past"));
    assert_eq!(r.forget_ruled(0), None);
    assert_eq!(r.forget_ruled(3), None);
    assert_eq!(r.ruled().len(), 2);
}
