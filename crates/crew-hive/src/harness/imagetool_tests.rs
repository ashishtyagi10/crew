//! A picture from the user's message and one a tool returned both reach the
//! next request, through the wrapper a swarm worker's tools sit behind
//! (`per_task`), and an old one is dropped when the turns are cut to fit.
use std::sync::{Arc, Mutex};

use super::*;
use crate::bus::{AgentId, EventBus};
use crate::provider::{Completion, CompletionRequest, Image, ProviderError, ToolInvocation, Turn};
use crate::tools::budget::ToolBudget;
use crate::tools::todo::Checklist;
use crate::tools::ToolSpec;

struct Scripted(Mutex<Vec<Completion>>, Arc<Mutex<Vec<CompletionRequest>>>);

impl Provider for Scripted {
    fn supports_tools(&self) -> bool {
        true
    }
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        self.1.lock().unwrap().push(req);
        let c = self.0.lock().unwrap().pop().unwrap_or_default();
        Box::pin(async move { Ok(c) })
    }
}

fn shot(name: &str) -> Image {
    Image::from_bytes(name, &crate::provider::image::fake_png(2, 2)).unwrap()
}

/// `fs:look` answers `image: <path>` and hands back a picture for it, like
/// `sys:read_file` of a PNG; it keeps a checklist so `per_task` wraps it.
struct Looks(Checklist);

impl Tools for Looks {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, _: &str, _: &str, args: &str) -> Result<String, String> {
        Ok(format!("image: {}", args.trim_matches('"')))
    }
    fn attachments(&self, _: &str, _: &str, args: &str, output: &str) -> Vec<Image> {
        match output.starts_with("image: ") {
            true => vec![shot(args.trim_matches('"'))],
            false => Vec::new(),
        }
    }
    fn checklist(&self) -> Option<&Checklist> {
        Some(&self.0)
    }
    fn specs(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            server: "fs".into(),
            tool: "look".into(),
            description: String::new(),
            input_schema: serde_json::json!({"type": "object"}),
        }]
    }
}

fn looking(n: usize) -> Completion {
    Completion {
        calls: (0..n)
            .map(|i| ToolInvocation {
                id: format!("id{i}"),
                name: "fs__look".into(),
                input: serde_json::json!(format!("p{i}.png")),
                bad_args: None,
            })
            .collect(),
        ..Default::default()
    }
}

async fn drive(n: usize) -> Vec<CompletionRequest> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let replies = vec![
        Completion {
            text: "done".into(),
            ..Default::default()
        },
        looking(n),
    ];
    let provider = Arc::new(Scripted(Mutex::new(replies), Arc::clone(&seen)));
    let ctx = TurnContext {
        agent: AgentId(1),
        events: Arc::new(EventBus::new(64)),
        budget: ToolBudget::turn(3),
        cancel: Default::default(),
        advice: None,
    };
    let spec = TurnSpec {
        effort: None,
        ask: "look".into(),
        prompt: "look".into(),
        images: vec![shot("pasted.png")],
        hinted: None,
        shed: None,
        ending: None,
        call_timeout: None,
        system: None,
        model: "claude-sonnet-5-5".into(),
        tier: ModelTier::Standard,
        max_tokens: 16,
    };
    let tools = crate::tools::todo::per_task(Arc::new(Looks(Checklist::default())));
    let ended = run(ctx, spec, provider, Some(tools), None).await;
    assert!(matches!(ended, Ended::Answered(_)), "{ended:?}");
    let seen = seen.lock().unwrap().clone();
    seen
}

#[tokio::test]
async fn the_users_picture_rides_every_request_and_a_tools_rides_its_result() {
    let seen = drive(1).await;
    assert_eq!(seen.len(), 2);
    // The opening message's picture is resent unchanged: earlier turns stay
    // byte-identical, so a prompt cache keeps hitting.
    assert_eq!(seen[0].images, seen[1].images);
    assert_eq!(seen[1].images.len(), 1);
    let Turn::ToolResults(r) = &seen[1].turns[1] else {
        panic!("{:?}", seen[1].turns)
    };
    assert_eq!(r[0].images.len(), 1);
    assert_eq!(r[0].images[0].name, "p0.png");
    assert!(seen[0].turns.is_empty());
}

#[tokio::test]
async fn a_round_attaches_five_pictures_and_notes_the_rest() {
    let seen = drive(6).await;
    let Turn::ToolResults(r) = &seen[1].turns[1] else {
        panic!()
    };
    assert_eq!(r.iter().map(|o| o.images.len()).sum::<usize>(), 5);
    assert!(r[5].images.is_empty());
    assert!(
        r[5].content
            .contains("[image omitted: p5.png, only 5 images per round]"),
        "{}",
        r[5].content
    );
}
