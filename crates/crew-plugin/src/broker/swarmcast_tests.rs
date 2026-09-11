//! The lead says what it chose — once, before the first worker speaks, and only when the
//! model actually chose.
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use crate::broker::session::sessiontest::crowded_tools;
use crate::broker::swarm::run_with;
use crate::broker::testenv;
use crate::broker::toolpick::BUDGET;
use crew_hive::StubPlanner;

/// A run over a crowded catalog with `reply` as the model's choice: every pane event, in order.
fn run(picker: Arc<Picker>) -> Vec<PluginEvent> {
    let _env = testenv::mock("unused");
    let tools = crowded_tools(true, Arc::clone(&picker));
    let factory = Arc::new(
        crew_hive::ApiFactory::new(Arc::new(super::super::tests::NativeProvider), 256)
            .with_tools(tools),
    );
    let mut evs = Vec::new();
    let mut sink = |ev| {
        evs.push(ev);
        Ok(())
    };
    let mut emit = announcing(&picker, &mut sink);
    run_with(
        "list the directory",
        Arc::new(StubPlanner { fanout: 2 }),
        factory,
        None,
        "test-model",
        Arc::new(AtomicBool::new(false)),
        None,
        &mut emit,
    )
    .unwrap();
    drop(emit);
    evs
}

fn messages(evs: &[PluginEvent]) -> Vec<(String, String)> {
    evs.iter()
        .filter_map(|e| match e {
            PluginEvent::Message { sender, text, .. } => Some((sender.clone(), text.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn a_crowded_catalog_with_a_chooser_is_said_once_before_the_first_worker_speaks() {
    let calls = Arc::new(AtomicUsize::new(0));
    let n = Arc::clone(&calls);
    let picker = Arc::new(Picker::fixed(Box::new(move |_: &str| {
        n.fetch_add(1, Ordering::SeqCst);
        Ok("TOOLS: noise:thing4".into())
    })));
    let msgs = messages(&run(picker));
    let tools_lines: Vec<&(String, String)> = msgs
        .iter()
        .filter(|(_, t)| t.starts_with("tools: chose"))
        .collect();
    assert_eq!(tools_lines.len(), 1, "{msgs:?}");
    let (sender, line) = tools_lines[0];
    assert_eq!(sender, SWARM_LEAD);
    // Chosen + every sys tool (find_tools among them), of the whole catalog.
    let sys = crate::broker::systools::tools().len();
    assert_eq!(
        line.trim_end_matches(|c| c != '\u{2014}'),
        format!("tools: chose {} of {} \u{2014}", sys + 1, sys + BUDGET + 8)
    );
    assert!(line.contains("noise:thing4"), "{line}");
    let at = msgs
        .iter()
        .position(|(_, t)| t.starts_with("tools: chose"))
        .unwrap();
    let planned = msgs
        .iter()
        .position(|(_, t)| t.starts_with("planned "))
        .unwrap();
    let worker = msgs
        .iter()
        .position(|(s, _)| s != SWARM_LEAD)
        .expect("a worker spoke");
    assert!(planned < at && at < worker, "{msgs:?}");
    // Three tasks (two leaves and a merge) share one text: one decision, one call.
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn with_the_scorer_deciding_nothing_is_said() {
    let msgs = messages(&run(Arc::new(Picker::off())));
    assert!(
        !msgs.iter().any(|(_, t)| t.starts_with("tools: chose")),
        "{msgs:?}"
    );
    assert!(
        msgs.iter().any(|(s, _)| s != SWARM_LEAD),
        "the run still ran"
    );
}
