//! Walking the tree behind the `/disk` pane: kicking a scan off on a worker
//! thread, draining what it finds, and answering what is known so far.
//!
//! Split out of [`crate::diskpane`] for the line cap. The scan is off the
//! winit thread by construction — a `read_dir` over a deep tree on the main
//! thread freezes every pane in the grid.
use crate::diskpane::{walk, Child, DiskPane, Scan};
use std::sync::atomic::Ordering;
use std::sync::Arc;

impl DiskPane {
    /// Kick off a walk of `self.root` on a worker thread.
    pub(crate) fn start(&mut self) {
        self.scan.cancel.store(true, Ordering::Relaxed); // stop any previous walk
        let scan = Arc::new(Scan::default());
        self.scan = Arc::clone(&scan);
        self.children.clear();
        self.total = 0;
        self.files = 0;
        self.selected = 0;
        self.scanning = true;
        let root = self.root.clone();
        std::thread::spawn(move || walk(&root, &scan));
    }

    /// Rescan the current directory.
    pub fn rescan(&mut self) {
        self.start();
    }

    /// The child a tile index names, if it is a directory.
    pub fn child(&self, i: usize) -> Option<&Child> {
        self.children.get(i)
    }

    #[cfg(test)]
    pub fn children(&self) -> &[Child] {
        &self.children
    }

    /// Put a finished scan in without a worker, so the tile renderer can be
    /// exercised on names of a chosen length (the same seam `DashPane` uses).
    #[cfg(test)]
    pub(crate) fn seed_children(&mut self, children: Vec<Child>) {
        self.total = children.iter().map(|c| c.bytes).sum();
        self.files = children.len() as u64;
        self.scanning = false;
        self.children = children;
    }

    /// Take the worker's latest numbers. Returns true when they moved.
    pub fn poll(&mut self) -> bool {
        let seen = self.scan.seen.load(Ordering::Relaxed);
        let done = self.scan.done.load(Ordering::Relaxed);
        let mut changed = seen != self.files || (done && self.scanning);
        self.files = seen;
        if changed {
            if let Ok(list) = self.scan.children.lock() {
                let mut next = list.clone();
                next.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.name.cmp(&b.name)));
                changed = next != self.children;
                self.total = next.iter().map(|c| c.bytes).sum();
                self.children = next;
            }
        }
        if done {
            self.scanning = false;
        }
        changed
    }

    pub fn is_scanning(&self) -> bool {
        self.scanning
    }

    /// Stand in for a completed scan, for tests and the shot harness.
    /// `#[cfg(test)]` now that it lives beside the scan rather than beside the
    /// tests that call it: its name always said it was a test seam, and out
    /// here the non-test build could see it was unused.
    #[cfg(test)]
    pub(crate) fn set_children_for_test(&mut self, kids: &[(&str, u64, bool)], selected: usize) {
        self.children = kids
            .iter()
            .map(|(n, b, d)| Child {
                name: (*n).into(),
                bytes: *b,
                is_dir: *d,
            })
            .collect();
        self.total = self.children.iter().map(|c| c.bytes).sum();
        self.scanning = false;
        self.selected = selected;
    }
}
