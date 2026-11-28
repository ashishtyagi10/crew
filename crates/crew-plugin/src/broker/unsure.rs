//! What the model says it is not sure of, as one line under its answer.
//!
//! The pane's "! not checked" row says which claims no tool proved; it cannot
//! say what the model assumed or could not verify. A staff engineer's report
//! does, so the working agent is asked for one `Unsure:` line, and the relay
//! lifts it out of the answer into smith's own row (`? unsure: …`), where it
//! reads at a glance instead of sitting in the prose. Only the agents that
//! write the user's answer are asked: never a judge, router or drafter.

/// The one sentence the working agent's reply framing carries.
pub(crate) const ASK: &str = "If your answer rests on something you did not verify or an \
     assumption you made, end the answer (before the closing control line) \
     with exactly one line `Unsure: <what, in under 20 words>`; when nothing is uncertain \
     leave that line out entirely.";

/// `how` — a reply framing that ends an answer — with the ask under it.
pub(crate) fn asking(how: String) -> String {
    format!("{how}\n{ASK}")
}

/// The row's label, as smith's row says it.
const ROW: &str = "? unsure: ";

/// `answer` without its closing `Unsure:` line, and what that line said.
///
/// Only the LAST non-empty line counts (control lines `@done`/`@next` after
/// it are skipped over and kept), and not one inside a code fence, where it
/// is a quoted example. A line that says nothing, or says "nothing", is
/// dropped without being reported: a model told to omit it sometimes writes
/// it anyway, and "Unsure: none" is noise, not a caveat.
pub(crate) fn split_unsure(answer: &str) -> (String, Option<String>) {
    let lines: Vec<&str> = answer.lines().collect();
    let control = |l: &str| {
        let l = l.trim().to_ascii_lowercase();
        l.starts_with("@done") || l.starts_with("@next")
    };
    let Some(at) = lines
        .iter()
        .rposition(|l| !l.trim().is_empty() && !control(l))
    else {
        return (answer.to_string(), None);
    };
    let line = lines[at].trim();
    let head = line.get(..7).unwrap_or_default();
    let fenced = lines[..at]
        .iter()
        .filter(|l| l.trim_start().starts_with("```"))
        .count()
        % 2
        == 1;
    if !head.eq_ignore_ascii_case("unsure:") || fenced {
        return (answer.to_string(), None);
    }
    let said = line[7..].trim();
    let rest: Vec<&str> = lines[..at]
        .iter()
        .chain(&lines[at + 1..])
        .copied()
        .collect();
    let kept = rest.join("\n").trim_end().to_string();
    let bare = said.trim_end_matches('.').to_ascii_lowercase();
    match matches!(bare.as_str(), "" | "nothing" | "none" | "n/a") {
        true => (kept, None),
        false => (kept, Some(said.to_string())),
    }
}

/// Smith's row for what the answer was unsure of.
pub(crate) fn row(said: &str) -> String {
    format!("{ROW}{said}")
}

#[cfg(test)]
#[path = "unsure_tests.rs"]
mod tests;
