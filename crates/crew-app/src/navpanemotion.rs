//! Motion for the PANES cards ([`crate::navpanecard`]): a state change
//! flashes the spine, new words on line two fade in, and a working card's
//! spine breathes. Small and honest: three colours as pure functions of a
//! time, and a memory of what each card last said so a change can be told
//! from a repaint.
//!
//! The memory is observed at draw time ([`observe_rows`], from
//! `pane_rows`), keyed by the pane's `born_ms` and place, and a pane seen
//! for the first time is recorded silently. Only the *words* of line two
//! count: `12s` → `13s` is the clock, not news. At Motion Off the memory
//! still keeps up, but every card draws as it always did and no frame is
//! asked for. Cards out of sight are forgotten, so returning is silent.
//!
//! Frames: a flash or fade is bounded, so [`live`] joins
//! `wants_animation_frame`; the breath has no end, so [`breathing`] is
//! renewed by each draw that sees a working card and lapses on its own
//! when the nav stops drawing — it rides the ambient pace.
use crate::motion::MotionLevel;

/// How long the spine flashes after its state changes, at full motion.
pub(crate) const FLASH_MS: u64 = 700;
/// How long new words take to settle from dim to their colour.
pub(crate) const FADE_MS: u64 = 350;
/// One breath of a working spine.
pub(crate) const BREATH_MS: u64 = 2400;
/// A breath renewed this recently is still owed frames.
pub(crate) const BREATH_LEASE_MS: u64 = 600;

/// What one card is doing right now. `Default` is the still frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fx {
    /// Motion is on: working cards breathe.
    pub on: bool,
    /// When the state last changed.
    pub state_at: Option<u64>,
    /// When the words of line two last changed.
    pub words_at: Option<u64>,
    /// When the pane was seen to be created ([`crate::navpanenter`]).
    pub born_at: Option<u64>,
}

/// `base` flashed toward `ink`, `since` ms after the change.
pub(crate) fn flash_fg(
    base: (u8, u8, u8),
    ink: (u8, u8, u8),
    since: u64,
    level: MotionLevel,
) -> (u8, u8, u8) {
    let dur = level.scale_ms(FLASH_MS);
    if dur == 0 || since >= dur {
        return base;
    }
    let left = 1.0 - crate::ease::out_cubic(since as f32 / dur as f32);
    crate::anim::lerp_rgb(base, ink, 0.6 * left)
}

/// `muted` rising from the page's dimness, `since` ms after new words.
pub(crate) fn fade_fg(
    muted: (u8, u8, u8),
    page: (u8, u8, u8),
    since: u64,
    level: MotionLevel,
) -> (u8, u8, u8) {
    let dur = level.scale_ms(FADE_MS);
    if dur == 0 || since >= dur {
        return muted;
    }
    let left = 1.0 - crate::ease::out_cubic(since as f32 / dur as f32);
    crate::anim::lerp_rgb(muted, page, 0.6 * left)
}

/// A working spine's breath at `now`.
pub(crate) fn breath(
    base: (u8, u8, u8),
    page: (u8, u8, u8),
    now: u64,
    level: MotionLevel,
) -> (u8, u8, u8) {
    if level == MotionLevel::Off {
        return base;
    }
    crate::anim::lerp_rgb(
        base,
        page,
        0.35 * crate::anim::tri(now, level.scale_ms(BREATH_MS)),
    )
}

/// The spine's colour: flashing if it just changed, else breathing if working.
pub(crate) fn spine_fg(fx: &Fx, base: (u8, u8, u8), working: bool, now: u64) -> (u8, u8, u8) {
    let t = crew_theme::theme();
    let level = crate::motion::level();
    if !fx.on {
        return base;
    }
    match fx.state_at.map(|a| now.saturating_sub(a)) {
        Some(s) if s < level.scale_ms(FLASH_MS) => flash_fg(base, t.ink, s, level),
        _ if working => breath(base, t.page_bg, now, level),
        _ => base,
    }
}

/// The state word's colour: the flash, without the breath.
pub(crate) fn word_fg(fx: &Fx, base: (u8, u8, u8), now: u64) -> (u8, u8, u8) {
    match fx.state_at.filter(|_| fx.on) {
        Some(a) => flash_fg(
            base,
            crew_theme::theme().ink,
            now.saturating_sub(a),
            crate::motion::level(),
        ),
        None => base,
    }
}

/// Line two's words colour while they fade in.
pub(crate) fn words_fg(fx: &Fx, muted: (u8, u8, u8), now: u64) -> (u8, u8, u8) {
    match fx.words_at.filter(|_| fx.on) {
        Some(a) => fade_fg(
            muted,
            crew_theme::theme().page_bg,
            now.saturating_sub(a),
            crate::motion::level(),
        ),
        None => muted,
    }
}

#[cfg(test)]
#[path = "navpanemotion_tests.rs"]
mod tests;
