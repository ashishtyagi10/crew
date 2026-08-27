/// Maximum number of panes shown at full size; the rest are minimized.
pub const MAX_FULL_TILES: usize = 6;

/// Tracks pane indices in most-recently-active-first order. The first
/// `MAX_FULL_TILES` are full tiles; the remainder are minimized (LRU).
#[derive(Debug, Clone, Default)]
pub struct GridLayout {
    /// Pane indices, most-recently-active first.
    order: Vec<usize>,
    /// Panes exempt from demotion, in the order they were pinned.
    ///
    /// The LRU is right about which pane you have not touched and wrong about
    /// whether that matters: the pane you are least likely to touch is often
    /// the agent you most want to keep watching. A pin says "this one stays
    /// on the grid", and nothing else about it changes.
    pinned: Vec<usize>,
}

impl GridLayout {
    #[cfg(test)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert `idx` as the most-recently-active pane. If present, moves it to
    /// the front rather than duplicating.
    pub fn add(&mut self, idx: usize) {
        self.order.retain(|x| *x != idx);
        self.order.insert(0, idx);
    }

    /// Move an existing `idx` to the front. No-op if `idx` is absent.
    pub fn touch(&mut self, idx: usize) {
        if let Some(pos) = self.order.iter().position(|x| *x == idx) {
            let v = self.order.remove(pos);
            self.order.insert(0, v);
        }
    }

    /// Drop every index `keep` rejects, without reindexing the rest — the
    /// panes vec is untouched (used for panes hidden into the left nav, which
    /// still exist at their index). Contrast [`Self::on_close`], which shifts.
    pub fn retain(&mut self, keep: impl Fn(usize) -> bool) {
        self.order.retain(|&x| keep(x));
        self.pinned.retain(|&x| keep(x));
    }

    /// Remove `idx`, then shift every stored index above it down by one to
    /// match `Vec::remove` reindexing the panes after a close.
    pub fn on_close(&mut self, idx: usize) {
        for list in [&mut self.order, &mut self.pinned] {
            list.retain(|x| *x != idx);
            for x in list.iter_mut() {
                if *x > idx {
                    *x -= 1;
                }
            }
        }
    }

    fn split(&self) -> usize {
        self.order.len().min(MAX_FULL_TILES)
    }

    /// Pin or unpin `idx`. Pinning a pane already pinned is a no-op, so a
    /// toggle is the caller's decision rather than this one's.
    pub fn set_pinned(&mut self, idx: usize, on: bool) {
        self.pinned.retain(|x| *x != idx);
        if on {
            self.pinned.push(idx);
        }
    }

    /// Every pinned index, for the frame that draws their markers.
    pub fn pinned_indices(&self) -> Vec<usize> {
        self.pinned.clone()
    }

    pub fn is_pinned(&self, idx: usize) -> bool {
        self.pinned.contains(&idx)
    }

    /// The order tiles are handed out in: pinned panes first, in the order
    /// they were pinned, then everything else by recency.
    ///
    /// More pins than tiles is possible and is not an error — the oldest pins
    /// win, and the rest demote like anything else. A pin cannot make room
    /// that does not exist.
    fn ranked(&self) -> Vec<usize> {
        let mut out: Vec<usize> = self
            .pinned
            .iter()
            .copied()
            .filter(|p| self.order.contains(p))
            .collect();
        let rest: Vec<usize> = self
            .order
            .iter()
            .copied()
            .filter(|x| !out.contains(x))
            .collect();
        out.extend(rest);
        out
    }

    /// Indices shown full (pinned first, then most-recently-active).
    pub fn full(&self) -> Vec<usize> {
        self.ranked()[..self.split()].to_vec()
    }

    /// Indices minimized (whatever the cap left over).
    pub fn minimized(&self) -> Vec<usize> {
        self.ranked()[self.split()..].to_vec()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.order.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}
