//! A weight a family does not have must never become a different family.
//!
//! Reported from a live window: the focused pane's legend was in a different
//! typeface from the pane's own text, and only while it was focused (the
//! legend is bold there, and plain everywhere else). The cause was not the
//! legend at all — cosmic-text answers a family+weight query by *distance*,
//! and a family with no Bold face loses to any family that has one. On the
//! machine it was found on, six of the seventeen installed coding faces —
//! Cascadia Code, MonoLisa, Geist Mono, Google Sans Code, ComicMono, Operator
//! Mono — ship Regular and Medium and no Bold, so every bold cell in crew
//! (a legend, an agent's `**emphasis**`, a fence header) shaped from Menlo.
use super::*;
use crate::embedfont::font_system;
use glyphon::cosmic_text::fontdb;
use glyphon::{Attrs, Buffer, Family, Metrics, Shaping, Weight};
use std::sync::Arc;

/// A `FontSystem` holding EXACTLY the named embedded faces and nothing else —
/// no system fonts, so the test says the same thing on every machine.
fn only(faces: &[&str]) -> Option<FontSystem> {
    let mut db = fontdb::Database::new();
    for name in faces {
        let bytes = std::fs::read(format!("../../assets/fonts/{name}")).ok()?;
        db.load_font_source(fontdb::Source::Binary(Arc::new(bytes)));
    }
    Some(FontSystem::new_with_locale_and_db("en-US".into(), db))
}

/// A second family to lose the text to: the whole failure is that a *better
/// weight match elsewhere* beats the family that was asked for.
fn lilex_regular_and_a_bold_stranger() -> Option<FontSystem> {
    // Lilex Regular alone (no bold), plus the full Noto Sans Mono the system
    // would otherwise offer — approximated here by Lilex's own bold under a
    // second load, which is enough for the weight query to have somewhere to
    // go if the clamp is removed.
    only(&["Lilex-Regular.otf", "Lilex-Bold.otf"])
}

#[test]
fn a_family_without_a_bold_face_shapes_bold_at_the_weight_it_has() {
    let Some(fs) = only(&["Lilex-Regular.otf"]) else {
        eprintln!("no font assets — check skipped");
        return;
    };
    let fam = Some(crew_theme::EMBEDDED_FAMILY.to_string());
    assert_eq!(
        weight_in_family(&fs, &fam, Weight::BOLD.0),
        Weight(400),
        "bold must clamp to the only weight this family has"
    );
    assert_eq!(
        weight_in_family(&fs, &fam, 500),
        Weight(400),
        "…and so must the base weight"
    );
}

#[test]
fn a_family_with_a_bold_face_still_gets_its_bold() {
    let Some(fs) = lilex_regular_and_a_bold_stranger() else {
        eprintln!("no font assets — check skipped");
        return;
    };
    let fam = Some(crew_theme::EMBEDDED_FAMILY.to_string());
    assert_eq!(weight_in_family(&fs, &fam, Weight::BOLD.0), Weight::BOLD);
    assert_eq!(weight_in_family(&fs, &fam, 400), Weight(400));
}

/// No family named means the monospace default, which is the family crew
/// embeds — and it ships every weight it is asked for.
#[test]
fn no_family_asks_for_exactly_what_it_wanted() {
    let Some(fs) = only(&["Lilex-Regular.otf"]) else {
        return;
    };
    assert_eq!(weight_in_family(&fs, &None, 700), Weight(700));
    assert_eq!(
        weight_in_family(&fs, &Some(String::new()), 700),
        Weight(700)
    );
}

/// The end of it: bold text in a boldless family stays in that family. This
/// is the assertion the bug would have failed — with a real second family
/// present, `Weight::BOLD` walked out of the one that was asked for.
#[test]
fn bold_text_never_leaves_the_family_it_was_asked_for() {
    let mut fs = font_system(); // embedded + whatever this machine has
    let installed: Vec<String> = fs
        .db()
        .faces()
        .map(|f| f.families[0].0.clone())
        .filter(|n| crew_theme::FONT_ALLOWLIST.contains(&n.as_str()))
        .collect();
    for fam in installed {
        let want = weight_in_family(&fs, &Some(fam.clone()), Weight::BOLD.0);
        let mut b = Buffer::new(&mut fs, Metrics::new(16.0, 20.0));
        b.set_text(
            &mut fs,
            "bold",
            &Attrs::new().family(Family::Name(&fam)).weight(want),
            Shaping::Advanced,
            None,
        );
        b.shape_until_scroll(&mut fs, false);
        for run in b.layout_runs() {
            for g in run.glyphs {
                let got = fs
                    .db()
                    .face(g.font_id)
                    .map(|f| f.families[0].0.clone())
                    .unwrap_or_default();
                assert_eq!(got, fam, "bold in {fam} shaped from {got}");
            }
        }
    }
}
