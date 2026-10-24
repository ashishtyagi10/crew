//! A native call as crew names it: `server:tool` from the wire name, the
//! near misses for a name the model invented, and the round's calls as the
//! engine's `Advice` reads them.

use crate::provider::{ToolInvocation, ToolOutcome};
use crate::tools::{ToolCall, ToolCatalog};

/// What a call resolved to, or why it did not.
pub(super) fn outcome_for_unknown(call: &ToolInvocation, catalog: &ToolCatalog) -> ToolOutcome {
    let known = catalog.names();
    // Naming the near misses turns an invented name into one recoverable
    // round instead of a repeat of the same wrong guess — and at retrieval
    // scale the whole list would be the wall the picker exists to avoid.
    let tail = match known.contains(&"sys__find_tools") {
        true => "; sys__find_tools searches them",
        false => "",
    };
    ToolOutcome {
        id: call.id.clone(),
        name: call.name.clone(),
        content: crate::tools::near::unknown("tool", &call.name, &known, tail),
        is_error: true,
    }
}

/// Each call of a round that names a tool, as crew names it, beside how it
/// came back `(ok, text)`: what the engine's `Advice` reads.
pub(super) fn ran_of(
    calls: &[ToolInvocation],
    results: &[ToolOutcome],
    catalog: &ToolCatalog,
) -> Vec<(ToolCall, (bool, String))> {
    calls
        .iter()
        .zip(results)
        .filter_map(|(c, r)| {
            let (server, tool) = catalog.resolve(&c.name)?;
            let call = ToolCall {
                server: server.to_string(),
                tool: tool.to_string(),
                args: c.input.to_string(),
            };
            Some((call, (!r.is_error, r.content.clone())))
        })
        .collect()
}

/// `server:tool` when the name resolves, else the name the model invented.
pub(super) fn label_of(call: &ToolInvocation, catalog: &ToolCatalog) -> String {
    match catalog.resolve(&call.name) {
        Some((s, t)) => format!("{s}:{t}"),
        None => call.name.clone(),
    }
}
