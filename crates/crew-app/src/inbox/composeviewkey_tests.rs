//! A key scroll after a wheel notch over the reply box scrolls the mail: the
//! wheel's position must not outlive the wheel.
use super::tests::{mail_scroll, spots, top, writing, COLS, ROWS};

#[test]
fn a_key_scroll_after_a_wheel_notch_over_the_reply_moves_the_mail() {
    key_scroll_moves_the_mail(|a| a.scroll_focused_page(false));
}

#[test]
fn shift_end_after_a_wheel_notch_over_the_reply_moves_the_mail() {
    key_scroll_moves_the_mail(|a| a.scroll_focused_end(false));
}

fn key_scroll_moves_the_mail(key: fn(&mut crate::app::CrewApp)) {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let mut p = writing();
    let (reply, _) = spots(&p);
    p.scroll_wheel(3, COLS, ROWS, Some(reply));
    p.motion.wheel_at = Some(reply);
    let (view, mail) = (top(&p), mail_scroll(&p));
    let mut pane = crate::gridsel::inbox_tests::pane();
    pane.grid = crew_term::GridSize {
        cols: COLS,
        rows: ROWS,
    };
    pane.content = crate::pane::PaneContent::Inbox(Box::new(p));
    let mut app = crate::app::CrewApp::default();
    app.panes.push(pane);
    app.focused = 0;
    key(&mut app);
    while crate::scrollglide::step_all(&mut app.panes, 16) {}
    let crate::pane::PaneContent::Inbox(p) = &app.panes[0].content else {
        unreachable!()
    };
    assert_eq!(top(p), view, "the reply's words stayed");
    assert_ne!(mail_scroll(p), mail, "the mail moved");
}
