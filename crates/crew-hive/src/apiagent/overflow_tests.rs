//! A native worker whose request outgrows the model's context: cut once and
//! sent again, and only for that error. Each test runs the whole agent against
//! a model with a window and reads the requests it was really sent.
use super::kit::{file, refuses, run_on, BAD_INPUT, DASHSCOPE};
use crate::board::TaskResult;
use crate::provider::{
    Completion, CompletionRequest, Provider, ProviderError, ToolInvocation, Turn,
};
use std::sync::{Arc, Mutex};

/// The window, in chars: `a.rs` and `b.rs` and the task fit, with `c.rs` too
/// they do not.
const N: usize = 10_000;

/// Every char a request carries, counted here rather than by `size`, so the
/// cut is not graded with its own ruler.
fn chars(r: &CompletionRequest) -> usize {
    let n = |s: &str| s.chars().count();
    let turns: usize = r
        .turns
        .iter()
        .map(|t| match t {
            Turn::Assistant { text, calls } => {
                n(text) + calls.iter().map(|c| n(&c.input.to_string())).sum::<usize>()
            }
            Turn::ToolResults(rs) => rs.iter().map(|r| n(&r.content)).sum(),
            Turn::User(s) => n(s),
        })
        .sum();
    n(&r.prompt) + r.system.as_deref().map_or(0, n) + turns
}

/// A native model with a window (`kit::refuses`), refusing with `refusal`.
struct Window {
    replies: Mutex<Vec<Completion>>,
    sent: Arc<Mutex<Vec<CompletionRequest>>>,
    refusal: &'static str,
    from: Option<usize>,
}

impl Provider for Window {
    fn supports_tools(&self) -> bool {
        true
    }
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        let mut sent = self.sent.lock().unwrap();
        let over = refuses(sent.len(), chars(&req), N, self.from);
        sent.push(req);
        let out = match over {
            true => Err(ProviderError::Api(self.refusal.into())),
            false => Ok(self.replies.lock().unwrap().pop().unwrap_or_default()),
        };
        Box::pin(async move { out })
    }
}

fn reading(path: &str) -> Completion {
    Completion {
        calls: vec![ToolInvocation {
            id: format!("call-{path}"),
            name: "sys__read_file".into(),
            input: serde_json::json!({ "path": path }),
            bad_args: None,
        }],
        ..Default::default()
    }
}

fn three_reads_then(answer: &str) -> Vec<Completion> {
    let mut replies: Vec<Completion> = ["a.rs", "b.rs", "c.rs"].map(reading).into();
    replies.push(Completion {
        text: answer.into(),
        ..Default::default()
    });
    replies
}

/// The task's result, the reads run, every request, and the notes said.
async fn run(
    replies: Vec<Completion>,
    refusal: &'static str,
    from: Option<usize>,
) -> (TaskResult, Vec<String>, Vec<CompletionRequest>, usize) {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(Window {
        replies: Mutex::new(replies.into_iter().rev().collect()),
        sent: Arc::clone(&sent),
        refusal,
        from,
    });
    let (out, ran, notes) = run_on(provider).await;
    let sent = sent.lock().unwrap().clone();
    (out, ran, sent, notes)
}

fn results(req: &CompletionRequest) -> Vec<String> {
    let each = |t: &Turn| match t {
        Turn::ToolResults(rs) => rs.iter().map(|r| r.content.clone()).collect(),
        _ => vec![],
    };
    req.turns.iter().flat_map(each).collect()
}

#[tokio::test]
async fn the_fourth_round_overflows_is_cut_and_the_task_succeeds() {
    let (out, ran, sent, notes) = run(three_reads_then("all three read"), DASHSCOPE, None).await;
    assert!(out.success, "{}", out.output);
    assert_eq!(out.output, "all three read");
    assert_eq!(ran, ["a.rs", "b.rs", "c.rs"]);
    assert_eq!(sent.len(), 5, "three reads, the refused call, the retry");
    assert!(chars(&sent[3]) > N, "{}", chars(&sent[3]));
    assert!(
        chars(&sent[4]) < N,
        "the retry is {} chars",
        chars(&sent[4])
    );
    let stub = |p: &str| {
        format!(
            "[result of sys:read_file {p} shortened to fit \u{2014} call it again if you need it]"
        )
    };
    assert_eq!(
        results(&sent[4]),
        [stub("a.rs"), stub("b.rs"), file("c.rs")]
    );
    // The calls and what the model wrote stay; only results are cut.
    assert_eq!(sent[4].turns.len(), sent[3].turns.len());
    assert_eq!(notes, 1, "said once");
}

#[tokio::test]
async fn an_overflow_on_every_try_is_two_tries_then_the_error() {
    let (out, _, sent, notes) = run(three_reads_then("never"), DASHSCOPE, Some(3)).await;
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
    let (out, _, sent, notes) = run(three_reads_then("never"), BAD_INPUT, None).await;
    assert!(!out.success);
    assert!(
        out.output.contains("unsupported input format"),
        "{}",
        out.output
    );
    assert_eq!(sent.len(), 4, "no second try");
    assert_eq!(notes, 0);
}

/// A read whose result was cut runs again when asked for, rather than being
/// answered with a pointer at a result the model can no longer see.
#[tokio::test]
async fn a_cut_read_asked_for_again_runs_again() {
    let mut replies = three_reads_then("a.rs read twice");
    replies.insert(3, reading("a.rs"));
    let (out, ran, sent, _) = run(replies, DASHSCOPE, None).await;
    assert!(out.success, "{}", out.output);
    assert_eq!(ran, ["a.rs", "b.rs", "c.rs", "a.rs"]);
    let last = results(sent.last().unwrap());
    assert_eq!(last.last(), Some(&file("a.rs")), "whole, not a pointer");
    assert!(last[..3].iter().all(|r| r.starts_with("[result of")));
}

/// Once cut, a round is whole only in the request right after it: a read
/// repeated in the next round runs again, since a pointer at the round
/// before would land in a request where that round is a stub.
#[tokio::test]
async fn once_cut_a_read_repeated_in_the_next_round_runs_again() {
    let mut replies = three_reads_then("d.rs read twice");
    replies.splice(3..3, [reading("d.rs"), reading("d.rs")]);
    let (out, ran, sent, _) = run(replies, DASHSCOPE, None).await;
    assert!(out.success, "{}", out.output);
    assert_eq!(ran, ["a.rs", "b.rs", "c.rs", "d.rs", "d.rs"]);
    let last = results(sent.last().unwrap());
    assert_eq!(last.last(), Some(&file("d.rs")), "whole, not a pointer");
}
