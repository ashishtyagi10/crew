//! What a click in the settings form DOES — the other half of [`super::click`],
//! which only says where the pointer landed.
//!
//! Split from `mod.rs` when the line cap found the boundary, and it is a real
//! one: where a control is belongs to the layout, what pressing it means
//! belongs to the form's state.
use super::{click, commit, cycle, form, Field, SettingsAction, SettingsPane, FIELDS};

impl SettingsPane {
    /// Answer a click at `(row, col)`: focus what was pressed, and move it
    /// when what was pressed is something that moves. Returns the action a
    /// button press asks for.
    ///
    /// Focus first, always: every control commits through the focused field
    /// (`commit_field`), so pressing one without focusing it would write the
    /// previous field's buffer into the draft.
    pub(crate) fn click(
        &mut self,
        cols: u16,
        rows: u16,
        row: u16,
        col: u16,
    ) -> Option<SettingsAction> {
        let hit = click::hit(self, cols, rows, row, col)?;
        if let click::Hit::Family(i) = hit {
            self.family_sel = i;
            commit::commit_family(self);
            return None;
        }
        let (field, step) = match hit {
            click::Hit::Field(f) => (f, None),
            click::Hit::Step(f, back) => (f, Some(back)),
            click::Hit::Family(_) => unreachable!("answered above"),
        };
        if field != self.focused_field() {
            commit::commit_field(self);
            if let Some(i) = FIELDS.iter().position(|f| *f == field) {
                self.focus = i;
            }
            self.family_open = false;
            commit::refresh_bufs(self);
        }
        match (field, step) {
            (Field::Save, _) => {
                commit::commit_field(self);
                return Some(SettingsAction::Apply(Box::new(commit::build_config(self))));
            }
            (Field::Cancel, _) => return Some(SettingsAction::Cancel),
            // The family box opens its list, which is what Enter does on it.
            (Field::FontFamily, _) => self.family_open = true,
            (_, Some(back)) => cycle::cycle_value(self, back),
            (_, None) => {}
        }
        None
    }

    pub fn scroll(&mut self, lines: i32) {
        if self.family_open {
            let n = self.filtered().len() as i64;
            if n > 0 {
                self.family_sel = (self.family_sel as i64 - lines as i64).clamp(0, n - 1) as usize;
            }
            return;
        }
        let up = lines > 0;
        for _ in 0..lines.unsigned_abs().min(FIELDS.len() as u32) {
            // The ends are the ends of the WALK, not of the declaration list
            // — the wheel must stop where the form stops rather than wrap,
            // and since 0.22.55 those are not the same fields.
            let order = form::tab_order(self.cols.get());
            let at = order.iter().position(|f| *f == self.focused_field());
            let end = if up { Some(0) } else { Some(order.len() - 1) };
            if at == end {
                break;
            }
            commit::commit_field(self);
            commit::move_focus(self, up);
        }
    }
}

impl crate::app::CrewApp {
    /// A click inside a settings pane acts where it lands — a box focuses, a
    /// checkbox flips, a picker's chevron steps, Save and Cancel close the
    /// form — and focuses the pane. Empty regions fall through to the normal
    /// focus path, exactly like the todo and disk panes above it.
    pub(crate) fn settings_click_at_cursor(&mut self) -> bool {
        let Some(i) = self.pane_at_cursor() else {
            return false;
        };
        if !matches!(self.panes[i].content, crate::pane::PaneContent::Settings(_)) {
            return false;
        }
        let Some((row, col)) = self.cursor_rowcol(i) else {
            return false;
        };
        let grid = self.panes[i].grid;
        let crate::pane::PaneContent::Settings(s) = &mut self.panes[i].content else {
            return false;
        };
        self.focused = i;
        self.input.focused = false;
        // Off the top or left edge of the grid: the press focused the pane
        // and nothing else, which is what a click on the card's frame means.
        let (Ok(row), Ok(col)) = (u16::try_from(row), u16::try_from(col)) else {
            return true;
        };
        let action = s.click(grid.cols, grid.rows, row, col);
        if let Some(action) = action {
            if let SettingsAction::Apply(cfg) = action {
                self.apply_settings(*cfg);
            }
            self.close_pane(i);
        }
        true
    }
}

#[cfg(test)]
#[path = "press_tests.rs"]
mod tests;
