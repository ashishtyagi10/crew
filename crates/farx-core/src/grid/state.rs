use super::geometry::MAX_FULL_TILES;

/// Tracks agent tile ids in most-recently-active-first order. The first
/// `MAX_FULL_TILES` are shown full; the rest are minimized (LRU).
#[derive(Debug, Clone, Default)]
pub struct GridLayout {
    /// Tile ids, most-recently-active first.
    order: Vec<usize>,
}

impl GridLayout {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert `id` as the most-recently-active tile. If it already exists it
    /// is moved to the front rather than duplicated.
    pub fn add(&mut self, id: usize) {
        self.order.retain(|x| *x != id);
        self.order.insert(0, id);
    }

    /// Remove `id` from the layout, if present.
    pub fn remove(&mut self, id: usize) {
        self.order.retain(|x| *x != id);
    }

    /// Move an existing `id` to the front (most-recently-active). No-op if
    /// `id` is not present.
    pub fn touch(&mut self, id: usize) {
        if let Some(pos) = self.order.iter().position(|x| *x == id) {
            let v = self.order.remove(pos);
            self.order.insert(0, v);
        }
    }

    fn split_point(&self) -> usize {
        self.order.len().min(MAX_FULL_TILES)
    }

    /// Ids shown at full size (the most-recently-active, up to the cap).
    pub fn full(&self) -> &[usize] {
        &self.order[..self.split_point()]
    }

    /// Ids that are minimized (least-recently-active beyond the cap).
    pub fn minimized(&self) -> &[usize] {
        &self.order[self.split_point()..]
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gridlayout_add_orders_most_recent_first() {
        let mut g = GridLayout::new();
        g.add(0);
        g.add(1);
        g.add(2);
        // Most-recently-added is first.
        assert_eq!(g.full(), &[2, 1, 0]);
        assert!(g.minimized().is_empty());
        assert_eq!(g.len(), 3);
    }

    #[test]
    fn gridlayout_add_is_idempotent_and_promotes() {
        let mut g = GridLayout::new();
        g.add(0);
        g.add(1);
        g.add(0); // re-add existing -> moves to front, no duplicate
        assert_eq!(g.full(), &[0, 1]);
        assert_eq!(g.len(), 2);
    }

    #[test]
    fn gridlayout_seventh_tile_minimizes_least_recent() {
        let mut g = GridLayout::new();
        for id in 0..7 {
            g.add(id); // 6 added most-recent-first, then a 7th
        }
        // Front six are full; the least-recently-active (id 0) is minimized.
        assert_eq!(g.full(), &[6, 5, 4, 3, 2, 1]);
        assert_eq!(g.minimized(), &[0]);
    }

    #[test]
    fn gridlayout_touch_promotes_into_full_set() {
        let mut g = GridLayout::new();
        for id in 0..7 {
            g.add(id);
        }
        // id 0 is minimized; touching it promotes it and demotes the current LRU (id 1).
        g.touch(0);
        assert_eq!(g.full()[0], 0);
        assert_eq!(g.minimized(), &[1]);
    }

    #[test]
    fn gridlayout_touch_absent_is_noop() {
        let mut g = GridLayout::new();
        g.add(0);
        g.touch(99);
        assert_eq!(g.full(), &[0]);
    }

    #[test]
    fn gridlayout_remove_drops_id() {
        let mut g = GridLayout::new();
        g.add(0);
        g.add(1);
        g.remove(1);
        assert_eq!(g.full(), &[0]);
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn gridlayout_empty_state() {
        let g = GridLayout::new();
        assert!(g.is_empty());
        assert_eq!(g.len(), 0);
        assert!(g.full().is_empty());
        assert!(g.minimized().is_empty());
    }
}
