use super::*;
use crate::ALL_THEMES;

/// `auto` walks every look one palette per step; each of those steps must
/// change the face too, or the walk is the old "theme rotates but the font
/// never changes" bug inside one look. The tube is exempt: green to amber
/// is one tube with another phosphor, and its squared terminal face (the
/// one crew embeds) is part of the tube.
#[test]
fn a_walked_look_changes_the_face_at_every_step() {
    for look in
        crate::auto_looks().filter(|&m| crate::auto_steps(m) > 1 && m != crate::RandomMode::Crt)
    {
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
/// step changed the colours and never the font. Two pairs are exempt by
/// design: `dark` to `light` is the same Ink Reader page inverted, and keeps
/// its face the way it keeps its layout, and green to amber is the same
/// tube with another phosphor.
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
            if (from, to) == (ThemeId::PaperDark, ThemeId::PaperLight)
                || (from, to) == (ThemeId::CrtGreen, ThemeId::CrtAmber)
            {
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
    assert!(
        steps >= ALL_THEMES.len() - 2,
        "only {steps} steps were checked"
    );
}
