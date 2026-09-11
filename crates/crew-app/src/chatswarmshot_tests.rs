//! Off-screen render of the chat pane mid-swarm — the status line, the task
//! rows with their spans, and the progress bar under a short transcript — at
//! the two widths the block changes shape between: narrow enough to lose the
//! bar and wide enough to keep it.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew swarm_block_shot -- --ignored`
use crate::chat::ChatPane;
use crate::shotgpu_tests::shot_at;
use crew_hive::{AgentId, AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

const H: u32 = 520;

fn spec(id: u64, specialty: &str, title: &str, deps: &[u64]) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: title.into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: "p".into(),
        specialty: specialty.into(),
        expertise: String::new(),
    }
}

/// Five tasks a third of the way in: two done, two running, one waiting.
fn live_pane() -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.connected = true;
    p.messages.push(crate::chatlayout::Message {
        sender: "user".into(),
        text: "compare the three sdf samplers and pick one".into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    });
    p.absorb_hive_plan(vec![
        spec(1, "scout", "survey the samplers in plot/sdf.rs", &[]),
        spec(2, "bench", "time each sampler at SUB 4 and 8", &[1]),
        spec(3, "critic", "check the ring track on paper light", &[1]),
        spec(4, "writer", "write the comparison", &[2, 3]),
        spec(5, "reviewer", "review the recommendation", &[4]),
    ]);
    // The frame clock starts at this process's first call, so a run "twelve
    // seconds ago" cannot be stamped in the past: let a second pass and put
    // the run inside it. The GPU warms up for a couple of seconds before the
    // frame is drawn, and the running task's bar grows over that; the
    // settled ones keep their share of the second either way.
    let t0 = crate::anim::now_ms();
    std::thread::sleep(std::time::Duration::from_millis(1_000));
    let s = p.swarm.as_mut().unwrap();
    let at = |ms: u64| t0 + ms;
    let ev = |task: u64, state: TaskState| HiveEvent::TaskStateChanged {
        task: TaskId(task),
        state,
    };
    s.apply_at(&ev(1, TaskState::Running), at(0));
    s.apply_at(&ev(1, TaskState::Done), at(600));
    s.apply_at(&ev(3, TaskState::Running), at(50));
    s.apply_at(&ev(3, TaskState::Done), at(900));
    s.apply_at(
        &HiveEvent::AgentSpawned {
            agent: AgentId(1),
            task: TaskId(2),
        },
        at(600),
    );
    s.apply_at(
        &HiveEvent::TokenDelta {
            agent: AgentId(1),
            input: 4_200,
            output: 610,
        },
        at(900),
    );
    s.apply_at(&HiveEvent::ToolBudget { used: 5, total: 20 }, at(900));
    p
}

/// Print the rows the block drew, so the PNG can be read against the cells.
fn dump(pane: &ChatPane, cols: u16) {
    let cells = crate::chatswarmview::block_cells(pane, cols, 0, crate::anim::now_ms());
    let rows = crate::chatswarmview::swarm_rows(pane, cols);
    for r in 0..rows {
        eprintln!(
            "{cols:>3}|{}",
            crate::chatswarmrows::tests::row_text(&cells, r)
        );
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn swarm_block_shot_at_sixty_and_a_hundred_columns() {
    let _g = crate::app::theme_test_guard();
    let pane = live_pane();
    for (name, w) in [("swarm-block-60", 520), ("swarm-block-100", 830)] {
        let Some(px) = shot_at(name, w, H, 13.0, "crew", |cols, rows, aspect| {
            eprintln!("{name}: {cols} cols x {rows} rows");
            dump(&pane, cols);
            crate::chatview::art(&pane, cols, rows, aspect)
        }) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 4000, "{name} drew");
    }
}
