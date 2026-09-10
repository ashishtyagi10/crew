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
        use crate::navweatherplace::{is_auto, resolve, OFF};
        if arg.is_empty() {
            let key = &self.config.weather_place;
            let note = match resolve(key) {
                None => "weather: off (/weather <place> | auto)".to_string(),
                Some(p) if is_auto(key) => {
                    format!("weather: {p} — your time zone's city (/weather <place> | off)")
                }
                Some(p) => format!("weather: {p} (/weather <place> | auto | off)"),
            };
            self.set_status(note);
            return;
        }
        if arg.eq_ignore_ascii_case(OFF) {
            self.config.weather_place = OFF.to_string();
            self.config.save();
            crate::navweather::set(crate::navweather::State::Off);
            self.set_status("weather: off");
            self.redraw();
            return;
        }
        // `auto` empties the key: the time zone's city again.
        self.config.weather_place = if arg.eq_ignore_ascii_case("auto") {
            String::new()
        } else {
            arg.to_string()
        };
        self.config.save();
        let Some(arg) = resolve(&self.config.weather_place) else {
            self.set_status("weather: your time zone names no city — /weather <place>");
            return;
        };
        // Fetch now rather than on the hourly clock — the user just asked.
        crate::navweather::set(crate::navweather::State::Looking(arg.clone()));
        self.weather_next = 0;
        self.set_status(format!("weather: looking up {arg}\u{2026}"));
    }
}
