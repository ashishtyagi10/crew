//! Real glyph coverage, per family: does the face cosmic-text resolves for a
//! family actually map a character, or would it draw tofu?
//!
//! The question comes up for the Private-Use-Area icons a Nerd Font carries
//! and every other face does not. Nothing about a family NAME answers it
//! ("FiraCode Nerd Font" does, "Fira Code" does not, and a user can install
//! either under any name), so the answer is read off the font's own
//! character map — the same `charmap().map(c) != 0` test `fontlist` uses
//! to prove a candidate really renders `i`, `m` and `0`.
//!
//! Building a `FontSystem` scans the whole font database, which is not a
//! per-frame cost, so one is built lazily per thread and kept; every
//! `(family, char)` answer is memoised, so a caller that asks on the render
//! path pays a hash lookup after the first frame.
use std::cell::RefCell;
use std::collections::HashMap;

use glyphon::cosmic_text::fontdb::{Family, Query, Weight};
use glyphon::FontSystem;

thread_local! {
    /// The private font database, built on first use (see module doc).
    static FONT_SYSTEM: RefCell<Option<FontSystem>> = const { RefCell::new(None) };
    /// `(family, char) → mapped?`. Coverage is a font property, so an answer
    /// never goes stale while the font database stands.
    static MEMO: RefCell<HashMap<(String, char), bool>> = RefCell::new(HashMap::new());
}

/// Whether `family` (the embedded [`crew_theme::EMBEDDED_FAMILY`] when
/// `None`, the same default the renderer applies) has a real glyph for `c`
/// at regular weight. A family that is not installed covers nothing.
pub fn has_glyph(family: Option<&str>, c: char) -> bool {
    let fam = family.unwrap_or(crew_theme::EMBEDDED_FAMILY);
    let key = (fam.to_string(), c);
    if let Some(hit) = MEMO.with(|m| m.borrow().get(&key).copied()) {
        return hit;
    }
    let hit = FONT_SYSTEM.with(|slot| {
        let mut slot = slot.borrow_mut();
        let fs = slot.get_or_insert_with(crate::embedfont::font_system);
        lookup(fs, fam, c)
    });
    MEMO.with(|m| m.borrow_mut().insert(key, hit));
    hit
}

/// The charmap test against the regular face `family` resolves to.
fn lookup(fs: &mut FontSystem, family: &str, c: char) -> bool {
    let query = Query {
        families: &[Family::Name(family)],
        weight: Weight::NORMAL,
        ..Query::default()
    };
    let Some(id) = fs.db().query(&query) else {
        return false;
    };
    let Some(font) = fs.get_font(id, Weight::NORMAL) else {
        return false;
    };
    font.as_swash().charmap().map(c) != 0
}

/// Whether `(family, c)` has been asked and answered on this thread.
#[cfg(test)]
pub(crate) fn memoised(family: &str, c: char) -> bool {
    MEMO.with(|m| m.borrow().contains_key(&(family.to_string(), c)))
}

#[cfg(test)]
#[path = "glyphmap_tests.rs"]
mod tests;
