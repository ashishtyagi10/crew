//! One agent's tool block (see [`crate::chattool`]): the lines of one run,
//! where it is anchored once the reply landed, and the tallies the summary
//! line reads. Its own file so the model (`chattool`) stays a model and the
//! counts stay in one place — the summary and the redraw predicate both
//! read them, and a count the two computed differently would draw a block
//! that says `3 tool calls` over four lines.
use crate::chattool::ToolLine;
use crate::chattoolkind::LineKind;

/// One agent's calls, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolBlock {
    pub agent_id: u64,
    /// The name the agent's cards go by (`stream_key`); `agent-N` until the
    /// broker's `Activity` names it.
    pub agent: String,
    pub lines: Vec<ToolLine>,
    /// The stamp of the settled reply the block sits above, once it landed.
    pub anchor: Option<String>,
    /// The block's run is over: it draws as a summary line, click to open.
    pub settled: bool,
    pub expanded: bool,
}

impl ToolBlock {
    /// Calls still waiting on a result.
    pub(crate) fn pending(&self) -> usize {
        self.lines.iter().filter(|l| l.done.is_none()).count()
    }

    /// `(calls, failed, total ms)` for the summary line — the tool calls
    /// alone; loads are counted by kind (`chattoolkind::summary_text`).
    pub(crate) fn tally(&self) -> (usize, usize, u64) {
        let calls = self.lines.iter().filter(|l| l.kind == LineKind::Tool);
        let (mut failed, mut ms, mut n) = (0, 0, 0);
        for l in calls {
            n += 1;
            if let Some(d) = &l.done {
                failed += usize::from(!d.ok);
                ms += d.ms;
            }
        }
        (n, failed, ms)
    }
}
