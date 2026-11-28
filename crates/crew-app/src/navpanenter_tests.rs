//! A new card starts at the page and arrives; first sight and Off stay still.
use super::*;
use crate::navpanememo::{live, observe_rows};
use crate::panestate::EntryState;
use MotionLevel::{Full, Off, Subtle};

fn card(now: u64, born: u64, rows_now: u64) -> Fx {
    // `born` is the pane's creation time; `rows_now` the pass that sees it.
    let _ = now;
    observe_rows(
        vec![(born, 0, EntryState::Idle, "w")].into_iter(),
        rows_now,
        true,
    )
    .unwrap()[0]
}

fn cell(col: u16, row: u16) -> CellView {
    CellView {
        col,
        row,
        fg: (200, 100, 50),
        ..Default::default()
    }
}

#[test]
fn a_card_born_after_the_section_was_seen_grows_in() {
    let _g = crate::app::theme_test_guard();
    crate::motion::set_level(Full);
    observe_rows(std::iter::empty(), 90_000, true);
    let fx = card(0, 90_050, 90_100);
    assert_eq!(fx.born_at, Some(90_100));
    let page = crew_theme::theme().page_bg;
    let start = apply(vec![cell(5, 3)], &fx, 3, 90_100);
    assert_eq!(start[0].fg, page, "starts at the page colour");
    let mid = apply(vec![cell(5, 3)], &fx, 3, 90_150)[0].fg;
    assert!(mid != page && mid != (200, 100, 50));
    let end = apply(vec![cell(5, 3)], &fx, 3, 90_100 + ENTER_MS);
    assert_eq!(end[0].fg, (200, 100, 50), "reaches its ink");
    assert!(live(90_100 + ENTER_MS - 1) && !live(90_100 + ENTER_MS));
    crate::motion::set_level(Off);
}

#[test]
fn the_spine_draws_down() {
    let _g = crate::app::theme_test_guard();
    crate::motion::set_level(Full);
    observe_rows(std::iter::empty(), 91_000, true);
    let fx = card(0, 91_000, 91_000);
    let cells = apply(vec![cell(1, 3), cell(1, 4)], &fx, 3, 91_030);
    let page = crew_theme::theme().page_bg;
    assert_ne!(cells[0].fg, page, "the upper row has begun");
    assert_eq!(cells[1].fg, page, "the lower row has not begun");
    crate::motion::set_level(Off);
}

#[test]
fn first_sight_is_seeded_silently() {
    let _g = crate::app::theme_test_guard();
    crate::motion::set_level(Full);
    // The section just came into view: even a brand-new pane is not news.
    observe_rows(std::iter::empty(), 1, false);
    assert_eq!(card(0, 92_000, 92_010).born_at, None);
    // And an old pane first seen after the section was seen is not new either.
    assert_eq!(
        observe_rows(
            vec![(5, 1, EntryState::Idle, "w")].into_iter(),
            93_000,
            true
        )
        .unwrap()[0]
            .born_at,
        None
    );
    crate::motion::set_level(Off);
}

#[test]
fn subtle_is_shorter_and_softer_and_off_is_still() {
    let _g = crate::app::theme_test_guard();
    let page = (0, 0, 0);
    let ink = (200, 100, 50);
    assert_ne!(rise(ink, page, 0, 0, Subtle), page, "softer");
    assert_eq!(rise(ink, page, Subtle.scale_ms(ENTER_MS), 0, Subtle), ink);
    assert_eq!(rise(ink, page, 0, 0, Off), ink);
    crate::motion::set_level(Off);
    observe_rows(std::iter::empty(), 94_000, true);
    let fx = card(0, 94_000, 94_010);
    assert_eq!(fx.born_at, None);
    assert_eq!(apply(vec![cell(5, 3)], &fx, 3, 94_010)[0].fg, ink);
    assert!(!live(94_011));
}
