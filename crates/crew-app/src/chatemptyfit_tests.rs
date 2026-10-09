//! The empty state on a short tile drops its example asks whole rather than
//! cutting them mid-clause. Split from `chatempty_tests` (at the line cap).
use super::*;

#[test]
fn a_short_tile_drops_the_examples_whole() {
    let _g = crate::app::theme_test_guard();
    let agents = vec![AgentInfo {
        name: "smith".into(),
        role: "lead".into(),
        model: String::new(),
    }];
    let cols = 40;
    let full = block(cols, true, &agents, 99);
    let blank = full.iter().rposition(|r| r.0.is_empty()).expect("a spacer");
    // One row short: the spacer goes and every word stays.
    let rows = fit(full.clone(), full.len() - 1, cols);
    assert_eq!(rows.len(), full.len() - 1, "{rows:?}");
    // Room for the hint and a row: the examples go whole, the hint stays.
    let rows = fit(full.clone(), blank + 1, cols);
    assert_eq!(rows.len(), blank, "{rows:?}");
    assert!(rows.iter().all(|r| !r.0.ends_with('\u{2026}')), "{rows:?}");
    assert!(rows.iter().all(|r| !r.0.contains("Try")), "{rows:?}");
}
