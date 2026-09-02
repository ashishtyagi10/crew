use super::{hud_text, shown, task_row};
use crate::chatwidth::str_w;

#[test]
fn the_hud_drops_its_cost_before_it_cuts_a_number() {
    let wide = hud_text(1, 3, 1, 42_000, 60);
    assert_eq!(wide, " live:1 done:3 failed:1 cost:$0.0420");
    // 32 columns held `cost:$0.` before — a price cut mid-number.
    let mid = hud_text(1, 3, 1, 42_000, 32);
    assert_eq!(mid, " live:1 done:3 failed:1");
    let tight = hud_text(1, 3, 1, 42_000, 12);
    assert_eq!(tight, " \u{25cf}1 \u{2713}3 \u{2717}1");
    for cols in [12u16, 16, 24, 32, 40, 60] {
        assert!(str_w(&hud_text(12, 345, 6, 1_234_567, cols)) <= cols as usize);
    }
    // Narrower than the glyph form: cut, but with the mark that says so.
    assert!(hud_text(12, 345, 6, 0, 6).ends_with('\u{2026}'));
}

#[test]
fn a_task_row_marks_its_cut_and_loses_the_tail_before_it_loses_sense() {
    let (title, tail) = task_row("bench the atlas", "error: atlas overflow", 60);
    assert_eq!(title, "bench the atlas");
    assert_eq!(tail, " \u{2014} error: atlas overflow");
    // 32 columns: title whole, tail cut with the mark (was `error: atla`).
    let (title, tail) = task_row("bench the atlas", "error: atlas overflow", 32);
    assert_eq!(title, "bench the atlas");
    assert_eq!(tail, " \u{2014} error: atl\u{2026}");
    assert_eq!(str_w(&title) + str_w(&tail), 29);
    // 22 columns: four columns left after the title — no word fits, so no tail.
    let (title, tail) = task_row("bench the atlas", "error: atlas overflow", 22);
    assert_eq!(title, "bench the atlas");
    assert_eq!(tail, "");
    // 12 columns: the title itself is cut, and says so.
    let (title, tail) = task_row("bench the atlas", "error", 12);
    assert_eq!(title, "bench th\u{2026}");
    assert_eq!(tail, "");
}

#[test]
fn shown_keeps_a_row_for_the_overflow_note_only_when_needed() {
    assert_eq!(shown(6, 22), 6, "all fit: every task named");
    assert_eq!(shown(6, 7), 6, "exactly fits under the HUD");
    assert_eq!(
        shown(14, 10),
        8,
        "9 rows under the HUD, one kept for the note"
    );
    assert_eq!(shown(14, 2), 0, "one row: only the note");
    assert_eq!(shown(14, 0), 0);
}
