//! Standing memory reaches the swarm: `#note` lines were applied on the
//! relay and fan paths but silently skipped on the default `/crew` shape —
//! the one most messages take. These drive the real `run_task` frame under
//! the mock provider (stub planner, whose task prompts ARE the framed goal),
//! so what the planner was handed is readable off the `HivePlan` event.
use super::*;
use crate::broker::testenv;

/// Run `task` through `run_task` under the mock arm and return the prompt
/// the planner turned into its first task — the goal exactly as framed.
fn planned_prompt(task: &str) -> String {
    let session = Session::new();
    let mut evs = Vec::new();
    run_task(task, false, &session, &mut |ev| {
        evs.push(ev);
        Ok(())
    })
    .unwrap();
    evs.iter()
        .find_map(|e| match e {
            PluginEvent::HivePlan { tasks } => Some(tasks[0].prompt.clone()),
            _ => None,
        })
        .expect("a swarm run announces its plan")
}

#[test]
fn a_saved_note_reaches_the_swarm_planner() {
    let _env = testenv::mock("hi");
    let dir = std::path::PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    std::fs::write(
        dir.join(".crew").join("memory.md"),
        "- deploys go out on Fridays\n",
    )
    .unwrap();
    let p = planned_prompt("plan the release");
    assert!(p.contains("deploys go out on Fridays"), "{p}");
    assert!(p.contains("plan the release"), "{p}");
    assert!(p.to_uppercase().contains("STANDING MEMORY"), "{p}");
    let mem = p.find("deploys go out").unwrap();
    let task = p.find("plan the release").unwrap();
    assert!(mem < task, "memory precedes the task, as on the relay: {p}");
}

/// No memory file: the planner gets exactly what it got before memory rode
/// the swarm — the skill-framed task (the frame lists this machine's own
/// skills, so it is compared against, not spelled out) with no wrapper.
#[test]
fn with_no_memory_the_planner_sees_the_task_exactly_as_before() {
    let _env = testenv::mock("hi");
    let before = crate::broker::skillframe::with_skills("plan the release").body;
    let p = planned_prompt("plan the release");
    assert_eq!(p, before);
    assert!(!p.contains("STANDING MEMORY"), "{p}");
}
