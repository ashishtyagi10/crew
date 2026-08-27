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
        // The four rotations first — they are what most people want — then
        // every individual palette under a heading. The palettes have always
        // PARSED; not offering them meant you had to already know the name of
        // the one you wanted, which is the opposite of what a picker is for.
        // The legacy `random-*` names still parse and are still not offered.
        "/theme" => Some(
            crew_theme::THEME_MODES
                .iter()
                .map(|m| (m.as_str().to_string(), m.describe().to_string()))
                // An empty value is a heading, not a choice (see
                // `suggest::menu_items`).
                .chain(std::iter::once((
                    String::new(),
                    "one palette, pinned".to_string(),
                )))
                .chain(
                    crew_theme::ALL_THEMES
                        .iter()
                        .map(|t| (t.as_str().to_string(), t.describe().to_string())),
                )
                .collect(),
        ),
        // `/copy` alone takes the whole scrollback; the one value it offers
        // is the narrower answer.
        "/copy" => Some(vec![(
            "out".to_string(),
            "just the last command's output, not the whole scrollback".to_string(),
        )]),
        "/marks" => Some(vec![
            (
                "on".to_string(),
                "the default \u{2014} ticks where a command began, bars beside errors".to_string(),
            ),
            (
                "off".to_string(),
                "a plain frame; the marks are still there to jump to".to_string(),
            ),
        ]),
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
        "/motion" => Some(vec![
            (
                "auto".to_string(),
                "the default \u{2014} follow the OS Reduce Motion switch".to_string(),
            ),
            (
                "off".to_string(),
                "no travel at all; every state draws settled".to_string(),
            ),
            (
                "subtle".to_string(),
                "the same choreography at 60% duration".to_string(),
            ),
            ("full".to_string(), "crew moves".to_string()),
        ]),
        "/shapes" => Some(vec![
            (
                "auto".to_string(),
                "the default \u{2014} follow the OS Differentiate Without Color switch".to_string(),
            ),
            ("off".to_string(), "colour carries it alone".to_string()),
            (
                "on".to_string(),
                "gauge tiers marked, a working pane gets its own glyph".to_string(),
            ),
        ]),
        "/contrast" => Some(vec![
            (
                "auto".to_string(),
                "the default \u{2014} follow the OS Increase Contrast switch".to_string(),
            ),
            (
                "normal".to_string(),
                "WCAG AA: 4.5 for text, 3.0 for marks".to_string(),
            ),
            (
                "high".to_string(),
                "WCAG AAA: 7.0 for text, 4.5 for marks; the wash backs off".to_string(),
            ),
        ]),
        "/density" => Some(vec![
            (
                "compact".to_string(),
                "a 4px gutter, no blank row between chat cards".to_string(),
            ),
            (
                "cozy".to_string(),
                "the default \u{2014} 8px, one blank row".to_string(),
            ),
            (
                "roomy".to_string(),
                "14px, two blank rows; easier on a big display".to_string(),
            ),
        ]),
        // The ladder only. A custom pair (`/gradient #a #b`) is freeform by
        // nature — there is no closed set of colours — and typing one still
        // submits, the way any unlisted value does.
        "/gradient" => Some(
            vec![
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
            ]
            .into_iter()
            // …then the shelf of named pairs under a heading of their own, so
            // the levels and the colours do not read as one list.
            .chain(std::iter::once((String::new(), "named pairs".to_string())))
            .chain(
                crew_theme::gradients::GRADIENTS
                    .iter()
                    .map(|g| (g.name.to_string(), g.about.to_string())),
            )
            .collect(),
        ),
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
        "/theme"
            | "/crt"
            | "/weight"
            | "/smooth"
            | "/marks"
            | "/copy"
            | "/gradient"
            | "/motion"
            | "/density"
            | "/contrast"
            | "/shapes"
            | "/model"
    )
}

/// The value a command is currently set to, so the picker can say which of
/// its rows is the one you are already on.
///
/// A closed-set picker that does not mark the current value asks you to
/// remember what you chose — which is the thing the picker exists to save you
/// from. `None` for commands whose "current" is not a single value.
pub(crate) fn current_value(cmd: &str, cfg: &crate::config::CrewConfig) -> Option<String> {
    let s = match cmd {
        "/theme" => cfg.theme.clone().unwrap_or_default(),
        "/gradient" => cfg
            .gradient_poles
            .clone()
            .and_then(|p| crew_theme::gradients::name_of(crate::gradientcmd::parse_poles(&p)?))
            .map(str::to_string)
            .unwrap_or_else(|| cfg.gradient.clone()),
        "/motion" => cfg.motion.clone(),
        "/density" => cfg.density.clone(),
        "/contrast" => cfg.contrast.clone(),
        "/shapes" => cfg.shape_cues.clone(),
        "/marks" => match cfg.border_marks {
            true => "on".to_string(),
            false => "off".to_string(),
        },
        _ => return None,
    };
    (!s.is_empty()).then_some(s)
}

#[cfg(test)]
#[cfg(test)]
#[path = "suggestvalues_tests.rs"]
mod tests;
