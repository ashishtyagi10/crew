use super::*;
use crate::config::CrewConfig;
use crate::settingspane::FIELDS;

fn pane(f: Field) -> SettingsPane {
    let mut p = SettingsPane::new(CrewConfig::default(), vec!["Lilex".into()]);
    p.focus = FIELDS.iter().position(|&g| g == f).unwrap();
    p
}

#[test]
fn a_step_past_a_bound_stops_at_the_bound() {
    let mut p = pane(Field::FontSize);
    for _ in 0..5 {
        step(&mut p, true, true);
    }
    assert_eq!(p.draft.font_size, 32.0);
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
