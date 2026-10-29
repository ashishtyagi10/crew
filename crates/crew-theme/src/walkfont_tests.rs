use super::*;
use crate::ALL_THEMES;

/// `auto` walks some looks one palette per step (`autowalk::WALKED`); each
/// of those steps must change the face too, or the walk is the old "theme
/// rotates but the font never changes" bug inside one look.
#[test]
fn a_walked_look_changes_the_face_at_every_step() {
    for look in crate::auto_looks().filter(|&m| crate::auto_steps(m) > 1) {
        let leads: Vec<&str> = ALL_THEMES
            .into_iter()
            .filter(|id| id.mode() == look)
            .map(|id| font_prefs(id)[0])
            .collect();
        let mut distinct = leads.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            leads.len(),
            "{look:?} repeats a lead: {leads:?}"
        );
    }
}

/// Every step of an `auto` lap changes the face, not only the steps inside a
/// walked look: `glass` hands over to `neon`, and when glass landed on
/// glass-clear or glass-night (SF Mono) and neon led with SF Mono too, that
/// step changed the colours and never the font. One pair is exempt by
/// design: `dark` to `light` is the same Ink Reader page inverted, and keeps
/// its face the way it keeps its layout.
#[test]
fn every_step_of_an_auto_lap_changes_the_face() {
    let lead = |id: ThemeId| font_prefs(id)[0];
    let mut steps = 0;
    for from in ALL_THEMES {
        let next: Vec<ThemeId> = match crate::auto_next_palette(from) {
            Some(to) => vec![to],
            None => ALL_THEMES
                .into_iter()
                .filter(|t| t.mode() == crate::auto_next(from))
                .collect(),
        };
        for to in next {
            if (from, to) == (ThemeId::PaperDark, ThemeId::PaperLight) {
                continue;
            }
            steps += 1;
            assert_ne!(
                lead(from),
                lead(to),
                "auto steps {} -> {} in one face",
                from.as_str(),
                to.as_str()
            );
        }
    }
    assert!(steps >= ALL_THEMES.len(), "only {steps} steps were checked");
}
