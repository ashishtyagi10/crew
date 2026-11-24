//! A raised level raises the room its thinking needs: the ceiling, the
//! deadline with it, and a stream so a long think is never a silent wait —
//! and a hand-typed level that is not a string costs nobody their roster.
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::broker::adapter::Adapter;
use crate::broker::apiadapter::ApiAdapter;
use crate::broker::specialists::{load_at, record_at};
use crew_hive::{Completion, CompletionRequest, Provider, ProviderError};

type Call = Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>;

/// Each call's ceiling, and whether it was asked as a stream.
#[derive(Default)]
struct Rec(Mutex<Vec<(u32, bool)>>);

impl Rec {
    fn answer(&self, req: CompletionRequest, streamed: bool) -> Call {
        self.0.lock().unwrap().push((req.max_tokens, streamed));
        Box::pin(async {
            Ok(Completion {
                text: "1. do it".into(),
                ..Default::default()
            })
        })
    }
}

impl Provider for Rec {
    fn complete(&self, req: CompletionRequest) -> Call {
        self.answer(req, false)
    }
    fn complete_streaming(&self, req: CompletionRequest, _: crew_hive::ChunkFn) -> Call {
        self.answer(req, true)
    }
}

fn dial(model: &str, effort: Option<Effort>) -> (u32, bool) {
    let rec = Arc::new(Rec::default());
    let a = ApiAdapter::new("planner", model, "", None, rec.clone()).unwrap();
    let _route = enter(effort);
    a.call("plan the scheduler", Duration::from_secs(5))
        .unwrap();
    let seen = rec.0.lock().unwrap().clone();
    seen[0]
}

#[test]
fn a_plan_routed_high_gets_the_room_to_think_and_streams_it() {
    assert_eq!(dial("claude-opus-5-5", Some(Effort::High)), (16_000, true));
    assert_eq!(dial("claude-opus-5-5", Some(Effort::Max)), (32_000, true));
    // The default and a level not sent keep the base ceiling, sent whole.
    assert_eq!(dial("claude-opus-5-5", None), (4096, false));
    assert_eq!(dial("claude-opus-5-5", Some(Effort::Low)), (4096, false));
    assert_eq!(dial("m", Some(Effort::High)), (4096, false));
}

#[test]
fn the_deadline_stretches_with_the_ceiling_and_only_then() {
    let t = Duration::from_secs(180);
    let (max, high) = room("claude-opus-5-5", Some(Effort::High), 4096, t);
    assert_eq!(max, 16_000);
    // 16K at the pace 180 s allowed 4096: about 703 s, never the 180 s that
    // a 16K reply at ~60 tok/s (~270 s) would outrun.
    assert!(
        high > Duration::from_secs(700) && high < Duration::from_secs(710),
        "{high:?}"
    );
    assert_eq!(room("claude-opus-5-5", None, 4096, t), (4096, t));
    assert_eq!(room("gpt-6-sol", Some(Effort::High), 4096, t), (4096, t));
}

#[test]
fn a_level_that_is_not_a_string_is_ignored_and_the_roster_survives() {
    let base = std::env::temp_dir().join(format!("crew-re-num-{}", std::process::id()));
    std::fs::create_dir_all(base.join(".crew")).unwrap();
    let json = r#"[{"name":"coder","role":"","last_used":2,"effort":3},
                  {"name":"writer","role":"docs","last_used":1,"effort":"max"}]"#;
    std::fs::write(base.join(".crew").join("specialists.json"), json).unwrap();
    let _route = enter(Some(Effort::Low));
    assert_eq!(for_turn_at(&base, "coder"), Some(Effort::Low));
    assert_eq!(for_turn_at(&base, "writer"), Some(Effort::Max));
    // The next run's write keeps everyone, and the hand-typed values too.
    record_at(&base, &[("tester".into(), "x".into())]);
    let kept = load_at(&base);
    let names: Vec<_> = kept.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(
        names.contains(&"coder") && names.contains(&"writer"),
        "{names:?}"
    );
    let coder = kept.iter().find(|s| s.name == "coder").unwrap();
    assert_eq!(coder.effort, Some(serde_json::json!(3)));
}
