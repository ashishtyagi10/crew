//! What each settings field is called and what its control displays — the
//! two pure `Field` mappings behind the renderer. Split from `render.rs` for
//! the 200-line cap when `auto`'s pairing pickers joined the Appearance card.
//!
//! Legends are tight on purpose: a half-width box holds about 13 characters
//! at an 80-column pane, and a legend wider than its border reads truncated.
//! `every_field_renders_on_a_tall_pane` is where that gets caught.
use super::{Field, SettingsPane};

pub(crate) fn label_of(f: Field) -> &'static str {
    match f {
        Field::FontFamily => "Font family",
        Field::FontSize => "Font size",
        Field::Smooth => "Smoothing",
        Field::NavWidth => "Nav width",
        Field::ShowNav => "Show nav",
        Field::Theme => "Theme",
        // Short by necessity: these sit side by side in half-width boxes, and
        // a legend wider than its border reads truncated (see Opacity %).
        // 13 characters is the whole budget in a half-width box at 80 cols
        // (`every_field_renders_on_a_tall_pane` is where that is enforced),
        // and these read in context: they sit under `Theme: auto` and above
        // the day-hours, so "Auto dark <crt>" is "when auto is dark, crt".
        Field::ThemeDark => "Auto dark",
        Field::ThemeLight => "Auto light",
        Field::LightFrom => "Auto day from",
        Field::LightTo => "Auto day to",
        Field::Accent => "Accent (#hex)",
        Field::PaperTexture => "Paper texture",
        Field::AmbientDrift => "Drifting background",
        Field::PaperGrain => "Grain (0-2)",
        Field::Glass => "Glass",
        Field::Motion => "Motion",
        // Kept short: this sits in a half-width box beside Nav width, and a
        // legend wider than its border is a legend the user reads truncated.
        Field::WindowOpacity => "Opacity %",
        Field::Maximized => "Launch maximized",
        Field::Notify => "Notifications",
        Field::NotifyAgentDone => "Notify: cmd done",
        Field::NotifyBell => "Notify: bell",
        Field::NotifyExit => "Notify: pane exit",
        Field::NotifyMinSecs => "Min secs",
        Field::NotifyPatterns => "Patterns (one per line)",
        // Millions of tokens; the unit is in the legend because the number
        // typed (5) is nothing like the number stored (5000000).
        Field::Budget5h => "5h budget (M)",
        Field::Budget7d => "7d budget (M)",
        Field::Save | Field::Cancel => "",
    }
}

/// The value text shown for a field, and whether it takes a typing cursor.
pub(crate) fn value_of(p: &SettingsPane, f: Field) -> (String, bool) {
    let onoff = |b: bool| (if b { "on" } else { "off" }).to_string();
    match f {
        Field::FontFamily => (p.family_query.clone(), true),
        Field::FontSize => (p.size_buf.clone(), true),
        Field::Smooth => (
            format!(
                "\u{2039} {} \u{203a}",
                crate::smoothlvl::label_of(p.draft.font_smooth)
            ),
            false,
        ),
        Field::NavWidth => (p.nav_buf.clone(), true),
        Field::ShowNav => (onoff(p.draft.show_nav), false),
        Field::Theme => (
            format!("\u{2039} {} \u{203a}", p.draft.theme_label()),
            false,
        ),
        Field::ThemeDark | Field::ThemeLight => {
            let (dark, light) = p.draft.auto_pool_selections();
            let side = if f == Field::ThemeDark { dark } else { light };
            (
                format!("\u{2039} {} \u{203a}", super::pairing::label(side)),
                false,
            )
        }
        Field::LightFrom => (p.light_from_buf.clone(), true),
        Field::LightTo => (p.light_to_buf.clone(), true),
        Field::Accent => (p.accent_buf.clone(), true),
        Field::PaperTexture => (onoff(p.draft.paper_texture), false),
        Field::AmbientDrift => (onoff(p.draft.ambient_drift), false),
        Field::PaperGrain => (p.grain_buf.clone(), true),
        Field::Glass => (
            format!("\u{2039} {} \u{203a}", p.draft.glass_level().as_str()),
            false,
        ),
        Field::Motion => (
            format!("\u{2039} {} \u{203a}", p.draft.motion_level().as_str()),
            false,
        ),
        Field::WindowOpacity => (p.opacity_buf.clone(), true),
        Field::Maximized => (onoff(p.draft.maximized), false),
        Field::Notify => (onoff(p.draft.notify), false),
        Field::NotifyAgentDone => (onoff(p.draft.notify_agent_done), false),
        Field::NotifyBell => (onoff(p.draft.notify_bell), false),
        Field::NotifyExit => (onoff(p.draft.notify_exit), false),
        Field::NotifyMinSecs => (p.minsecs_buf.clone(), true),
        Field::NotifyPatterns => (p.patterns_buf.clone(), true),
        Field::Budget5h => (p.budget5h_buf.clone(), true),
        Field::Budget7d => (p.budget7d_buf.clone(), true),
        Field::Save | Field::Cancel => (String::new(), false),
    }
}
