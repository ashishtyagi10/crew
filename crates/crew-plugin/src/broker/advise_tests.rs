//! The advisor as a relay test installs it: a fake call on this thread
//! (`live` reads it), and a surface whose `sys:run` fails as the session's
//! does. The advisor's own behaviour is crew-hive's to test
//! (`crew_hive::advisor`).
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use super::*;
use crate::broker::toolcall::ToolRunner;

/// The briefs an advisor was given, in order.
pub(super) type Briefs = Arc<Mutex<Vec<String>>>;

/// What one fake advice call cost.
const SPENT: Spent = Spent {
    input: 40,
    output: 9,
    micros_usd: 120,
};

/// The failure of a build that did not build.
pub(super) const BROKEN: &str = "exit 101\n   Compiling crew v0.1.0\nerror[E0308]: \
                                 mismatched types\n  --> src/a.rs:3:5\nerror: could not compile";

type Reply = Result<&'static str, &'static str>;

/// One fake advisor call, answered as it is made.
pub(super) type Fake = dyn Fn(&str) -> Result<(String, Spent), String> + Send + Sync;

thread_local! {
    /// The advisor this thread's test installed.
    static INSTALLED: RefCell<Option<Arc<Fake>>> = const { RefCell::new(None) };
}

/// An advisor that answers `reply` and keeps every brief it was given.
fn fake(reply: Reply, briefs: Briefs) -> Box<Fake> {
    Box::new(move |brief: &str| {
        briefs.lock().unwrap().push(brief.to_string());
        reply
            .map(|r| (r.to_string(), SPENT))
            .map_err(str::to_string)
    })
}

/// The seam `live` reads under test: this thread's advisor, else none.
pub(super) fn installed() -> Option<Ask> {
    let fake = INSTALLED.with(|i| i.borrow().clone())?;
    Some(Arc::new(move |brief: String| -> Asked {
        let got = fake(&brief);
        Box::pin(async move { got })
    }))
}

/// Advice as this thread's turns get it, until the guard drops.
pub(super) struct Installed;

impl Drop for Installed {
    fn drop(&mut self) {
        INSTALLED.with(|i| *i.borrow_mut() = None);
    }
}

/// `ask` as this thread's advisor.
pub(super) fn install(ask: Box<Fake>) -> Installed {
    INSTALLED.with(|i| *i.borrow_mut() = Some(Arc::from(ask)));
    Installed
}

pub(super) fn advising(reply: Reply) -> (Briefs, Installed) {
    let briefs = Briefs::default();
    let on = install(fake(reply, Arc::clone(&briefs)));
    (briefs, on)
}

/// A surface whose `sys:run` fails on exit 101, as `SessionTools` says.
pub(super) struct Runs;

impl ToolRunner for Runs {
    fn hint(&self) -> String {
        "TOOLS: sys:run".into()
    }
    fn call(&self, _s: &str, _t: &str, args: &str) -> Result<String, String> {
        Ok(match args.contains("build") {
            true => BROKEN.into(),
            false => "exit 0\nfine".into(),
        })
    }
    fn failed(&self, s: &str, t: &str, out: &str) -> bool {
        (s, t) == ("sys", "run") && out.starts_with("exit 101")
    }
}
