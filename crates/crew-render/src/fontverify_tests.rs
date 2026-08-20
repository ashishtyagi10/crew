//! The point of these tests is that a *proportional* face must be rejected.
//! Asserting only that a good font passes would leave the check free to return
//! `true` unconditionally — which is exactly the bug it exists to catch.
use super::*;
use crate::embedfont::font_system;
use glyphon::cosmic_text::fontdb;
use std::sync::Arc;

const SIZE: f32 = 16.0;
const CELL_W: f32 = 10.0; // (16 * 0.6).round()

/// Load one extra face by path into a FontSystem that already has the embedded
/// family. Returns None when the file is absent, so the suite still runs on a
/// machine without it.
fn with_face(path: &str) -> Option<(FontSystem, String)> {
    let bytes = std::fs::read(path).ok()?;
    let mut fs = font_system();
    let ids = fs
        .db_mut()
        .load_font_source(fontdb::Source::Binary(Arc::new(bytes)));
    let id = *ids.first()?;
    let name = fs.db().face(id)?.families[0].0.clone();
    Some((fs, name))
}

#[test]
fn the_embedded_family_snaps_to_every_cell() {
    let mut fs = font_system();
    assert!(
        snaps_to_cells(&mut fs, crew_theme::EMBEDDED_FAMILY, SIZE, CELL_W, 600),
        "the face crew ships fails its own grid check — every fallback path \
         ends here, so nothing would render correctly"
    );
}

/// The failing case, built rather than hoped for: a genuinely proportional
/// face must be refused. Without this, `snaps_to_cells` returning `true`
/// always would look perfectly healthy.
#[test]
fn a_proportional_face_is_refused() {
    // Any of these is proportional; take whichever the machine has.
    let candidates = [
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
    ];
    let Some((mut fs, name)) = candidates.iter().find_map(|p| with_face(p)) else {
        eprintln!("no proportional face on this machine — check skipped");
        return;
    };
    assert!(
        !snaps_to_cells(&mut fs, &name, SIZE, CELL_W, 600),
        "{name:?} is proportional but passed the grid check — this is the \
         Windows bug: its narrow glyphs round to a ZERO advance and stack on \
         their neighbours"
    );
}

/// A name nothing resolves to must fail rather than silently pass. This is the
/// shape of the reported bug: crew logged `font → Noto San…` while drawing in
/// something else, because the name did not resolve at shape time.
#[test]
fn a_family_that_does_not_resolve_is_refused() {
    let mut fs = font_system();
    assert!(
        !snaps_to_cells(&mut fs, "No Such Family At All 12345", SIZE, CELL_W, 600),
        "an unresolvable family passed — crew would apply it, log it, and then \
         draw in whatever the platform substituted"
    );
    assert!(
        !snaps_to_cells(&mut fs, "", SIZE, CELL_W, 600),
        "the empty family must be refused, not treated as generic monospace"
    );
}

/// A real fixed-pitch face other than the embedded one must still pass, or the
/// check would reject everything and pin every machine to Lilex.
#[test]
fn a_genuine_monospace_face_still_passes() {
    let path = "/private/tmp/claude-501/-Users-atyagi-code-crew/a65242fb-67f9-447a-bc1d-5556ffdcca1e/scratchpad/NotoSansMono.ttf";
    let Some((mut fs, name)) = with_face(path) else {
        eprintln!("Noto Sans Mono not staged — check skipped");
        return;
    };
    assert!(
        snaps_to_cells(&mut fs, &name, SIZE, CELL_W, 600),
        "{name:?} is fixed-pitch but was refused — the check is too strict and \
         would strip real fonts out of the picker"
    );
}
