//! The memory: a first sighting is quiet, a change flashes, a clock is not news.
use super::*;
use crate::navpanemotion::BREATH_LEASE_MS;
use MotionLevel::{Full, Off};

fn seen(s: EntryState, at: u64) -> Fx {
    observe_rows(vec![(7, 0, s, "w")].into_iter(), at, true).unwrap()[0]
}

#[test]
fn rows_are_remembered_by_pane_and_forgotten_when_gone() {
    let _g = crate::app::theme_test_guard();
    crate::motion::set_level(Full);
    assert_eq!(seen(EntryState::Working, 5000).state_at, None);
    let fx = seen(EntryState::Done, 6000);
    assert_eq!(fx.state_at, Some(6000));
    assert!(live(6100) && !live(6000 + FLASH_MS + 1));
    observe_rows(std::iter::empty(), 7000, true);
    assert_eq!(seen(EntryState::Idle, 8000).state_at, None);
    seen(EntryState::Working, 9000);
    assert!(breathing(9100) && !breathing(9000 + BREATH_LEASE_MS));
    crate::motion::set_level(Off);
    assert!(!live(6100) && !breathing(9100));
}

#[test]
fn what_changed_out_of_sight_does_not_flash_on_return() {
    let _g = crate::app::theme_test_guard();
    crate::motion::set_level(Full);
    seen(EntryState::Working, 20_000);
    let away = vec![(7, 0, EntryState::Working, "w")].into_iter();
    assert!(
        observe_rows(away, 21_000, false).is_none(),
        "no cards drawn"
    );
    assert_eq!(seen(EntryState::Done, 22_000).state_at, None);
}

#[test]
fn a_subtle_flash_asks_for_no_frames_past_its_end() {
    let _g = crate::app::theme_test_guard();
    crate::motion::set_level(MotionLevel::Subtle);
    seen(EntryState::Working, 30_000);
    seen(EntryState::Done, 31_000);
    let end = MotionLevel::Subtle.scale_ms(FLASH_MS);
    assert!(live(31_000 + end - 1) && !live(31_000 + end));
}

#[test]
fn a_reading_ticking_is_not_new_words() {
    let _g = crate::app::theme_test_guard();
    crate::motion::set_level(Full);
    let say = |w: &'static str, at| {
        observe_rows(vec![(9, 0, EntryState::Idle, w)].into_iter(), at, true).unwrap()[0]
    };
    say("cpu 19% \u{b7} mem 72%", 40_000);
    assert_eq!(say("cpu 95% \u{b7} mem 71%", 41_000).words_at, None);
    assert_eq!(
        say("cpu 95% \u{b7} disk 71%", 42_000).words_at,
        Some(42_000)
    );
    crate::motion::set_level(Off);
}
