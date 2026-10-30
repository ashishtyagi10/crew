use super::{headline, totals};

#[test]
fn the_head_says_how_much_changed_in_lines() {
    let line = "1 file changed: ~ crates/crew-theme/src/lib.rs \u{2014} /diff shows them";
    assert_eq!(
        headline(1, None, line, (1, 1)),
        "task #1: 1 file changed (+1 \u{2212}1): ~ crates/crew-theme/src/lib.rs \u{2014} /diff shows them"
    );
    assert_eq!(
        headline(4, None, "2 files changed: + a.rs", (0, 0)),
        "task #4: 2 files changed: + a.rs"
    );
}

#[test]
fn totals_count_lines_not_file_headers() {
    let patch = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n-old\n+new\n+more\n keep\n";
    assert_eq!(totals(patch), (2, 1));
}

/// A stopped or failed turn says so at the head of its one card.
#[test]
fn a_stop_or_a_failure_heads_its_receipt() {
    let line = "1 file changed: ~ lib.rs";
    assert_eq!(
        headline(3, Some("stopped"), line, (2, 1)),
        "task #3 stopped \u{b7} 1 file changed (+2 \u{2212}1): ~ lib.rs"
    );
    assert_eq!(
        headline(3, Some("model refused"), line, (0, 0)),
        "task #3 failed \u{b7} 1 file changed: ~ lib.rs"
    );
}
