//! Tab-completion for the crew composer: `@ag<Tab>` completes agent names
//! (including the segment after a `+` in multi-target selectors) and
//! `/lo<Tab>` completes construct names. Pure string-in/string-out so it's
//! trivially testable.
use crew_plugin::AgentInfo;

/// Every composer slash action: broker constructs plus the pane-local
/// `/export`, `/theme`, `/compact`, and `/exit` (see `chatexport` /
/// `chattheme` / `chatcompact` / `chat`).
pub(crate) const CONSTRUCTS: [&str; 29] = [
    "/help",
    "/agents",
    "/model",
    "/fan",
    "/loop",
    "/goal",
    "/plan",
    "/approve",
    "/reject",
    "/checkpoint",
    "/checkpoints",
    "/restore",
    "/diff",
    "/cwd",
    "/skills",
    "/skill",
    "/memory",
    "/commit",
    "/review",
    "/resume",
    "/doctor",
    "/mcp",
    "/tasks",
    "/stop",
    "/status",
    "/export",
    "/theme",
    "/compact",
    "/exit",
];

/// One-line description for each construct, shown as the dim hint in the
/// composer's slash palette. Falls back to "" for anything unlisted.
pub(crate) fn describe(construct: &str) -> &'static str {
    match construct {
        "/help" => "list the constructs",
        "/agents" => "show the crew roster",
        "/model" => "set an agent's model",
        "/fan" => "fan a task out to every agent",
        "/loop" => "run a task on a loop",
        "/goal" => "set the crew's shared goal",
        "/plan" => "draft a plan for approval",
        "/approve" => "approve the drafted plan",
        "/reject" => "reject the drafted plan",
        "/checkpoint" => "snapshot the session",
        "/checkpoints" => "list checkpoints",
        "/restore" => "restore a checkpoint",
        "/diff" => "show working-tree changes",
        "/cwd" => "show the working directory",
        "/skills" => "list available skills",
        "/skill" => "run a skill",
        "/memory" => "show the standing memory (add with #<note>)",
        "/commit" => "draft an AI commit message (apply to run)",
        "/review" => "AI code review of the working diff",
        "/resume" => "continue the previous session as context",
        "/doctor" => "health-check the AI stack",
        "/mcp" => "list MCP servers and tools",
        "/tasks" => "list running background tasks",
        "/stop" => "stop all tasks (/stop #n for one)",
        "/status" => "show session status",
        "/export" => "export the transcript",
        "/theme" => "list or switch the color theme",
        "/compact" => "fold away older messages",
        "/exit" => "close this pane",
        _ => "",
    }
}

/// Complete `input`'s leading token. Returns the new input when something
/// completed (unique match, or extended to the candidates' common prefix).
pub(crate) fn complete(input: &str, agents: &[AgentInfo]) -> Option<String> {
    // Only the first token completes, and only while the cursor is inside it
    // (the composer has no mid-line cursor — input is append-only).
    if input.contains(char::is_whitespace) {
        return None;
    }
    if let Some(rest) = input.strip_prefix('@') {
        // Complete the segment after the last '+' (multi-target selectors).
        let (done, part) = match rest.rfind('+') {
            Some(i) => (&rest[..=i], &rest[i + 1..]),
            None => ("", rest),
        };
        let names: Vec<&str> = agents.iter().map(|a| a.name.as_str()).collect();
        let (ext, unique) = match extend(part, &names) {
            Some(pair) => pair,
            // Prefix matching found nothing — fall back to a fuzzy (opencode-
            // style subsequence) match, but only if it's unambiguous.
            None => (fuzzy_unique(part, &names)?.to_string(), true),
        };
        let tail = if unique && done.is_empty() { " " } else { "" };
        return Some(format!("@{done}{ext}{tail}"));
    }
    if input.starts_with('/') {
        let (ext, unique) = match extend(input, &CONSTRUCTS) {
            Some(pair) => pair,
            None => (fuzzy_unique(input, &CONSTRUCTS)?.to_string(), true),
        };
        let tail = if unique { " " } else { "" };
        return Some(format!("{ext}{tail}"));
    }
    None
}

/// Extend `prefix` against `candidates`: the full name when exactly one
/// matches (`(name, true)`), else the longest common prefix when it grows the
/// input (`(lcp, false)`). Case-insensitive; `None` when nothing matches or
/// nothing would change.
fn extend(prefix: &str, candidates: &[&str]) -> Option<(String, bool)> {
    let low = prefix.to_lowercase();
    let hits: Vec<&&str> = candidates
        .iter()
        .filter(|c| c.to_lowercase().starts_with(&low))
        .collect();
    match hits.as_slice() {
        [] => None,
        [one] => Some((one.to_string(), true)),
        many => {
            let first = many[0].to_lowercase();
            let mut lcp = first.len();
            for c in many.iter().skip(1) {
                let c = c.to_lowercase();
                lcp = first
                    .chars()
                    .zip(c.chars())
                    .take(lcp)
                    .take_while(|(a, b)| a == b)
                    .count();
            }
            (lcp > prefix.len()).then(|| (first[..lcp].to_string(), false))
        }
    }
}

/// Case-insensitive subsequence match: is every char of `needle` found in
/// `hay` in order? (`"gl"` matches `"goal"`, `"pnr"` matches `"planner"`.)
/// Case-folds and delegates to `crate::suggest`'s identical (already
/// case-normalized-by-caller) helper.
fn is_subsequence(needle: &str, hay: &str) -> bool {
    crate::suggest::is_subsequence(&needle.to_lowercase(), &hay.to_lowercase())
}

/// The single candidate that fuzzy-matches `needle`, or `None` if zero or
/// more than one do.
fn fuzzy_unique<'a>(needle: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let mut hits = candidates.iter().filter(|c| is_subsequence(needle, c));
    let first = *hits.next()?;
    match hits.next() {
        Some(_) => None,
        None => Some(first),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agents(names: &[&str]) -> Vec<AgentInfo> {
        names
            .iter()
            .map(|n| AgentInfo {
                name: (*n).into(),
                role: String::new(),
                model: String::new(),
            })
            .collect()
    }

    #[test]
    fn completes_a_unique_agent_with_trailing_space() {
        let a = agents(&["planner", "coder", "reviewer"]);
        assert_eq!(complete("@pl", &a).unwrap(), "@planner ");
        assert_eq!(complete("@CO", &a).unwrap(), "@coder ");
    }

    #[test]
    fn completes_the_segment_after_a_plus() {
        let a = agents(&["planner", "coder", "reviewer"]);
        assert_eq!(complete("@planner+co", &a).unwrap(), "@planner+coder");
    }

    #[test]
    fn ambiguous_prefix_extends_to_common_prefix() {
        let a = agents(&["planner", "plotter"]);
        assert_eq!(complete("@p", &a).unwrap(), "@pl");
        // Already at the common prefix → nothing to add.
        assert_eq!(complete("@pl", &a), None);
    }

    #[test]
    fn completes_constructs() {
        assert_eq!(complete("/go", &[]).unwrap(), "/goal ");
        assert_eq!(complete("/lo", &[]).unwrap(), "/loop ");
        // '/st' IS the common prefix of /stop and /status → nothing to add…
        assert_eq!(complete("/st", &[]), None);
        // …but one more character disambiguates.
        assert_eq!(complete("/sta", &[]).unwrap(), "/status ");
    }

    #[test]
    fn completes_and_describes_diff() {
        assert_eq!(complete("/di", &[]).unwrap(), "/diff ");
        assert_eq!(describe("/diff"), "show working-tree changes");
    }

    #[test]
    fn completes_and_describes_cwd() {
        assert_eq!(complete("/cw", &[]).unwrap(), "/cwd ");
        assert_eq!(describe("/cwd"), "show the working directory");
    }

    #[test]
    fn ignores_mid_sentence_and_plain_text() {
        let a = agents(&["planner"]);
        assert_eq!(complete("@planner do the", &a), None);
        assert_eq!(complete("hello", &a), None);
        assert_eq!(complete("", &a), None);
        assert_eq!(complete("@ghost", &a), None);
    }

    #[test]
    fn fuzzy_fallback_completes_a_unique_subsequence_match() {
        assert_eq!(complete("/gl", &[]).unwrap(), "/goal ");
        let a = agents(&["planner", "coder", "reviewer"]);
        assert_eq!(complete("@pnr", &a).unwrap(), "@planner ");
    }

    #[test]
    fn fuzzy_fallback_is_none_when_ambiguous() {
        let a = agents(&["planner", "cleaner"]);
        // "an" is a subsequence of both "planner" and "cleaner".
        assert_eq!(complete("@an", &a), None);
    }

    #[test]
    fn prefix_match_still_wins_over_fuzzy() {
        // "/st" is the shared prefix of /stop and /status (and a fuzzy
        // subsequence of several other constructs too) — stays ambiguous.
        assert_eq!(complete("/st", &[]), None);
    }

    #[test]
    fn is_subsequence_cases() {
        assert!(is_subsequence("gl", "goal"));
        assert!(is_subsequence("pnr", "planner"));
        assert!(is_subsequence("", "anything"));
        assert!(!is_subsequence("xyz", "goal"));
        assert!(!is_subsequence("lg", "goal")); // wrong order
    }
}
