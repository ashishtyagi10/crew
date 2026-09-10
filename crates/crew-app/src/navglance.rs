//! The nav's glance cards — what the LOG's slot shows instead of the log:
//! who SERVES smith work (provider, model, the 5h / 7d windows) and what is
//! WAITING ON YOU (prompt-blocked panes, a plan awaiting yes or no, tasks
//! in flight). The LOG said font switches and update polls; these say
//! something about the work. `/nav log` brings the tail back.
use crate::pane::{Pane, PaneContent};
use crate::usageledger::Windows;

/// Why a row is on the WAITING card, most urgent first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Wait {
    /// A terminal stopped at a prompt (see `blocked`).
    Blocked,
    /// A drafted plan wants Enter or Esc.
    Plan,
    /// Broker tasks in flight — not waiting on you, but worth a line.
    Running,
    /// Nothing at all: the card says so rather than vanishing.
    Quiet,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct WaitRow {
    pub kind: Wait,
    pub text: String,
    /// The pane a click on the row focuses.
    pub pane: Option<usize>,
}

/// Who serves the API-backed seats right now.
#[derive(Clone, Default, Debug)]
pub(crate) struct Serving {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub windows: Windows,
}

#[derive(Clone, Default, Debug)]
pub(crate) struct Glance {
    /// The sky over the place the user named (`/weather`), when known.
    pub weather: Option<crate::navweather::Weather>,
    pub serving: Serving,
    pub waiting: Vec<WaitRow>,
}

/// One pane's signals, as the pure row builder reads them.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct Signal {
    pub blocked: bool,
    pub plan: bool,
    pub running: usize,
}

/// The WAITING rows for `(index, title, signal)` per pane: blocked panes
/// first, then plans, then running counts — and one quiet row when there is
/// nothing, so the card never reads as broken. Pure.
pub(crate) fn rows_from(panes: impl IntoIterator<Item = (usize, String, Signal)>) -> Vec<WaitRow> {
    let mut out = Vec::new();
    for (i, title, s) in panes {
        let row = |kind, text| WaitRow {
            kind,
            text,
            pane: Some(i),
        };
        if s.blocked {
            out.push(row(Wait::Blocked, format!("\u{2691} {title}")));
        }
        if s.plan {
            out.push(row(Wait::Plan, format!("\u{21b5} plan \u{00b7} {title}")));
        }
        if s.running > 0 {
            out.push(row(
                Wait::Running,
                format!("\u{25b6} {} running \u{00b7} {title}", s.running),
            ));
        }
    }
    out.sort_by_key(|r| r.kind);
    if out.is_empty() {
        out.push(WaitRow {
            kind: Wait::Quiet,
            text: "nothing \u{2014} all quiet".into(),
            pane: None,
        });
    }
    out
}

fn signal(p: &Pane, now: u64) -> Signal {
    match &p.content {
        PaneContent::Chat(c) => Signal {
            blocked: false,
            plan: c.plan_pending,
            running: c.running_tasks.len(),
        },
        _ => Signal {
            blocked: crate::blocked::pane_blocked(p, now),
            ..Default::default()
        },
    }
}

impl crate::app::CrewApp {
    /// The glance cards' state this frame — `None` while the nav shows the
    /// LOG. Reads only what the app already holds: no I/O on the winit
    /// thread.
    pub(crate) fn glance(&self) -> Option<Glance> {
        if self.config.nav_card() != crate::navmode::NavCard::Glance {
            return None;
        }
        let now = crate::anim::now_ms();
        let model = self
            .panes
            .iter()
            .find_map(|p| match &p.content {
                PaneContent::Chat(c) => crate::chatpalette::shared_model(&c.agents),
                _ => None,
            })
            .map(|m| crate::summaryfit::short_model(&m).to_string());
        let serving = Serving {
            provider: crate::modelsignin::serving(),
            model,
            windows: crate::usageledger::windows(now),
        };
        let waiting = rows_from(
            self.panes
                .iter()
                .enumerate()
                .map(|(i, p)| (i, p.title_text(), signal(p, now))),
        );
        Some(Glance {
            weather: crate::navweather::now(),
            serving,
            waiting,
        })
    }

    /// How the nav's variable slot is filled this frame, for the layout.
    pub(crate) fn nav_tail(&self) -> crate::navlayout::Tail {
        crate::navslot::tail(self.glance().as_ref(), self.log.len())
    }

    /// The pane a click on WAITING row `rel_row` (from the card's outer top)
    /// focuses — the card's whole point is to take you there.
    pub(crate) fn waiting_pane_at(
        &self,
        rel_row: u16,
        l: &crate::navlayout::NavLayout,
    ) -> Option<usize> {
        if l.waiting_lines == 0 {
            return None;
        }
        // +1 for the border row, +1 to skip the section rule.
        let top = l.waiting_top + 2;
        let i = rel_row.checked_sub(top)? as usize;
        if i >= l.waiting_lines {
            return None;
        }
        self.glance()?.waiting.get(i)?.pane
    }
}

#[cfg(test)]
#[path = "navglance_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "sidebarglanceshot_tests.rs"]
mod shot_tests;
