//! A text-path worker (`@tool` lines, no native tools) whose follow-up
//! outgrows the model's context: its log is cut once and the prompt sent
//! again, and only for that error.
use super::kit::{file, refuses, run_on, BAD_INPUT, DASHSCOPE};
use crate::board::TaskResult;
use crate::provider::{Completion, CompletionRequest, Provider, ProviderError};
use std::sync::{Arc, Mutex};

/// The window, in chars: the task and two reads' follow-up fit, the third's
/// (the first read shortened, `b.rs` and `c.rs` whole) does not.
const N: usize = 8_000;

/// A model with no native tools and a window (`kit::refuses`).
struct Window {
    replies: Mutex<Vec<String>>,
    prompts: Arc<Mutex<Vec<String>>>,
    refusal: &'static str,
    from: Option<usize>,
}

impl Provider for Window {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        let mut prompts = self.prompts.lock().unwrap();
        let chars = req.prompt.chars().count() + req.system.map_or(0, |s| s.chars().count());
        let over = refuses(prompts.len(), chars, N, self.from);
        prompts.push(req.prompt);
        let out = match over {
            true => Err(ProviderError::Api(self.refusal.into())),
            false => Ok(Completion {
                text: self.replies.lock().unwrap().pop().unwrap_or_default(),
                ..Default::default()
            }),
        };
        Box::pin(async move { out })
    }
}

/// Three reads, then the answer; the task's result, every prompt, the notes.
async fn run(refusal: &'static str, from: Option<usize>) -> (TaskResult, Vec<String>, usize) {
    let mut replies: Vec<String> = ["a.rs", "b.rs", "c.rs"]
        .map(|p| format!("reading {p}\n@tool sys:read_file {{\"path\": \"{p}\"}}"))
        .into();
    replies.push("all three read".into());
    replies.reverse();
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(Window {
        replies: Mutex::new(replies),
        prompts: Arc::clone(&prompts),
        refusal,
        from,
    });
    let (out, _, notes) = run_on(provider).await;
    let prompts = prompts.lock().unwrap().clone();
    (out, prompts, notes)
}

#[tokio::test]
async fn the_fourth_prompt_overflows_is_cut_and_the_task_succeeds() {
    let (out, sent, notes) = run(DASHSCOPE, None).await;
    assert!(out.success, "{}", out.output);
    assert_eq!(out.output, "all three read");
    assert_eq!(sent.len(), 5, "three reads, the refused call, the retry");
    let len = |i: usize| sent[i].chars().count();
    assert!(len(3) > N, "{}", len(3));
    assert!(len(4) < N, "the retry is {} chars", len(4));
    let retry = &sent[4];
    assert!(
        retry.contains(
            "CALLED sys:read_file {\"path\": \"a.rs\"}\n\
             CALLED sys:read_file {\"path\": \"b.rs\"}\n(the results of the calls above"
        ),
        "{retry}"
    );
    assert!(!retry.contains("a.rs line 000") && !retry.contains("b.rs line 000"));
    assert!(retry.contains(&file("c.rs")), "the last round is whole");
    assert_eq!(notes, 1, "said once");
}

#[tokio::test]
async fn an_overflow_on_every_try_is_two_tries_then_the_error() {
    let (out, sent, notes) = run(DASHSCOPE, Some(3)).await;
    assert!(!out.success);
    assert!(
        out.output.contains("Range of input length"),
        "{}",
        out.output
    );
    assert_eq!(sent.len(), 5, "three reads and exactly two tries");
    assert_eq!(notes, 1);
}

#[tokio::test]
async fn another_400_is_not_retried() {
    let (out, sent, notes) = run(BAD_INPUT, None).await;
    assert!(!out.success);
    assert_eq!(sent.len(), 4, "no second try");
    assert_eq!(notes, 0);
}
