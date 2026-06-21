use crew_render::CellView;
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use crate::config::CrewConfig;

const ACCENT: (u8, u8, u8) = (0, 255, 160);
const TEXT: (u8, u8, u8) = (200, 200, 200);
const BG: (u8, u8, u8) = (8, 8, 16);

enum KeyAction {
    Up,
    Down,
    Inc,
    Dec,
}

pub struct SettingsChange {
    pub config: CrewConfig,
}

pub struct SettingsPane {
    cfg: CrewConfig,
    selected: usize,
}

impl SettingsPane {
    pub fn new(cfg: CrewConfig) -> Self {
        Self { cfg, selected: 0 }
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render_settings(&self.cfg, self.selected, cols, rows)
    }

    pub fn on_key(&mut self, key: &KeyEvent) -> Option<SettingsChange> {
        if !key.state.is_pressed() {
            return None;
        }
        let action = match &key.logical_key {
            Key::Named(NamedKey::ArrowUp) => KeyAction::Up,
            Key::Named(NamedKey::ArrowDown) => KeyAction::Down,
            Key::Named(NamedKey::ArrowLeft) => KeyAction::Dec,
            Key::Named(NamedKey::ArrowRight) => KeyAction::Inc,
            Key::Character(s) if s.as_str() == "-" => KeyAction::Dec,
            Key::Character(s) if s.as_str() == "+" || s.as_str() == "=" => KeyAction::Inc,
            _ => return None,
        };
        if reduce_key(&mut self.cfg, &mut self.selected, action) {
            Some(SettingsChange { config: self.cfg })
        } else {
            None
        }
    }
}

/// Mutate cfg/selected per action; return true iff a config VALUE changed.
fn reduce_key(cfg: &mut CrewConfig, selected: &mut usize, action: KeyAction) -> bool {
    match action {
        KeyAction::Up => {
            *selected = selected.saturating_sub(1);
            false
        }
        KeyAction::Down => {
            *selected = (*selected + 1).min(2);
            false
        }
        KeyAction::Inc => adjust_field(cfg, *selected, true),
        KeyAction::Dec => adjust_field(cfg, *selected, false),
    }
}

fn adjust_field(cfg: &mut CrewConfig, selected: usize, inc: bool) -> bool {
    match selected {
        0 => {
            cfg.font_size = if inc {
                cfg.font_size + 1.0
            } else {
                cfg.font_size - 1.0
            }
            .clamp(12.0, 32.0);
            true
        }
        1 => {
            cfg.nav_width = if inc {
                cfg.nav_width + 10.0
            } else {
                cfg.nav_width - 10.0
            }
            .clamp(160.0, 320.0);
            true
        }
        2 => {
            cfg.show_nav = !cfg.show_nav;
            true
        }
        _ => false,
    }
}

/// Render one row per field (rows 0,1,2 if they fit). Selected row in ACCENT, others in TEXT.
pub fn render_settings(cfg: &CrewConfig, selected: usize, cols: u16, rows: u16) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let fields: [(&str, String); 3] = [
        ("Font size", format!("{}", cfg.font_size as i32)),
        ("Nav width", format!("{}", cfg.nav_width as i32)),
        (
            "Show nav ",
            if cfg.show_nav {
                "on".into()
            } else {
                "off".into()
            },
        ),
    ];
    let mut out = Vec::new();
    for (row_idx, (name, value)) in fields.iter().enumerate() {
        if row_idx as u16 >= rows {
            break;
        }
        let is_selected = row_idx == selected;
        let prefix = if is_selected { "> " } else { "  " };
        let line = format!("{prefix}{name}  {value}");
        let fg = if is_selected { ACCENT } else { TEXT };
        for (col, c) in line.chars().take(cols as usize).enumerate() {
            out.push(CellView {
                col: col as u16,
                row: row_idx as u16,
                c,
                fg,
                bg: BG,
                bold: false,
                italic: false,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CrewConfig;

    #[test]
    fn render_has_rows_0_1_2_and_first_cell_is_arrow() {
        let cells = render_settings(&CrewConfig::default(), 0, 40, 3);
        let row_set: std::collections::HashSet<u16> = cells.iter().map(|c| c.row).collect();
        assert_eq!(row_set, [0u16, 1, 2].into_iter().collect());
        let first = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(first.c, '>');
    }

    #[test]
    fn inc_font_size_from_default() {
        let mut cfg = CrewConfig::default();
        let mut sel = 0usize;
        let changed = reduce_key(&mut cfg, &mut sel, KeyAction::Inc);
        assert!(changed);
        assert_eq!(cfg.font_size, 15.0);
    }

    #[test]
    fn dec_font_size_clamped_at_12() {
        let mut cfg = CrewConfig::default();
        let mut sel = 0usize;
        for _ in 0..4 {
            reduce_key(&mut cfg, &mut sel, KeyAction::Dec);
        }
        assert_eq!(cfg.font_size, 12.0);
    }

    #[test]
    fn nav_down_twice_then_inc_toggles_show_nav() {
        let mut cfg = CrewConfig::default();
        let mut sel = 0usize;
        reduce_key(&mut cfg, &mut sel, KeyAction::Down);
        reduce_key(&mut cfg, &mut sel, KeyAction::Down);
        assert_eq!(sel, 2);
        let changed = reduce_key(&mut cfg, &mut sel, KeyAction::Inc);
        assert!(changed);
        assert!(cfg.show_nav);
    }
}
