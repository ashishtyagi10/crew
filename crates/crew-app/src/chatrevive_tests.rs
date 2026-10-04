use crew_plugin::Plugin;

use crate::chat::ChatPane;

/// A "broker" that exits after `n` seconds, writing each start to `file`.
fn mortal(file: &std::path::Path, secs: f32) -> ChatPane {
    let sh = format!("echo started >> '{}'; sleep {secs}", file.display());
    let plugin = Plugin::spawn("sh", &["-c".to_string(), sh]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

fn starts(file: &std::path::Path) -> usize {
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .count()
}

/// Poll until `done` holds, or five seconds pass.
fn until(p: &mut ChatPane, done: impl Fn(&ChatPane) -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !done(p) && std::time::Instant::now() < deadline {
        p.poll();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// The broker dies mid-task: the pane stops showing it running, says which
/// task did not finish and how to send it again, and starts the broker anew.
#[test]
fn a_broker_that_dies_mid_task_is_named_restarted_and_not_left_running() {
    let file = std::env::temp_dir().join(format!("crew-revive-{}-a", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let mut p = mortal(&file, 0.2);
    p.absorb_task(4, true);
    p.awaiting = true;
    until(&mut p, |p| {
        p.messages.iter().any(|m| m.text.contains("started again"))
    });
    let note = p
        .messages
        .iter()
        .rev()
        .find(|m| m.text.contains("stopped"))
        .map(|m| m.text.clone());
    let note = note.expect("the pane said its broker stopped");
    assert!(note.contains("Task #4 did not finish"), "{note}");
    assert!(note.contains("send your last message again"), "{note}");
    assert!(
        p.running_tasks.is_empty() && !p.awaiting,
        "nothing is left running"
    );
    until(&mut p, |_| starts(&file) >= 2);
    assert!(starts(&file) >= 2, "the broker was started again");
    let _ = std::fs::remove_file(&file);
}

/// A broker that keeps dying is left stopped after three restarts in five
/// minutes, and the pane says where its crash is recorded.
#[test]
fn a_broker_that_keeps_dying_is_left_stopped() {
    let file = std::env::temp_dir().join(format!("crew-revive-{}-b", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let mut p = mortal(&file, 0.0);
    until(&mut p, |p| {
        p.messages.iter().any(|m| m.text.contains("left stopped"))
    });
    let last = p
        .messages
        .last()
        .map(|m| m.text.clone())
        .unwrap_or_default();
    assert!(
        last.contains("left stopped") && last.contains("crash.log"),
        "{last}"
    );
    std::thread::sleep(std::time::Duration::from_millis(200));
    p.poll();
    assert_eq!(
        starts(&file),
        4,
        "the first start and three restarts, then no more"
    );
    let _ = std::fs::remove_file(&file);
}

/// A worker that crashed mid-swarm ends its task without its agents going
/// idle or its swarm folding: the task's end settles both, so the pane is
/// not left busy with nothing behind it.
#[test]
fn a_task_that_ended_mid_swarm_leaves_nothing_live() {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.absorb_task(2, true);
    p.absorb_hive_plan(vec![crew_hive::TaskSpec {
        id: crew_hive::TaskId(0),
        title: "research".into(),
        agent: crew_hive::AgentKind::Api { system: None },
        model: crew_hive::ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
        specialty: String::new(),
        expertise: String::new(),
    }]);
    p.active.push(crate::chatflow::ActiveAgent {
        name: "coder".into(),
        from: String::new(),
        since: std::time::Instant::now(),
        tool: None,
    });
    assert!(p.is_busy());
    p.absorb_task(2, false);
    assert!(
        p.swarm.is_none() && p.active.is_empty(),
        "nothing live is left"
    );
    assert!(!p.is_busy(), "the pane is idle again");
}
