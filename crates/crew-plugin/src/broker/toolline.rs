//! How a tool call reads in the transcript.
//!
//! Every call is already logged as a hop, so the pane shows what an agent is
//! doing to your machine — but it showed the WIRE FORM:
//!
//! ```text
//! [tool] sys:write_file {"path": "src/lib.rs", "content": "use std::fmt;\nuse …
//! ```
//!
//! The one thing a reader wants (which file? which command?) is buried in
//! punctuation, and for `write_file` the line is mostly the first 150 bytes of
//! the file being written. The same call reads:
//!
//! ```text
//! [tool] sys:write_file  src/lib.rs
//! ```
//!
//! Nothing is hidden that a reader was getting — the arguments were clipped at
//! 200 characters anyway, so a long call was already a truncated blob.

/// Argument names that ARE the call, in the order they should win. `cmd` and
/// `path` cover the built-in `sys` surface; the rest are the conventional
/// spellings an MCP server is likely to use for its subject.
const HEADLINE: &[&str] = &["cmd", "command", "path", "file", "url", "query", "pattern"];

/// The transcript line for one tool call: `server:tool  <subject>`.
///
/// Falls back to the raw arguments when nothing identifiable is there, so a
/// tool this knows nothing about still shows what it was given rather than
/// nothing at all.
pub(crate) fn call_line(label: &str, args: &str, cap: usize) -> String {
    match subject(args) {
        Some(s) => format!("{label}  {}", clip(&s, cap)),
        None => {
            let a = args.trim();
            if a.is_empty() || a == "{}" {
                label.to_string()
            } else {
                format!("{label} {}", clip(a, cap))
            }
        }
    }
}

/// The argument worth showing, if the call has one.
fn subject(args: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(args).ok()?;
    let obj = v.as_object()?;
    for key in HEADLINE {
        if let Some(s) = obj.get(*key).and_then(|v| v.as_str()) {
            if !s.trim().is_empty() {
                return Some(s.trim().to_string());
            }
        }
    }
    // A single-field call is unambiguous whatever the field is called.
    if obj.len() == 1 {
        if let Some(s) = obj.values().next().and_then(|v| v.as_str()) {
            if !s.trim().is_empty() {
                return Some(s.trim().to_string());
            }
        }
    }
    None
}

/// Clip to `cap` characters (not bytes — this is display text) with an
/// ellipsis, and flatten newlines: a transcript line is one line.
fn clip(s: &str, cap: usize) -> String {
    let flat: String = s
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    if flat.chars().count() <= cap {
        return flat;
    }
    let head: String = flat.chars().take(cap).collect();
    format!("{head}\u{2026}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_call_reads_as_the_command() {
        assert_eq!(
            call_line("sys:run", r#"{"cmd": "cargo test --workspace"}"#, 200),
            "sys:run  cargo test --workspace"
        );
    }

    /// The case the wire form was worst for: the line used to be the first
    /// 150 bytes of the file being written.
    #[test]
    fn a_write_reads_as_the_path_not_the_file_body() {
        let args = r#"{"path": "src/lib.rs", "content": "use std::fmt;\nfn main() {}"}"#;
        let line = call_line("sys:write_file", args, 200);
        assert_eq!(line, "sys:write_file  src/lib.rs");
        assert!(!line.contains("use std"), "the body leaked: {line}");
    }

    #[test]
    fn a_read_and_a_listing_read_as_their_path() {
        assert_eq!(
            call_line(
                "sys:read_file",
                r#"{"path": "README.md", "offset": 0}"#,
                200
            ),
            "sys:read_file  README.md"
        );
        assert_eq!(
            call_line("sys:list_dir", r#"{"path": "src"}"#, 200),
            "sys:list_dir  src"
        );
    }

    /// An unknown MCP tool still shows what it was handed — falling back to
    /// nothing would hide the one thing this line exists for.
    #[test]
    fn an_unrecognised_shape_still_shows_its_arguments() {
        let line = call_line("jira:issue", r#"{"a": 1, "b": 2}"#, 200);
        assert!(line.starts_with("jira:issue "), "{line}");
        assert!(line.contains("\"a\""), "{line}");
    }

    /// …and a single string field is unambiguous whatever it is called.
    #[test]
    fn a_lone_string_argument_is_the_subject() {
        assert_eq!(
            call_line("jira:issue", r#"{"ticket": "CREW-12"}"#, 200),
            "jira:issue  CREW-12"
        );
        // Two fields, neither of them a headline name: not guessable.
        let line = call_line("jira:issue", r#"{"ticket": "CREW-12", "note": "x"}"#, 200);
        assert!(line.contains('{'), "guessed between two fields: {line}");
    }

    #[test]
    fn a_call_with_no_arguments_is_just_its_name() {
        assert_eq!(call_line("sys:list_dir", "{}", 200), "sys:list_dir");
        assert_eq!(call_line("sys:list_dir", "  ", 200), "sys:list_dir");
    }

    /// A transcript line is ONE line, and a bounded one — a command with an
    /// embedded newline must not break the layout.
    #[test]
    fn the_subject_is_flattened_and_bounded() {
        let line = call_line("sys:run", r#"{"cmd": "echo one\necho two"}"#, 200);
        assert_eq!(line, "sys:run  echo one echo two");
        let long = format!(r#"{{"cmd": "{}"}}"#, "x".repeat(300));
        let line = call_line("sys:run", &long, 200);
        assert!(line.ends_with('\u{2026}'), "{line}");
        assert_eq!(line.chars().count(), "sys:run  ".chars().count() + 201);
    }

    /// Malformed JSON is a real possibility (a model wrote it): show it
    /// rather than dropping the call from the transcript.
    #[test]
    fn malformed_arguments_are_shown_not_swallowed() {
        let line = call_line("sys:run", "{not json", 200);
        assert!(line.contains("not json"), "{line}");
    }
}
