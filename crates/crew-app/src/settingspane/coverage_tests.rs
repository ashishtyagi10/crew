//! Does the form still cover the config?
//!
//! Split from `mod_tests.rs` along the line between how the form BEHAVES and
//! what it REACHES: this half reads `config.rs` itself, so every key is either
//! editable here or listed as deliberately absent with its reason. The old
//! version hand-listed the fields it expected, which made a key nobody added
//! to the list invisible to it — exactly how the `auto` light-hours window
//! shipped as a config-file-only setting.
use super::{Field, FIELDS};

/// The config field a form field edits.
fn edits(f: Field) -> &'static str {
    match f {
        Field::FontFamily => "font_family",
        Field::FontSize => "font_size",
        Field::Smooth => "font_smooth",
        Field::FontGamma => "font_gamma",
        Field::NavWidth => "nav_width",
        Field::ShowNav => "show_nav",
        Field::Theme => "theme",
        Field::ThemeDark => "theme_dark",
        Field::ThemeLight => "theme_light",
        Field::LightFrom => "auto_light_from",
        Field::LightTo => "auto_light_to",
        Field::Accent => "accent",
        Field::BorderMarks => "border_marks",
        Field::Invisibles => "invisibles",
        Field::Lsp => "lsp",
        Field::PaperTexture => "paper_texture",
        Field::AmbientDrift => "ambient_drift",
        Field::PaperGrain => "paper_grain",
        Field::Glass => "glass",
        Field::Motion => "motion",
        Field::Density => "density",
        Field::Leading => "leading",
        Field::Contrast => "contrast",
        Field::ShapeCues => "shape_cues",
        Field::Gradient => "gradient",
        Field::WindowOpacity => "window_opacity",
        Field::Maximized => "maximized",
        Field::Notify => "notify",
        Field::NotifyAgentDone => "notify_agent_done",
        Field::NotifyBell => "notify_bell",
        Field::NotifyExit => "notify_exit",
        Field::NotifyMinSecs => "notify_min_secs",
        Field::NotifyPatterns => "notify_patterns",
        Field::Budget5h => "usage_budget_5h",
        Field::Budget7d => "usage_budget_7d",
        Field::Save | Field::Cancel => "",
    }
}

/// Keys the form deliberately does not carry, each with the reason — "we
/// forgot" is not a third option.
const NOT_IN_FORM: [(&str, &str); 13] = [
    (
        "nav_collapsed",
        "set live by the nav's own chevron, which is on screen in both states",
    ),
    ("nav_card", "set live by /nav"),
    ("weather_place", "set live by /weather"),
    ("last_seen_version", "bookkeeping: drives the version note"),
    (
        "command_recents",
        "bookkeeping: the palette's most-recently-run list",
    ),
    ("last_dir", "bookkeeping: restored window state"),
    ("win_w", "bookkeeping: restored window state"),
    ("win_h", "bookkeeping: restored window state"),
    ("model_recents", "bookkeeping: the /model picker's history"),
    (
        "font_random",
        "set by the font-rotation toggle, not a form field",
    ),
    ("font_weight", "set live by /weight"),
    (
        "crt",
        "set live by /crt; an override over the theme's own flag",
    ),
    (
        "gradient_poles",
        "set live by /gradient <a> <b>; an override over the theme's own poles",
    ),
];

/// Every config key is either editable in the form or listed as deliberately
/// absent.
///
/// The old version of this test hand-listed the fields it expected and
/// asserted they were present, so a config key nobody added to the list was
/// invisible to it — which is exactly how the `auto` light-hours window
/// shipped as a config-file-only setting. This reads the struct instead.
#[test]
fn every_config_key_is_editable_or_listed_as_absent() {
    let src = include_str!("../config.rs");
    let body = {
        let decl = "pub struct CrewConfig {";
        // From AFTER the declaration line, or the header itself parses as a
        // field named `struct CrewConfig {`.
        let at = src.find(decl).expect("CrewConfig struct") + decl.len();
        let rest = &src[at..];
        &rest[..rest.find("\n}").expect("struct end")]
    };
    let keys: Vec<&str> = body
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub "))
        .filter_map(|l| l.split(':').next())
        .collect();
    assert!(
        keys.len() > 20,
        "only found {} keys — the struct parse has broken and this test is \
         asserting nothing",
        keys.len()
    );
    let editable: Vec<&str> = FIELDS.iter().map(|&f| edits(f)).collect();
    for k in &keys {
        assert!(
            editable.contains(k) || NOT_IN_FORM.iter().any(|(n, _)| n == k),
            "config key `{k}` is neither editable in the form nor listed in \
             NOT_IN_FORM with a reason"
        );
    }
    // ...and the other way, so a renamed config key does not leave a form
    // field editing nothing.
    for f in FIELDS
        .iter()
        .filter(|f| !matches!(f, Field::Save | Field::Cancel))
    {
        assert!(
            keys.contains(&edits(*f)),
            "{f:?} claims to edit `{}`, which is not a CrewConfig key",
            edits(*f)
        );
    }
    for (n, _) in NOT_IN_FORM {
        assert!(keys.contains(&n), "NOT_IN_FORM lists `{n}`, which is gone");
    }
}
