//! The words on the welcome screen, and the colour they carry.
//!
//! Split out of [`crate::welcome`] (which owns the rain field, the frame and
//! the nameplate) for the 200-line cap, and because all three of this
//! surface's text defects were here rather than in the drawing:
//!
//! * the release headline came straight out of `CHANGELOG.md` and showed its
//!   **markdown** — the first screen of the app read ``new in 0.19.53 ·
//!   `/keys`, shot whole``, backticks and all;
//! * a line was allowed to be one column narrower than the card, so the
//!   widest-fitting hint sat *touching* both frame strokes;
//! * and the one line telling a new user what to press was drawn in the same
//!   muted grey as the prose around it, so nothing on crew's first screen was
//!   crew's own colour.
use crate::palette::accent;

pub(crate) const TAGLINE: &str = "fast terminals. clean flow.";

/// The opening hint, widest form first. It named the shell and the command
/// palette and not the agents, which are the reason crew is not just a
/// terminal — and a first run that never mentions them is a first run that
/// never finds them.
///
/// Chosen by width rather than dropped: the whole line used to vanish on a
/// narrow window, which is the wrong trade for the one piece of guidance a
/// new user gets.
const HINTS: &[&str] = &[
    "Cmd+T  shell    \u{00b7}    Cmd+J  agents    \u{00b7}    /  commands",
    "Cmd+T  shell  \u{00b7}  Cmd+J  agents  \u{00b7}  /  commands",
    "Cmd+T shell \u{00b7} Cmd+J agents \u{00b7} / commands",
    "Cmd+J  agents    \u{00b7}    /  commands",
    "Cmd+J agents",
];

/// Columns of air a centred line keeps between itself and the card's frame.
/// Without it a line whose width is `cols - 1` sits against the stroke, and
/// text touching a border reads as a rendering fault rather than a layout.
pub(crate) const MARGIN: usize = 2;

/// Whether a line `w` columns wide fits `cols` with its margins.
pub(crate) fn fits(w: usize, cols: u16) -> bool {
    w + 2 * MARGIN <= cols as usize
}

/// The widest hint that fits `cols` with air on both sides, or `None` when
/// even the shortest does not.
pub(crate) fn hint_for(cols: u16) -> Option<&'static str> {
    HINTS
        .iter()
        .copied()
        .find(|h| fits(h.chars().count(), cols))
}

/// The hint, coloured: the chords are the actionable half of the line, so
/// they wear the accent and the words around them stay muted. This is the
/// only colour on the first screen, which is the point — a monochrome welcome
/// says nothing about which characters are the ones to type.
pub(crate) fn hint_spans(
    hint: &str,
    key: (u8, u8, u8),
    word: (u8, u8, u8),
) -> Vec<(char, (u8, u8, u8))> {
    let mut out = Vec::new();
    for token in hint.split_inclusive(' ') {
        let bare = token.trim_end();
        let fg = match is_chord(bare) {
            true => key,
            false => word,
        };
        out.extend(token.chars().map(|c| (c, fg)));
    }
    out
}

/// A chord is what you press: a `Cmd+…`/`Ctrl+…` combination, or anything
/// starting with `/` — the bare slash that opens the palette, and the
/// `/restore` on the offer line, which is a thing to type like any other.
fn is_chord(token: &str) -> bool {
    token.starts_with('/') || token.starts_with("Cmd+") || token.starts_with("Ctrl+")
}

/// The accent the hint's chords wear.
pub(crate) fn chord_fg() -> (u8, u8, u8) {
    accent()
}

/// One extra hint row when a saved session exists: `restore` carries the
/// snapshot's shell count (cleared once `/restore` spends it).
pub(crate) fn restore_hint(n: usize) -> String {
    format!(
        "{n} pane{} from last session    \u{00b7}    /restore",
        if n == 1 { "" } else { "s" }
    )
}

/// Markdown as plain cells. The changelog is written in markdown and this
/// screen has no renderer — a backtick or an asterisk here is not emphasis,
/// it is a character the reader has to look past.
pub(crate) fn plain(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '`' | '*' | '_'))
        .collect()
}

/// The current release's headline, as one centred line — the first bold
/// sentence of the newest changelog entry, which is written to be exactly
/// that. `None` when there is no room for it or nothing to say.
///
/// Trimmed to a sentence: the entries themselves run to paragraphs, and a
/// welcome screen is not where anyone reads one.
pub(crate) fn whats_new(cols: usize) -> Option<String> {
    let body = crate::appregister::CHANGELOG;
    let heading = body.find("\n## ")? + 4;
    let rest = &body[heading..];
    let bold = rest.find("**")? + 2;
    let end = rest[bold..].find("**")?;
    let head: String = plain(&rest[bold..bold + end])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let version = rest[..rest.find('\n')?].trim();
    let lead = format!("new in {version} \u{b7} ");
    let head = head.trim_end_matches('.');
    // The window has to hold the version and something worth reading of the
    // headline; below that there is nothing useful to say. A headline longer
    // than the window is the changelog's doing, not the window's, so it is
    // clipped rather than dropped — the first clause is the part that names
    // the release.
    let room = cols.checked_sub(lead.chars().count() + 2 * MARGIN)?;
    if room < 12 {
        return None;
    }
    let head = match head.chars().count() > room {
        true => format!(
            "{}\u{2026}",
            head.chars().take(room - 1).collect::<String>()
        ),
        false => head.to_string(),
    };
    Some(format!("{lead}{head}"))
}

#[cfg(test)]
#[path = "welcometext_tests.rs"]
mod tests;
