//! What a full pane of joining script costs to build, against English.
use super::*;
use crate::celltext::build_pane_text;
use crate::scriptrun::tests::{cells, params};

/// What a full pane of Hindi costs against one of English, cold and with
/// the run cache warm. Prints; the goal's record keeps the numbers.
/// `cargo test --release -p crew-render scriptrun_cost -- --ignored --nocapture`
#[test]
#[ignore = "timing; prints"]
fn scriptrun_cost() {
    let p = params();
    let (cols, rows) = (120usize, 40usize);
    let pane = |line: &str| -> Vec<crate::cellgrid::CellView> {
        (0..rows as u16)
            .flat_map(|r| {
                cells(&line.repeat(8), r)
                    .into_iter()
                    .filter(|c| usize::from(c.col) < cols)
            })
            .collect()
    };
    let hindi = pane("भारत ने आज नया उपग्रह भेजा ");
    let english = pane("India sent a new satellite ");
    let mut fs = crate::embedfont::font_system();
    let (w, h) = (cols as f32 * p.cell_w, rows as f32 * p.line_height);
    let blank = pane("                           ");
    let runs = [
        ("english", &english),
        ("english", &english),
        ("blank", &blank),
        ("hindi", &hindi),
        ("hindi warm", &hindi),
    ];
    for (name, cs) in runs {
        let t = std::time::Instant::now();
        let (buf, specs) = build_pane_text(&mut fs, cs, cols, rows, w, h, &p);
        let built = t.elapsed();
        let areas = place(&mut fs, &buf, &specs, &p);
        let placed = t.elapsed() - built;
        eprintln!(
            "{name}: grid {built:?} + runs {placed:?}, {} areas",
            areas.len()
        );
    }
}
