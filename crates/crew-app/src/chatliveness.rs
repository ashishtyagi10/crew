//! The header's liveness colour: the active agent's name, lit while its
//! tokens flow. The broker's mid-reply `StatsTick` — the named form of the
//! hive's `TokenDelta` — stamps `token_pulse`; `shimmer::pulse_color` reads
//! the stamp back at draw time. Split from `chatflow` for the 200-line cap.
use crate::chat::ChatPane;
use crate::chatflow::stream_key;
use crate::shimmer::Color;

impl ChatPane {
    /// A burst of tokens from `agent` landed at `now` (the animation clock).
    pub(crate) fn note_tokens(&mut self, agent: &str, now: u64) {
        self.token_pulse.insert(stream_key(agent).to_string(), now);
    }

    /// The header's active label — `coder · 12s` — with the colour it wears
    /// at `now`: one agent keeps its roster colour, brightened toward the
    /// page ink for `shimmer::PULSE_MS` after each token burst; a parallel
    /// pack goes accent. `None` while nobody is thinking.
    pub(crate) fn header_active(&self, now: u64) -> Option<(String, u64, Color)> {
        let (label, secs) = self.active_status()?;
        let color = match self.active_names().as_slice() {
            [one] => {
                let t = crew_theme::theme();
                crate::shimmer::pulse_color(
                    crate::chatroster::agent_color(one),
                    t.ink,
                    t.page_bg,
                    self.token_pulse.get(*one).copied(),
                    now,
                    crate::motion::level(),
                )
            }
            _ => crate::palette::accent(),
        };
        Some((label, secs, color))
    }
}

#[cfg(test)]
#[path = "chatliveness_tests.rs"]
mod tests;
