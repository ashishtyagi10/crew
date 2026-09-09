//! Skills apply themselves: a relay task that names a loaded skill gets its
//! playbook framed into the body with no /skill command typed; loaded but
//! unmatched skills ride along as a one-line roster so the model can name one.
use std::sync::{Arc, Mutex};

use crate::broker::relay::relay_turn;
use crate::broker::testenv;
use crate::broker::{Adapter, Broker, Registry};

/// Records every prompt it was called with, then finishes the thread.
struct Capturing {
    name: String,
    calls: Arc<Mutex<Vec<String>>>,
}

impl Adapter for Capturing {
    fn name(&self) -> &str {
        &self.name
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: std::time::Duration) -> Result<String, String> {
        self.calls.lock().unwrap().push(body.to_string());
        Ok("ok\n@done".into())
    }
}

/// A guard whose project dir holds one seeded skill file.
fn seeded_guard(name: &str, body: &str) -> testenv::MockEnv {
    let g = testenv::mock("unused");
    let dir = std::path::PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    std::fs::create_dir_all(dir.join(".crew/skills")).unwrap();
    std::fs::write(dir.join(".crew/skills").join(format!("{name}.md")), body).unwrap();
    g
}

/// Run one relay turn on a capturing agent; return the prompt it saw.
fn first_prompt(task: &str) -> String {
    turn(task).0
}

/// One relay turn: the first prompt the agent saw, and every event emitted.
fn turn(task: &str) -> (String, Vec<crate::PluginEvent>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let agent: Box<dyn Adapter> = Box::new(Capturing {
        name: "claude".into(),
        calls: Arc::clone(&calls),
    });
    let broker = Broker::new(
        Registry::new(vec![agent]),
        6,
        std::time::Duration::from_secs(1),
    );
    let mut evs = Vec::new();
    let mut sink = |ev| {
        evs.push(ev);
        Ok(())
    };
    relay_turn(
        &broker,
        "claude",
        task,
        "t1",
        &crate::broker::tick::noop_tick_emit(),
        &mut sink,
    )
    .unwrap();
    let c = calls.lock().unwrap();
    (c[0].clone(), evs)
}

/// The `Loaded` events among `evs`, as `(agent, kind, name, detail)`.
fn loaded(evs: &[crate::PluginEvent]) -> Vec<(String, String, String, String)> {
    evs.iter()
        .filter_map(|e| match e {
            crate::PluginEvent::Hive {
                event:
                    crew_hive::HiveEvent::Loaded {
                        agent,
                        kind,
                        name,
                        detail,
                    },
            } => Some((agent.clone(), kind.clone(), name.clone(), detail.clone())),
            _ => None,
        })
        .collect()
}

/// An applied playbook is ANNOUNCED — one `Loaded` per skill, under the
/// dialled agent, before its reply — where it used to rewrite the prompt in
/// silence. An unmatched roster announces nothing: nothing was applied.
#[test]
fn an_applied_skill_is_announced_once_under_the_dialled_agent() {
    let _g = seeded_guard("review", "Check unsafe blocks first.");
    let (_, evs) = turn("run a review of lib.rs");
    assert_eq!(
        loaded(&evs),
        [(
            "claude".to_string(),
            "skill".to_string(),
            "review".to_string(),
            "applied \u{b7} Check unsafe blocks first.".to_string()
        )]
    );
    let first_reply = evs
        .iter()
        .position(|e| matches!(e, crate::PluginEvent::Message { .. }))
        .expect("a reply");
    let announce = evs
        .iter()
        .position(|e| matches!(e, crate::PluginEvent::Hive { .. }))
        .unwrap();
    assert!(
        announce < first_reply,
        "announced before the reply it heads"
    );
    let (_, evs) = turn("say hello");
    assert!(
        loaded(&evs).is_empty(),
        "nothing applied, nothing announced"
    );
}

#[test]
fn a_task_naming_a_skill_gets_its_playbook_with_no_command() {
    let _g = seeded_guard("review", "Check unsafe blocks first.");
    let p = first_prompt("run a review of lib.rs");
    assert!(p.contains("SKILL \u{201c}review\u{201d}"), "{p}");
    assert!(p.contains("Check unsafe blocks first."), "{p}");
}

#[test]
fn loaded_but_unmatched_skills_ride_along_as_a_roster_only() {
    // Frontmatter description ≠ body, so the roster (which shows the
    // description) can be told apart from an inlined playbook.
    let _g = seeded_guard(
        "deploy",
        "---\ndescription: ship safely\n---\nStep one: run the canary.",
    );
    let p = first_prompt("say hello");
    assert!(p.contains("AVAILABLE SKILLS"), "{p}");
    assert!(p.contains("deploy"), "{p}");
    assert!(
        !p.contains("Step one: run the canary."),
        "an unmatched playbook body must not inline: {p}"
    );
}
