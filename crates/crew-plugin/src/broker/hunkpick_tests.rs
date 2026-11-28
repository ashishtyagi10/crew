use super::*;
use serde_json::json;

const BODY: &str = "one\ntwo\nthree\nfour\nfive\nsix\nseven\n";

/// A file of its own in a directory of its own (tests run side by side).
fn file(body: &str) -> String {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("crew-hunkpick-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.txt");
    std::fs::write(&path, body).unwrap();
    path.display().to_string()
}

/// Three separate changes: line 2, line 4, line 7.
fn three(path: &str) -> String {
    json!({"path": path, "edits": [
        {"old": "two\n", "new": "TWO\n"},
        {"old": "four\n", "new": "FOUR\nand a half\n"},
        {"old": "seven\n", "new": ""},
    ]})
    .to_string()
}

#[test]
fn the_question_shows_each_change_as_a_numbered_hunk() {
    let path = file(BODY);
    let p = preview("sys:edit", &three(&path)).unwrap();
    let (lead, body) = p.more();
    assert_eq!(lead, "3 changes \u{b7} ");
    assert_eq!(
        body,
        "\n[1] @@ 2 @@\n- two\n+ TWO\n[2] @@ 4 @@\n- four\n+ FOUR\n+ and a half\n[3] @@ 7 @@\n- seven\n\
         Enter all \u{b7} Esc none \u{b7} type 1 3 to keep those"
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        BODY,
        "a preview writes nothing"
    );
}

#[test]
fn a_write_of_an_existing_file_is_diffed_and_a_new_file_is_not() {
    let path = file(BODY);
    let args = json!({"path": path, "content": "one\n2\nthree\nfour\nfive\nsix\nseven\n"});
    let p = preview("sys:write_file", &args.to_string()).unwrap();
    assert_eq!(p.more().0, "1 change \u{b7} ");
    let fresh = json!({"path": format!("{path}.new"), "content": "a\nb\n"}).to_string();
    let p = preview("sys:write_file", &fresh).unwrap();
    assert_eq!(p.more().0, "new file, 2 lines \u{b7} ");
    assert!(p.hunks.is_empty());
}

#[test]
fn only_edits_and_writes_have_a_preview() {
    assert!(preview("sys:run", &json!({"cmd": "ls"}).to_string()).is_none());
    let missing = json!({"path": "/no/such/file", "old": "a", "new": "b"}).to_string();
    assert!(preview("sys:edit", &missing).is_none());
}

#[test]
fn keeping_hunks_one_and_three_writes_exactly_those() {
    let path = file(BODY);
    let args = three(&path);
    let p = choose(preview("sys:edit", &args), Some(vec![1, 3]))
        .unwrap()
        .unwrap();
    let said = p.run().unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "one\nTWO\nthree\nfour\nfive\nsix\n"
    );
    assert!(
        said.contains("APPLIED: hunk 1 (line 2, -1 +1), hunk 3 (line 7, -1 +0)"),
        "{said}"
    );
    assert!(
        said.contains("REJECTED by the user: hunk 2 (line 4, -1 +2)"),
        "{said}"
    );
    assert!(said.contains("NOT in the file"), "{said}");
}

#[test]
fn a_file_changed_since_the_question_is_refused_and_left_alone() {
    let path = file(BODY);
    let args = three(&path);
    let p = choose(preview("sys:edit", &args), Some(vec![2]))
        .unwrap()
        .unwrap();
    let moved = BODY.replace("five", "5ive");
    std::fs::write(&path, &moved).unwrap();
    let why = p.run().unwrap_err();
    assert!(
        why.contains("changed after the question") && why.contains("NOTHING"),
        "{why}"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), moved);
}

#[test]
fn naming_every_hunk_or_none_runs_the_call_whole() {
    let path = file(BODY);
    let args = three(&path);
    assert!(choose(preview("sys:edit", &args), None).unwrap().is_none());
    assert!(choose(preview("sys:edit", &args), Some(vec![3, 1, 2]))
        .unwrap()
        .is_none());
}

#[test]
fn hunks_that_do_not_exist_or_have_nothing_to_pick_from_refuse() {
    let path = file(BODY);
    let args = three(&path);
    let why = choose(preview("sys:edit", &args), Some(vec![1, 4]))
        .err()
        .unwrap();
    assert!(
        why.contains("hunk 4 does not exist") && why.contains("has 3"),
        "{why}"
    );
    assert!(choose(preview("sys:edit", &args), Some(vec![0])).is_err());
    assert!(choose(None, Some(vec![1])).is_err());
    let fresh = json!({"path": format!("{path}.new"), "content": "a\n"}).to_string();
    assert!(choose(preview("sys:write_file", &fresh), Some(vec![1])).is_err());
}

#[test]
fn the_hunks_arrive_before_the_answer_and_are_taken_once() {
    deliver("hp-1", true, Some(vec![2]));
    assert_eq!(super::super::approval::take_answer("hp-1"), Some(true));
    assert_eq!(take("hp-1"), Some(vec![2]));
    assert_eq!(take("hp-1"), None);
    deliver("hp-2", true, None);
    assert_eq!(take("hp-2"), None);
    let _ = super::super::approval::take_answer("hp-2");
}

/// An old host sends no `hunks`; a new broker must still read it as "the whole change". And a
/// new host answering a plain yes must not add a field an old broker's strict reader could trip on.
#[test]
fn an_approve_without_hunks_still_parses_and_is_written_without_them() {
    use crate::PluginCommand;
    let old = r#"{"type":"approve","id":"a1","granted":true}"#;
    assert!(matches!(
        serde_json::from_str::<PluginCommand>(old),
        Ok(PluginCommand::Approve { hunks: None, .. })
    ));
    let plain = PluginCommand::Approve {
        id: "a1".into(),
        granted: true,
        hunks: None,
    };
    assert_eq!(serde_json::to_string(&plain).unwrap(), old);
    let some = r#"{"type":"approve","id":"a1","granted":true,"hunks":[1,3]}"#;
    assert!(matches!(
        serde_json::from_str::<PluginCommand>(some),
        Ok(PluginCommand::Approve { hunks: Some(h), .. }) if h == [1, 3]
    ));
}
