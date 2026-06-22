//! Super-chord (Cmd/Ctrl + key) dispatch and pane reordering.
use crate::app::CrewApp;

/// The next index after `from` (wrapping) whose `active` flag is set, or `None`.
pub(crate) fn next_active_index(active: &[bool], from: usize) -> Option<usize> {
    let n = active.len();
    if n == 0 {
        return None;
    }
    (1..=n).map(|s| (from + s) % n).find(|&i| active[i])
}

/// Index to swap the pane at `i` with when moving in `dir` (-1 left, +1 right),
/// or `None` at the ends / with fewer than two panes.
pub(crate) fn swap_target(i: usize, n: usize, dir: i32) -> Option<usize> {
    if n < 2 {
        None
    } else if dir < 0 {
        i.checked_sub(1)
    } else if i + 1 < n {
        Some(i + 1)
    } else {
        None
    }
}

impl CrewApp {
    /// Focus the next pane (wrapping) that has unseen activity.
    pub(crate) fn focus_next_active(&mut self) {
        let active: Vec<bool> = self.panes.iter().map(|p| p.activity).collect();
        if let Some(i) = next_active_index(&active, self.focused) {
            self.focused = i;
            self.input.focused = false;
        }
    }

    /// Move the focused pane one slot left/right in the grid order.
    pub(crate) fn move_pane(&mut self, dir: i32) {
        if let Some(j) = swap_target(self.focused, self.panes.len(), dir) {
            self.panes.swap(self.focused, j);
            self.focused = j;
        }
    }

    /// Handle a Super-chord key.  Returns `true` if the app should exit.
    pub(crate) fn handle_super_chord(&mut self, s: &str) -> bool {
        let n = self.panes.len().max(1);
        match s {
            "i" => self.input.focused = !self.input.focused,
            "," => self.spawn_settings_pane(),
            "g" => self.toggle_sidebar(),
            "t" => self.spawn_new_pane(),
            "j" => {
                let cmd = Self::echo_plugin_cmd();
                self.spawn_chat_pane(&cmd);
            }
            "o" => {
                let cmd = Self::orchestrator_plugin_cmd();
                self.spawn_chat_pane(&cmd);
            }
            "w" => return self.close_pane(self.focused),
            "m" => self.toggle_maximize(),
            "[" => self.focused = (self.focused + n - 1) % n,
            "]" => self.focused = (self.focused + 1) % n,
            "{" => self.move_pane(-1),
            "}" => self.move_pane(1),
            "z" => self.zoomed = !self.zoomed,
            "a" => self.focus_next_active(),
            "s" => {
                self.broadcast = !self.broadcast;
                self.input.broadcast = self.broadcast;
            }
            "v" => self.paste(),
            // Font zoom: Cmd+= / Cmd+- grow/shrink, Cmd+0 resets to default.
            "=" | "+" => self.set_font(self.config.font_size + 1.0),
            "-" | "_" => self.set_font(self.config.font_size - 1.0),
            "0" => self.set_font(14.0),
            s if s.len() == 1 => {
                if let Some(d) = s.chars().next().and_then(|c| c.to_digit(10)) {
                    if d >= 1 {
                        let i = (d - 1) as usize;
                        if i < self.panes.len() {
                            self.focused = i;
                        }
                    }
                }
            }
            _ => {}
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{next_active_index, swap_target};

    #[test]
    fn next_active_wraps_and_skips() {
        let active = [false, true, false, true];
        assert_eq!(next_active_index(&active, 0), Some(1));
        assert_eq!(next_active_index(&active, 1), Some(3));
        assert_eq!(next_active_index(&active, 3), Some(1)); // wraps past the end
        assert_eq!(next_active_index(&[false, false], 0), None);
        assert_eq!(next_active_index(&[], 0), None);
    }

    #[test]
    fn swap_target_bounds() {
        assert_eq!(swap_target(0, 1, 1), None); // single pane
        assert_eq!(swap_target(0, 3, -1), None); // already leftmost
        assert_eq!(swap_target(1, 3, -1), Some(0));
        assert_eq!(swap_target(2, 3, 1), None); // already rightmost
        assert_eq!(swap_target(1, 3, 1), Some(2));
    }
}
