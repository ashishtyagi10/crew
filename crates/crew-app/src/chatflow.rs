//! Tracks the crew pane's live activity: which agents are currently thinking,
//! who handed each of them the work, and folding `Activity`/`Stats` events
//! into the pane's pulse (hop history) and per-agent totals.
use std::time::Instant;

/// One currently-thinking agent, as tracked from `Activity` events.
pub(crate) struct ActiveAgent {
    /// The agent doing the work.
    pub name: String,
    /// Who handed it the work (`"user"`, a peer agent, …; may be empty).
    /// Carried over from the wire event; unread since the activity row that
    /// rendered it was replaced by the chip grid, kept for a future consumer.
    #[allow(dead_code)]
    pub from: String,
    /// When it started thinking — drives the spinner and elapsed label.
    pub since: Instant,
}

impl crate::chat::ChatPane {
    /// Fold one `Stats` event into the pane's totals: turn-level events (empty
    /// `agent`) feed the token meter and turn counter; reply-level events feed
    /// that agent's `(replies, total ms)` for the roster chips, and — when the
    /// backend reported real usage — its live context fill (`ctx`).
    pub(crate) fn absorb_stats(&mut self, tokens: u64, agent: String, ms: u64, ctx: u64) {
        if agent.is_empty() {
            self.tokens = self.tokens.saturating_add(tokens);
            self.turns = self.turns.saturating_add(1);
        } else {
            if ctx > 0 {
                self.ctx.insert(agent.clone(), ctx);
            }
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
