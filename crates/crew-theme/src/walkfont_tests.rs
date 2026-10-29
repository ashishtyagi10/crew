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
