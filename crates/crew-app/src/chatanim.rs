//! Roster animation state: eased values (bars, token count-up) and one-shot
//! handoff flashes for the crew pane's agent rows. Time is always a `now`
//! parameter (tests inject it); reads are pure so render paths take `&self`,
//! and targets are set only from the `&mut self` absorb paths.

use std::collections::HashMap;

/// Ease windows (ms): bar sweeps, token count-up, handoff flash.
pub(crate) const BAR_MS: u64 = 250;
pub(crate) const TOK_MS: u64 = 400;
pub(crate) const FLASH_MS: u64 = 400;

/// Cubic ease-out on `t` in 0..=1: fast start, gentle landing.
fn ease_out(t: f32) -> f32 {
    let inv = 1.0 - t.clamp(0.0, 1.0);
    1.0 - inv * inv * inv
}

/// One value easing toward a target over a fixed window. Reads are pure —
/// `value(now)` interpolates from what was on screen when the target was set.
pub(crate) struct Eased {
    from: f32,
    target: f32,
    since_ms: u64,
    window_ms: u64,
}

impl Eased {
    pub(crate) fn new(window_ms: u64) -> Self {
        Eased {
            from: 0.0,
            target: 0.0,
            since_ms: 0,
            window_ms,
        }
    }

    /// Retarget, restarting the ease from the currently shown value.
    pub(crate) fn set_target(&mut self, now: u64, v: f32) {
        self.from = self.value(now);
        self.target = v;
        self.since_ms = now;
    }

    pub(crate) fn value(&self, now: u64) -> f32 {
        if self.window_ms == 0 || now >= self.since_ms + self.window_ms {
            return self.target;
        }
        let t = (now.saturating_sub(self.since_ms)) as f32 / self.window_ms as f32;
        self.from + (self.target - self.from) * ease_out(t)
    }

    pub(crate) fn settled(&self, now: u64) -> bool {
        self.from == self.target || now >= self.since_ms + self.window_ms
    }
}

/// Per-agent animation state for the roster grid: eased ctx/shr fractions,
/// eased token counts, and one-shot handoff flashes.
pub(crate) struct RosterAnim {
    ctx: HashMap<String, Eased>,
    shr: HashMap<String, Eased>,
    tok: HashMap<String, Eased>,
    flash: HashMap<String, u64>,
}

impl RosterAnim {
    pub(crate) fn new() -> Self {
        RosterAnim {
            ctx: HashMap::new(),
            shr: HashMap::new(),
            tok: HashMap::new(),
            flash: HashMap::new(),
        }
    }

    fn set(map: &mut HashMap<String, Eased>, window: u64, agent: &str, now: u64, v: f32) {
        map.entry(agent.to_string())
            .or_insert_with(|| Eased::new(window))
            .set_target(now, v);
    }

    fn get(map: &HashMap<String, Eased>, agent: &str, now: u64) -> f32 {
        map.get(agent).map(|e| e.value(now)).unwrap_or(0.0)
    }

    pub(crate) fn set_ctx(&mut self, agent: &str, now: u64, frac: f32) {
        Self::set(&mut self.ctx, BAR_MS, agent, now, frac);
    }
    pub(crate) fn set_shr(&mut self, agent: &str, now: u64, frac: f32) {
        Self::set(&mut self.shr, BAR_MS, agent, now, frac);
    }
    pub(crate) fn set_tok(&mut self, agent: &str, now: u64, tokens: f32) {
        Self::set(&mut self.tok, TOK_MS, agent, now, tokens);
    }

    pub(crate) fn ctx(&self, agent: &str, now: u64) -> f32 {
        Self::get(&self.ctx, agent, now)
    }
    pub(crate) fn shr(&self, agent: &str, now: u64) -> f32 {
        Self::get(&self.shr, agent, now)
    }
    pub(crate) fn tok(&self, agent: &str, now: u64) -> f32 {
        Self::get(&self.tok, agent, now)
    }

    #[cfg(test)]
    // Test observers: production reads go through the eased getters.
    pub(crate) fn ctx_target(&self, agent: &str) -> f32 {
        self.ctx.get(agent).map(|e| e.target).unwrap_or(0.0)
    }
    #[cfg(test)]
    // Test observers: production reads go through the eased getters.
    pub(crate) fn shr_target(&self, agent: &str) -> f32 {
        self.shr.get(agent).map(|e| e.target).unwrap_or(0.0)
    }

    /// Record a handoff flash for `agent`, dropping expired entries so the
    /// map never outgrows the roster.
    pub(crate) fn flash(&mut self, agent: &str, now: u64) {
        self.gc(now);
        self.flash.insert(agent.to_string(), now);
    }

    /// 1.0 at flash start → 0.0 at FLASH_MS, linear; 0.0 when never flashed.
    pub(crate) fn flash_t(&self, agent: &str, now: u64) -> f32 {
        self.flash
            .get(agent)
            .map(|&start| {
                let age = now.saturating_sub(start) as f32;
                (1.0 - age / FLASH_MS as f32).max(0.0)
            })
            .unwrap_or(0.0)
    }

    pub(crate) fn gc(&mut self, now: u64) {
        self.flash
            .retain(|_, &mut start| now.saturating_sub(start) < FLASH_MS);
    }

    /// Anything still moving? Drives the redraw tail after a turn ends.
    pub(crate) fn active(&self, now: u64) -> bool {
        let easing = |m: &HashMap<String, Eased>| m.values().any(|e| !e.settled(now));
        easing(&self.ctx)
            || easing(&self.shr)
            || easing(&self.tok)
            || self
                .flash
                .values()
                .any(|&start| now.saturating_sub(start) < FLASH_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eased_converges_and_settles() {
        let mut e = Eased::new(250);
        e.set_target(1000, 1.0);
        assert_eq!(e.value(1000), 0.0, "starts at prior shown value");
        let mid = e.value(1125);
        assert!(mid > 0.0 && mid < 1.0, "mid-ease is between: {mid}");
        // Ease-out: the first half covers more than half the distance.
        assert!(mid > 0.5, "ease-out front-loads: {mid}");
        assert_eq!(e.value(1250), 1.0, "lands exactly on target");
        assert_eq!(e.value(9999), 1.0, "stays on target after the window");
        assert!(!e.settled(1100));
        assert!(e.settled(1250));
    }

    #[test]
    fn eased_retarget_mid_ease_starts_from_shown() {
        let mut e = Eased::new(250);
        e.set_target(0, 1.0);
        let shown = e.value(125);
        e.set_target(125, 0.0);
        // Immediately after retarget the value is what was on screen.
        assert!((e.value(125) - shown).abs() < 1e-6);
        assert_eq!(e.value(375), 0.0);
    }

    #[test]
    fn eased_zero_window_snaps() {
        let mut e = Eased::new(0);
        e.set_target(10, 0.7);
        assert_eq!(e.value(10), 0.7);
        assert!(e.settled(10));
    }

    #[test]
    fn roster_anim_defaults_and_targets() {
        let mut ra = RosterAnim::new();
        assert_eq!(ra.ctx("planner", 0), 0.0, "unknown agent reads 0");
        ra.set_ctx("planner", 1000, 0.21);
        assert_eq!(ra.ctx_target("planner"), 0.21);
        assert_eq!(ra.ctx("planner", 1250), 0.21, "settled after window");
        ra.set_tok("planner", 1000, 7200.0);
        assert_eq!(ra.tok("planner", 1400), 7200.0);
    }

    #[test]
    fn flash_fades_linearly_and_expires() {
        let mut ra = RosterAnim::new();
        ra.flash("coder", 1000);
        assert!((ra.flash_t("coder", 1000) - 1.0).abs() < 1e-6);
        assert!((ra.flash_t("coder", 1200) - 0.5).abs() < 1e-6);
        assert_eq!(ra.flash_t("coder", 1400), 0.0);
        assert_eq!(ra.flash_t("nobody", 1000), 0.0);
    }

    #[test]
    fn active_goes_false_after_everything_settles() {
        let mut ra = RosterAnim::new();
        assert!(!ra.active(0), "fresh store is inactive");
        ra.set_ctx("planner", 1000, 0.5);
        ra.flash("coder", 1000);
        assert!(ra.active(1100), "ease + flash in flight");
        assert!(ra.active(1399), "flash window (400ms) still open");
        assert!(!ra.active(1401), "everything settled → inactive");
    }
}
