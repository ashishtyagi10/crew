use super::*;
use crate::config::CrewConfig;
use crate::settingspane::FIELDS;

fn pane() -> SettingsPane {
    SettingsPane::new(CrewConfig::default(), Vec::new())
}

fn row_text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| (c.col, c.c))
        .collect();
    v.sort_unstable();
    // Gap-preserving: pad to each cell's column (blank cells are not emitted).
    let mut s = String::new();
    for (col, c) in v {
        while s.chars().count() < col as usize {
            s.push(' ');
        }
        s.push(c);
    }
    s
}

fn dump(cells: &[CellView], rows: u16) -> String {
    (0..rows).map(|r| row_text(cells, r) + "\n").collect()
}

#[test]
fn every_field_renders_on_a_tall_pane() {
    // Tall enough for the whole form plus the gap and pinned button row,
    // computed rather than pinned at a number: this used to be a literal 30,
    // which the form outgrew the moment `auto` gained its pairing pickers —
    // and a test that fails because the form got bigger says nothing about
    // whether the new fields render.
    let rows = form::layout(80).height + 2;
    let cells = pane().cells(80, rows);
    let all = dump(&cells, rows);
    for f in FIELDS.iter().take(FIELDS.len() - 2) {
        assert!(
            all.contains(label_of(*f)),
            "missing field '{}' in:\n{all}",
            label_of(*f)
        );
    }
    assert!(all.contains("[ Save \u{2318}S ]"), "save button: {all}");
    assert!(all.contains("[ Cancel esc ]"), "cancel button: {all}");
}

#[test]
fn cards_have_legends() {
    let all = dump(&pane().cells(80, 30), 30);
    for t in ["APPEARANCE", "WINDOW", "NOTIFICATIONS"] {
        assert!(all.contains(t), "missing card '{t}' in:\n{all}");
    }
}

#[test]
fn focused_input_carries_cursor() {
    // Focus starts on FontFamily; its box content row carries the cursor.
    let all = dump(&pane().cells(80, 30), 30);
    assert!(all.contains('\u{2588}'), "cursor missing:\n{all}");
}

#[test]
fn short_pane_scrolls_to_keep_focus_visible() {
    let mut p = pane();
    p.focus = FIELDS
        .iter()
        .position(|&f| f == Field::NotifyPatterns)
        .unwrap();
    let cells = p.cells(80, 12);
    let all = dump(&cells, 12);
    assert!(
        all.contains("Watch patterns"),
        "focused field visible:\n{all}"
    );
    assert!(all.contains('\u{2191}'), "up hint expected:\n{all}");
}

#[test]
fn narrow_pane_still_renders_all_cards() {
    let all = dump(&pane().cells(48, 60), 60);
    for t in ["APPEARANCE", "WINDOW", "NOTIFICATIONS"] {
        assert!(all.contains(t), "missing card '{t}' in:\n{all}");
    }
}

#[test]
fn tiny_pane_renders_nothing() {
    assert!(pane().cells(10, 4).is_empty());
}

#[test]
fn smooth_value_names_the_level_or_shows_the_raw_number() {
    // Default strength is the ladder's `medium`.
    let (v, cursor) = value_of(&pane(), Field::Smooth);
    assert!(v.contains("medium"), "got: {v}");
    assert!(!cursor, "smoothing is a picker, not a text field");
    // A custom `/smooth 42` strength shows as its number, not a nearby name.
    let cfg = CrewConfig {
        font_smooth: 42,
        ..Default::default()
    };
    let (v, _) = value_of(&SettingsPane::new(cfg, Vec::new()), Field::Smooth);
    assert!(v.contains("42"), "got: {v}");
}

#[test]
fn theme_value_names_the_current_theme() {
    // An unset config labels as `auto` — the fresh-install default follows
    // the OS appearance.
    let (v, cursor) = value_of(&pane(), Field::Theme);
    assert!(v.contains("auto"), "got: {v}");
    assert!(!cursor, "theme is a picker, not a text field");
    // A saved mode keeps its own label — never relabeled to auto.
    let cfg = CrewConfig {
        theme: Some("dark".into()),
        ..Default::default()
    };
    let (v, _) = value_of(&SettingsPane::new(cfg, Vec::new()), Field::Theme);
    assert!(v.contains("dark"), "got: {v}");
}

#[test]
fn each_pairing_picker_shows_its_own_side() {
    // Distinct values, so a picker reading the wrong field is visible rather
    // than hidden behind two sides that happen to match.
    let cfg = CrewConfig {
        theme_dark: Some("crt".into()),
        theme_light: Some("blossom".into()),
        ..Default::default()
    };
    let p = SettingsPane::new(cfg, Vec::new());
    let (dark, cursor) = value_of(&p, Field::ThemeDark);
    assert!(dark.contains("crt"), "dark side shows: {dark}");
    assert!(
        !dark.contains("blossom"),
        "dark side shows the light one: {dark}"
    );
    assert!(!cursor, "the pairing is a picker, not a text field");
    let (light, _) = value_of(&p, Field::ThemeLight);
    assert!(light.contains("blossom"), "light side shows: {light}");
    assert!(
        !light.contains("crt"),
        "light side shows the dark one: {light}"
    );

    // An unset side reads as the built-in pairing, not as an empty box.
    let (v, _) = value_of(&pane(), Field::ThemeDark);
    assert!(v.contains("default"), "unset shows: {v}");
}
