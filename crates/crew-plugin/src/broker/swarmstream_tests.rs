//! The closing answer over a scripted provider that streams: its pieces are
//! the lead's live card and the whole answer settles it. (The judge after it
//! streams nothing: `swarmverify_tests`.)
use super::super::swarmanswer::combine;
use super::super::swarmgap::Gap;
use super::*;
use crate::broker::cutoff::CUT_OFF;
use crate::broker::testenv;
use crew_hive::{AgentKind, ModelTier, ProviderError, TaskGraph, TaskId, TaskResult, TaskSpec};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

type Call = Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>;

/// Past the gate's gap, so every piece is its own `Delta` — a stamp taken
/// after a sleep is never early, so this cannot flake the other way.
const APART: Duration = Duration::from_millis(crate::broker::tick::TEXT_GAP_MS + 20);

/// Answers with its pieces joined (text and reasoning apart); when it
/// streams, says each piece first, `apart` from the last.
struct Scripted {
    pieces: Vec<Chunk<'static>>,
    truncated: bool,
    apart: Option<Duration>,
}

impl Provider for Scripted {
    fn complete(&self, _req: CompletionRequest) -> Call {
        let mut c = Completion {
            truncated: self.truncated,
            ..Default::default()
        };
        for piece in &self.pieces {
            match piece {
                Chunk::Text(t) => c.text.push_str(t),
                Chunk::Thought(t) => c.thought.push_str(t),
            }
        }
        Box::pin(async move { Ok(c) })
    }

    fn complete_streaming(&self, req: CompletionRequest, on_chunk: ChunkFn) -> Call {
        let (pieces, whole) = (self.pieces.clone(), self.complete(req));
        let Some(apart) = self.apart else {
            return whole; // what the trait's default does
        };
        Box::pin(async move {
            for (i, piece) in pieces.into_iter().enumerate() {
                if i > 0 && !apart.is_zero() {
                    tokio::time::sleep(apart).await;
                }
                on_chunk(piece);
            }
            whole.await
        })
    }
}

fn answer(pieces: Vec<Chunk<'static>>, truncated: bool, apart: Option<Duration>) -> Box<AnswerFn> {
    let p = Scripted {
        pieces,
        truncated,
        apart,
    };
    over(Arc::new(p), "m".into(), 2048)
}

/// Three independent tasks, all finished: three sinks, so the run wants the
/// lead's answer.
fn finished() -> (TaskGraph, Vec<TaskResult>) {
    let spec = |i: u64| TaskSpec {
        id: TaskId(i),
        title: format!("t{i}"),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: vec![],
        prompt: format!("t{i}"),
        specialty: format!("t{i}::x"),
        expertise: String::new(),
    };
    let done = |i: u64| TaskResult {
        task: TaskId(i),
        output: format!("out {i}"),
        success: true,
    };
    let graph = TaskGraph::new((0..3).map(spec).collect()).unwrap();
    (graph, (0..3).map(done).collect())
}

/// The closing call alone: every event it emits, each as one line — the
/// sequence is what the tests pin.
fn closing(call: &AnswerFn) -> Vec<String> {
    let _env = testenv::mock("unused");
    let (graph, results) = finished();
    let mut evs = Vec::new();
    let gap = Gap::default();
    let mut emit = |ev: PluginEvent| {
        evs.push(match ev {
            PluginEvent::Delta { agent, text, sub } => format!("delta {agent} sub={sub}: {text}"),
            PluginEvent::Thought { agent, text } => format!("thought {agent}: {text}"),
            PluginEvent::Message { sender, text, .. } => format!("message {sender}: {text}"),
            PluginEvent::Activity { agent, state, .. } => format!("activity {agent} {state}"),
            other => format!("{other:?}"),
        });
        Ok(())
    };
    combine("compare", &graph, &results, &gap, Some(call), &mut emit).unwrap();
    evs
}

#[test]
fn the_answer_types_itself_out_as_the_leads_card_and_the_whole_of_it_settles_it() {
    let pieces = vec![
        Chunk::Text("Both "),
        Chunk::Text("agree"),
        Chunk::Text(": X."),
    ];
    // Every piece and the settled whole go out under the name smith ANSWERS
    // under — a reply card from the first word; only his activity (the
    // header's pulse) keeps the bare name his chrome speaks in.
    assert_eq!(
        closing(&*answer(pieces, false, Some(APART))),
        [
            "activity agent smith thinking",
            "delta agent smith \u{2192} user sub=false: Both ",
            "delta agent smith \u{2192} user sub=false: agree",
            "delta agent smith \u{2192} user sub=false: : X.",
            "message agent smith \u{2192} user: Both agree: X.",
            "activity agent smith idle",
        ]
    );
}

#[test]
fn a_cut_answer_says_so_once_in_the_message_and_never_in_the_stream() {
    let pieces = vec![Chunk::Text("The answer is "), Chunk::Text("forty")];
    let evs = closing(&*answer(pieces, true, Some(APART)));
    let deltas: Vec<&String> = evs.iter().filter(|e| e.starts_with("delta")).collect();
    assert_eq!(deltas.len(), 2, "{evs:?}");
    assert!(!deltas.iter().any(|d| d.contains(CUT_OFF)), "{deltas:?}");
    let settled: Vec<&String> = evs.iter().filter(|e| e.starts_with("message")).collect();
    assert_eq!(
        settled,
        [&format!(
            "message agent smith \u{2192} user: The answer is forty\n\n{CUT_OFF}"
        )]
    );
}

#[test]
fn reasoning_is_the_leads_thought_before_the_answer_never_answer_text() {
    // All at once: the gate holds the second thought, and the answer's first
    // word pushes it out ahead of itself.
    let pieces = vec![
        Chunk::Thought("weighing "),
        Chunk::Thought("both"),
        Chunk::Text("X."),
    ];
    let evs = closing(&*answer(pieces, false, Some(Duration::ZERO)));
    assert_eq!(
        evs,
        [
            "activity agent smith thinking",
            "thought agent smith \u{2192} user: weighing ",
            "thought agent smith \u{2192} user: both",
            "delta agent smith \u{2192} user sub=false: X.",
            "message agent smith \u{2192} user: X.",
            "activity agent smith idle",
        ]
    );
}

#[test]
fn a_provider_that_cannot_stream_settles_the_pane_with_the_one_message() {
    let pieces = vec![Chunk::Text("Both "), Chunk::Text("agree.")];
    assert_eq!(
        closing(&*answer(pieces, false, None)),
        [
            "activity agent smith thinking",
            "message agent smith \u{2192} user: Both agree.",
            "activity agent smith idle",
        ]
    );
}
