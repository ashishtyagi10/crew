//! Value-picker definitions for the input-bar palette: which commands offer a
//! closed set of values (`expands`) and what those values are (`options_for`).
//! Split from `suggest.rs` (sibling module) to keep that file under the house
//! line cap. Kept as a pair rather than split further — `expands(cmd)` must
//! always equal `options_for(cmd).is_some()` for every command, and a test
//! beside this code (`suggestvalues_tests`) walks `COMMANDS` asserting exactly
//! that. Co-locating the two functions keeps that invariant local and obvious
//! instead of spread across modules.

/// The predefined `(value, description)` choices a command offers, or `None` for
/// a freeform / no-value command. **The single extension point** for the value
/// picker: give a command a closed set of values here and it gains an inline
/// picker for free (its rows run on Enter; unknown text still submits freeform).
pub(crate) fn options_for(cmd: &str) -> Option<Vec<(String, String)>> {
    match cmd {
        // The four themes — each a rotation over its own palette pool, auto's
        // following the OS appearance. The individual palettes and the legacy
        // `random-*` names still parse but aren't offered here.
        "/theme" => Some(
            crew_theme::THEME_MODES
                .iter()
                .map(|m| (m.as_str().to_string(), m.describe().to_string()))
                .collect(),
        ),
        "/crt" => Some(vec![
            ("on".to_string(), "force the CRT tube look on".to_string()),
            ("off".to_string(), "force the CRT tube look off".to_string()),
            (
                "auto".to_string(),
                "follow the theme (on for crt-* themes)".to_string(),
            ),
        ]),
        "/weight" => Some(vec![
            ("normal".to_string(), "400 — regular".to_string()),
            ("medium".to_string(), "500 — the old default".to_string()),
            ("semibold".to_string(), "600 — the new default".to_string()),
            (
                "bold".to_string(),
                "700 — thickest for body text".to_string(),
            ),
        ]),
        // The ladder only. A custom pair (`/gradient #a #b`) is freeform by
        // nature — there is no closed set of colours — and typing one still
        // submits, the way any unlisted value does.
        "/gradient" => Some(vec![
            ("off".to_string(), "the theme's colour, fixed".to_string()),
            (
                "subtle".to_string(),
                "the default — the colour leans \u{b1}16\u{b0}".to_string(),
            ),
            (
                "lively".to_string(),
                "a wide breath — \u{b1}38\u{b0}".to_string(),
            ),
            (
                "reset".to_string(),
                "drop a custom pair, back to the theme's poles".to_string(),
            ),
        ]),
        "/smooth" => Some(vec![
            (
                "off".to_string(),
                "0 — raw outlines, no darkening".to_string(),
            ),
            ("light".to_string(), "60 — a hint of fullness".to_string()),
            (
                "medium".to_string(),
                format!(
                    "{} — the default, Terminal.app-like",
                    crew_render::DEFAULT_SMOOTH
                ),
            ),
            (
                "heavy".to_string(),
                "170 — thick, high-contrast".to_string(),
            ),
        ]),
        // Model picker for the agent smith pane — the catalog grouped by
        // provider (see `modelpick`), applied to every agent (forwarded as
        // `/model all <slug>`). Any other slug still works: type it freeform
        // after `/model `. The value picker only takes flat (value, desc)
        // pairs, so section headers are dropped here (`.filter` below) for
        // this input-bar surface; the composer's `Kind::Model` popup
        // (chatpalette) is where the grouped sections actually render.
        "/model" => Some(
            crate::modelpick::rows("", None)
                .into_iter()
                .filter(|i| !i.header)
                .map(|i| (i.fill, i.desc))
                .collect(),
        ),
        _ => None,
    }
}

/// Whether `cmd` expands into a value picker rather than running on Enter.
/// Kept separate from `options_for` so the palette can answer "does this
/// expand?" for every matched command without building any rows — `/model`'s
/// arm builds the whole catalog, which used to happen on every keystroke.
/// Must stay in lockstep with `options_for`'s arms; `suggestvalues_tests` walks
/// `COMMANDS` and asserts the two never disagree.
pub(crate) fn expands(cmd: &str) -> bool {
    matches!(
        cmd,
        "/theme" | "/crt" | "/weight" | "/smooth" | "/gradient" | "/model"
    )
}

#[cfg(test)]
#[path = "suggestvalues_tests.rs"]
mod tests;
