//! Making a crew pane, and the small questions asked of one: whether it is
//! busy, what it draws, which messages are in view.
//!
//! Split from [`crate::chat`] for the line cap, which the struct's own field
//! documentation had already nearly filled on its own.
use crew_plugin::Plugin;
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatlayout::Message;

impl ChatPane {
    pub fn new(plugin: Plugin, channel: String) -> Self {
        ChatPane {
            readouts: crate::readout::Readouts::default(),
            plugin,
            channel,
            messages: Vec::new(),
            input: String::new(),
            connected: false,
            agents: Vec::new(),
            scroll: 0,
            awaiting: false,
            active: Vec::new(),
            tokens: 0,
            tok_in: 0,
            tok_out: 0,
            cost_microusd: 0,
            turns: 0,
            agent_stats: std::collections::HashMap::new(),
            ctx: std::collections::HashMap::new(),
            unread: 0,
            pulse: crate::chatpulse::Pulse::new(),
            mention: None,
            palette: None,
            histsearch: None,
            find: None,
            keyentry: None,
            oauth: None,
            show_source: false,
            compact_view: false,
            swarm: None,
            queued: std::collections::VecDeque::new(),
            folded: 0,
            plan_pending: false,
            git_branch: None,
            cwd: None,
            pending_recent: None,
            running_tasks: Vec::new(),
            streaming: Vec::new(),
            pending_reply_usage: None,
            history: crate::chathistory::History::default(),
        }
    }

    /// Whether the pane is awaiting a reply (busy), for the progress sweep —
    /// either our own send is unanswered or agents are mid-turn.
    pub fn is_busy(&self) -> bool {
        self.awaiting || !self.active.is_empty() || self.swarm.is_some()
    }

    /// Render the channel as CellView cells: a status header, the agent roster
    /// (when known), role-styled message cards, and the input composer. Tiny
    /// panes (no room for a header) fall back to the plain body.
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        crate::chatview::cells(self, cols, rows)
    }

    /// Every card the transcript should draw this frame. ONE source, so the
    /// scroll clamp, the scrollbar, the link hit-test and the unread pill can
    /// never disagree about what is on screen — the same reason `View` is
    /// threaded through all four render entry points.
    pub(crate) fn visible_messages(&self) -> Vec<&Message> {
        self.messages.iter().chain(self.streaming.iter()).collect()
    }
}
