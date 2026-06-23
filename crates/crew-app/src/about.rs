//! `/about`: flash Crew's version and tagline on the status line — a quick,
//! discoverable way to check the running build without opening `/keys`.
use crate::app::CrewApp;

/// The one-line about string, e.g. `Crew v0.4.6 — native GPU terminal`.
pub(crate) fn about_text() -> String {
    format!("Crew v{} — native GPU terminal", env!("CARGO_PKG_VERSION"))
}

impl CrewApp {
    /// Show the version + tagline on the input-bar status line.
    pub(crate) fn show_about(&mut self) {
        self.set_status(about_text());
    }
}

#[cfg(test)]
mod tests {
    use super::about_text;

    #[test]
    fn about_text_has_name_and_version() {
        let s = about_text();
        assert!(s.contains("Crew v"));
        assert!(s.contains(env!("CARGO_PKG_VERSION")));
    }
}
