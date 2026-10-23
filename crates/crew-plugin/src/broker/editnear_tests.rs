use super::nearest;

const RAMP: &str = "fn spread() {\n    let mut free: Vec<f32> = Vec::new();\n    for c in all() {\n        free.push(c);\n    }\n}\n";

/// The probe's miss: the line retyped without its type is offered as it is.
#[test]
fn a_retyped_line_is_offered_as_it_is_with_its_number() {
    let got = nearest(RAMP, "let mut free = Vec::new();").expect("a near line");
    assert_eq!(got, "2\u{2502}     let mut free: Vec<f32> = Vec::new();");
}

/// As many lines as `old` has, from the nearest down.
#[test]
fn a_several_line_old_gets_as_many_lines() {
    let got = nearest(RAMP, "for c in all {\n    free.push(c)\n}").expect("near");
    assert_eq!(got.lines().count(), 3);
    assert!(got.starts_with("3\u{2502}     for c in all() {"), "{got}");
}

/// Nothing alike: nothing offered, rather than a line that misleads.
#[test]
fn nothing_alike_offers_nothing() {
    assert_eq!(nearest(RAMP, "struct Nothing;"), None);
    assert_eq!(nearest(RAMP, "   \n"), None);
}

/// Through the tool: the miss names the line and keeps the way out.
#[test]
fn the_miss_says_where_the_line_is() {
    let dir = std::env::temp_dir().join(format!("crew-editnear-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("ramp.rs");
    std::fs::write(&p, RAMP).unwrap();
    let args = serde_json::json!({"path": p, "old": "let mut free = Vec::new();", "new": "x"});
    let err = crate::broker::systools::call("edit", &args.to_string()).unwrap_err();
    assert!(err.contains("the nearest is line 2"), "{err}");
    assert!(
        err.contains("let mut free: Vec<f32> = Vec::new();"),
        "{err}"
    );
    assert_eq!(std::fs::read_to_string(&p).unwrap(), RAMP, "untouched");
    let _ = std::fs::remove_dir_all(&dir);
}
