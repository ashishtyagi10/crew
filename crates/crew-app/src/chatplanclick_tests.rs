use super::*;
use std::ops::Range;

const COLS: u16 = 60;
const ROWS: u16 = 20;

/// A pane with a plan pending, and the row its buttons sit on.
fn pending() -> (ChatPane, u16) {
    let mut p = crate::chat::tests::pane();
    p.plan_pending = true;
    let row = p
        .plan_row(COLS, ROWS)
        .expect("a pending plan has a button row");
    (p, row)
}

fn span(btn: Btn) -> Range<u16> {
    buttons(COLS, crate::glyphs::on(), None, None)
        .unwrap()
        .spans
        .into_iter()
        .find(|(b, _)| *b == btn)
        .unwrap()
        .1
}

fn click(p: &mut ChatPane, row: u16, col: u16) -> Option<&'static str> {
    assert!(
        p.plan_press_at(COLS, ROWS, row, col),
        "the press lands on a button"
    );
    p.plan_release_at(COLS, ROWS, Some((row, col)), false)
}

#[test]
fn the_button_row_is_drawn_where_the_hit_test_says_it_is() {
    let _g = crate::app::theme_test_guard();
    let _plain = crate::glyphs::force(false);
    let (p, row) = pending();
    let drawn: Vec<_> = crate::chatview::cells(&p, COLS, ROWS)
        .into_iter()
        .filter(|c| c.c == '\u{25b6}' || c.c == '\u{2717}')
        .collect();
    assert_eq!(drawn.len(), 2, "one mark per button");
    assert!(
        drawn.iter().all(|c| c.row == row),
        "both on the budgeted row"
    );
    assert_eq!(p.plan_btn_at(COLS, ROWS, row, drawn[0].col), Some(Btn::Run));
    assert_eq!(
        p.plan_btn_at(COLS, ROWS, row, drawn[1].col),
        Some(Btn::Discard)
    );
    let idle = crate::chat::tests::pane();
    assert_eq!(idle.plan_row(COLS, ROWS), None, "no plan, no row");
}

#[test]
fn clicking_run_sends_approve_and_clears_the_plan() {
    let _g = crate::app::motion_test_guard();
    let (mut p, row) = pending();
    assert_eq!(
        click(&mut p, row, span(Btn::Run).start + 1),
        Some("approve")
    );
    assert!(!p.plan_pending, "the plan is answered");
    assert!(p.press_btn.is_none(), "the press is consumed");
    assert_eq!(p.answer_plan(true), None, "nothing pending: nothing sent");
}

#[test]
fn clicking_discard_sends_reject() {
    let _g = crate::app::motion_test_guard();
    let (mut p, row) = pending();
    assert_eq!(
        click(&mut p, row, span(Btn::Discard).end - 1),
        Some("reject")
    );
    assert!(!p.plan_pending);
}

#[test]
fn a_click_between_or_off_the_buttons_sends_nothing() {
    let _g = crate::app::motion_test_guard();
    let (mut p, row) = pending();
    let gap = span(Btn::Run).end; // the one column between the badges
    assert!(
        !p.plan_press_at(COLS, ROWS, row, gap),
        "the gap is not a button"
    );
    assert!(!p.plan_press_at(COLS, ROWS, row.saturating_sub(1), span(Btn::Run).start + 1));
    assert!(p.plan_pending, "nothing answered");
    assert_eq!(p.plan_release_at(COLS, ROWS, Some((row, gap)), false), None);
}

#[test]
fn a_press_that_drags_out_of_its_button_cancels() {
    let _g = crate::app::motion_test_guard();
    let (mut p, row) = pending();
    let run = span(Btn::Run);
    assert!(p.plan_press_at(COLS, ROWS, row, run.start + 1));
    // Released over the OTHER button: neither fires.
    let over_discard = Some((row, span(Btn::Discard).start + 1));
    assert_eq!(p.plan_release_at(COLS, ROWS, over_discard, false), None);
    assert!(
        p.plan_pending && p.press_btn.is_none(),
        "cancelled, not left armed"
    );
    // A drag that moved, even one ending back on the button.
    assert!(p.plan_press_at(COLS, ROWS, row, run.start + 1));
    assert_eq!(
        p.plan_release_at(COLS, ROWS, Some((row, run.start + 1)), true),
        None
    );
    assert!(p.plan_pending);
    // Released off the pane entirely.
    assert!(p.plan_press_at(COLS, ROWS, row, run.start + 1));
    assert_eq!(p.plan_release_at(COLS, ROWS, None, false), None);
    assert!(p.plan_pending);
}

#[test]
fn a_press_starts_the_invert_flash_and_off_skips_it() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let (mut p, row) = pending();
    assert!(p.plan_press_at(COLS, ROWS, row, span(Btn::Run).start + 1));
    let now = crate::anim::now_ms();
    assert_eq!(crate::chatplanbtn::pressed_btn(&p, now), Some(Btn::Run));
    assert!(p.chrome_animating(now), "the flash asks for frames");
    assert_eq!(
        crate::chatplanbtn::pressed_btn(&p, now + PRESS_MS),
        None,
        "and settles after {PRESS_MS} ms"
    );
    crate::motion::set_level(crate::motion::MotionLevel::Off);
    assert!(p.plan_press_at(COLS, ROWS, row, span(Btn::Run).start + 1));
    assert_eq!(
        crate::chatplanbtn::pressed_btn(&p, now),
        None,
        "Off: no invert"
    );
}

#[test]
fn hovering_publishes_the_button_and_leaving_clears_it() {
    let _g = crate::app::theme_test_guard();
    let (mut p, row) = pending();
    let on_run = Some((row, span(Btn::Run).start + 1));
    assert!(p.plan_hover_at(COLS, ROWS, on_run), "entering repaints");
    assert_eq!(p.hover_btn, Some(Btn::Run));
    assert!(!p.plan_hover_at(COLS, ROWS, on_run), "staying put does not");
    assert!(p.plan_hover_at(COLS, ROWS, Some((row, span(Btn::Discard).start + 1))));
    assert_eq!(p.hover_btn, Some(Btn::Discard));
    assert!(
        p.plan_hover_at(COLS, ROWS, Some((row.saturating_sub(1), 2))),
        "leaving repaints"
    );
    assert_eq!(p.hover_btn, None);
    assert!(
        !p.plan_hover_at(COLS, ROWS, None),
        "and off the pane stays clear"
    );
}
