//! The hued syntax classes: keyword, type, call, number, attribute.
//!
//! `chatink`'s ladder separates comment / string / code by LIGHTNESS, which
//! is the one axis a single-phosphor tube has. On a paper preset that left
//! every keyword the same cyan as the code around it, told apart by weight
//! alone, while the theme's palette held five more hues nobody was drawing
//! with. These classes take those hues — magenta keywords, yellow types,
//! blue calls — from the theme's own ANSI slots, the way the ladder already
//! takes its colours.
//!
//! A hue is not a licence to skip the floors. Every colour here goes through
//! the same three checks the ladder answers to — readable on the page
//! ([`PAGE_FLOOR`]), apart from body text ([`SEPARATION_FLOOR`]), readable
//! on the code field ([`CODE_ON_FIELD_FLOOR`]) — and a slot that cannot
//! clear all three on some preset is not drawn a little too faint there: it
//! falls back to the code colour, which is what that preset drew before.
//! The sweep in `chathue_tests` asserts the three floors per preset per
//! class, so a fallback is the worst a preset can get.
use std::sync::OnceLock;

use crate::chatbody::Color;
use crate::chatink::{self, CODE_ON_FIELD_FLOOR, PAGE_FLOOR, SEPARATION_FLOOR};
use crate::md::syntax::Token;
use crew_theme::{contrast_ratio, readable, Theme, ALL_THEMES};

/// The hued classes for one preset. Computed once per preset like `chatink`'s
/// table: the lift is an oklch walk plus a lerp walk, not per-glyph work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Hue {
    /// `fn`, `let`, `def`… — `ansi[5]`, magenta on the paper presets.
    pub(crate) keyword: Color,
    /// A capitalised identifier — `ansi[3]`, yellow: the marker slot, which
    /// every theme already tunes to read on its page.
    pub(crate) ty: Color,
    /// An identifier followed by `(` — `ansi[4]`, blue.
    pub(crate) func: Color,
    /// A numeric literal — `ansi[6]`, the code slot itself.
    pub(crate) number: Color,
    /// `#[derive]` / `@decorator` — the comment rung; `chatspan` slants it.
    pub(crate) attr: Color,
}

/// Whether `c` clears every floor a colour drawn inside a code field owes.
pub(crate) fn clears_floors(c: Color, t: &Theme, code_bg: Color) -> bool {
    contrast_ratio(c, t.page_bg) >= PAGE_FLOOR
        && contrast_ratio(c, t.ink) >= SEPARATION_FLOOR
        && contrast_ratio(c, code_bg) >= CODE_ON_FIELD_FLOOR
}

/// One ANSI slot as a syntax hue on `t`: lifted to the page floor at its own
/// hue (`readable::against` walks lightness, keeping chroma), then pulled
/// from body text by the ladder's own walk — and handed back as `code` when
/// even that cannot clear the floors. Falling back rather than drawing the
/// best it reached is the point: "a bit faint on sepia-light" is exactly the
/// kind of defect the ladder was built to stop shipping.
fn hued(slot: Color, t: &Theme, code: Color, code_bg: Color) -> Color {
    let lifted = readable::against(slot, t.page_bg, PAGE_FLOOR);
    let c = chatink::separated_to(lifted, t, SEPARATION_FLOOR, PAGE_FLOOR);
    if clears_floors(c, t, code_bg) {
        c
    } else {
        code
    }
}

/// Derive one preset's hues. Pure in `t`, so the sweep can assert all
/// presets without touching the global theme.
pub(crate) fn derive(t: &Theme) -> Hue {
    let base = chatink::derive(t);
    let (code, bg) = (base.code, base.code_bg);
    Hue {
        keyword: hued(t.ansi[5], t, code, bg),
        ty: hued(t.ansi[3], t, code, bg),
        func: hued(t.ansi[4], t, code, bg),
        number: hued(t.ansi[6], t, code, bg),
        attr: base.comment,
    }
}

fn table() -> &'static [Hue; ALL_THEMES.len()] {
    static TABLE: OnceLock<[Hue; ALL_THEMES.len()]> = OnceLock::new();
    TABLE.get_or_init(|| ALL_THEMES.map(|id| derive(id.theme())))
}

/// The active theme's hues.
fn hue() -> Hue {
    let id = crew_theme::current_id();
    let i = ALL_THEMES.iter().position(|&t| t == id).unwrap_or(0);
    table()[i]
}

/// The colour for one hued token; `chatink::token_fg` routes these here and
/// keeps the ladder classes for itself. Anything else answers with `code`,
/// so a wrong route draws as plain code rather than as nothing.
pub(crate) fn token_fg(token: Token) -> Color {
    let h = hue();
    match token {
        Token::Keyword => h.keyword,
        Token::Type => h.ty,
        Token::Func => h.func,
        Token::Number => h.number,
        Token::Attr => h.attr,
        _ => chatink::code_fg(),
    }
}

/// The block colour a fence's language badge draws on: one of the active
/// theme's hued classes, chosen by language family so `rust` and `python`
/// read as different badges the way their keywords read as different
/// inks. Every class already clears the page and the code field, so the
/// badge stands off the field it sits in on every preset — and where a
/// class fell back to `code`, the badge does too, still off the field.
pub(crate) fn lang_hue(lang: &str) -> Color {
    let h = hue();
    match crate::glyphlang::key(lang).as_str() {
        "python" | "py" | "go" | "golang" | "sql" => h.func,
        "js" | "javascript" | "jsx" | "ts" | "typescript" | "tsx" | "json" | "yaml" | "yml" => h.ty,
        "sh" | "bash" | "zsh" | "shell" | "console" | "diff" | "patch" => h.number,
        _ => h.keyword,
    }
}

#[cfg(test)]
#[path = "chathue_tests.rs"]
mod tests;
