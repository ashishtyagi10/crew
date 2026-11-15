use super::*;

/// The faces measured with CoreText on 2026-11-15, `(name, x_height, advance)`.
const MEASURED: [(&str, f32, f32); 9] = [
    ("JetBrains Mono", 0.550, 0.600),
    ("Lilex", 0.516, 0.600),
    ("IBM Plex Mono", 0.516, 0.600),
    ("Geist Mono", 0.538, 0.600),
    ("Comic Shanns Mono", 0.450, 0.550),
    ("Comic Mono", 0.485, 0.545),
    ("MonoLisa", 0.550, 0.640),
    ("SF Mono", 0.529, 0.618),
    ("Fira Code", 0.541, 0.615),
];

fn f(name: &str) -> f32 {
    let (_, xh, adv) = MEASURED.iter().find(|m| m.0 == name).unwrap();
    factor(*xh, *adv, CELL_W_RATIO)
}

#[test]
fn the_faces_crew_was_tuned_on_do_not_move() {
    for name in ["JetBrains Mono", "Lilex", "IBM Plex Mono", "Geist Mono"] {
        assert_eq!(f(name), 1.0, "{name}");
    }
}

#[test]
fn a_short_face_grows_toward_the_pack_but_never_past_its_cell() {
    for name in ["Comic Shanns Mono", "Comic Mono"] {
        let (_, xh, adv) = MEASURED.iter().find(|m| m.0 == name).unwrap();
        let k = f(name);
        assert!(k > 1.08, "{name} grows: {k}");
        assert!(adv * k <= CELL_W_RATIO + 1e-6, "{name} fits its cell");
        assert!(xh * k <= TARGET_X_HEIGHT + 1e-6, "{name} not past the pack");
    }
}

#[test]
fn a_face_wider_than_the_cell_is_drawn_into_it() {
    for name in ["MonoLisa", "SF Mono", "Fira Code"] {
        let (_, _, adv) = MEASURED.iter().find(|m| m.0 == name).unwrap();
        let k = f(name);
        assert!(k < 1.0, "{name} shrinks: {k}");
        assert!(
            (adv * k - CELL_W_RATIO).abs() < 1e-5,
            "{name} fills the cell"
        );
    }
}

#[test]
fn the_spread_of_apparent_sizes_narrows() {
    let size = |adjust: bool| -> (f32, f32) {
        MEASURED
            .iter()
            .fold((f32::MAX, 0.0f32), |(lo, hi), (n, xh, _)| {
                let x = xh * if adjust { f(n) } else { 1.0 };
                (lo.min(x), hi.max(x))
            })
    };
    let (lo0, hi0) = size(false);
    let (lo1, hi1) = size(true);
    assert!(
        hi1 / lo1 < 0.6 * (hi0 / lo0 - 1.0) + 1.0,
        "{lo0}-{hi0} → {lo1}-{hi1}"
    );
}

#[test]
fn nonsense_metrics_leave_the_size_alone() {
    assert_eq!(factor(0.0, 0.6, 0.6), 1.0);
    assert_eq!(factor(0.5, 0.0, 0.6), 1.0);
    assert_eq!(factor(f32::NAN, 0.6, 0.6), 1.0);
}

#[test]
fn the_embedded_face_reads_its_own_tables_and_keeps_its_size() {
    let mut fs = crate::embedfont::font_system();
    let (xh, adv) = measure(&mut fs, None, 500).expect("Lilex is embedded");
    assert!((adv - 0.6).abs() < 0.005, "advance {adv}");
    assert!((xh - 0.516).abs() < 0.02, "x-height {xh}");
    assert_eq!(scale(&mut fs, None, 500), 1.0);
    assert_eq!(scale(&mut fs, Some("No Such Face"), 500), 1.0);
}
