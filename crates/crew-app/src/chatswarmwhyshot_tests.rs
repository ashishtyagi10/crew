//! Off-screen render of a swarm run with a failure in it — the reason line
//! under the failed row and each finished row's spend at its right end — live
//! and after it folds into the record, at a narrow and a normal width, on a
//! dark page and a light one (where a muted count or a red line can fade).
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew swarm_why_shot -- --ignored`
use crate::chat::ChatPane;
use crate::chatswarmshot_tests::{dump, spec, H};
use crate::shotgpu_tests::shot_at;
use crew_hive::{AgentId, HiveEvent, TaskId, TaskState};
use crew_plugin::Plugin;

/// Five tasks: three finished with their spend, one failed with the
/// provider's words, the last still running — or, with `fold`, cancelled,
/// so the run folds into its record (opened).
fn failed_pane(fold: bool) -> ChatPane {
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
        spec(4, "writer", "write the comparison", &[2]),
        spec(5, "reviewer", "review the recommendation", &[4]),
    ]);
    // The frame clock cannot be stamped in the past: let a second pass and
    // put the run inside it (see `chatswarmshot_tests::live_pane`).
    let t0 = crate::anim::now_ms();
    std::thread::sleep(std::time::Duration::from_millis(1_000));
    let s = p.swarm.as_mut().unwrap();
    let runs = [
        (1, 0, 300, (1_234, 340), TaskState::Done),
        (3, 60, 520, (18_400, 2_050), TaskState::Failed),
        (2, 300, 700, (6_900, 880), TaskState::Done),
        (4, 700, 900, (3_100, 1_460), TaskState::Done),
    ];
    for (task, from, to, (input, output), state) in runs {
        let agent = AgentId(10 + task);
        let (a, t) = (agent.clone(), TaskId(task));
        s.apply_at(&HiveEvent::AgentSpawned { agent: a, task: t }, t0 + from);
        let spend = HiveEvent::TokenDelta {
            agent: agent.clone(),
            input,
            output,
        };
        s.apply_at(&spend, t0 + to);
        if state == TaskState::Failed {
            let error = "api error: model qwen-maxx does not exist\nhint: /model".into();
            s.apply_at(&HiveEvent::Failed { agent, error }, t0 + to);
        }
        s.apply_at(&HiveEvent::TaskStateChanged { task: t, state }, t0 + to);
    }
    let five = |state| HiveEvent::TaskStateChanged {
        task: TaskId(5),
        state,
    };
    s.apply_at(&five(TaskState::Running), t0 + 900);
    if fold {
        p.absorb_hive(&five(TaskState::Cancelled));
        let record = p.messages.last_mut().unwrap();
        record.expanded = true;
        record.ts.clear(); // settled, not fading in under the GPU warm-up
    }
    p
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn swarm_why_shot_live_and_folded_in_dark_and_light_at_fifty_and_a_hundred_columns() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (theme, id) in [
        ("dark", crew_theme::ThemeId::PaperDark),
        ("light", crew_theme::ThemeId::PaperLight),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        for (kind, fold) in [("live", false), ("record", true)] {
            let pane = failed_pane(fold);
            for (w, px_w) in [(50, 450), (100, 830)] {
                let name = format!("swarm-why-{kind}-{theme}-{w}");
                let shot = shot_at(&name, px_w, H, 13.0, "crew", |cols, rows, aspect| {
                    eprintln!("{name}: {cols} cols x {rows} rows");
                    if !fold {
                        dump(&pane, cols);
                    }
                    crate::chatview::art(&pane, cols, rows, aspect)
                });
                let Some(px) = shot else {
                    eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
                    return;
                };
                assert!(crate::shotgpu_tests::ink(&px) > 4000, "{name} drew");
            }
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
