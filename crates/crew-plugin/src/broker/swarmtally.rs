//! The run-wide sums a swarm's drain loop keeps for the aggregate `Stats`
//! (tokens, their split, cost), and the one thing it SAYS mid-run: that the
//! tool pool ran dry.
//!
//! Every agent already learns that privately — a refused call, a note in
//! its own output — but the person watching the pane saw N workers go quiet
//! at once with no line saying why. The pool's draws arrive as
//! `HiveEvent::ToolBudget`; the first one that reads `used == total` earns
//! one quiet line under the lead's name, and no later one repeats it.
use crew_hive::HiveEvent;

use crate::broker::relay::msg;
use crate::protocol::PluginEvent;

pub(super) struct Tally {
    pub tokens: u64,
    pub tok_in: u64,
    pub tok_out: u64,
    pub cost: u64,
    tasks: usize,
    spent_said: bool,
}

impl Tally {
    /// Sums start at zero; `tasks` is the plan's size, for the note's wording.
    pub fn new(tasks: usize) -> Self {
        Tally {
            tokens: 0,
            tok_in: 0,
            tok_out: 0,
            cost: 0,
            tasks,
            spent_said: false,
        }
    }

    /// Fold one event into the sums; the line to emit, if this event earned
    /// one (only ever the pool's first empty reading).
    pub fn observe(&mut self, ev: &HiveEvent) -> Option<PluginEvent> {
        match ev {
            HiveEvent::TokenDelta { input, output, .. } => {
                let (i, o) = (u64::from(*input), u64::from(*output));
                self.tokens += i + o;
                self.tok_in += i;
                self.tok_out += o;
            }
            HiveEvent::CostDelta { micros_usd, .. } => self.cost += micros_usd,
            HiveEvent::ToolBudget { used, total } if used >= total && !self.spent_said => {
                self.spent_said = true;
                return Some(msg("agent smith", spent_note(*total, self.tasks)));
            }
            _ => {}
        }
        None
    }
}

/// `tool budget spent — 12 calls over 3 tasks; the workers answer from what
/// they have`: the pool, the plan it was sized from, and what happens next.
pub(super) fn spent_note(total: u32, tasks: usize) -> String {
    let s = |n: u64| if n == 1 { "" } else { "s" };
    format!(
        "tool budget spent \u{2014} {total} call{} over {tasks} task{}; \
         the workers answer from what they have",
        s(u64::from(total)),
        s(tasks as u64)
    )
}
