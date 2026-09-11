//! The model chooses which tools a crowded task is shown; the word scorer is the fallback.
//!
//! WHY: `toolpick` scores a bag of words — `calendar` in the task, `calendar` in a name — and
//! that is a fine fallback and a poor judge: it cannot know that "what's on this afternoon"
//! wants `gcal:events`, or that "the failing build" wants `sys:run` and nothing from a
//! workspace server's forty. Above the budget the agent is shown a subset either way, so the
//! only question is WHO cuts it. agent smith is the brain of the run; which tools a task gets
//! is a decision, and decisions are the model's. The scorer never co-decides: it answers
//! only when the model could not (off, keyless, mock, a failed call, an off-grammar reply).
//!
//! The shape is the one every other one-shot here has (`intent::decision`, `elect`): one
//! bounded call, a strict first-line grammar, an unparsable reply that falls back rather
//! than guesses. One thing is this file's own: the call runs on a thread of its own, because
//! the swarm asks from INSIDE its runtime, where a nested `block_on` is a panic, not a wait.
//! The per-task memo that makes one decision one call is `toolmemo`'s.
//!
//! `CREW_TOOL_PICK=0` turns the model step off (scorer only). A child of `session`, so
//! `broker` items are reached through `crate::broker::`.
use crate::broker::route::clip;
use crate::broker::toolpick::BUDGET;
use crate::mcp::McpTool;

/// The one-shot's signature — `swarmanswer::SynthFn`'s shape, plus the bounds that let the
/// surface holding it be shared across a swarm's workers.
pub(crate) type ChooseFn = dyn Fn(&str) -> Result<String, String> + Send + Sync;
/// The call, or `None` when none may run — borrowed, so a test's counter can live on its
/// stack (an alias behind `&` would otherwise demand `'static`).
pub(crate) type Chooser<'a> =
    Option<&'a (dyn Fn(&str) -> Result<String, String> + Send + Sync + 'a)>;

/// Output ceiling: `TOOLS:` and up to `BUDGET` names sit well under it.
const CHOICE_MAX_TOKENS: u32 = 256;
/// Chars of the task carried into the prompt.
const TASK_CAP: usize = 1_500;
/// Chars of a tool's description (first line) on its catalog row.
const DESC_CAP: usize = 80;
/// Bytes the catalog listing may take: forty tools are ~4 KB, so this is the ceiling for a
/// two-hundred-tool workspace, where the tail is COUNTED rather than silently dropped.
pub(crate) const LIST_CAP: usize = 12 * 1024;
/// Chars of names the pane line shows before it says `… +N`.
const LINE_CAP: usize = 96;

/// `CREW_TOOL_PICK=0` — the only env read in this file. Scorer only, no call, ever.
pub(crate) fn disabled() -> bool {
    std::env::var("CREW_TOOL_PICK").is_ok_and(|v| v == "0")
}

/// The live call, when one may run: routing's gates (`CREW_INTENT=0`, keyless, mock) and
/// this file's own. The completion runs on a scoped thread — see the module doc.
pub(crate) fn live() -> Option<Box<ChooseFn>> {
    if disabled() {
        return None;
    }
    let call = crate::broker::intent::live_call(CHOICE_MAX_TOKENS)?;
    Some(Box::new(move |p: &str| {
        std::thread::scope(|s| {
            s.spawn(|| call(p))
                .join()
                .unwrap_or_else(|_| Err("the tool chooser panicked".into()))
        })
    }))
}

pub(super) fn label(t: &McpTool) -> String {
    format!("{}:{}", t.server, t.name)
}

/// The prompt: the grammar first, the catalog next, the task last — `route::frame`'s
/// cache-aware order, since the catalog is the same for every task of a session.
pub(crate) fn prompt(task: &str, catalog: &[McpTool]) -> String {
    let (rows, more) = listing(catalog);
    let tail = (more > 0).then(|| format!("\n\u{2026} {more} more not listed"));
    format!(
        "You choose which tools ONE agent is shown for its task. The FIRST line of your reply \
         must be exactly `TOOLS: <server:tool>, <server:tool>, \u{2026}` \u{2014} up to {BUDGET} \
         names, spelled exactly as listed below, only the ones this task could plausibly \
         need \u{2014} or `TOOLS: none` when none of them helps. Nothing else.\n\n\
         Tools:\n{rows}{}\n\nTask: {}",
        tail.unwrap_or_default(),
        clip(task, TASK_CAP)
    )
}

/// One row per tool, `sys` first, cut at [`LIST_CAP`] bytes: the rows, and how many did not fit.
fn listing(catalog: &[McpTool]) -> (String, usize) {
    let (sys, rest): (Vec<&McpTool>, Vec<&McpTool>) =
        catalog.iter().partition(|t| t.server == "sys");
    let mut out = String::new();
    let mut shown = 0;
    for t in sys.into_iter().chain(rest) {
        let one = t.description.lines().next().unwrap_or("");
        let row = format!("{} \u{2014} {}\n", label(t), clip(one, DESC_CAP));
        if out.len() + row.len() > LIST_CAP {
            break;
        }
        out.push_str(&row);
        shown += 1;
    }
    (out.trim_end().to_string(), catalog.len() - shown)
}

/// The names a `TOOLS:` line keeps, in the CATALOG's order: exact `server:tool` spellings
/// only, unknown names dropped, duplicates collapsed, at most `BUDGET` (the first named
/// win). `TOOLS: none` is an empty choice. `None` is NO choice — no `TOOLS:` first line, or
/// one naming nothing the catalog has, which is off-grammar in a different spelling.
pub(crate) fn parse(reply: &str, catalog: &[McpTool]) -> Option<Vec<String>> {
    let first = reply.lines().map(str::trim).find(|l| !l.is_empty())?;
    let (head, tail) = first
        .trim_matches(|c: char| matches!(c, '*' | '`' | '_'))
        .split_once(':')?;
    if !head.trim().eq_ignore_ascii_case("tools") {
        return None;
    }
    let tail = tail.trim();
    if tail.eq_ignore_ascii_case("none") {
        return Some(Vec::new());
    }
    let mut named: Vec<&str> = Vec::new();
    for raw in tail.split([',', ' ']) {
        let name = raw.trim_matches(|c: char| matches!(c, '`' | '*' | '_' | '.' | ';'));
        let known = !name.is_empty() && catalog.iter().any(|t| label(t) == name);
        if known && !named.contains(&name) {
            named.push(name);
        }
        if named.len() == BUDGET {
            break;
        }
    }
    if named.is_empty() {
        return None;
    }
    let mut ordered: Vec<String> = catalog
        .iter()
        .map(label)
        .filter(|l| named.contains(&l.as_str()))
        .collect();
    ordered.dedup();
    Some(ordered)
}

/// The model's choice for `task`, or `None` when the scorer must decide: under the budget
/// (no call is made), no chooser, a failed call, an off-grammar reply. The set handed back
/// is the chosen tools plus every `sys` tool, in the catalog's order, with how many of the
/// catalog that leaves out — the door is `toolselect::with_door`'s to add.
pub(crate) fn choose(
    catalog: &[McpTool],
    task: &str,
    chooser: Chooser<'_>,
) -> Option<(Vec<McpTool>, usize)> {
    if catalog.len() <= BUDGET {
        return None;
    }
    let names = parse(&chooser?(&prompt(task, catalog)).ok()?, catalog)?;
    let kept: Vec<McpTool> = catalog
        .iter()
        .filter(|t| t.server == "sys" || names.contains(&label(t)))
        .cloned()
        .collect();
    let left_out = catalog.len() - kept.len();
    Some((kept, left_out))
}

/// One decision, kept for the pane: how many tools were on the table, and the ones the
/// agent was shown (`sys` and the door included, in catalog order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Chosen {
    pub(crate) of: usize,
    pub(crate) names: Vec<String>,
}

impl Chosen {
    /// `tools: chose 6 of 41 — sys:run, gcal:events, … +3`: one line, cut where the names
    /// stop fitting.
    pub(crate) fn line(&self) -> String {
        let mut shown: Vec<&str> = Vec::new();
        let mut len = 0;
        for n in &self.names {
            if len + n.len() + 2 > LINE_CAP && !shown.is_empty() {
                break;
            }
            len += n.len() + 2;
            shown.push(n);
        }
        let rest = self.names.len() - shown.len();
        let tail = if rest > 0 {
            format!(", \u{2026} +{rest}")
        } else {
            String::new()
        };
        let (n, of) = (self.names.len(), self.of);
        format!(
            "tools: chose {n} of {of} \u{2014} {}{tail}",
            shown.join(", ")
        )
    }
}

#[cfg(test)]
#[path = "toolchoice_tests.rs"]
mod tests;
