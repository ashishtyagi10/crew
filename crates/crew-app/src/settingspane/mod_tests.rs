use super::commit::{build_config, commit_family, commit_field, escape, move_focus};
use super::{Field, SettingsAction, SettingsPane, DEFAULT_FAMILY_LABEL, FIELDS};
use crate::config::CrewConfig;

fn pane() -> SettingsPane {
    SettingsPane::new(
        CrewConfig::default(),
        vec!["Menlo".into(), "JetBrains Mono".into()],
    )
}

/// Focus the pane on `field` (must be in FIELDS).
fn focus(p: &mut SettingsPane, field: Field) {
    p.focus = FIELDS.iter().position(|&f| f == field).unwrap();
}

#[test]
fn filtered_leads_with_default_label() {
    assert_eq!(pane().filtered().first().unwrap(), DEFAULT_FAMILY_LABEL);
}

#[test]
fn filtered_narrows_on_query() {
    let mut p = pane();
    p.family_query = "jet".into();
    assert_eq!(p.filtered(), vec!["JetBrains Mono".to_string()]);
}

#[test]
fn commit_font_size_clamps_low() {
    let mut p = pane();
    focus(&mut p, Field::FontSize);
    p.size_buf = "3".into();
    commit_field(&mut p);
    assert_eq!(p.draft.font_size, 12.0);
    assert_eq!(p.size_buf, "12");
}

#[test]
fn commit_family_sets_draft() {
    let mut p = pane();
    p.family_query = "jet".into();
    p.family_sel = 0;
    commit_family(&mut p);
    assert_eq!(p.draft.font_family.as_deref(), Some("JetBrains Mono"));
    assert!(!p.family_open);
}

#[test]
fn move_focus_wraps_backwards_to_cancel() {
    let mut p = pane();
    move_focus(&mut p, true);
    assert_eq!(p.focused_field(), Field::Cancel);
}

#[test]
fn esc_closes_dropdown_then_cancels() {
    let mut p = pane();
    p.family_open = true;
    assert!(escape(&mut p).is_none()); // first Esc closes the dropdown
    assert!(!p.family_open);
    assert!(matches!(escape(&mut p), Some(SettingsAction::Cancel)));
}

#[test]
fn scroll_steps_field_focus_clamped() {
    let mut p = pane();
    p.scroll(-99); // wheel down → last field
    assert_eq!(p.focused_field(), Field::Cancel);
    p.scroll(99); // wheel up → first field
    assert_eq!(p.focused_field(), Field::FontFamily);
}

#[test]
fn build_config_returns_edited_draft() {
    let mut p = pane();
    focus(&mut p, Field::FontSize);
    p.size_buf = "20".into();
    commit_field(&mut p);
    assert_eq!(build_config(&p).font_size, 20.0);
}

#[test]
fn commit_accent_valid_normalizes_and_sets_draft() {
    let mut p = pane();
    focus(&mut p, Field::Accent);
    p.accent_buf = "#AABBCC".into();
    commit_field(&mut p);
    // Stored canonical lowercase; the buffer mirrors it.
    assert_eq!(p.draft.accent.as_deref(), Some("#aabbcc"));
    assert_eq!(p.accent_buf, "#aabbcc");
}

#[test]
fn commit_accent_invalid_keeps_previous() {
    let mut p = pane();
    focus(&mut p, Field::Accent);
    p.draft.accent = Some("#001122".into());
    p.accent_buf = "nope".into();
    commit_field(&mut p);
    assert_eq!(p.draft.accent.as_deref(), Some("#001122"));
    assert_eq!(p.accent_buf, "#001122");
}

#[test]
fn commit_accent_empty_clears_to_builtin() {
    let mut p = pane();
    focus(&mut p, Field::Accent);
    p.draft.accent = Some("#001122".into());
    p.accent_buf = "   ".into();
    commit_field(&mut p);
    assert_eq!(p.draft.accent, None);
    assert!(p.accent_buf.is_empty());
}

#[test]
fn commit_grain_clamps_and_formats() {
    let mut p = pane();
    focus(&mut p, Field::PaperGrain);
    p.grain_buf = "9.7".into();
    commit_field(&mut p);
    assert_eq!(p.draft.paper_grain, 2.0);
    assert_eq!(p.grain_buf, "2.0");
}

#[test]
fn commit_min_secs_clamps_up_from_zero() {
    let mut p = pane();
    focus(&mut p, Field::NotifyMinSecs);
    p.minsecs_buf = "0".into();
    commit_field(&mut p);
    assert_eq!(p.draft.notify_min_secs, 1);
}

#[test]
fn commit_patterns_splits_lines_and_drops_blanks() {
    let mut p = pane();
    focus(&mut p, Field::NotifyPatterns);
    p.patterns_buf = " error \n\n DONE ".into();
    commit_field(&mut p);
    assert_eq!(
        p.draft.notify_patterns,
        vec!["error".to_string(), "DONE".to_string()]
    );
    assert_eq!(p.patterns_buf, "error\nDONE"); // normalized display
}

/// Opacity is typed as a percentage but stored as a fraction — the form is the
/// only place it can be set now, so a wrong unit here is unreachable elsewhere.
#[test]
fn commit_opacity_reads_a_percentage() {
    let mut p = pane();
    focus(&mut p, Field::WindowOpacity);
    p.opacity_buf = "70".into();
    commit_field(&mut p);
    assert!((p.draft.window_opacity - 0.70).abs() < 1e-6);
    assert_eq!(p.opacity_buf, "70");
}

/// The floor is the difference between a translucent window and a lost one.
#[test]
fn commit_opacity_floors_absurd_transparency() {
    let mut p = pane();
    focus(&mut p, Field::WindowOpacity);
    p.opacity_buf = "0".into();
    commit_field(&mut p);
    assert_eq!(p.draft.window_opacity, crate::config::MIN_WINDOW_OPACITY);
    // The buffer shows what was actually applied, not what was typed.
    assert_eq!(p.opacity_buf, "35");
}

#[test]
fn glass_cycles_through_every_level_both_ways() {
    let mut p = pane();
    focus(&mut p, Field::Glass);
    // Default is medium; forward wraps high → off.
    for want in ["high", "off", "low", "medium"] {
        super::cycle::cycle_value(&mut p, false);
        assert_eq!(p.draft.glass, want);
    }
    super::cycle::cycle_value(&mut p, true);
    assert_eq!(p.draft.glass, "low", "Left must step backward");
}

/// Glass and window opacity are separate knobs; setting one must not disturb
/// the other (they now share a form, which is exactly when this can regress).
#[test]
fn glass_level_and_window_opacity_are_independent() {
    let mut p = pane();
    focus(&mut p, Field::WindowOpacity);
    p.opacity_buf = "80".into();
    commit_field(&mut p);
    focus(&mut p, Field::Glass);
    super::cycle::cycle_value(&mut p, false);
    assert!((p.draft.window_opacity - 0.80).abs() < 1e-6);
}

#[test]
fn smooth_cycles_the_named_ladder_both_ways() {
    let mut p = pane();
    focus(&mut p, Field::Smooth);
    // Default is medium (100); forward wraps heavy → off.
    for want in [170u8, 0, 60, crew_render::DEFAULT_SMOOTH] {
        super::cycle::cycle_value(&mut p, false);
        assert_eq!(p.draft.font_smooth, want);
    }
    super::cycle::cycle_value(&mut p, true);
    assert_eq!(p.draft.font_smooth, 60, "Left must step backward");
}

/// A `/smooth 42` custom strength survives opening the form untouched — the
/// picker only moves the value when the user actually cycles it.
#[test]
fn smooth_custom_strength_is_kept_until_cycled() {
    let cfg = CrewConfig {
        font_smooth: 42,
        ..Default::default()
    };
    let mut p = SettingsPane::new(cfg, Vec::new());
    focus(&mut p, Field::Smooth);
    commit_field(&mut p); // focus-commit must not disturb it
    assert_eq!(p.draft.font_smooth, 42);
    super::cycle::cycle_value(&mut p, false);
    assert_eq!(p.draft.font_smooth, 60, "cycling joins the named ladder");
}

/// The saved config must carry the smoothing pick through `clamped()` — a
/// literal there (the `last_seen_version` bug) would silently reset it, and
/// the field would look editable while changing nothing.
#[test]
fn save_applies_the_smooth_pick_through_clamped() {
    let mut p = pane();
    focus(&mut p, Field::Smooth);
    super::cycle::cycle_value(&mut p, false); // medium → heavy
    let SettingsAction::Apply(cfg) = p.save() else {
        panic!("save must apply");
    };
    assert_eq!(cfg.font_smooth, 170);
}

/// Parity with `/smooth`: both surfaces read and write `font_smooth`, so a
/// strength set by the command is what the form shows, and vice versa.
#[test]
fn smooth_field_and_command_share_the_config_key() {
    let cfg = CrewConfig {
        font_smooth: crate::smoothlvl::strength_of("heavy").unwrap(),
        ..Default::default()
    };
    let mut p = SettingsPane::new(cfg, Vec::new());
    assert_eq!(p.draft.font_smooth, 170, "command's value reaches the form");
    focus(&mut p, Field::Smooth);
    super::cycle::cycle_value(&mut p, false); // heavy wraps → off
    let SettingsAction::Apply(out) = p.save() else {
        panic!("save must apply");
    };
    assert_eq!(
        crate::smoothlvl::label_of(out.font_smooth),
        "off",
        "form's value is what /smooth would then report"
    );
}

#[test]
fn save_commits_the_focused_edit_and_applies() {
    let mut p = pane();
    focus(&mut p, Field::FontSize);
    p.size_buf = "20".into();
    let SettingsAction::Apply(cfg) = p.save() else {
        panic!("save must apply");
    };
    assert_eq!(cfg.font_size, 20.0);
}

#[test]
fn every_config_property_is_editable_in_the_form() {
    // The goal: all user-configurable properties appear in the settings page.
    // Persisted window state (last_dir, win_w/h) is bookkeeping, not a setting.
    for f in [
        Field::FontFamily,
        Field::FontSize,
        Field::Smooth,
        Field::NavWidth,
        Field::ShowNav,
        Field::Theme,
        Field::Accent,
        Field::PaperTexture,
        Field::PaperGrain,
        Field::Glass,
        Field::WindowOpacity,
        Field::Maximized,
        Field::Notify,
        Field::NotifyAgentDone,
        Field::NotifyBell,
        Field::NotifyExit,
        Field::NotifyMinSecs,
        Field::NotifyPatterns,
    ] {
        assert!(FIELDS.contains(&f), "{f:?} missing from the form");
    }
}
