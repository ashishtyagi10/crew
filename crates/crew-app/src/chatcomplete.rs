//! Tab-completion for the crew composer: `@ag<Tab>` completes agent names
//! (including the segment after a `+` in multi-target selectors) and
//! `/lo<Tab>` completes construct names. Pure string-in/string-out so it's
//! trivially testable.
use crew_plugin::AgentInfo;

/// Every composer slash action: broker constructs plus the pane-local
/// `/export` (see `chatexport`).
pub(crate) const CONSTRUCTS: [&str; 12] = [
    "/help", "/agents", "/model", "/fan", "/loop", "/goal", "/skills", "/skill", "/mcp", "/stop",
    "/status", "/export",
];

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
        let (ext, unique) = extend(part, &names)?;
        let tail = if unique && done.is_empty() { " " } else { "" };
        return Some(format!("@{done}{ext}{tail}"));
    }
    if input.starts_with('/') {
        let (ext, unique) = extend(input, &CONSTRUCTS)?;
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
    fn ignores_mid_sentence_and_plain_text() {
        let a = agents(&["planner"]);
        assert_eq!(complete("@planner do the", &a), None);
        assert_eq!(complete("hello", &a), None);
        assert_eq!(complete("", &a), None);
        assert_eq!(complete("@ghost", &a), None);
    }
}
