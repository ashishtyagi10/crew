//! Where everything in the list sits: each day's band on a row of its own
//! (`days`), then that day's threads, one or two lines each. Drawing, the
//! paint under it and a click all read this one answer, so they cannot
//! disagree about which thread is on which row.
//!
//! The list breathes (goal 2026-10-20-inbox-list-look, made quiet on
//! 2026-10-21): a row of air under the pane's top edge and over the foot, a
//! blank row between two-line threads (none at crew's `compact` density),
//! and a blank row before each new day.
use super::InboxPane;

/// The row the list starts on, under a row of air.
pub(crate) const TOP: u16 = 1;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Slot {
    /// A day's band, by its name.
    Band(String),
    /// A thread, by its index in the list.
    Thread(usize),
    /// The thread folding away (`motion`), and the lines it still takes.
    Fold(u16),
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
            Slot::Band(_) | Slot::Fold(_) => None,
        })
    }
}

/// The blank rows between two threads of one day.
pub(crate) fn gap(lines: u16) -> u16 {
    match lines {
        2 => crate::density::level().card_gap_rows().min(1) as u16,
        _ => 0,
    }
}

impl InboxPane {
    /// The list laid out in the `rows` above the foot, as it is now.
    pub(crate) fn shown(&self, rows: u16) -> Shown {
        self.shown_at(rows, crate::anim::now_ms())
    }

    /// The list laid out at `now`. The selected thread is always on screen,
    /// with its day's band over the first thread shown; the list scrolls
    /// only as far as it must to keep it there. A thread folding away
    /// (`motion`) holds its place, a line shorter as it goes.
    pub(crate) fn shown_at(&self, rows: u16, now: u64) -> Shown {
        let lines = if self.density.two(rows) { 2 } else { 1 };
        let gap = gap(lines);
        // A row of air over the foot.
        let room = rows.saturating_sub(1);
        // Each thread, or the one folding (`None`): its time and its lines.
        let mut items: Vec<(Option<usize>, i64, u16)> = (0..self.rows.len())
            .map(|i| (Some(i), self.rows[i].at_ms, lines))
            .collect();
        if let Some((at, th, left, _)) = self.motion.folding(lines, now) {
            items.insert(at.min(items.len()), (None, th.at_ms, left));
        }
        if items.is_empty() {
            return Shown {
                lines,
                slots: Vec::new(),
            };
        }
        let wall = chrono::Utc::now().timestamp_millis();
        let bands: Vec<String> = items
            .iter()
            .map(|it| super::days::band(it.1, wall))
            .collect();
        // What item `i` costs below the one before it: the air, its day's
        // band when the day changes, and its lines.
        let below = |i: usize| match bands[i] != bands[i - 1] {
            true => 1 + 1 + items[i].2,
            false => gap + items[i].2,
        };
        let sel = self.sel.min(self.rows.len().saturating_sub(1));
        let sel = items.iter().position(|it| it.0 == Some(sel)).unwrap_or(0);
        // The earliest item from which the selected one still fits. The
        // first item shown always wears its day's band, so an item added
        // above costs what the old first one now costs below it.
        let (mut top, mut used) = (sel, TOP + 1 + items[sel].2);
        while top > 0 {
            let more = used + below(top) + items[top - 1].2 - items[top].2;
            if more > room {
                break;
            }
            (top, used) = (top - 1, more);
        }
        let mut slots = Vec::new();
        let mut r = TOP;
        for i in top..items.len() {
            let band = i == top || bands[i] != bands[i - 1];
            let air = match i == top {
                true => 0,
                false => below(i) - items[i].2 - u16::from(band),
            };
            if r + air + u16::from(band) + items[i].2 > room {
                break;
            }
            r += air;
            if band {
                slots.push((r, Slot::Band(bands[i].clone())));
                r += 1;
            }
            slots.push((r, items[i].0.map_or(Slot::Fold(items[i].2), Slot::Thread)));
            r += items[i].2;
        }
        Shown { lines, slots }
    }
}
