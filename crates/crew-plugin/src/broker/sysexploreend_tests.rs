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
    assert!(out.chars().count() < CAP + 120, "{}", out.len());
    assert!(out.contains("summary cut"));
}

#[test]
fn the_goal_is_required() {
    let (p, _) = script(vec![]);
    let stop = Arc::new(AtomicBool::new(false));
    let bill = Bill::default();
    for args in [r#"{}"#, r#"{"goal":"  "}"#, "nope"] {
        assert!(explore(args, p.clone(), "m", Limits::default(), &stop, &bill).is_err());
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
    assert!(out.unwrap().starts_with("ok:1"));
}
