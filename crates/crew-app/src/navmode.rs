//! Which filling the nav's variable slot shows: the glance cards or the
//! LOG tail (`nav_card`, set live by `/nav`). Split from `navglance` so
//! that file stays under the line cap.
/// What the nav's variable slot shows (`nav_card`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NavCard {
    Glance,
    Log,
}

impl NavCard {
    #[cfg(test)]
    pub(crate) const ALL: [NavCard; 2] = [NavCard::Glance, NavCard::Log];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            NavCard::Glance => "glance",
            NavCard::Log => "log",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "glance" => Some(NavCard::Glance),
            "log" => Some(NavCard::Log),
            _ => None,
        }
    }
}

impl crate::config::CrewConfig {
    /// The nav's variable slot; an unknown name falls back to `glance`.
    pub(crate) fn nav_card(&self) -> NavCard {
        NavCard::parse(&self.nav_card).unwrap_or(NavCard::Glance)
    }
}

#[cfg(test)]
#[path = "navmode_tests.rs"]
mod tests;
