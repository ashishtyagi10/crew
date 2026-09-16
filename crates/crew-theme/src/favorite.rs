//! The faces the user asked for more of, and the one key that says two
//! spellings are the same typeface.
//!
//! Split from [`crate::fonts`], which is the per-theme preference data: this
//! is about the `/font random` rotation, which is the other thing that decides
//! what you are reading.

/// The faces the user asked crew to show more of (2026-09-15), by canonical
/// name — [`typeface_key`] matches them in whatever spelling a machine has
/// them installed under, so `ComicMono Nerd Font Mono` is Comic Mono here.
///
/// They are not a *pool*: the rotation still reaches every allowlisted face
/// that is installed. They simply come up [`FAVORITE_WEIGHT`] times as often
/// as the rest, which is what "more often" means when the answer has to leave
/// room for the others.
///
/// This list weights the ROTATION, which is a thing the user turns on. It is
/// not `font_prefs`, which is what crew picks with nobody asking — `Noto Sans
/// Mono` is here by request (2026-09-15) and still appears in no theme's
/// preference list, because the two Windows bugs in the 0.17.x notes were
/// exactly a generic face standing at the front of those.
///
/// `Operator Mono` (2026-09-16) is out of the preference lists for the
/// opposite reason: it is a face you buy, so on most machines it is simply
/// not there — and a preference list is crew guessing for you, where a guess
/// that resolves nothing costs a fallback. In the rotation, which only runs
/// on the machine that turned it on, it comes up as often as the rest.
pub const FAVORITES: &[&str] = &[
    "Comic Mono",
    "JetBrains Mono",
    "SF Mono",
    "IBM Plex Mono",
    "MonoLisa",
    "Lilex",
    "Noto Sans Mono",
    "Operator Mono",
];

/// How many times more often a favourite comes up in the `/font random`
/// rotation than any other installed face.
pub const FAVORITE_WEIGHT: usize = 3;

/// One key per TYPEFACE, however it is spelled: lowercased with the `nerd`,
/// `font`, `nfm`, `nf` and `mono` decorations and every separator removed.
///
/// `JetBrains Mono`, `JetBrainsMono NF`, `JetBrainsMono NFM` and
/// `JetBrainsMono Nerd Font Mono` are one face installed four ways, and this
/// machine carries all four. Listed as four families they fill the picker with
/// rows that change nothing, take four times the tickets in a weighted
/// rotation, and let the rotation "change" the font to the face already on
/// screen. `nfm` is stripped before `nf` — taking `nf` out of `nfm` leaves a
/// stray `m` and a key that matches nothing.
///
/// `MonoLisa` keys as `lisa` and `Cascadia Mono` as `cascadia`, which are
/// unlovely but unique, and unique is the whole requirement.
pub fn typeface_key(family: &str) -> String {
    let mut k = family.to_ascii_lowercase();
    for word in ["nerd", "font", "nfm", "nf", "mono"] {
        k = k.replace(word, "");
    }
    k.retain(|c| c.is_ascii_alphanumeric());
    k
}

/// How wanted one SPELLING of a typeface is, when a machine has several.
///
/// The icon-bearing builds win — a Nerd Font spelling is the same outlines
/// plus the glyphs crew's marks are drawn from — and among those the `Mono`
/// build, whose icons are one cell wide, which is what a cell grid wants. The
/// written-out name beats the abbreviation at equal rank, since this is what
/// the status line says out loud when the font changes.
pub fn spelling_rank(family: &str) -> u8 {
    let f = family.to_ascii_lowercase();
    let nerd = f.contains("nerd font");
    match (
        nerd,
        f.ends_with("mono"),
        f.ends_with(" nfm"),
        f.ends_with(" nf"),
    ) {
        (true, true, _, _) => 4,
        (_, _, true, _) => 3,
        (true, false, _, _) => 2,
        (_, _, _, true) => 1,
        _ => 0,
    }
}

/// Whether `family` is one of the [`FAVORITES`], in any spelling.
pub fn is_favorite(family: &str) -> bool {
    let k = typeface_key(family);
    !k.is_empty() && FAVORITES.iter().any(|f| typeface_key(f) == k)
}

#[cfg(test)]
#[path = "favorite_tests.rs"]
mod tests;
