//! Address resolution and the cooperative-sentinel wrap/scan for inter-pane
//! `ask`. Pure string/lookup logic — the socket, injection, and liveness
//! wait live elsewhere (ipc.rs / askpump / askwait.rs).
use crate::pane::Pane;

/// Split a federated ask address into `(pane-local target, instance)`:
/// `schema@alpha` → `("schema", Some("alpha"))`; a bare `schema` →
/// `("schema", None)`. The instance selects which crew's socket the client
/// dials; the pane part is resolved locally by that crew, unchanged. This is
/// the v3 address widening — the resolver/transport widen, the engine does not
/// (docs/vision/sentinel-network.md). Splits on the LAST `@` so a pane label
/// may itself contain one; an empty instance is ignored.
pub(crate) fn split_instance(addr: &str) -> (&str, Option<&str>) {
    match addr.rsplit_once('@') {
        Some((pane, inst)) if !inst.is_empty() && !pane.is_empty() => (pane, Some(inst)),
        _ => (addr, None),
    }
}

/// Resolve an address to a pane index: an exact `/name`-or-label match first,
/// else the `p{index}` fallback form (every pane is addressable even unnamed).
pub(crate) fn resolve(panes: &[Pane], addr: &str) -> Option<usize> {
    if let Some(i) = panes
        .iter()
        .position(|p| p.name.as_deref() == Some(addr) || p.label.as_deref() == Some(addr))
    {
        return Some(i);
    }
    addr.strip_prefix('p')
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|&i| i < panes.len())
}

/// The line-start answer marker for ask `id`. The agent replies on a line
/// that BEGINS with this token — a single line-start marker (not a wrapping
/// pair) so the target tty's echo of the injected instruction, which only
/// mentions the token mid-sentence, can never be mistaken for the answer.
fn marker(id: &str) -> String {
    format!("CREW-ANS-{id}:")
}

/// The instruction injected into the target pane's live session (visible):
/// the question plus how to answer. `id` namespaces the marker so concurrent
/// asks to one pane don't collide.
pub(crate) fn wrap(from: &str, id: &str, question: &str) -> String {
    format!(
        "\n[\u{21d0} ask from \"{from}\" \u{00b7} {id}] {question}\n\
         When you have the answer, print it on its own line that BEGINS with \
         the marker {}  (nothing before the marker), then your answer.\n",
        marker(id)
    )
}

/// Extract the answer from `captured` output: the first line that begins with
/// this ask's marker, with the marker stripped. `None` until such a line
/// arrives. The echoed instruction mentions the marker mid-line, so it never
/// matches (the marker must be at line start).
pub(crate) fn scan_answer(captured: &str, id: &str) -> Option<String> {
    let tag = marker(id);
    captured
        .lines()
        .find_map(|l| l.trim_start().strip_prefix(&tag))
        .map(|rest| rest.trim().to_string())
}

#[cfg(test)]
#[path = "askroute_tests.rs"]
mod tests;
