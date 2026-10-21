//! Where everything in the list sits: each day's band on a row of its own
//! (`days`), then that day's threads, one or two lines each. Drawing, the
//! paint under it and a click all read this one answer, so they cannot
//! disagree about which thread is on which row.
use super::InboxPane;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Slot {
    /// A day's band, by its name.
    Band(String),
    /// A thread, by its index in the list.
    Thread(usize),
}

pub(crate) struct Shown {
    /// The lines a thread takes: one or two.
    pub lines: u16,
    /// Top to bottom: the row each slot starts on, and the slot.
    pub slots: Vec<(u16, Slot)>,
}

impl Shown {
    /// The threads on screen: the row each starts on, and its index.
    pub(crate) fn threads(&self) -> impl Iterator<Item = (u16, usize)> + '_ {
        self.slots.iter().filter_map(|(r, s)| match s {
            Slot::Thread(i) => Some((*r, *i)),
            Slot::Band(_) => None,
        })
    }
}

impl InboxPane {
    /// The list laid out in a pane `rows` tall, above the foot. The selected
    /// thread is always on screen, with its day's band over the first thread
    /// shown; the list scrolls only as far as it must to keep it there.
    pub(crate) fn shown(&self, rows: u16) -> Shown {
        let lines = if self.density.two(rows) { 2 } else { 1 };
        let room = rows.saturating_sub(1);
        let now = chrono::Utc::now().timestamp_millis();
        let bands: Vec<String> = self
            .rows
            .iter()
            .map(|t| super::days::band(t.at_ms, now))
            .collect();
        let sel = self.sel.min(self.rows.len().saturating_sub(1));
        // The earliest thread from which the selected one still fits: each
        // thread added above costs its lines, and a band when its day differs.
        let (mut top, mut used) = (sel, 1 + lines);
        while top > 0 {
            let more = used + lines + u16::from(bands[top] != bands[top - 1]);
            if more > room {
                break;
            }
            (top, used) = (top - 1, more);
        }
        let mut slots = Vec::new();
        let mut r = 0;
        for i in top..self.rows.len() {
            let band = i == top || bands[i] != bands[i - 1];
            if r + u16::from(band) + lines > room {
                break;
            }
            if band {
                slots.push((r, Slot::Band(bands[i].clone())));
                r += 1;
            }
            slots.push((r, Slot::Thread(i)));
            r += lines;
        }
        Shown { lines, slots }
    }
}
