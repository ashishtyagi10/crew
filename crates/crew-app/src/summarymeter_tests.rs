use super::*;
use crate::metertrack::{as_drawn, floor as trough_floor};

/// The trough is a groove, not a decoration: it says how long the meter is,
/// which is what makes a fill a fraction of something. A fixed fade toward
/// the page gives every theme a different answer — on the dark pages it
/// landed at 1.6:1 and on the light ones at 1.3:1, where it stopped being
/// there at all.
#[test]
fn the_trough_reads_on_every_page() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let mut thin: Vec<String> = Vec::new();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let page = crew_theme::theme().page_bg;
        let r = crew_theme::contrast_ratio(as_drawn(meter_track(), page), page);
        if r < trough_floor() - 0.01 {
            thin.push(format!("{id:?}: {r:.2}"));
        }
    }
    assert!(
        thin.is_empty(),
        "troughs that vanish into the page: {thin:?}"
    );
}

/// …and never louder than what it holds. A trough that outshines its own
/// fill reads as a full meter.
#[test]
fn the_trough_never_outshines_the_fill() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let page = crew_theme::theme().page_bg;
        let trough = crew_theme::contrast_ratio(as_drawn(meter_track(), page), page);
        for t in [0.0, 0.5, 1.0] {
            let fill = crew_theme::contrast_ratio(meter_shade(t), page);
            assert!(
                fill > trough,
                "{id:?}: the trough ({trough:.2}) is louder than the fill at {t} ({fill:.2})"
            );
        }
    }
}
