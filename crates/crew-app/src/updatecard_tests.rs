use super::*;

#[test]
fn downloading_shows_version_transition() {
    // A Downloading stage renders the spinner lead and a "vCUR → vNEW" detail.
    let cells = stage_cells(Stage::Downloading("9.9.9".into()));
    let line1: String = row_text(&cells, 1);
    assert!(
        line1.contains("9.9.9"),
        "detail names target, got {line1:?}"
    );
    assert!(
        line1.contains('→'),
        "detail shows transition, got {line1:?}"
    );
}

#[test]
fn done_says_a_restart_is_coming() {
    let cells = stage_cells(Stage::Done("9.9.9".into()));
    assert!(cells.iter().any(|c| c.c == '✓'), "success glyph present");
    assert!(row_text(&cells, 1).contains("restarting"));
}

/// The one place an update reports a failure used to clip the failure:
/// the message went on ONE row of a narrow nav column and the second row
/// stayed blank.
#[test]
fn a_long_note_uses_every_row_the_card_has() {
    let msg = "update failed: could not reach github.com (connection refused)";
    let cells = stage_cells(Stage::Note(msg.into()));
    let (r0, r1) = (row_text(&cells, 0), row_text(&cells, 1));
    assert!(r1.trim().len() > 4, "the second row is used: {r1:?}");
    assert!(
        r0.contains("failed") && r1.contains("github"),
        "the message continues onto it: {r0:?} / {r1:?}"
    );
    assert!(r1.ends_with('\u{2026}'), "and says there is more: {r1:?}");
    for row in [r0, r1] {
        assert!(row.chars().count() <= 24, "row overruns the card: {row:?}");
    }
}

/// A failure must not read like "already up to date".
#[test]
fn a_failed_note_wears_the_bell_colour_and_its_own_lead() {
    let _g = crate::app::theme_test_guard();
    let bad = stage_cells(Stage::Note("update failed: no such host".into()));
    let good = stage_cells(Stage::Note("already up to date (v1.2.3)".into()));
    let bell = crew_theme::theme().bell;
    assert!(bad.iter().any(|c| c.c == '!' && c.fg == bell));
    assert!(bad.iter().all(|c| c.fg == bell));
    assert!(good.iter().all(|c| c.fg != bell), "a note is not an alarm");
}

#[test]
fn narrow_card_renders_nothing() {
    let u = UpdateState::for_test(Stage::Checking);
    assert!(update_cells(&u, 3, 2).is_empty());
}

fn stage_cells(stage: Stage) -> Vec<CellView> {
    update_cells(&UpdateState::for_test(stage), 24, 2)
}

fn row_text(cells: &[CellView], row: u16) -> String {
    let mut r: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
    r.sort_by_key(|c| c.col);
    r.iter().map(|c| c.c).collect()
}
