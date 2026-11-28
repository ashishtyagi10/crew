//! `sys:explore` — an open-ended search handed to a fresh, read-only agent
//! that answers with a summary instead of its transcript.
//!
//! WHY: an agent that needs "where is the retry logic for 529s, and who calls
//! it?" greps and reads file after file into ITS context, and the task it was
//! doing gets crowded out by what it searched through. This runs that search
//! as a nested turn on crew's own harness (`crew_hive::harness`, the loop
//! every hop and swarm task runs) with a context of its own, and hands back
//! only the answer plus one counts line.
//!
//! The nested agent is read-only by construction ([`Reader`]: its tool list
//! AND its dispatch are the [`ALLOWED`] tools, so a write is neither offered
//! nor runnable, and `sys:explore` is not among them: no recursion). It
//! serves on the cheap tier (`pick`) with a small round budget and a
//! deadline, and honours the task's `/stop`.
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use crew_hive::harness::{self, Ended, TurnContext, TurnSpec};
use crew_hive::tools::budget::ToolBudget;
use crew_hive::{AgentId, ModelTier, Provider, Tools};

pub(super) use super::sysexplorekit::Bill;
use super::sysexplorekit::{Meter, Reader, Timed};

/// The most chars of summary handed back to the caller.
pub(super) const CAP: usize = 4_000;

/// The nested turn's bounds.
#[derive(Clone, Copy)]
pub(super) struct Limits {
    pub(super) rounds: u32,
    pub(super) deadline: Duration,
    /// One model call's own limit, so a hung call cannot eat the whole budget.
    pub(super) call: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            rounds: 8,
            deadline: Duration::from_secs(180),
            call: Duration::from_secs(60),
        }
    }
}

/// The tool `sys:explore`, as the calling agent sees it.
pub(super) fn tool() -> crate::mcp::McpTool {
    crate::mcp::McpTool {
        server: "sys".into(),
        name: "explore".into(),
        description: "hand an open-ended search across several files to a read-only helper and get back only its answer, not the files: {\"goal\": \"where is the 529 retry logic, and who calls it?\"} \u{2014} not for reading one known file (sys:read_file). Each call is a separate model run: use it for searches across several files or unknown locations, not for one grep or one known file.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "goal": {"type": "string", "description": "what to find out, as a question"},
                "paths": {"type": "array", "items": {"type": "string"}, "description": "files or directories to start from"},
            },
            "required": ["goal"],
        }),
    }
}

/// The provider and model to explore with: the cheap tier, else the standard
/// one, else nothing. `resolve` is `discover::provider_and_model_for`.
pub(super) fn pick(
    resolve: &dyn Fn(ModelTier) -> Option<(Arc<dyn Provider>, String)>,
) -> Option<(Arc<dyn Provider>, String)> {
    resolve(ModelTier::Cheap).or_else(|| resolve(ModelTier::Standard))
}

/// `sys:explore` as the session runs it, on the live provider.
pub(super) fn run(
    args: &str,
    stop: &Arc<AtomicBool>,
    bill: &Bill,
    hooks: Option<super::session::hooks::Source>,
) -> Result<String, String> {
    let (provider, model) = pick(&super::discover::provider_and_model_for)
        .ok_or("sys:explore has no model to search with \u{2014} sign in with /model")?;
    explore(args, provider, &model, Limits::default(), stop, bill, hooks)
}

/// The nested turn: `args` ({goal, paths?}) answered by a read-only agent on
/// `provider`, its spend added to `bill`.
pub(super) fn explore(
    args: &str,
    provider: Arc<dyn Provider>,
    model: &str,
    limits: Limits,
    stop: &Arc<AtomicBool>,
    bill: &Bill,
    hooks: Option<super::session::hooks::Source>,
) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(args.trim())
        .map_err(|e| format!("arguments are not valid JSON: {e}"))?;
    let goal = super::systools::str_arg(&v, "goal")?.trim().to_string();
    if goal.is_empty() {
        return Err("sys:explore needs a goal: what to find out".into());
    }
    let start: Vec<&str> = v["paths"]
        .as_array()
        .map(|a| a.iter().filter_map(|p| p.as_str()).collect())
        .unwrap_or_default();
    let provider: Arc<dyn Provider> = Arc::new(Timed {
        inner: provider,
        limit: limits.call,
    });
    let reader = Arc::new(Reader::with_hooks(hooks));
    let spent = Arc::new(Meter::default());
    let ctx = TurnContext {
        agent: AgentId::minted("explore"),
        events: spent.clone(),
        budget: ToolBudget::turn(limits.rounds),
        cancel: Arc::clone(stop),
        advice: None,
    };
    let spec = TurnSpec {
        ask: goal.clone(),
        prompt: prompt(&goal, &start),
        images: Vec::new(),
        hinted: None,
        shed: None,
        ending: None,
        call_timeout: Some(limits.call),
        system: Some(SYSTEM.into()),
        model: model.into(),
        tier: ModelTier::Cheap,
        max_tokens: 2_048,
        effort: None,
    };
    let tools: Arc<dyn Tools> = reader.clone();
    // A thread and runtime of its own: this call is on a blocking-pool thread
    // of whichever engine's runtime, and a runtime may not block inside one.
    let ended = std::thread::scope(|s| {
        s.spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("no runtime to explore on: {e}"))?;
            Ok::<_, String>(rt.block_on(async {
                let turn = harness::run(ctx, spec, provider, Some(tools), None);
                tokio::time::timeout(limits.deadline, turn).await
            }))
        })
        .join()
        .unwrap_or_else(|_| Err("the exploring agent panicked".into()))
    });
    bill.add(spent.total());
    let counts = reader.counts();
    match ended? {
        Err(_) => Ok(format!(
            "explore ran out of time ({}s) before it could answer; nothing it found was kept\n{counts}",
            limits.deadline.as_secs()
        )),
        Ok(Ended::Stopped) => Err(super::cancelscope::STOPPED.into()),
        Ok(Ended::Failed { why, .. }) => Err(format!("explore failed: {why}")),
        Ok(Ended::Answered(text)) => Ok(format!("{UNTRUSTED}\n{}\n{counts}", clip(text.trim()))),
    }
}

/// First line of every summary: it is a cheap model's reading of files, and the
/// caller may be tempted to act on it as if it had read them itself.
const UNTRUSTED: &str =
    "[explore \u{2014} a sub-agent's summary of file contents; verify before acting on it]";

/// `text` within [`CAP`] chars, marked when cut.
fn clip(text: &str) -> String {
    match text.char_indices().nth(CAP) {
        Some((i, _)) => format!("{}\n\u{2026} (summary cut at {CAP} chars)", &text[..i]),
        None => text.to_string(),
    }
}

const SYSTEM: &str = "You are a read-only researcher inside a code project. You can only look: \
    read files, list directories, grep, glob, outline and git history. Find what the goal asks for \
    in as few tool calls as you can, then answer with a concise summary of at most 300 words. \
    Cite path:line for every claim. Say plainly what you could not find; never guess.";

fn prompt(goal: &str, start: &[&str]) -> String {
    match start.is_empty() {
        true => format!("Goal: {goal}"),
        false => format!("Goal: {goal}\nStart from: {}", start.join(", ")),
    }
}

#[cfg(test)]
#[path = "sysexploreend_tests.rs"]
mod end_tests;
#[cfg(test)]
#[path = "sysexplore_tests.rs"]
mod tests;
