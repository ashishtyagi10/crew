//! The crew pane's live activity row: while agents work, one chip per active
//! agent showing who handed it the work — `⠹ user ⇢ planner 4s` — so the pane
//! shows the crew's interactions as they happen, not just a busy flag.
use std::time::Instant;

use crew_render::CellView;

use crate::chatroster::agent_color;

/// One currently-thinking agent, as tracked from `Activity` events.
pub(crate) struct ActiveAgent {
    /// The agent doing the work.
    pub name: String,
    /// Who handed it the work (`"user"`, a peer agent, …; may be empty).
    pub from: String,
    /// When it started thinking — drives the spinner and elapsed label.
    pub since: Instant,
}

/// Milliseconds per spinner frame.
const FRAME_MS: u128 = 120;

impl crate::chat::ChatPane {
    /// Fold one `Stats` event into the pane's totals: turn-level events (empty
    /// `agent`) feed the token meter and turn counter; reply-level events feed
    /// that agent's `(replies, total ms)` for the roster chips.
    pub(crate) fn absorb_stats(&mut self, tokens: u64, agent: String, ms: u64) {
        if agent.is_empty() {
            self.tokens = self.tokens.saturating_add(tokens);
            self.turns = self.turns.saturating_add(1);
        } else {
            let e = self.agent_stats.entry(agent).or_default();
            e.0 = e.0.saturating_add(1);
            e.1 = e.1.saturating_add(ms);
        }
    }

    /// Fold one `Activity` event into the live set and the pulse's hop record:
    /// `thinking` starts an agent's clock (and a fresh waterfall when the
    /// previous turn had settled); a per-agent `idle` (fan replies) stops it
    /// and records the hop; the empty-agent idle (turn over) flushes everyone
    /// and freezes the waterfall.
    pub(crate) fn absorb_activity(&mut self, agent: String, state: &str, from: String) {
        match (state, agent.is_empty()) {
            ("thinking", false) => {
                self.pulse.begin_hop();
                if !self.active.iter().any(|a| a.name == agent) {
                    self.active.push(ActiveAgent {
                        name: agent,
                        from,
                        since: Instant::now(),
                    });
                }
            }
            ("idle", false) => {
                if let Some(i) = self.active.iter().position(|a| a.name == agent) {
                    let a = self.active.remove(i);
                    self.pulse
                        .record_hop(&a.name, a.since.elapsed().as_millis() as u64);
                }
            }
            _ => self.flush_active_hops(),
        }
    }

    /// A reply landed: in a relay the broker sends no per-agent idle, so the
    /// message itself (`sender` = `"agent → to"`) is the hop-end signal — stop
    /// that agent's clock and record the hop.
    pub(crate) fn note_reply(&mut self, sender: &str) {
        let name = sender.split(" \u{2192} ").next().unwrap_or(sender);
        if let Some(i) = self.active.iter().position(|a| a.name == name) {
            let a = self.active.remove(i);
            self.pulse
                .record_hop(&a.name, a.since.elapsed().as_millis() as u64);
        }
    }

    /// Turn over (or broker gone): record any still-running agents' elapsed
    /// time so a cancelled turn's waterfall stays truthful, then freeze it.
    pub(crate) fn flush_active_hops(&mut self) {
        for a in self.active.drain(..) {
            self.pulse
                .record_hop(&a.name, a.since.elapsed().as_millis() as u64);
        }
        self.pulse.end_turn();
    }

    /// Whether the crew has done or is doing anything worth charting — gates
    /// the pulse block (a fresh pane keeps the roster + onboarding).
    pub(crate) fn engaged(&self) -> bool {
        self.turns > 0 || !self.active.is_empty() || !self.pulse.is_empty()
    }

    /// Whether the newest message is still fading in — keeps redraw frames
    /// flowing for the fade's few hundred ms after a reply lands.
    pub(crate) fn is_fading(&self) -> bool {
        self.messages
            .last()
            .is_some_and(|m| crate::chatmsgs::fade_t(&m.ts, crate::chattime::unix_now_ms()) < 1.0)
    }

    /// The live status label: the thinking agent's name (one active) or a
    /// `N working` count (parallel fan), with the oldest elapsed seconds.
    pub(crate) fn active_status(&self) -> Option<(String, u64)> {
        let secs = self
            .active
            .iter()
            .map(|a| a.since.elapsed().as_secs())
            .max()?;
        match &self.active[..] {
            [one] => Some((one.name.clone(), secs)),
            many => Some((format!("{} working", many.len()), secs)),
        }
    }

    /// Names of every agent currently thinking (roster highlights them all).
    pub(crate) fn active_names(&self) -> Vec<&str> {
        self.active.iter().map(|a| a.name.as_str()).collect()
    }

    /// The live activity entries, for the pane's interaction row.
    pub(crate) fn active_agents(&self) -> &[ActiveAgent] {
        &self.active
    }
}

/// Append `s` at `(row, col..)` in `fg`, clipped to `cols`; returns the next column.
fn push(
    cells: &mut Vec<CellView>,
    row: u16,
    col: u16,
    cols: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) -> u16 {
    let bg = crew_theme::theme().page_bg;
    // Width-aware placement (wide glyphs advance two columns — see `chatwidth`).
    crate::chatwidth::place_row(col, cols, s.chars().map(|c| (c, fg)), |x, c, fg| {
        cells.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold,
            italic: false,
        });
    })
}

/// Build the activity row at `row`: `⠹ from ⇢ agent Ns` chips, two spaces
/// apart, clipped to the pane width. Empty when nobody is working.
pub(crate) fn activity_cells(cols: u16, row: u16, active: &[ActiveAgent]) -> Vec<CellView> {
    let mut cells = Vec::new();
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let mut x = 0u16;
    for (i, a) in active.iter().enumerate() {
        if i > 0 {
            x += 2; // gap between chips
        }
        if x >= cols {
            break;
        }
        let frame =
            ((a.since.elapsed().as_millis() / FRAME_MS) as usize) % crate::update::SPINNER.len();
        let spin = crate::update::SPINNER[frame];
        x = push(&mut cells, row, x, cols, &format!("{spin} "), accent, false);
        if !a.from.is_empty() {
            x = push(&mut cells, row, x, cols, &a.from, t.text_muted, false);
            // A dashed arrow (⇢): work flowing from the sender to the agent.
            x = push(&mut cells, row, x, cols, " \u{21e2} ", t.text_muted, false);
        }
        x = push(
            &mut cells,
            row,
            x,
            cols,
            &a.name,
            agent_color(&a.name),
            true,
        );
        let secs = format!(" {}s", a.since.elapsed().as_secs());
        x = push(&mut cells, row, x, cols, &secs, t.text_muted, false);
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active(name: &str, from: &str) -> ActiveAgent {
        ActiveAgent {
            name: name.into(),
            from: from.into(),
            since: Instant::now(),
        }
    }

    fn text(cells: &[CellView], cols: usize) -> String {
        let mut line = vec![' '; cols];
        for c in cells {
            line[c.col as usize] = c.c;
        }
        line.into_iter().collect()
    }

    #[test]
    fn chip_shows_who_works_for_whom_with_elapsed() {
        let cells = activity_cells(80, 2, &[active("planner", "user")]);
        let line = text(&cells, 80);
        assert!(line.contains("user \u{21e2} planner 0s"), "got: {line}");
        assert!(cells.iter().all(|c| c.row == 2));
        // The worker's name is the bold part of the chip.
        assert!(cells.iter().any(|c| c.bold));
    }

    #[test]
    fn parallel_agents_get_one_chip_each() {
        let cells = activity_cells(80, 2, &[active("planner", "user"), active("coder", "user")]);
        let line = text(&cells, 80);
        assert!(line.contains("planner"), "got: {line}");
        assert!(line.contains("coder"), "got: {line}");
    }

    #[test]
    fn empty_from_skips_the_arrow() {
        let line = text(&activity_cells(80, 2, &[active("coder", "")]), 80);
        assert!(!line.contains('\u{21e2}'), "got: {line}");
        assert!(line.contains("coder 0s"), "got: {line}");
    }

    #[test]
    fn clips_to_width_and_empty_when_idle() {
        assert!(activity_cells(80, 2, &[]).is_empty());
        let cells = activity_cells(10, 2, &[active("a-very-long-name", "user")]);
        assert!(cells.iter().all(|c| c.col < 10));
    }
}
