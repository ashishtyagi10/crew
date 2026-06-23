//! `/font [size]`: set the font size to an exact value. The `Cmd+=` / `Cmd+-`
//! chords only step by one; this jumps straight to a size (handy for screenshots
//! or presentations). With no argument it reports the current size.
use crate::app::CrewApp;

impl CrewApp {
    /// Set the font size from `arg` (a number), or report the current size when
    /// `arg` is empty. Out-of-range values are clamped (12–32) by `set_font`.
    pub(crate) fn set_font_cmd(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.is_empty() {
            self.set_status(format!(
                "font size {} — /font <n> to set",
                self.config.font_size as i32
            ));
            return;
        }
        match arg.parse::<f32>() {
            Ok(n) => self.set_font(n),
            Err(_) => self.set_status(format!("font: not a number: {arg}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;

    #[test]
    fn parses_and_clamps_to_range() {
        let mut app = CrewApp::default();
        app.set_font_cmd("18");
        assert_eq!(app.config.font_size, 18.0);
        app.set_font_cmd("5"); // below min → clamps up
        assert_eq!(app.config.font_size, 12.0);
        app.set_font_cmd("999"); // above max → clamps down
        assert_eq!(app.config.font_size, 32.0);
    }

    #[test]
    fn rejects_non_number_without_changing_size() {
        let mut app = CrewApp::default();
        let before = app.config.font_size;
        app.set_font_cmd("big");
        assert_eq!(app.config.font_size, before);
        assert!(app.active_status().is_some());
    }
}
