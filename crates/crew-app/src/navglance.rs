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
    /// Under the pointer this frame: the row lifts, and no PANES row does.
    pub hovered: bool,
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
    /// The sky over the place the user named (`/weather`): a reading, a
    /// lookup in flight, a place not found, or nothing asked for.
    pub weather: crate::navweather::State,
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
            hovered: false,
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
            hovered: false,
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
    /// The glance cards' state this frame, the pointer applied — `None`
    /// while the nav shows the LOG.
    pub(crate) fn glance(&self) -> Option<Glance> {
        let mut g = self.glance_base()?;
        // The hover reads the nav's geometry, whose layout sizes the slot
        // from `glance_base` — never from here, or the two recurse until the
        // stack is gone (v0.22.0–v0.22.2 aborted on the first frame).
        if let Some(r) = self
            .hovered_waiting_row()
            .and_then(|k| g.waiting.get_mut(k))
        {
            r.hovered = true;
        }
        Some(g)
    }

    /// The cards before the pointer is applied: what the layout (`nav_tail`)
    /// and a click read. Reads only what the app already holds: no I/O on
    /// the winit thread.
    pub(crate) fn glance_base(&self) -> Option<Glance> {
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
            weather: crate::navweather::state(),
            serving,
            waiting,
        })
    }

    /// How the nav's variable slot is filled this frame, for the layout.
    pub(crate) fn nav_tail(&self) -> crate::navlayout::Tail {
        crate::navslot::tail(self.glance_base().as_ref(), self.log.len())
    }

    /// The pane a click on WAITING row `rel_row` (from the card's outer top)
    /// focuses — the card's whole point is to take you there.
    pub(crate) fn waiting_pane_at(
        &self,
        rel_row: u16,
        l: &crate::navlayout::NavLayout,
    ) -> Option<usize> {
        let i = crate::navwaitrow::at(rel_row, l)?;
        self.glance_base()?.waiting.get(i)?.pane
    }

    /// The WAITING row under the pointer, if the pointer is on the card.
    pub(crate) fn hovered_waiting_row(&self) -> Option<usize> {
        let (rel_row, l) = self.sidebar_row()?;
        crate::navwaitrow::at(rel_row, &l)
    }
}

#[cfg(test)]
#[path = "navglance_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "sidebarglanceshot_tests.rs"]
mod shot_tests;

#[cfg(test)]
#[path = "navglancesample_tests.rs"]
pub(crate) mod sample;
