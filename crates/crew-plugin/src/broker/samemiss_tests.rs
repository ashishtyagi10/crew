//! A failed edit sent again with nothing written since is answered from the
//! first failure, not run (`crew_hive::tools::seen::Seen::missed`).
use std::sync::Mutex;

use super::run_once;
use crate::broker::toolcall::ToolRunner;
use crew_hive::tools::exchanges::Exchanges;
use crew_hive::tools::{seen::Seen, ToolCall};

/// Every edit misses; counts the ones that ran.
struct Misses(Mutex<u32>);

impl ToolRunner for Misses {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        *self.0.lock().unwrap() += 1;
        Err("edit a.rs: `old` is not in the file \u{2014} the nearest is line 2".into())
    }
    fn repeatable(&self, _s: &str, _t: &str) -> bool {
        false
    }
}

#[test]
fn the_same_failing_edit_runs_once() {
    let runner = Misses(Mutex::new(0));
    let (mut seen, log) = (Seen::default(), Exchanges::default());
    let edit = ToolCall {
        server: "sys".into(),
        tool: "edit".into(),
        args: r#"{"path": "a.rs", "old": "let x = 1;", "new": "let x = 2;"}"#.into(),
    };
    let (ok, first) = run_once(&runner, &mut seen, &log, &edit, 1);
    assert!(!ok && first.starts_with("ERROR: edit a.rs"), "{first}");
    let (ok, again) = run_once(&runner, &mut seen, &log, &edit, 2);
    assert!(!ok);
    assert!(again.contains("failed in round 1"), "{again}");
    assert!(
        again.contains("the nearest is line 2"),
        "the first reason: {again}"
    );
    assert_eq!(*runner.0.lock().unwrap(), 1, "sent once");
}
