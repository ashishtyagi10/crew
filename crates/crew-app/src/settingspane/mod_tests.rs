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

/// The config field a form field edits, for the coverage check below.
fn edits(f: Field) -> &'static str {
    match f {
        Field::FontFamily => "font_family",
        Field::FontSize => "font_size",
        Field::Smooth => "font_smooth",
        Field::NavWidth => "nav_width",
        Field::ShowNav => "show_nav",
        Field::Theme => "theme",
        Field::ThemeDark => "theme_dark",
        Field::ThemeLight => "theme_light",
        Field::LightFrom => "auto_light_from",
        Field::LightTo => "auto_light_to",
        Field::Accent => "accent",
        Field::PaperTexture => "paper_texture",
        Field::AmbientDrift => "ambient_drift",
        Field::PaperGrain => "paper_grain",
        Field::Glass => "glass",
        Field::Motion => "motion",
        Field::Density => "density",
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

/// Config keys the form deliberately does not carry, each with the reason.
/// A key must be here or editable — "we forgot" is not a third option, which
/// is how `auto_light_from` / `auto_light_to` shipped config-only.
const NOT_IN_FORM: [(&str, &str); 10] = [
    ("last_seen_version", "bookkeeping: drives the version note"),
    (
        "command_recents",
        "bookkeeping: the palette's own most-recently-run list",
    ),
    ("last_dir", "bookkeeping: restored window state"),
    ("win_w", "bookkeeping: restored window state"),
    ("win_h", "bookkeeping: restored window state"),
    (
        "model_recents",
        "bookkeeping: the /model picker's own history",
    ),
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

/// The light-hours boxes accept `HH:MM` and nothing else, normalise what they
/// accept, and keep the previous value rather than coercing a typo — a value
/// the form took but `light_hours()` will not parse is a setting that reads
/// back fine and does nothing.
#[test]
fn the_light_hours_boxes_take_hhmm_and_normalise_it() {
    let mut p = SettingsPane::new(CrewConfig::default(), Vec::new());
    focus(&mut p, Field::LightFrom);
    assert_eq!(p.light_from_buf, "07:00", "seeded from the live window");

    p.light_from_buf = "5:5".to_string();
    super::commit::commit_field(&mut p);
    assert_eq!(p.draft.auto_light_from, "05:05", "normalised on commit");
    assert_eq!(p.light_from_buf, "05:05", "and mirrored back into the box");

    // A typo keeps the previous value instead of landing on midnight.
    for typo in ["25:00", "07:60", "nope", ""] {
        p.light_from_buf = typo.to_string();
        super::commit::commit_field(&mut p);
        assert_eq!(p.draft.auto_light_from, "05:05", "`{typo}` was accepted");
    }

    // Whatever survives, `light_hours()` must agree with — the form and the
    // config parser reading the same string differently is the whole bug.
    p.light_from_buf = "21:45".to_string();
    super::commit::commit_field(&mut p);
    assert_eq!(p.draft.light_hours().0, 21 * 60 + 45);
}

/// Only `HH:MM` can be typed: five characters, digits, at most one colon, and
/// never a leading one. Drives the same predicate the key handler does.
#[test]
fn the_light_hours_boxes_reject_anything_that_is_not_hhmm() {
    for f in [Field::LightFrom, Field::LightTo] {
        let mut buf = String::new();
        for c in ":a1b2:3:4:5:6".chars() {
            if super::keys::allowed(f, &buf, c) {
                buf.push(c);
            }
        }
        assert_eq!(buf, "12:34", "{f:?} accepted `{buf}`");
        // Full: nothing more goes in, so the buffer can never outgrow what
        // the commit is able to parse.
        assert!(!super::keys::allowed(f, &buf, '9'));
        // And a colon may not lead, or `:30` would parse as nothing.
        assert!(!super::keys::allowed(f, "", ':'));
    }
}

/// A config file holding an unparseable window shows the window that is
/// actually in effect, not the typo echoed back as if it were live.
#[test]
fn an_unparseable_saved_window_displays_the_fallback() {
    let cfg = CrewConfig::from_toml_str("auto_light_from = \"nope\"\nauto_light_to = \"21:05\"\n");
    let p = SettingsPane::new(cfg, Vec::new());
    assert_eq!(p.light_from_buf, "07:00");
    assert_eq!(p.light_to_buf, "21:05");
}

/// Each pairing picker edits its OWN side. A two-branch cycle is exactly
/// where a copy-paste writes the wrong field, and both sides holding the same
/// value looks plausible enough to ship.
#[test]
fn each_pairing_picker_edits_only_its_own_side() {
    for (field, other) in [
        (Field::ThemeDark, Field::ThemeLight),
        (Field::ThemeLight, Field::ThemeDark),
    ] {
        let mut p = pane();
        focus(&mut p, field);
        let before = (p.draft.theme_dark.clone(), p.draft.theme_light.clone());
        super::cycle_value(&mut p, false);
        let after = (p.draft.theme_dark.clone(), p.draft.theme_light.clone());
        let (mine, theirs) = if field == Field::ThemeDark {
            ((before.0, after.0), (before.1, after.1))
        } else {
            ((before.1, after.1), (before.0, after.0))
        };
        assert_ne!(mine.0, mine.1, "{field:?} did not change its own side");
        assert_eq!(theirs.0, theirs.1, "{field:?} also moved {other:?}");
    }
}

/// The pickers walk the whole list and land back where they started, driven
/// through `cycle_value` rather than the pure helper — so the wiring from the
/// focused field to the config key is what is under test.
#[test]
fn cycling_a_pairing_picker_returns_to_default() {
    let mut p = pane();
    focus(&mut p, Field::ThemeDark);
    let n = super::pairing::values().len();
    let mut seen = Vec::new();
    for _ in 0..n {
        super::cycle_value(&mut p, false);
        seen.push(p.draft.theme_dark.clone());
    }
    assert_eq!(p.draft.theme_dark, None, "did not wrap back to unset");
    assert_eq!(
        seen.iter().filter(|v| v.is_some()).count(),
        n - 1,
        "some entries were visited twice or skipped: {seen:?}"
    );
    // Every visited value survives a round trip through the config file.
    for v in seen.into_iter().flatten() {
        let cfg = CrewConfig::from_toml_str(&format!("theme_dark = \"{v}\"\n"));
        assert_eq!(cfg.theme_dark.as_deref(), Some(v.as_str()));
        assert!(cfg.auto_pool_selections().0.is_some(), "`{v}` was dropped");
    }
}

/// The budget boxes are typed in millions and stored in tokens — the same
/// trade the opacity box makes with percentages.
#[test]
fn the_budget_boxes_type_millions_and_store_tokens() {
    let mut p = pane();
    focus(&mut p, Field::Budget5h);
    assert_eq!(p.budget5h_buf, "5", "5000000 tokens reads as 5");
    assert_eq!(p.budget7d_buf, "25");

    p.budget5h_buf = "7.5".to_string();
    commit_field(&mut p);
    assert_eq!(p.draft.usage_budget_5h, 7_500_000);
    assert_eq!(p.budget5h_buf, "7.5", "mirrored back into the box");
    assert_eq!(p.draft.usage_budget_7d, 25_000_000, "the other side moved");

    focus(&mut p, Field::Budget7d);
    p.budget7d_buf = "40".to_string();
    commit_field(&mut p);
    assert_eq!(p.draft.usage_budget_7d, 40_000_000);
    assert_eq!(p.draft.usage_budget_5h, 7_500_000, "the other side moved");

    // A typo keeps what was there rather than zeroing a budget the footer
    // divides by.
    focus(&mut p, Field::Budget5h);
    for typo in ["nope", ""] {
        p.budget5h_buf = typo.to_string();
        commit_field(&mut p);
        assert_eq!(p.draft.usage_budget_5h, 7_500_000, "`{typo}` was accepted");
    }
}

/// Opening Settings and tabbing past a hand-set budget must not round it.
/// Every focus move runs `commit_field`, so without the no-op guard simply
/// LOOKING at the form would quantise a config the user set by hand.
#[test]
fn tabbing_past_a_budget_never_rewrites_it() {
    let odd = 5_123_456;
    let cfg = CrewConfig {
        usage_budget_5h: odd,
        ..Default::default()
    };
    let mut p = SettingsPane::new(cfg, Vec::new());
    assert_eq!(p.budget5h_buf, "5.12", "the display is lossy, as designed");
    focus(&mut p, Field::Budget5h);
    for _ in 0..3 {
        commit_field(&mut p);
        move_focus(&mut p, false);
    }
    assert_eq!(p.draft.usage_budget_5h, odd, "the form rounded it");
    // ...and a Save of the untouched form carries the original through.
    assert_eq!(build_config(&p).usage_budget_5h, odd);
}

/// Only a number can be typed: digits and at most one decimal point.
#[test]
fn the_budget_boxes_reject_anything_that_is_not_a_number() {
    for f in [Field::Budget5h, Field::Budget7d] {
        let mut buf = String::new();
        for c in "1a2.b3.4-".chars() {
            if super::keys::allowed(f, &buf, c) {
                buf.push(c);
            }
        }
        assert_eq!(buf, "12.34", "{f:?} accepted `{buf}`");
    }
}

/// The contract the `sepia-light` bug broke, now enforced at every width the
/// form can be laid out at: **no field is ever narrower than what it has to
/// draw**. A clipped leading chevron reads as a rendering fault rather than as
/// a layout that ran out of room, which is why the original went unnoticed.
///
/// Sweeping widths is the point. The bug was invisible at the width it was
/// written at and only appeared at 80 columns, so a single-width test is
/// exactly the test that would have passed.
#[test]
fn no_field_is_ever_laid_out_narrower_than_it_needs() {
    let mut bad: Vec<String> = Vec::new();
    for cols in 40u16..=240 {
        let l = crate::settingspane::form::layout(cols);
        for (f, r) in &l.rects {
            // Toggles are one row of text with no box, and the two buttons
            // carry no legend or value at all.
            if r.height < 3 || matches!(f, Field::Save | Field::Cancel) {
                continue;
            }
            let need = crate::settingspane::fit::min_cols(*f);
            if r.width < need {
                bad.push(format!("{cols} cols: {f:?} got {}, needs {need}", r.width));
            }
        }
    }
    // One line per offending field rather than per width, or a single
    // regression prints two hundred near-identical lines.
    bad.dedup_by(|a, b| a.split(": ").nth(1) == b.split(": ").nth(1));
    assert!(bad.is_empty(), "{}", bad.join("\n  "));
}

/// Pairing has to actually happen when there is room, or "responsive" is just
/// a stacked form with extra steps.
#[test]
fn fields_pair_up_on_a_wide_form_and_stack_on_a_narrow_one() {
    let row_of = |cols: u16, want: Field| -> Option<(u16, u16)> {
        crate::settingspane::form::layout(cols)
            .rects
            .iter()
            .find(|(f, _)| *f == want)
            .map(|(_, r)| (r.y, r.width))
    };
    // A wide form pairs everything it can; a narrow one stacks rather than
    // clipping. `Auto day from` is a thirteen-column legend, so its box needs
    // nineteen — comfortable on a wide pane, impossible on a narrow one.
    let (fy, _) = row_of(200, Field::LightFrom).expect("LightFrom");
    let (ty, _) = row_of(200, Field::LightTo).expect("LightTo");
    assert_eq!(fy, ty, "two short boxes must pair on a wide form");
    let (fy, _) = row_of(64, Field::LightFrom).expect("LightFrom");
    let (ty, _) = row_of(64, Field::LightTo).expect("LightTo");
    assert_ne!(fy, ty, "…and stack rather than clip on a narrow one");

    // The palette pickers carry theme names. Narrow, they stack; wide, they
    // pair — which is the whole point of taking the decision from the width.
    let (dy, _) = row_of(80, Field::ThemeDark).expect("ThemeDark");
    let (ly, _) = row_of(80, Field::ThemeLight).expect("ThemeLight");
    assert_ne!(dy, ly, "palette pickers must stack when they do not fit");
    let (dy, _) = row_of(240, Field::ThemeDark).expect("ThemeDark");
    let (ly, _) = row_of(240, Field::ThemeLight).expect("ThemeLight");
    assert_eq!(
        dy, ly,
        "…and pair once there is room — the decision belongs to the width"
    );
}
