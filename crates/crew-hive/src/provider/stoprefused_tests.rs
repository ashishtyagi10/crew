//! A model that declines a request says so, once: both wire shapes read it,
//! and the reply loop neither asks again nor shows an empty answer.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::provider::openai_http::parse_response;
use crate::provider::{AnthropicProvider, Completion, CompletionRequest, Provider};

#[test]
fn both_wire_shapes_say_the_model_declined() {
    let openai = r#"{"choices":[{"message":{"content":""},"finish_reason":"content_filter"}],"usage":{"prompt_tokens":9,"completion_tokens":0}}"#;
    assert!(parse_response(openai).unwrap().refused);
    let anthropic = r#"{"id":"m","type":"message","role":"assistant","model":"claude-opus-5-5",
        "content":[],"stop_reason":"refusal","usage":{"input_tokens":9,"output_tokens":0}}"#;
    assert!(
        AnthropicProvider::parse_response(anthropic)
            .unwrap()
            .refused
    );
    let answered = r#"{"choices":[{"message":{"content":"hi"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1}}"#;
    assert!(!parse_response(answered).unwrap().refused);
}

/// Declines every request, counting them.
struct Declines(Mutex<usize>);

impl Provider for Declines {
    fn complete(
        &self,
        _req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Completion, crate::provider::ProviderError>>
                + Send,
        >,
    > {
        *self.0.lock().unwrap() += 1;
        Box::pin(async {
            Ok(Completion {
                refused: true,
                ..Default::default()
            })
        })
    }
}

#[test]
fn a_declined_request_is_not_asked_again_and_says_why() {
    let p = Arc::new(Declines(Mutex::new(0)));
    let provider: Arc<dyn Provider> = p.clone();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let req = CompletionRequest {
        model: "m".into(),
        prompt: "task".into(),
        ..Default::default()
    };
    let got = rt.block_on(crate::provider::cutoff::whole(
        provider,
        req,
        None,
        Duration::from_secs(5),
    ));
    let said = got.unwrap_err().to_string();
    assert!(said.contains("declined to answer"), "{said}");
    assert_eq!(*p.0.lock().unwrap(), 1, "asked once, not three times");
}
