//! `/nav` and `/weather` — the nav's variable slot and the clock's weather
//! strip. Both persist through the config (`nav_card`, `weather_place`) and
//! take effect on the next frame.
use crate::app::CrewApp;
use crate::navmode::NavCard;

impl CrewApp {
    /// `/nav [glance|log]` — what the nav shows in the LOG's old slot.
    pub(crate) fn nav_command(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.is_empty() {
            self.set_status(format!(
                "nav card: {} (/nav [glance|log])",
                self.config.nav_card().as_str()
            ));
            return;
        }
        let Some(card) = NavCard::parse(arg) else {
            self.set_status("usage: /nav [glance|log]");
            return;
        };
        self.config.nav_card = card.as_str().to_string();
        self.config.save();
        self.set_status(format!("nav card: {}", card.as_str()));
        self.redraw();
    }

    /// `/weather <place>` sets the place the clock's strip reports for;
    /// `/weather off` clears it. Bare, it says what is set.
    pub(crate) fn weather_command(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.is_empty() {
            let note = if self.config.weather_place.is_empty() {
                "weather: off (/weather <place>, e.g. /weather Berlin)".to_string()
            } else {
                format!(
                    "weather: {} (/weather <place> | off)",
                    self.config.weather_place
                )
            };
            self.set_status(note);
            return;
        }
        if arg.eq_ignore_ascii_case("off") {
            self.config.weather_place.clear();
            self.config.save();
            crate::navweather::set(None);
            self.set_status("weather: off");
            self.redraw();
            return;
        }
        self.config.weather_place = arg.to_string();
        self.config.save();
        // Fetch now rather than on the hourly clock — the user just asked.
        crate::navweather::set(None);
        self.weather_next = 0;
        self.set_status(format!("weather: looking up {arg}\u{2026}"));
    }
}
