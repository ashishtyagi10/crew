//! Card motion: a first sighting is quiet, a change flashes and settles
//! exactly, a ticking clock is not news, and Off is the still frame.
use super::*;
use crate::navpanememo::{observe, Memo};
use crate::panestate::EntryState;
use MotionLevel::{Full, Off};

const BASE: (u8, u8, u8) = (200, 40, 40);
const INK: (u8, u8, u8) = (250, 250, 250);
const PAGE: (u8, u8, u8) = (10, 10, 10);

fn seen(memo: Option<Memo>, s: EntryState, w: &str, now: u64, l: MotionLevel) -> Memo {
    observe(memo, s, w, now, l)
}

#[test]
fn a_first_sighting_does_not_flash_or_fade() {
    let m = seen(None, EntryState::Working, "Pondering 3s", 100, Full);
    assert_eq!((m.fx.state_at, m.fx.words_at), (None, None));
    assert!(m.fx.on);
}

#[test]
fn a_state_change_flashes_then_settles_to_the_exact_colour() {
    let m = seen(None, EntryState::Working, "x", 0, Full);
    let m = seen(Some(m), EntryState::Done, "x", 1000, Full);
    assert_eq!(m.fx.state_at, Some(1000));
    assert_ne!(flash_fg(BASE, INK, 0, Full), BASE);
    assert!(flash_fg(BASE, INK, 0, Full).0 > BASE.0);
    assert_ne!(flash_fg(BASE, INK, 300, Full), BASE);
    assert_eq!(flash_fg(BASE, INK, 700, Full), BASE);
    assert_eq!(flash_fg(BASE, INK, 5000, Full), BASE);
}

#[test]
fn only_new_words_fade_not_the_timer_ticking() {
    let m = seen(None, EntryState::Working, "Pondering", 0, Full);
    let m = seen(Some(m), EntryState::Working, "Pondering", 50, Full);
    assert_eq!(m.fx.words_at, None, "same words, timer aside");
    let m = seen(Some(m), EntryState::Working, "Reading file", 90, Full);
    assert_eq!(m.fx.words_at, Some(90));
    assert_ne!(fade_fg(BASE, PAGE, 0, Full), BASE);
    assert_eq!(fade_fg(BASE, PAGE, 350, Full), BASE);
}

#[test]
fn off_is_the_still_frame() {
    let m = seen(None, EntryState::Working, "a", 0, Off);
    let m = seen(Some(m), EntryState::Done, "b", 10, Off);
    assert!(!m.fx.on);
    assert_eq!(flash_fg(BASE, INK, 0, Off), BASE);
    assert_eq!(fade_fg(BASE, PAGE, 0, Off), BASE);
    assert_eq!(breath(BASE, PAGE, 1200, Off), BASE);
    assert_eq!(
        Fx::default(),
        Fx {
            on: false,
            state_at: None,
            words_at: None,
            born_at: None,
        }
    );
}

#[test]
fn the_breath_swings_toward_the_page_and_back() {
    assert_eq!(breath(BASE, PAGE, 0, Full), BASE);
    let low = breath(BASE, PAGE, 1200, Full);
    assert!(low.0 < BASE.0 && low.0 > PAGE.0, "subtle: {low:?}");
    assert_eq!(breath(BASE, PAGE, 2400, Full), BASE);
}

#[test]
fn a_card_with_a_fresh_flash_draws_a_brighter_spine() {
    let _g = crate::app::theme_test_guard();
    crate::motion::set_level(Full);
    let now = crate::anim::now_ms();
    let base = (60, 60, 60);
    let still = Fx::default();
    let lit = Fx {
        on: true,
        state_at: Some(now),
        words_at: None,
        born_at: None,
    };
    assert_eq!(spine_fg(&still, base, false, now), base);
    assert_ne!(spine_fg(&lit, base, false, now), base);
    assert_eq!(spine_fg(&lit, base, false, now + FLASH_MS + 1), base);
    assert_eq!(word_fg(&still, base, now), base);
    assert_ne!(word_fg(&lit, base, now), base);
    let said = Fx {
        on: true,
        state_at: None,
        words_at: Some(now),
        born_at: None,
    };
    assert_ne!(words_fg(&said, base, now), base);
}
