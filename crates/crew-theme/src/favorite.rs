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
pub const FAVORITES: &[&str] = &[
    "Comic Mono",
    "JetBrains Mono",
    "SF Mono",
    "IBM Plex Mono",
    "MonoLisa",
    "Lilex",
];

/// How many times more often a favourite comes up in the `/font random`
/// rotation than any other installed face.
pub const FAVORITE_WEIGHT: usize = 3;

/// One key per TYPEFACE, however it is spelled: lowercased with the `nerd`,
/// `font`, `nf` and `mono` decorations and every separator removed.
///
/// `JetBrains Mono`, `JetBrainsMono NF` and `JetBrainsMono Nerd Font Mono` are
/// one face installed three ways, and a machine can carry all three. Counted
/// as three families they take three times the tickets in a weighted rotation,
/// and — worse — the rotation "changes" the font to a spelling of the face
/// already on screen, which looks exactly like a rotation that did nothing.
///
/// `MonoLisa` keys as `lisa` and `Cascadia Mono` as `cascadia`, which are
/// unlovely but unique, and unique is the whole requirement.
pub fn typeface_key(family: &str) -> String {
    let mut k = family.to_ascii_lowercase();
    for word in ["nerd", "font", "nf", "mono"] {
        k = k.replace(word, "");
    }
    k.retain(|c| c.is_ascii_alphanumeric());
    k
}

/// Whether `family` is one of the [`FAVORITES`], in any spelling.
pub fn is_favorite(family: &str) -> bool {
    let k = typeface_key(family);
    !k.is_empty() && FAVORITES.iter().any(|f| typeface_key(f) == k)
}

#[cfg(test)]
#[path = "favorite_tests.rs"]
mod tests;
