//! The nav LOG's lines by weight and by cut.
use super::*;

fn info(s: &str) -> LogEntry {
    LogEntry {
        level: LogLevel::Info,
        text: s.to_string(),
    }
}

/// An error is bold as well as in the bell colour — on amber the bell and
/// the muted ink are one hue 1.43:1 apart — and a line too long for the
/// rail is cut between words, not mid-word.
#[test]
fn an_error_is_bold_and_a_long_line_ends_on_a_word() {
    let _g = crate::app::theme_test_guard();
    let entries = [
        info("restored four panes from the last session"),
        LogEntry {
            level: LogLevel::Error,
            text: "build failed".to_string(),
        },
    ];
    let cells = log_cells(&entries, 24, 5, 0);
    let row = |r: u16| -> Vec<&CellView> {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == r).collect();
        v.sort_by_key(|c| c.col);
        v
    };
    assert!(row(2)
        .iter()
        .filter(|c| c.c.is_alphabetic())
        .all(|c| c.bold));
    assert!(row(1).iter().all(|c| !c.bold), "info stays regular");
    let text: String = row(1).iter().map(|c| c.c).collect();
    let body = text.trim_end().trim_end_matches('\u{2026}');
    assert!(
        "restored four panes from the last session"
            .split(' ')
            .any(|w| body.ends_with(w)),
        "cut mid-word: {text:?}"
    );
}
