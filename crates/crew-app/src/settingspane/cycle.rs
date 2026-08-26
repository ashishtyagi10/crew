//! Value cycling for the settings form's toggles and ‹ › pickers — Space and
//! the arrow keys step these instead of typing. Split from `keys.rs` for the
//! 200-line cap when the Smoothing picker joined the form.
use super::{Field, SettingsPane};

/// Flip the focused toggle, or step the focused picker: motion, glass,
/// smoothing (the `/smooth` ladder), or the three consolidated theme modes.
pub(crate) fn cycle_value(p: &mut SettingsPane, back: bool) {
    let field = p.focused_field();
    let d = &mut p.draft;
    match field {
        Field::ShowNav => d.show_nav = !d.show_nav,
        Field::PaperTexture => d.paper_texture = !d.paper_texture,
        Field::AmbientDrift => d.ambient_drift = !d.ambient_drift,
        Field::Maximized => d.maximized = !d.maximized,
        Field::Notify => d.notify = !d.notify,
        Field::NotifyAgentDone => d.notify_agent_done = !d.notify_agent_done,
        Field::NotifyBell => d.notify_bell = !d.notify_bell,
        Field::NotifyExit => d.notify_exit = !d.notify_exit,
        Field::Smooth => d.font_smooth = crate::smoothlvl::cycle(d.font_smooth, back),
        Field::Motion => {
            let all = crate::motion::MotionPref::ALL;
            let cur = all.iter().position(|&l| l == d.motion_pref()).unwrap_or(0);
            let next = if back {
                (cur + all.len() - 1) % all.len()
            } else {
                (cur + 1) % all.len()
            };
            d.motion = all[next].as_str().to_string();
        }
        Field::Density => {
            let all = crate::density::Density::ALL;
            let cur = all.iter().position(|&l| l == d.density()).unwrap_or(0);
            let next = if back {
                (cur + all.len() - 1) % all.len()
            } else {
                (cur + 1) % all.len()
            };
            d.density = all[next].as_str().to_string();
        }
        Field::Contrast => {
            const ALL: [&str; 3] = ["auto", "normal", "high"];
            let cur = ALL.iter().position(|v| *v == d.contrast).unwrap_or(0);
            let next = if back {
                (cur + ALL.len() - 1) % ALL.len()
            } else {
                (cur + 1) % ALL.len()
            };
            d.contrast = ALL[next].to_string();
        }
        Field::ShapeCues => {
            const ALL: [&str; 3] = ["auto", "off", "on"];
            let cur = ALL.iter().position(|v| *v == d.shape_cues).unwrap_or(0);
            let next = if back {
                (cur + ALL.len() - 1) % ALL.len()
            } else {
                (cur + 1) % ALL.len()
            };
            d.shape_cues = ALL[next].to_string();
        }
        Field::Gradient => {
            let all = crate::gradientlvl::GradientLevel::ALL;
            let cur = all
                .iter()
                .position(|&l| l == d.gradient_level())
                .unwrap_or(0);
            let next = if back {
                (cur + all.len() - 1) % all.len()
            } else {
                (cur + 1) % all.len()
            };
            d.gradient = all[next].as_str().to_string();
        }
        Field::Glass => {
            const LEVELS: [crew_theme::GlassLevel; 4] = [
                crew_theme::GlassLevel::Off,
                crew_theme::GlassLevel::Low,
                crew_theme::GlassLevel::Medium,
                crew_theme::GlassLevel::High,
            ];
            let cur = LEVELS
                .iter()
                .position(|&l| l == d.glass_level())
                .unwrap_or(0);
            let next = if back {
                (cur + LEVELS.len() - 1) % LEVELS.len()
            } else {
                (cur + 1) % LEVELS.len()
            };
            d.glass = LEVELS[next].as_str().to_string();
        }
        Field::ThemeDark | Field::ThemeLight => {
            // Cycle the PARSED value, so a config string this build no longer
            // recognises enters the list at `default` instead of wedging the
            // picker on an index it can never match.
            let (dark, light) = d.auto_pool_selections();
            if field == Field::ThemeDark {
                d.theme_dark = super::pairing::cycle(dark, back);
            } else {
                d.theme_light = super::pairing::cycle(light, back);
            }
        }
        Field::Theme => {
            let modes = crew_theme::THEME_MODES;
            let cur = d
                .theme
                .as_deref()
                .and_then(crew_theme::parse_selection)
                .and_then(|sel| match sel {
                    crew_theme::Selection::Mode(m) => modes.iter().position(|&x| x == m),
                    crew_theme::Selection::Fixed(_) => None,
                })
                .unwrap_or(0);
            let next = if back {
                (cur + modes.len() - 1) % modes.len()
            } else {
                (cur + 1) % modes.len()
            };
            d.theme = Some(modes[next].as_str().to_string());
        }
        _ => {}
    }
}
