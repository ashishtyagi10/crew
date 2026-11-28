//! How a nested `sys:explore` ends: the deadline, `/stop`, the bill, the cap.
use super::tests::*;
use std::sync::Mutex;

use crew_hive::ModelTier;

use super::*;

#[test]
fn the_deadline_ends_it_with_a_plain_line() {
    let (mut p, _) = script(vec![]);
    Arc::get_mut(&mut p).unwrap().hang = true;
    let limits = Limits {
        deadline: std::time::Duration::from_millis(150),
        ..Limits::default()
    };
    let out = go(p, limits, false, &Bill::default()).unwrap();
    assert!(out.contains("ran out of time"), "{out}");
    assert!(out.contains("explored: 0 files read, 0 searches"));
}

#[test]
fn stop_cancels_it() {
    let (p, seen) = script(vec![answering("late")]);
    let err = go(p, Limits::default(), true, &Bill::default()).unwrap_err();
    assert_eq!(err, crate::broker::cancelscope::STOPPED);
    assert!(
        seen.lock().unwrap().is_empty(),
        "a stopped turn still called the model"
    );
}

#[test]
fn its_tokens_go_on_the_bill() {
    let (p, _) = script(vec![answering("a:1")]);
    let bill = Bill::default();
    go(p, Limits::default(), false, &bill).unwrap();
    assert_eq!(bill.take().tokens(), 110);
    assert!(bill.take().is_zero(), "taken once");
}

#[test]
fn the_summary_is_capped() {
    let (p, _) = script(vec![answering(&"w".repeat(CAP * 2))]);
    let out = go(p, Limits::default(), false, &Bill::default()).unwrap();
    assert!(out.chars().count() < CAP + 220, "{}", out.len());
    assert!(out.contains("summary cut"));
}

#[test]
fn the_goal_is_required() {
    let (p, _) = script(vec![]);
    let stop = Arc::new(AtomicBool::new(false));
    let bill = Bill::default();
    for args in [r#"{}"#, r#"{"goal":"  "}"#, "nope"] {
        assert!(explore(args, p.clone(), "m", Limits::default(), &stop, &bill, None).is_err());
    }
}

#[test]
fn the_cheap_tier_is_tried_first_and_standard_is_the_fallback() {
    let tried = Mutex::new(Vec::new());
    let (p, _) = script(vec![]);
    let provider: Arc<dyn Provider> = p;
    let cheap = |t: ModelTier| {
        tried.lock().unwrap().push(t);
        Some((Arc::clone(&provider), format!("{t:?}")))
    };
    assert_eq!(pick(&cheap).unwrap().1, "Cheap");
    assert_eq!(*tried.lock().unwrap(), [ModelTier::Cheap]);
    let only_standard =
        |t: ModelTier| (t == ModelTier::Standard).then(|| (Arc::clone(&provider), "S".into()));
    assert_eq!(pick(&only_standard).unwrap().1, "S");
    assert!(pick(&|_| None).is_none());
}

#[test]
fn several_explores_in_one_reply_run_together() {
    let s = crate::broker::session::Session::new();
    let tools = s.tools_with_sys(true).unwrap();
    assert!(tools.repeatable("sys", "explore"));
}

#[test]
fn it_runs_from_inside_a_tokio_blocking_thread() {
    // The engines call tools on `spawn_blocking`; a nested runtime must not panic there.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let out = rt.block_on(async {
        let (p, _) = script(vec![answering("ok:1")]);
        tokio::task::spawn_blocking(move || go(p, Limits::default(), false, &Bill::default()))
            .await
            .unwrap()
    });
    assert!(out.unwrap().contains("\nok:1"));
}

/// A hooks file in a directory of its own, holding one rule at `stage`.
fn hooks(tag: &str, stage: &str, cmd: &str) -> crate::broker::session::hooks::Source {
    let dir = std::env::temp_dir().join(format!("crew-explore-hooks-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("hooks.json");
    let rule = serde_json::json!({"match": "sys:read_file", "cmd": cmd});
    std::fs::write(&path, serde_json::json!({ stage: [rule] }).to_string()).unwrap();
    crate::broker::session::hooks::Source {
        path,
        deadline: std::time::Duration::from_secs(5),
        cwd: Some(dir),
    }
}

#[test]
fn a_pre_hook_that_refuses_a_read_refuses_the_sub_agents_read_too() {
    let src = hooks("pre", "pre_tool", "echo 'no secrets' >&2; exit 2");
    let r = Reader::with_hooks(Some(src));
    let err = r
        .call("sys", "read_file", r#"{"path":".env"}"#)
        .unwrap_err();
    assert_eq!(err, "blocked by hook: no secrets");
    assert_eq!(r.counts(), "explored: 0 files read, 0 searches");
}

#[test]
fn a_post_hook_answers_back_under_the_nested_result() {
    let path = file("posthook", "body\n");
    let r = Reader::with_hooks(Some(hooks("post", "post_tool", "echo LINTED")));
    let out = r
        .call("sys", "read_file", &format!(r#"{{"path":{path:?}}}"#))
        .unwrap();
    assert!(out.contains("body") && out.contains("LINTED"), "{out}");
}

#[test]
fn one_hung_model_call_is_cut_off_without_spending_the_whole_deadline() {
    let (mut p, _) = script(vec![]);
    Arc::get_mut(&mut p).unwrap().hang = true;
    let limits = Limits {
        call: std::time::Duration::from_millis(100),
        ..Limits::default()
    };
    let t = std::time::Instant::now();
    let out = go(p, limits, false, &Bill::default());
    assert!(t.elapsed() < std::time::Duration::from_secs(30));
    let err = out.unwrap_err();
    assert!(err.contains("no answer from the model"), "{err}");
}
