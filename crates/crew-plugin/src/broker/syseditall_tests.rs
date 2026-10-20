//! `sys:edit` with `"all": true` renames: every occurrence, in one call. Live
//! (2026-10-20) a rename asked for `old: "relative_luminance"`, met "appears
//! 3 times", and spent four more rounds on one line each.
use serde_json::json;

fn file(body: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("crew-editall-{}", rand_tag()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("a.rs");
    std::fs::write(&p, body).unwrap();
    p
}

fn rand_tag() -> String {
    format!("{:?}{}", std::thread::current().id(), std::process::id()).replace(['(', ')'], "")
}

const BODY: &str = "use crate::lum;\nfn a() { lum(1); }\nfn b() { lum(2); }\n";

#[test]
fn all_replaces_every_occurrence_and_says_how_many() {
    let p = file(BODY);
    let args = json!({"path": p, "old": "lum", "new": "luma", "all": true}).to_string();
    let got = super::call("edit", &args).unwrap();
    assert!(
        got.starts_with("edited a.rs in 3 places (every one)"),
        "{got}"
    );
    assert_eq!(
        std::fs::read_to_string(&p).unwrap(),
        BODY.replace("lum", "luma")
    );
    let _ = std::fs::remove_dir_all(p.parent().unwrap());
}

#[test]
fn without_all_a_repeat_is_refused_and_the_refusal_names_the_way_out() {
    let p = file(BODY);
    let args = json!({"path": p, "old": "lum", "new": "luma"}).to_string();
    let err = super::call("edit", &args).unwrap_err();
    assert!(
        err.contains("appears 3 times") && err.contains("\"all\": true"),
        "{err}"
    );
    assert_eq!(std::fs::read_to_string(&p).unwrap(), BODY, "untouched");
    let _ = std::fs::remove_dir_all(p.parent().unwrap());
}

#[test]
fn all_rides_an_edits_batch_too() {
    let p = file(BODY);
    let edits = json!([{"old": "lum", "new": "luma", "all": true}, {"old": "fn b", "new": "fn c"}]);
    let args = json!({"path": p, "edits": edits}).to_string();
    super::call("edit", &args).unwrap();
    assert_eq!(
        std::fs::read_to_string(&p).unwrap(),
        BODY.replace("lum", "luma").replace("fn b", "fn c")
    );
    let _ = std::fs::remove_dir_all(p.parent().unwrap());
}
