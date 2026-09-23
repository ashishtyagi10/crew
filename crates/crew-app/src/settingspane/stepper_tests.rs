use super::*;
use crate::config::{CrewConfig, MIN_WINDOW_OPACITY};
use crate::settingspane::FIELDS;

fn pane(f: Field) -> SettingsPane {
    let mut p = SettingsPane::new(CrewConfig::default(), vec!["Lilex".into()]);
    p.focus = FIELDS.iter().position(|&g| g == f).unwrap();
    p
}

#[test]
fn up_and_down_step_opacity_by_five_percent() {
    let mut p = pane(Field::WindowOpacity);
    p.draft.window_opacity = 0.80;
    crate::settingspane::commit::refresh_bufs(&mut p);
    assert!(step(&mut p, true, false));
    assert_eq!(p.opacity_buf, "85");
    assert!((p.draft.window_opacity - 0.85).abs() < 1e-6);
    step(&mut p, false, false);
    step(&mut p, false, false);
    assert_eq!(p.opacity_buf, "75");
}

#[test]
fn a_step_past_a_bound_stops_at_the_bound() {
    let mut p = pane(Field::WindowOpacity);
    step(&mut p, true, true); // from 100: already at the top
    assert_eq!(p.opacity_buf, "100");
    for _ in 0..30 {
        step(&mut p, false, false);
    }
    assert!((p.draft.window_opacity - MIN_WINDOW_OPACITY).abs() < 1e-6);

    let mut p = pane(Field::FontSize);
    for _ in 0..5 {
        step(&mut p, true, true);
    }
    assert_eq!(p.draft.font_size, 32.0);
}

#[test]
fn grain_steps_in_tenths_without_float_dust() {
    let mut p = pane(Field::PaperGrain);
    p.draft.paper_grain = 1.3;
    crate::settingspane::commit::refresh_bufs(&mut p);
    step(&mut p, true, false);
    assert_eq!(p.grain_buf, "1.4");
    assert!((p.draft.paper_grain - 1.4).abs() < 1e-6);
}

#[test]
fn a_half_typed_value_is_stepped_from_what_it_says() {
    let mut p = pane(Field::FontSize);
    p.size_buf = "20".into();
    step(&mut p, true, false);
    assert_eq!(p.draft.font_size, 21.0);
}

#[test]
fn a_field_that_is_not_a_number_is_left_alone() {
    for f in [
        Field::Accent,
        Field::LightFrom,
        Field::NotifyPatterns,
        Field::Theme,
    ] {
        let mut p = pane(f);
        assert!(!step(&mut p, true, false), "{f:?} stepped");
    }
}
