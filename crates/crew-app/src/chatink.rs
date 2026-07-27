//! Where chat markdown gets its colours. Every helper reads the ACTIVE theme
//! at call time, so a live `/theme` switch repaints with nothing to
//! invalidate.
//!
//! Code and marker colours START from the theme's own 16-slot ANSI palette
//! rather than from new `Theme` fields: all 13 presets already tune those
//! slots, so a new field would be 13 values to hand-maintain.
//!
//! But a starting point is not an answer. The CRT presets are single-phosphor
//! by design — CRT_GREEN's "cyan" is (0, 255, 200) against an `ink` of
//! (0, 255, 102) — so on a tube the raw slots collapse into body text: code
//! measured 1.04:1 against prose, which is the same colour. The semantic
//! palette shipped in v0.6.34 was therefore invisible on every CRT theme.
//! [`separated`] fixes that generally: each colour is pulled toward the page
//! until it clears [`SEPARATION_FLOOR`] against `ink`, and a theme that
//! already separates is left exactly as its author tuned it.
//!
//! Named `chatink`, not `chatpal`: `chatpalette.rs` is the slash-command
//! palette UI and the two are easy to confuse.
use crate::chatbody::Color;
use crew_theme::{contrast_ratio, Theme, ALL_THEMES};
use std::sync::OnceLock;

/// How far every semantic colour must sit from BODY text (`ink`), as a WCAG
/// ratio. 1.0 is literally the same colour; the CRT presets landed at 1.04
/// before this floor existed. Deliberately modest — this buys "you can see
/// these are different", not "these shout at each other", and prose remains
/// the brightest thing on the card.
pub(crate) const SEPARATION_FLOOR: f32 = 1.8;

/// How readable a semantic colour must stay against the page. Separation is
/// never bought below this: a code colour that is distinct from prose but
/// unreadable on the card has traded one bug for a worse one. Matches the
/// 4.5:1 that `crew-theme`'s `contrast_thresholds` already holds `ansi[3]`
/// and `ansi[6]` to.
pub(crate) const PAGE_FLOOR: f32 = 4.5;

/// Rungs above [`SEPARATION_FLOOR`] for the syntax classes, so they separate
/// from plain code as well as from prose. Both are capped by [`PAGE_FLOOR`] —
/// a comment that has faded into the card is not a subtler comment, it is an
/// unreadable one — so on a theme with little headroom the ladder compresses
/// rather than breaking readability.
const STRING_FLOOR: f32 = 2.3;
const COMMENT_FLOOR: f32 = 3.1;

/// Comments get a lower readability floor than everything else, and only
/// comments. They are the one class meant to recede — 4.5:1 is the threshold
/// for text you are reading, and a comment you are reading is a comment you
/// can still read at 3.5:1. Without this the light presets ran out of
/// headroom and the ladder collapsed there (sepia-light measured 1.17 between
/// comment and code), which is the failure this whole ladder exists to avoid.
const COMMENT_PAGE_FLOOR: f32 = 3.5;

/// How far the code card's background is nudged from the page toward `ink`.
/// Was 0.08, which measured 1.10-1.12:1 against the page on the CRT presets —
/// nothing survives scanlines and bloom at that distance. The card is the
/// second half of "this is code": foreground separation alone is a thin
/// signal for a one-line snippet.
const CODE_BG_MIX: f32 = 0.18;

/// The four derived colours for one theme. Computed once per preset (see
/// [`table`]) rather than per span: [`separated`] walks a lerp and
/// `contrast_ratio` calls `powf` six times a step, which is not something to
/// run per glyph on the winit thread.
#[derive(Clone, Copy)]
struct Ink {
    code: Color,
    marker: Color,
    quote: Color,
    code_bg: Color,
    comment: Color,
    string: Color,
    keyword: Color,
}

/// Pull `c` away from body text until it is distinguishable from it.
///
/// The walk goes toward `page_bg`, and only that way, because it is the only
/// direction with headroom where this matters: on a single-phosphor tube
/// `ink` already sits near the top of the phosphor's range (CRT_GREEN's is
/// (0, 255, 102)), so there is nothing brighter to move to. Toward the page
/// works on paper themes too — light or dark, the page is by definition the
/// far end of the contrast axis from body text.
///
/// Returns `c` untouched when the theme already separates, which is every
/// paper preset: this is a floor, not a restyling.
fn separated(c: Color, t: &Theme) -> Color {
    separated_to(c, t, SEPARATION_FLOOR, PAGE_FLOOR)
}

/// [`separated`] with an explicit target, which is what makes syntax classes
/// distinguishable from EACH OTHER rather than merely from prose.
///
/// Pulling every class to the same floor was the first attempt and it failed
/// for a reason worth recording: the walk stops the moment it clears the
/// floor, so every colour landed at the same lightness — measured 1.02:1
/// between comment and code on paper-dark, 1.00 on graphite. A single floor
/// guarantees "not body text" and accidentally guarantees "all alike". The
/// classes therefore sit on a LADDER, and on a single-phosphor tube — where
/// hue cannot vary at all — that ladder is the whole of the highlighting.
fn separated_to(c: Color, t: &Theme, floor: f32, page_floor: f32) -> Color {
    if contrast_ratio(c, t.ink) >= floor {
        return c;
    }
    let mut best = c;
    let mut mix = 0.0_f32;
    while mix < 1.0 {
        mix += 0.02;
        let cand = crate::anim::lerp_rgb(c, t.page_bg, mix);
        // Stop before readability goes: a preset with no room to separate
        // keeps the last legible candidate rather than fading into the page.
        if contrast_ratio(cand, t.page_bg) < page_floor {
            break;
        }
        best = cand;
        if contrast_ratio(cand, t.ink) >= floor {
            break;
        }
    }
    best
}

/// Derive one preset's semantic colours. Pure in `t` so the floor can be
/// asserted for all 13 presets without touching the global theme atomic.
fn derive(t: &Theme) -> Ink {
    Ink {
        code: separated(t.ansi[6], t),
        marker: separated(t.ansi[3], t),
        quote: separated(t.text_muted, t),
        code_bg: crate::anim::lerp_rgb(t.page_bg, t.ink, CODE_BG_MIX),
        // Syntax classes, from the theme's own slots for the same reason the
        // rest are: 13 presets already tune them, and a single-phosphor tube
        // keeps its hue for free. Each still goes through `separated`, so a
        // token colour can never collapse into body text.
        // The ladder. Comments sit furthest back, strings between, plain code
        // nearest to prose. Keywords take no rung of their own — they draw in
        // the code colour and are marked by WEIGHT (see `chatmd`), because a
        // fourth rung would either crowd the other three or push a colour past
        // the page floor on the darker tubes.
        comment: separated_to(t.text_muted, t, COMMENT_FLOOR, COMMENT_PAGE_FLOOR),
        string: separated_to(t.ansi[2], t, STRING_FLOOR, PAGE_FLOOR),
        keyword: separated(t.ansi[6], t),
    }
}

/// The colour for one run of code, given what the tokenizer made of it.
/// `Plain` is the ordinary code colour, which is most of any line.
pub(crate) fn token_fg(token: crate::md::syntax::Token) -> Color {
    use crate::md::syntax::Token;
    let i = ink();
    match token {
        Token::Plain => i.code,
        Token::Comment => i.comment,
        Token::Str => i.string,
        Token::Keyword => i.keyword,
    }
}

/// Every preset's derived colours, computed once on first use. All 13 up
/// front rather than lazily per theme: they are static data, the whole table
/// costs one pass, and it keeps the live-`/theme` read down to an index.
fn table() -> &'static [Ink; ALL_THEMES.len()] {
    static TABLE: OnceLock<[Ink; ALL_THEMES.len()]> = OnceLock::new();
    TABLE.get_or_init(|| ALL_THEMES.map(|id| derive(id.theme())))
}

/// The active theme's derived colours.
fn ink() -> Ink {
    let id = crew_theme::current_id();
    let i = ALL_THEMES.iter().position(|&t| t == id).unwrap_or(0);
    table()[i]
}

/// The code card's background: the page nudged toward the ink colour, so the
/// block reads as a card in every theme without a dedicated theme slot.
pub(crate) fn code_bg() -> Color {
    ink().code_bg
}

/// Code text — fenced blocks and inline spans alike.
pub(crate) fn code_fg() -> Color {
    ink().code
}

/// Structural marker glyphs: list bullets/ordinals and the blockquote bar.
pub(crate) fn marker_fg() -> Color {
    ink().marker
}

/// Quoted prose — one step back from body text.
pub(crate) fn quote_fg() -> Color {
    ink().quote
}

/// Headings, at every level. Deliberately `ink` itself, and so deliberately
/// exempt from [`SEPARATION_FLOOR`]: headings are separated by WEIGHT (they
/// render bold) and by their own line. Tinting them as well would make a
/// document with several headings read as several documents.
pub(crate) fn heading_fg() -> Color {
    crew_theme::theme().ink
}

/// Link tint: reuse the terminal pane's own URL-highlight colour (`linkhl`)
/// so a link reads the same whether it's in a pane or a chat card.
pub(crate) fn link_color() -> Color {
    crate::linkhl::LINK_FG
}

#[cfg(test)]
#[path = "chatink_tests.rs"]
mod tests;
