//! The arms read the thread back on the next turn and record the one they
//! answered: the swarm's framed goal, the relay's first hop
//! (`relay_counting`) and the fan (`fan_recorded`).
use std::sync::{Arc, Mutex};

use crate::broker::session::Session;
use crate::broker::swarm::run_task;
use crate::broker::thread::{lock, with_context, CONTEXT_CAP};
use crate::broker::{specialists, testenv};
use crate::protocol::PluginEvent;

/// The framed goal, read off the plan the stub planner turns it into.
fn planned_prompt(session: &Session, task: &str) -> String {
    let mut evs = Vec::new();
    run_task(task, false, session, &mut |ev| {
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
fn with_one_turn_the_planner_goal_carries_the_block_between_the_skills_and_the_memory() {
    let _env = testenv::mock("hi");
    let dir = std::path::PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    std::fs::write(dir.join(".crew/memory.md"), "- deploys go out on Fridays\n").unwrap();
    let session = Session::new();
    lock(&session.thread).record("list the crates", "crew-app, crew-hive, crew-plugin");
    let skilled = crate::broker::skillframe::with_skills("plan the release").body;
    let expected = crate::broker::memory::prepend(
        crate::broker::memory::load(),
        &with_context(&session.thread, &skilled),
    );
    let p = planned_prompt(&session, "plan the release");
    assert_eq!(p, expected);
    let mem = p.find("deploys go out").unwrap();
    let block = p.find("Earlier in this conversation:").unwrap();
    let now = p.find("\n\nNow:\n").unwrap();
    let task = p.rfind("plan the release").unwrap();
    assert!(
        mem < block && block < now && now < task,
        "memory, then the thread, then the goal: {p}"
    );
}

/// One relay through the real entry; the first hop's prompt size, in the
/// mock provider's words (`Stats.ctx`), and the events.
fn relay_ctx(session: &Session, input: &str) -> u64 {
    let mut evs = Vec::new();
    crate::broker::stdio::relay_counting(
        input,
        session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs.iter()
        .find_map(|e| match e {
            PluginEvent::Stats { agent, ctx, .. } if agent == "helper" => Some(*ctx),
            _ => None,
        })
        .expect("the dialled agent's reply stat")
}

#[test]
fn the_relays_first_hop_carries_the_block_and_its_done_body_becomes_the_turn() {
    let _env = testenv::mock("ok\n@done");
    let base = std::path::PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    specialists::record_at(&base, &[("helper".into(), String::new())]);
    let session = Session::new();
    let before = relay_ctx(&session, "@helper summarize the crates");
    {
        let t = lock(&session.thread);
        let turn = t.turns().next().expect("the relay recorded its turn");
        assert_eq!(turn.asked, "@helper summarize the crates");
        assert_eq!(turn.answered, "ok");
    }
    let block = lock(&session.thread).context(CONTEXT_CAP).unwrap();
    let after = relay_ctx(&session, "@helper summarize the crates");
    // The mock counts prompt words. The block reaches the model at least
    // once (the framed task); the hop's transcript keeps a 400-char copy of
    // the task too, so on a machine with no `~/.claude/skills` — the CI
    // runners — the block sits inside that copy and is counted twice, while
    // a machine whose skills frame pushes it past the clip counts it once.
    // Either is the block arriving; the exact multiple is not the claim.
    let words = block.split_whitespace().count() as u64 + 1;
    assert!(
        after - before >= words && after - before <= 2 * words,
        "block:\n{block}\n(delta {})",
        after - before
    );
    assert_eq!(lock(&session.thread).len(), 2);
}

/// A fan agent that keeps every prompt and answers in its own name.
struct Echo(&'static str, Arc<Mutex<Vec<String>>>);

impl crate::Adapter for Echo {
    fn name(&self) -> &str {
        self.0
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: std::time::Duration) -> Result<String, String> {
        self.1.lock().unwrap().push(body.to_string());
        Ok(format!("{} says hi\n@done", self.0))
    }
}

#[test]
fn a_fan_records_its_combined_replies_and_the_next_fan_reads_them() {
    let _env = testenv::mock("unused");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let reg = crate::Registry::new(vec![
        Box::new(Echo("a", Arc::clone(&seen))) as Box<dyn crate::Adapter>,
        Box::new(Echo("b", Arc::clone(&seen))),
    ]);
    let names = vec!["a".to_string(), "b".to_string()];
    let session = Session::new();
    let tick = crate::broker::tick::noop_tick_emit();
    let mut sink = |_| Ok(());
    crate::broker::intent::fan_recorded(&session, &reg, &names, "task one", &tick, &mut sink)
        .unwrap();
    {
        let t = lock(&session.thread);
        assert_eq!(t.len(), 1);
        let turn = t.turns().next().unwrap();
        assert_eq!(turn.asked, "task one");
        assert!(turn.answered.contains("a: a says hi"), "{}", turn.answered);
        assert!(turn.answered.contains("b: b says hi"), "{}", turn.answered);
    }
    assert!(seen
        .lock()
        .unwrap()
        .iter()
        .all(|p| !p.contains("Earlier in")));
    crate::broker::intent::fan_recorded(&session, &reg, &names, "task two", &tick, &mut sink)
        .unwrap();
    let prompts = seen.lock().unwrap();
    let second: Vec<&String> = prompts.iter().skip(2).collect();
    assert_eq!(second.len(), 2);
    assert!(second
        .iter()
        .all(|p| p.contains("Earlier in this conversation:")
            && p.contains("you asked: task one")
            && p.contains("Now:\ntask two")));
    assert_eq!(lock(&session.thread).len(), 2);
}
