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

/// The form shows the colours it is offering, the same way the command
/// palette's pickers do — reading `harbor` and pressing Save to find out what
/// it looks like is the form failing at its job.
#[test]
fn the_theme_field_draws_the_palettes_it_would_rotate() {
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    p.draft.theme = Some("dark".to_string());
    assert!(
        value_of(&p, Field::Theme).0.contains("dark"),
        "the fixture does not show the theme under test"
    );
    let cells = super::render(&p, 120, 40);
    let chips = crate::swatch::for_value("/theme", "dark");
    assert!(!chips.is_empty());
    for chip in &chips {
        assert!(
            cells
                .iter()
                .any(|c| c.c == chip.c && c.fg == chip.fg && c.bg == chip.bg.unwrap()),
            "no chip drawn for {chip:?}"
        );
    }
}

/// A hex accent is its own chip, drawn inside its box.
#[test]
fn a_hex_accent_draws_its_colour() {
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    p.accent_buf = "#ff8800".into();
    let cells = super::render(&p, 120, 40);
    assert!(
        cells
            .iter()
            .any(|c| c.c == '\u{2588}' && c.fg == (255, 136, 0)),
        "the accent's own colour is not shown"
    );
}

/// The chips answer to the VALUE, not to the field: a theme picker showing
/// something that names no palette draws nothing rather than the last thing
/// that did.
#[test]
fn a_field_that_is_not_a_colour_draws_no_chips() {
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    p.draft.theme = Some("not-a-palette".to_string());
    let with_theme = super::render(&p, 120, 40);
    let chip_count = |cells: &[crew_render::CellView]| {
        // `▀` is the palette chip; `█` is the text caret, which is not one.
        cells.iter().filter(|c| c.c == '\u{2580}').count()
    };
    // The two auto-pairing pickers still name real pools, so what must drop
    // is the Theme field's own chips — count them by difference.
    p.draft.theme = Some("dark".to_string());
    let real = chip_count(&super::render(&p, 120, 40));
    assert!(real > chip_count(&with_theme), "a bogus theme drew chips");
}

/// The one colour a person picks by hand is the one nobody was measuring.
#[test]
fn a_hand_picked_accent_shows_how_it_reads_on_the_page() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let text = |hex: &str| -> String {
        let mut p = pane();
        p.accent_buf = hex.into();
        let cells = super::render(&p, 120, 40);
        let mut v: Vec<&crew_render::CellView> = cells.iter().collect();
        v.sort_by_key(|c| (c.row, c.col));
        v.iter().map(|c| c.c).collect()
    };
    // A bright accent on the near-black page reads well…
    let bright = text("#ffd166");
    assert!(bright.contains(":1"), "no contrast readout: {bright}");
    // …and the number is the real one.
    let cr = crew_theme::contrast_ratio((255, 209, 102), crew_theme::theme().page_bg);
    assert!(bright.contains(&format!("{cr:.1}:1")), "{bright}");
}

/// Below the floor every derived role is held to, the number is drawn in the
/// alarm colour: an accent that cannot be read is the mistake this field
/// makes easy.
#[test]
fn an_unreadable_accent_is_flagged_rather_than_merely_reported() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let flagged = |hex: &str| -> bool {
        let mut p = pane();
        p.accent_buf = hex.into();
        let bell = crew_theme::theme().bell;
        super::render(&p, 120, 40)
            .iter()
            .any(|c| c.c == ':' && c.fg == bell)
    };
    assert!(flagged("#101014"), "a near-black accent went unflagged");
    assert!(!flagged("#ffd166"), "a readable accent was flagged");
}

/// No hex, no number — the field is empty when the accent follows the theme.
#[test]
fn an_empty_accent_field_shows_no_measurement() {
    let _g = crate::app::theme_test_guard();
    let p = pane();
    let cells = super::render(&p, 120, 40);
    let text: String = cells.iter().map(|c| c.c).collect();
    assert!(!text.contains(":1"), "{text}");
}
