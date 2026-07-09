use super::*;

#[test]
fn review_prompt_carries_the_diff_and_the_reviewer_contract() {
    let p = review_prompt("+ let x = unwrap();");
    assert!(p.contains("+ let x = unwrap();"), "diff included");
    let lower = p.to_lowercase();
    for word in ["blocker", "warn", "nit"] {
        assert!(
            lower.contains(word),
            "severity vocabulary names {word}: {p}"
        );
    }
    assert!(lower.contains("file"), "asks findings to name the file");
    assert!(lower.contains("verdict"), "asks for a closing verdict");
    assert!(
        lower.contains("clean") || lower.contains("no findings"),
        "tells the reviewer what to say for a clean diff"
    );
}

#[test]
fn review_prompt_orders_findings_by_severity() {
    let lower = review_prompt("x").to_lowercase();
    let (b, w, n) = (
        lower.find("blocker").unwrap(),
        lower.find("warn").unwrap(),
        lower.find("nit").unwrap(),
    );
    assert!(b < w && w < n, "severities are introduced worst-first");
}
