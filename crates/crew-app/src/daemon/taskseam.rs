//! Test seams into the bridge's private state — a child module, so it reaches the fields
//! without widening them.
use super::{Bridge, Route};

impl Bridge {
    /// Put `addr` into the awaiting state without a session — the seam a test uses to prove
    /// that nothing else claims what a blocked conversation says.
    pub(crate) fn hold(&mut self, addr: &str, id: &str) {
        self.routes.insert(
            addr.to_string(),
            Route {
                session: String::new(),
                reply: addr.to_string(),
                cursor: 0,
                awaiting: Some(id.to_string()),
            },
        );
    }
}
