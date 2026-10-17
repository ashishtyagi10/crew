use std::sync::{Arc, Mutex};

use crew_hive::tools::ToolCall;
use crew_hive::HiveEvent;

use super::{call, result, RESULT_KEEP};
use crate::broker::adapter::HopStream;
use crate::broker::toolbatch::Ran;
use crate::broker::Envelope;

fn captured() -> (Arc<Mutex<Vec<HiveEvent>>>, HopStream) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let keep = log.clone();
    let mut s = HopStream::noop();
    s.on_tool = Arc::new(move |e| keep.lock().unwrap().push(e));
    (log, s)
}

fn env() -> Envelope {
    Envelope {
        from: "user".into(),
        to: "engineer".into(),
        thread_id: "t1".into(),
        body: String::new(),
        hop: 1,
    }
}

fn run() -> ToolCall {
    ToolCall {
        server: "sys".into(),
        tool: "run".into(),
        args: r#"{"cmd":"cargo test"}"#.into(),
    }
}

#[test]
fn a_call_and_its_result_are_the_hive_events_claude_codes_are() {
    let (log, s) = captured();
    call(&s, &env(), &run());
    let ran = Ran {
        ok: false,
        text: "exit 101\n".into(),
        ms: 800,
    };
    result(&s, &env(), &run(), &ran);
    let evs = log.lock().unwrap();
    assert!(
        matches!(&evs[0], HiveEvent::ToolCall { label, args, .. } if label == "sys:run" && args.contains("cargo test")),
        "{evs:?}"
    );
    assert!(
        matches!(&evs[1], HiveEvent::ToolResult { label, ok: false, ms: 800, text, .. } if label == "sys:run" && text == "exit 101"),
        "{evs:?}"
    );
}

#[test]
fn a_long_result_keeps_its_head_and_says_it_was_cut() {
    let (log, s) = captured();
    let ran = Ran {
        ok: true,
        text: "y\n".repeat(RESULT_KEEP),
        ms: 1,
    };
    result(&s, &env(), &run(), &ran);
    let evs = log.lock().unwrap();
    let HiveEvent::ToolResult { text, .. } = &evs[0] else {
        panic!("{evs:?}")
    };
    assert!(text.ends_with('\u{2026}'));
    assert!(text.chars().count() <= RESULT_KEEP + 2);
}
