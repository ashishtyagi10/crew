//! The pane is busy for the whole turn, not only while a line is arriving.
use crate::chat::ChatPane;
use crew_plugin::Plugin;

fn pane() -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

/// Between smith's routing line (which clears `awaiting`) and the answering
/// agent's first activity, only the broker's running task says work is on.
#[test]
fn a_running_task_keeps_the_pane_busy_through_the_routing_gap() {
    let mut p = pane();
    p.absorb_task(1, true);
    p.awaiting = false;
    p.absorb_activity("agent smith".into(), "thinking", "user".into());
    p.absorb_activity("agent smith".into(), "idle", String::new());
    assert!(p.is_busy(), "the task is still running");
    p.absorb_task(1, false);
    assert!(!p.is_busy(), "and idle once it ends");
}
