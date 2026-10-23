//! Which advisor a turn gets (`resolve`): none when the knob is off, keyless
//! or mocked, and otherwise one plain, bounded completion on the model asked
//! for.
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use super::*;
use crew_hive::{Completion, CompletionRequest, ProviderError};

type Asked = Arc<Mutex<Vec<CompletionRequest>>>;
/// What a provider lookup finds.
type Found = Option<(Arc<dyn Provider>, String)>;

/// A provider that keeps every request it was sent and answers one line.
struct Kept(Asked);

impl Provider for Kept {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        self.0.lock().unwrap().push(req);
        Box::pin(async {
            Ok(Completion {
                text: "read the E0308".into(),
                input_tokens: 12,
                output_tokens: 4,
                ..Default::default()
            })
        })
    }
}

fn kept() -> (Asked, impl FnOnce() -> Found) {
    let asked = Asked::default();
    let provider: Arc<dyn Provider> = Arc::new(Kept(Arc::clone(&asked)));
    (asked, move || Some((provider, "qwen-max".to_string())))
}

#[test]
fn the_knob_turns_it_off_before_anything_is_resolved() {
    assert!(on(None) && on(Some("1")) && on(Some("")));
    assert!(!on(Some("0")) && !on(Some(" 0 ")));
    let found = || -> Found { panic!("resolved while off") };
    assert!(resolve(Some("0"), None, found).is_none());
}

#[test]
fn no_advisor_under_the_mock_or_with_no_provider() {
    let mock: Arc<dyn Provider> = Arc::new(crew_hive::MockProvider {
        reply: "advice".into(),
    });
    assert!(resolve(None, None, || Some((mock, "mock".into()))).is_none());
    assert!(resolve(None, None, || None).is_none());
}

#[test]
fn one_plain_bounded_completion_on_the_tier_model() {
    let (asked, found) = kept();
    let ask = resolve(None, None, found).expect("a provider resolved");
    let (reply, spent) = ask("the brief").unwrap();
    assert_eq!(reply, "read the E0308");
    assert_eq!((spent.input, spent.output), (12, 4));
    let asked = asked.lock().unwrap();
    let req = &asked[0];
    assert_eq!(req.model, "qwen-max");
    assert_eq!(req.prompt, "the brief");
    assert_eq!(req.max_tokens, 1024, "room for a model that thinks first");
    assert!(req.system.is_none() && req.tools.is_empty() && req.turns.is_empty());
}

#[test]
fn the_model_knob_names_the_model_and_a_blank_one_does_not() {
    let (asked, found) = kept();
    let ask = resolve(None, Some("qwen3.8-max".into()), found).unwrap();
    ask("b").unwrap();
    assert_eq!(asked.lock().unwrap()[0].model, "qwen3.8-max");
    let (asked, found) = kept();
    let ask = resolve(None, Some("  ".into()), found).unwrap();
    ask("b").unwrap();
    assert_eq!(asked.lock().unwrap()[0].model, "qwen-max");
}
