//! The memory: a first sighting is quiet, a change flashes, a clock is not news.
use super::*;
use crate::navpanemotion::BREATH_LEASE_MS;
use MotionLevel::{Full, Off};

#[test]
fn rows_are_remembered_by_pane_and_forgotten_when_gone() {
    let _g = crate::app::theme_test_guard();
    crate::motion::set_level(Full);
    let row = |s| vec![(7, 0, s, "w")].into_iter();
    assert_eq!(
        observe_rows(row(EntryState::Working), 5000)[0].state_at,
        None
    );
    let fx = observe_rows(row(EntryState::Done), 6000)[0];
    assert_eq!(fx.state_at, Some(6000));
    assert!(live(6100) && !live(6000 + FLASH_MS + 1));
    observe_rows(std::iter::empty(), 7000);
    assert_eq!(observe_rows(row(EntryState::Idle), 8000)[0].state_at, None);
    observe_rows(row(EntryState::Working), 9000);
    assert!(breathing(9100) && !breathing(9000 + BREATH_LEASE_MS));
    crate::motion::set_level(Off);
    assert!(!live(6100) && !breathing(9100));
}
