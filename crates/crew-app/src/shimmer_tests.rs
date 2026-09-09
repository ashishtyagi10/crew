use super::*;
use crate::motion::MotionLevel;

const BASE: Color = (120, 120, 120);
const HI: Color = (255, 255, 255);
const PAGE: Color = (0, 0, 0);
const WORD: &str = "thinking";

/// Index of the most-lit cell.
fn brightest(cells: &[(char, Color)]) -> usize {
    cells
        .iter()
        .enumerate()
        .max_by_key(|(_, (_, fg))| fg.0)
        .map(|(i, _)| i)
        .unwrap()
}

#[test]
fn the_window_moves_left_to_right_with_time() {
    // A quarter, half and three quarters of the way through one sweep: the
    // brightest cell walks rightward, and it is a window — a handful of
    // cells lit, the rest at base — not a wash over the whole word.
    let q = |t| cells(WORD, t, BASE, HI, PAGE, SHIMMER_MS, MotionLevel::Full);
    let (a, b, c) = (q(SHIMMER_MS / 4), q(SHIMMER_MS / 2), q(SHIMMER_MS * 3 / 4));
    assert!(
        brightest(&a) < brightest(&b) && brightest(&b) < brightest(&c),
        "{} {} {}",
        brightest(&a),
        brightest(&b),
        brightest(&c)
    );
    let lit = b.iter().filter(|(_, fg)| *fg != BASE).count();
    assert!((1..=5).contains(&lit), "window, not a wash: {lit} lit");
    assert_eq!(b.iter().map(|(c, _)| *c).collect::<String>(), WORD);
}

#[test]
fn the_sweep_wraps_every_period_and_subtle_runs_slower() {
    let at = |t, l| cells(WORD, t, BASE, HI, PAGE, SHIMMER_MS, l);
    assert_eq!(
        at(700, MotionLevel::Full),
        at(700 + SHIMMER_MS, MotionLevel::Full)
    );
    let subtle = period(SHIMMER_MS, MotionLevel::Subtle);
    assert_eq!(subtle, SHIMMER_MS * 8 / 5);
    assert_eq!(
        at(700, MotionLevel::Subtle),
        at(700 + subtle, MotionLevel::Subtle)
    );
    // Half a Full period in, the Subtle sweep is still on the left half.
    assert!(
        brightest(&at(SHIMMER_MS / 2, MotionLevel::Subtle))
            < brightest(&at(SHIMMER_MS / 2, MotionLevel::Full))
    );
}

#[test]
fn off_is_flat_base() {
    assert_eq!(period(SHIMMER_MS, MotionLevel::Off), 0);
    for t in [0, 350, 700, 1050, 5_000] {
        let c = cells(WORD, t, BASE, HI, PAGE, SHIMMER_MS, MotionLevel::Off);
        assert!(c.iter().all(|(_, fg)| *fg == BASE), "t={t}: {c:?}");
    }
    assert!(cells("", 700, BASE, HI, PAGE, SHIMMER_MS, MotionLevel::Full).is_empty());
}

/// The blend of two readable colours is not itself guaranteed readable —
/// so every cell of every sweep on every preset is measured.
#[test]
fn every_shimmer_cell_clears_the_text_floor_on_every_preset() {
    let _g = crate::app::theme_test_guard();
    let floor = crew_theme::contrast::text_floor();
    let mut presets = 0;
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let t = crew_theme::theme();
        let hi = crate::palette::accent();
        for now in (0..SHIMMER_MS).step_by(35) {
            let c = cells(
                WORD,
                now,
                t.text_muted,
                hi,
                t.page_bg,
                SHIMMER_MS,
                MotionLevel::Full,
            );
            for (ch, fg) in c {
                let r = crew_theme::contrast_ratio(fg, t.page_bg);
                assert!(
                    r >= floor,
                    "{}: '{ch}' at {now}ms reads {r:.2} (need {floor})",
                    id.as_str()
                );
            }
        }
        presets += 1;
    }
    assert_eq!(presets, crew_theme::ALL_THEMES.len());
}

#[test]
fn pulse_mix_is_lit_on_the_burst_and_decays_to_nothing() {
    let full = MotionLevel::Full;
    assert_eq!(pulse_mix(None, 1_000, full), 0.0);
    assert_eq!(pulse_mix(Some(1_000), 1_000, full), 1.0);
    let (a, b) = (
        pulse_mix(Some(1_000), 1_050, full),
        pulse_mix(Some(1_000), 1_150, full),
    );
    assert!(1.0 > a && a > b && b > 0.0, "{a} {b}");
    assert_eq!(pulse_mix(Some(1_000), 1_000 + PULSE_MS, full), 0.0);
    // Ease-out: most of the drop happens early.
    assert!(a < 0.6, "fast departure: {a}");
    assert_eq!(pulse_mix(Some(1_000), 1_010, MotionLevel::Off), 0.0);
    // Subtle runs the same shape over 60% of the time.
    assert_eq!(pulse_mix(Some(1_000), 1_150, MotionLevel::Subtle), 0.0);
    assert!(pulse_mix(Some(1_000), 1_100, MotionLevel::Subtle) > 0.0);
}

#[test]
fn breath_is_static_at_off_and_cycles_otherwise() {
    for t in [0, 800, 1_600, 4_000] {
        assert_eq!(breath(t, MotionLevel::Off), 1.0);
    }
    assert!((breath(0, MotionLevel::Full) - 1.0).abs() < 1e-5);
    assert!(breath(BREATH_MS / 2, MotionLevel::Full).abs() < 1e-5);
    assert!((breath(BREATH_MS, MotionLevel::Full) - 1.0).abs() < 1e-5);
    assert!(breath(BREATH_SUBTLE_MS / 2, MotionLevel::Subtle).abs() < 1e-5);
    for t in (0..10_000).step_by(97) {
        let b = breath(t, MotionLevel::Full);
        assert!((0.0..=1.0).contains(&b), "t={t}: {b}");
    }
}

#[test]
fn breath_colour_never_drops_under_the_mark_floor() {
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let t = crew_theme::theme();
        for now in (0..BREATH_MS).step_by(100) {
            let c = breath_color(now, MotionLevel::Full, t.dim, t.activity, t.page_bg);
            let r = crew_theme::contrast_ratio(c, t.page_bg);
            assert!(
                r >= crew_theme::readable::MARK_FLOOR,
                "{}: dot at {now}ms reads {r:.2}",
                id.as_str()
            );
        }
        assert_eq!(
            breath_color(0, MotionLevel::Off, t.dim, t.activity, t.page_bg),
            t.activity,
            "{}: Off is the lit dot",
            id.as_str()
        );
    }
}
