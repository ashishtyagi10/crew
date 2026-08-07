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
        Field::Maximized => d.maximized = !d.maximized,
        Field::Notify => d.notify = !d.notify,
        Field::NotifyAgentDone => d.notify_agent_done = !d.notify_agent_done,
        Field::NotifyBell => d.notify_bell = !d.notify_bell,
        Field::NotifyExit => d.notify_exit = !d.notify_exit,
        Field::Smooth => d.font_smooth = crate::smoothlvl::cycle(d.font_smooth, back),
        Field::Motion => {
            let all = crate::motion::MotionLevel::ALL;
            let cur = all.iter().position(|&l| l == d.motion_level()).unwrap_or(0);
            let next = if back {
                (cur + all.len() - 1) % all.len()
            } else {
                (cur + 1) % all.len()
            };
            d.motion = all[next].as_str().to_string();
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
