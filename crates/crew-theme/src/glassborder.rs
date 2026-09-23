//! Brighter frames while the window is sheer.
//!
//! A translucent crew is frosted glass: the page under every pane, the nav
//! and the input bar lets a blurred desktop through. A frame tuned to sit a
//! quiet step off a flat page loses that step the moment a bright wallpaper
//! is blurred in behind it, and cards stop reading as cards. So while the
//! window is sheer, [`crate::theme`] serves each palette with its borders
//! LIFTED: pushed away from the page (same hue, same chroma — see
//! [`crate::readable::against`]) until they clear a text-grade contrast floor,
//! and the focused frame further still, so the lit card stays the lit card.
//!
//! One switch here rather than a branch at every site that draws a frame:
//! there are dozens of them (cards, the nav's glance cards, the input bar,
//! gauges), and they all read `theme().border_*` already.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use crate::readable::against;
use crate::{Theme, ThemeId};

/// Unfocused frames and legends: the AA text floor — a frame over glass has
/// to be as findable as a word.
pub const NORMAL_FLOOR: f32 = 4.5;
/// The focused frame: AAA, a full step above the others so focus still reads
/// at a glance when every frame got brighter.
pub const FOCUSED_FLOOR: f32 = 7.0;

static SHEER: AtomicBool = AtomicBool::new(false);

/// One lifted copy per palette, built on first use and kept — a palette is
/// `'static`, so its lifted twin can be too, and `theme()` stays a reference.
/// Sized with slack over the palette count so a new theme cannot index past it.
static LIFTED: [OnceLock<Theme>; 32] = [const { OnceLock::new() }; 32];

/// Publish whether the window is sheer (opacity below 100%).
pub fn set_sheer(on: bool) {
    SHEER.store(on, Ordering::Relaxed);
}

/// Whether frames are currently being served lifted.
pub fn sheer() -> bool {
    SHEER.load(Ordering::Relaxed)
}

/// `t` with its frames lifted for glass. Pure, so it can be measured per
/// palette without touching the global switch.
pub fn lift(t: &Theme) -> Theme {
    let mut l = *t;
    l.border_normal = against(t.border_normal, t.page_bg, NORMAL_FLOOR);
    l.border_focused = against(t.border_focused, t.page_bg, FOCUSED_FLOOR);
    l.legend_off = against(t.legend_off, t.page_bg, NORMAL_FLOOR);
    l
}

/// The lifted twin of `id`'s palette.
pub(crate) fn lifted(id: ThemeId) -> &'static Theme {
    LIFTED[usize::from(id.as_u8())].get_or_init(|| lift(id.theme()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contrast_ratio;

    #[test]
    fn every_palette_gets_frames_that_read_over_glass() {
        for id in crate::ALL_THEMES {
            let t = lift(id.theme());
            let n = contrast_ratio(t.border_normal, t.page_bg);
            let f = contrast_ratio(t.border_focused, t.page_bg);
            // `against` keeps the hue, and a hue can top out short of the
            // floor; allow the last step's worth of shortfall, no more.
            assert!(n >= NORMAL_FLOOR - 0.3, "{id:?}: normal frame {n:.2}");
            assert!(f >= FOCUSED_FLOOR - 0.6, "{id:?}: focused frame {f:.2}");
        }
    }

    #[test]
    fn lifting_never_dims_a_frame() {
        for id in crate::ALL_THEMES {
            let base = id.theme();
            let t = lift(base);
            for (was, now) in [
                (base.border_normal, t.border_normal),
                (base.border_focused, t.border_focused),
                (base.legend_off, t.legend_off),
            ] {
                assert!(
                    contrast_ratio(now, t.page_bg) >= contrast_ratio(was, t.page_bg),
                    "{id:?}"
                );
            }
        }
    }

    #[test]
    fn only_the_frames_move() {
        let base = ThemeId::PaperDark.theme();
        let t = lift(base);
        assert_eq!(t.page_bg, base.page_bg);
        assert_eq!(t.ink, base.ink);
        assert_eq!(t.ansi, base.ansi);
    }
}
