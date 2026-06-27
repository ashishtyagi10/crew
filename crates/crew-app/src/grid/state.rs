/// Maximum number of panes shown at full size; the rest are minimized.
pub const MAX_FULL_TILES: usize = 6;

/// Tracks pane indices in most-recently-active-first order. The first
/// `MAX_FULL_TILES` are full tiles; the remainder are minimized (LRU).
#[derive(Debug, Clone, Default)]
pub struct GridLayout {
    /// Pane indices, most-recently-active first.
    order: Vec<usize>,
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

    /// Remove `idx`, then shift every stored index above it down by one to
    /// match `Vec::remove` reindexing the panes after a close.
    pub fn on_close(&mut self, idx: usize) {
        self.order.retain(|x| *x != idx);
        for x in &mut self.order {
            if *x > idx {
                *x -= 1;
            }
        }
    }

    fn split(&self) -> usize {
        self.order.len().min(MAX_FULL_TILES)
    }

    /// Indices shown full (most-recently-active, up to the cap).
    pub fn full(&self) -> &[usize] {
        &self.order[..self.split()]
    }

    /// Indices minimized (least-recently-active beyond the cap).
    pub fn minimized(&self) -> &[usize] {
        &self.order[self.split()..]
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}
