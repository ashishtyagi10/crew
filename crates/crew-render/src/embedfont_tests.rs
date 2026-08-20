//! These tests must hold on a machine with **no fonts installed at all** —
//! that is the case the shipped app broke on. Anything that queries the host's
//! font database instead proves nothing off the dev machine, which is exactly
//! how the Windows regression survived: the guard test accepted `Menlo`.
use super::*;
use glyphon::cosmic_text::fontdb;
use glyphon::Family;

/// A database holding ONLY the embedded faces — a bare machine, simulated, so
/// the assertions below cannot be satisfied by something the host happens to
/// have installed.
fn bare_db() -> fontdb::Database {
    let mut db = fontdb::Database::new();
    for src in sources() {
        db.load_font_source(src);
    }
    db
}

#[test]
fn all_eight_faces_parse_and_register_under_one_family() {
    let db = bare_db();
    assert_eq!(
        db.len(),
        EMBEDDED.len(),
        "{} of {} embedded faces failed to parse",
        EMBEDDED.len() - db.len(),
        EMBEDDED.len()
    );
    for face in db.faces() {
        let fam = &face.families[0].0;
        assert_eq!(
            fam,
            crew_theme::EMBEDDED_FAMILY,
            "an embedded face reports family {fam:?}, but the whole scheme \
             hangs on them all answering to {:?}",
            crew_theme::EMBEDDED_FAMILY
        );
    }
}

/// crew asks for weights across 300–900 (`/weight`) and always 700 for bold
/// cells. `fontdb` has no `fvar` support, so each of those must land on a
/// *distinct* face or every weight would render identically.
#[test]
fn the_weight_range_maps_onto_distinct_faces() {
    let db = bare_db();
    let pick = |w: u16| {
        db.query(&fontdb::Query {
            families: &[fontdb::Family::Name(crew_theme::EMBEDDED_FAMILY)],
            weight: fontdb::Weight(w),
            ..Default::default()
        })
        .unwrap_or_else(|| panic!("weight {w} resolved to no face at all"))
    };
    // 300 and 800/900 are clamped by CSS matching onto the nearest embedded
    // weight; the four crew actually ships must stay distinguishable.
    let ids: Vec<_> = [400, 500, 600, 700].map(pick).to_vec();
    let mut uniq = ids.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(
        uniq.len(),
        4,
        "400/500/600/700 collapse onto {} face(s) — /weight would do nothing",
        uniq.len()
    );
    assert!(
        db.query(&fontdb::Query {
            families: &[fontdb::Family::Name(crew_theme::EMBEDDED_FAMILY)],
            weight: fontdb::Weight(500),
            style: fontdb::Style::Italic,
            ..Default::default()
        })
        .is_some_and(|id| !ids.contains(&id)),
        "italic at the base weight resolves to an upright face — italic runs \
         would render upright"
    );
}

/// The bug, stated as a test: `Family::Monospace` must resolve.
///
/// cosmic-text maps it to the family name in the database's monospace slot,
/// which it sets to `"Noto Sans Mono"` — absent on Windows. The miss sent
/// shaping into the platform common fallback (`Segoe UI`, proportional), and
/// `set_monospace_width` then rounded its narrow advances to zero.
#[test]
fn generic_monospace_resolves_on_a_machine_with_no_fonts_installed() {
    let mut db = bare_db();
    // Reproduce what cosmic-text does to a fresh database…
    db.set_monospace_family("Noto Sans Mono");
    assert!(
        db.query(&fontdb::Query {
            families: &[fontdb::Family::Monospace],
            ..Default::default()
        })
        .is_none(),
        "'Noto Sans Mono' is somehow present — this test no longer reproduces \
         the bare-machine case it exists to cover"
    );
    // …and then what `font_system` does about it.
    db.set_monospace_family(crew_theme::EMBEDDED_FAMILY);
    let id = db.query(&fontdb::Query {
        families: &[fontdb::Family::Monospace],
        ..Default::default()
    });
    assert!(
        id.is_some(),
        "generic monospace resolves to nothing — shaping would fall through \
         to the platform's proportional common fallback"
    );
}

/// …and that the shipped constructor is the one that applies the fix, not just
/// this test's hand-built database.
#[test]
fn font_system_registers_the_embedded_family_as_generic_monospace() {
    let fs = font_system();
    let id = fs.db().query(&fontdb::Query {
        families: &[fontdb::Family::Monospace],
        ..Default::default()
    });
    let id = id.expect("font_system() leaves generic monospace unresolvable");
    let fam = fs.db().face(id).map(|f| f.families[0].0.clone());
    assert_eq!(
        fam.as_deref(),
        Some(crew_theme::EMBEDDED_FAMILY),
        "generic monospace resolves to {fam:?} rather than the embedded family"
    );
}

/// The embedded face has to survive `fontlist`'s own policy — it measures
/// candidates and drops anything not fixed-pitch. A face crew ships but the
/// picker hides would leave `/font` and the rotation pool empty on a bare box.
#[test]
fn the_embedded_family_passes_the_fixed_pitch_measurement() {
    let mut fs = font_system();
    let fams = crate::fontlist::monospace_families(&mut fs);
    assert!(
        fams.iter().any(|f| f == crew_theme::EMBEDDED_FAMILY),
        "{:?} is embedded but not offered by the font picker",
        crew_theme::EMBEDDED_FAMILY
    );
}

/// Shaping a plain ASCII line must produce one glyph per character with no
/// fallback: proof the grid renders from the embedded face rather than
/// borrowing glyphs from whatever the platform substitutes.
#[test]
fn ascii_shapes_to_one_glyph_per_cell_from_the_embedded_face() {
    use glyphon::{Attrs, Buffer, Metrics, Shaping};
    let mut fs = font_system();
    let mut buf = Buffer::new(&mut fs, Metrics::new(16.0, 20.0));
    let text = "crew |il.'1 ─│┌┐";
    buf.set_text(
        &mut fs,
        text,
        &Attrs::new().family(Family::Name(crew_theme::EMBEDDED_FAMILY)),
        Shaping::Advanced,
        None,
    );
    buf.shape_until_scroll(&mut fs, false);
    let run = buf.layout_runs().next().expect("no layout run");
    let missing: Vec<char> = run
        .glyphs
        .iter()
        .filter(|g| g.glyph_id == 0)
        .filter_map(|g| text[g.start..g.end].chars().next())
        .collect();
    assert!(
        missing.is_empty(),
        "the embedded face has no glyph for {missing:?} — those cells would \
         fall back to another font with a different advance"
    );
    let adv: Vec<f32> = run.glyphs.iter().map(|g| g.w).collect();
    let first = adv[0];
    assert!(
        adv.iter().all(|w| (w - first).abs() < 0.01),
        "advances differ across the run ({adv:?}) — the face is not \
         fixed-pitch as shaped"
    );
}
/// The embedded face's natural advance must match the cell box crew draws.
///
/// `cell_metrics` hardcodes `cell_w = font_size * 0.6` and is deliberately
/// family-independent, and `set_monospace_width` then snaps every advance to
/// that box. A face designed to a different ratio still *renders* — it just
/// sits loose or cramped in every cell, which is the sort of thing that gets
/// noticed as "the font looks slightly wrong" and never traced back here. So
/// pin it: swapping the embedded face for one at a different ratio should
/// fail loudly rather than quietly degrade the grid.
#[test]
fn the_embedded_face_is_drawn_to_crew_s_own_cell_ratio() {
    use glyphon::{Attrs, Buffer, Metrics, Shaping};
    let mut fs = font_system();
    for size in [16.0f32, 32.0] {
        let mut buf = Buffer::new(&mut fs, Metrics::new(size, size * 1.25));
        buf.set_text(
            &mut fs,
            "mmmm",
            &Attrs::new().family(Family::Name(crew_theme::EMBEDDED_FAMILY)),
            Shaping::Advanced,
            None,
        );
        buf.shape_until_scroll(&mut fs, false);
        let run = buf.layout_runs().next().expect("no layout run");
        let ratio = run.glyphs[0].w / size;
        assert!(
            (ratio - crate::celltext::CELL_W_RATIO).abs() < 0.005,
            "the embedded face advances at {ratio} of the font size, but crew \
             draws cells at {} — every glyph would be snapped loose or cramped",
            crate::celltext::CELL_W_RATIO
        );
    }
}
