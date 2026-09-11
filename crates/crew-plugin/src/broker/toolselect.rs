//! Which tools a task is shown — on both paths — and the door to the rest.
//!
//! `toolpick` scores and cuts; this is the layer above it that `SessionTools` actually asks,
//! and it exists for the one invariant the picker cannot keep alone: **nothing is
//! unreachable.** When the picker leaves tools out, `sys:find_tools` is on the list handed
//! back whether or not the `sys` surface is on. It is a search of the session's own catalog,
//! not a process tool, so the `sys` switch — which exists to keep a shell away from an agent —
//! has no business hiding it; and a list that admits to hiding tools while withholding the one
//! way to reach them is the invariant broken in the prompt itself.
//!
//! That was the native path until this file: `specs_for` filtered exactly like the text hint
//! but said nothing, so on a provider with tool support and forty tools the model was never
//! told sixteen were missing — and with `sys` off it did not even have the door. A child of
//! `session`, so `broker` items are reached through `crate::broker::`.
use crate::broker::toolpick::{pick, BUDGET};
use crate::mcp::McpTool;

/// [`pick`] at the budget, plus the door when anything was left out. Below the budget nothing
/// changes: a crew with `sys` off and ten tools sees exactly its ten, as it always did.
pub(super) fn select(tools: Vec<McpTool>, task: &str) -> (Vec<McpTool>, usize) {
    let (mut kept, left_out) = pick(tools, task, BUDGET);
    if left_out > 0
        && !kept
            .iter()
            .any(|t| t.server == "sys" && t.name == "find_tools")
    {
        // The one `sys` tool the surface being off does not hide. Its spec stays with the
        // others in `systools`, so there is exactly one description of it.
        kept.extend(
            crate::broker::systools::tools()
                .into_iter()
                .filter(|t| t.name == "find_tools"),
        );
    }
    (kept, left_out)
}

/// The native path's admission — the twin of `toolpick::omitted_note`. `None` when nothing
/// was left out, so a prompt with every tool on the wire stays byte-identical. It names the
/// WIRE spelling, `sys__find_tools`, because that is the name the model can call there:
/// `sys:find_tools` is not a legal function name on any provider.
pub(super) fn native_note(left_out: usize) -> Option<String> {
    (left_out > 0).then(|| {
        format!(
            "{left_out} more tool(s) are connected but not among the tools you were given; \
             call sys__find_tools with q set to what you need to search them by name and \
             description."
        )
    })
}

/// The `q` of a `sys:find_tools` call. A malformed or missing argument searches for nothing,
/// which lists nothing and says how many tools there are — more useful than an error, because
/// the model's next move is to search again with a word in it.
pub(super) fn search_query(args: &str) -> String {
    serde_json::from_str::<serde_json::Value>(args)
        .ok()
        .and_then(|v| v.get("q")?.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Tool descriptors in the shape a provider is handed.
pub(super) fn specs_of(tools: Vec<McpTool>) -> Vec<crew_hive::tools::ToolSpec> {
    tools
        .into_iter()
        .map(|t| crew_hive::tools::ToolSpec {
            server: t.server,
            tool: t.name,
            description: t.description,
            input_schema: t.input_schema,
        })
        .collect()
}

#[cfg(test)]
#[path = "toolselect_tests.rs"]
mod tests;
