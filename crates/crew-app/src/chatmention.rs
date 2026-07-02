//! `@file` mentions in the crew composer: detect the trailing `@token` being
//! typed (the message-leading token stays the `@agent` selector), fuzzy-filter
//! the file index against it, and splice the accepted path back into the
//! input. Pure string-in/string-out, like `chatcomplete`.

/// The query of a file mention being typed: the trailing token starts with
/// `@` and is not the leading token (`hey @sr` → `Some("sr")`).
pub(crate) fn pending_mention(input: &str) -> Option<&str> {
    // rfind gives the LAST whitespace: everything after it is the trailing
    // token. No whitespace at all → the leading token → agent selector.
    let (i, c) = input
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())?;
    input[i + c.len_utf8()..].strip_prefix('@')
}

/// Files matching `query`, best first: filename-prefix, then path-substring,
/// then path-subsequence matches; ties break shorter-path-first. Capped.
pub(crate) fn filter(files: &[String], query: &str) -> Vec<String> {
    let q = query.to_lowercase();
    let mut scored: Vec<(u8, &String)> = files
        .iter()
        .filter_map(|f| rank(f, &q).map(|r| (r, f)))
        .collect();
    scored.sort_by(|(ra, fa), (rb, fb)| (ra, fa.len(), fa).cmp(&(rb, fb.len(), fb)));
    scored.truncate(MAX_MATCHES);
    scored.into_iter().map(|(_, f)| f.clone()).collect()
}

/// Cap on returned matches: the popup shows 10 and scrolls; beyond ~50 the
/// tail is noise.
const MAX_MATCHES: usize = 50;

/// Match quality of `path` against lowercased `q`: filename prefix beats
/// path substring beats path subsequence; `None` for no match.
fn rank(path: &str, q: &str) -> Option<u8> {
    let low = path.to_lowercase();
    let name = low.rsplit('/').next().unwrap_or(&low);
    if name.starts_with(q) {
        Some(0)
    } else if low.contains(q) {
        Some(1)
    } else if crate::suggest::is_subsequence(q, &low) {
        Some(2)
    } else {
        None
    }
}

/// Replace the trailing `@query` token with `@path ` (trailing space ends the
/// mention so the popup closes).
pub(crate) fn accept(input: &str, path: &str) -> String {
    let cut = input
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    format!("{}@{path} ", &input[..cut])
}

/// Largest file inlined into a message; bigger mentions become a skip note
/// instead of blowing up the agents' context.
pub(crate) const MAX_FILE_BYTES: usize = 64 * 1024;

/// Expand `@path` mentions in an outgoing message: every non-leading token
/// that resolves to a file under `cwd` gets its contents appended as a
/// `--- file: … ---` block. The tokens stay in place; unresolvable ones are
/// left alone (they may be genuine prose). Never blocks sending.
pub(crate) fn expand(text: &str, cwd: &std::path::Path) -> String {
    let mut out = text.to_string();
    let mut seen: Vec<&str> = Vec::new();
    for (i, tok) in text.split_whitespace().enumerate() {
        // Token 0 is the @agent selector position, never a file mention.
        if i == 0 {
            continue;
        }
        let Some(rel) = tok.strip_prefix('@') else {
            continue;
        };
        if rel.is_empty() || seen.contains(&rel) {
            continue;
        }
        let path = cwd.join(rel);
        if !path.is_file() {
            continue;
        }
        seen.push(rel);
        out.push_str(&attachment(rel, &path));
    }
    out
}

/// One mention's appended block: contents, or a one-line skip note.
fn attachment(rel: &str, path: &std::path::Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) if bytes.len() > MAX_FILE_BYTES => {
            format!("\n\n--- file: {rel} skipped: too large ---")
        }
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => format!("\n\n--- file: {rel} ---\n{s}\n--- end file ---"),
            Err(_) => format!("\n\n--- file: {rel} skipped: binary ---"),
        },
        Err(e) => format!("\n\n--- file: {rel} skipped: {e} ---"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn pending_mention_is_the_trailing_at_token() {
        assert_eq!(pending_mention("hey @sr"), Some("sr"));
        assert_eq!(pending_mention("hey @"), Some(""));
        assert_eq!(pending_mention("@coder fix @src/ma"), Some("src/ma"));
    }

    #[test]
    fn leading_token_is_the_agent_selector_not_a_mention() {
        assert_eq!(pending_mention("@coder"), None);
        assert_eq!(pending_mention("@pl"), None);
    }

    #[test]
    fn plain_text_and_ended_tokens_are_no_mention() {
        assert_eq!(pending_mention("hello"), None);
        assert_eq!(pending_mention("hey @src/main.rs "), None);
        assert_eq!(pending_mention("mail a@b"), None); // '@' mid-word is not a mention
        assert_eq!(pending_mention(""), None);
    }

    #[test]
    fn filter_ranks_name_prefix_over_substring_over_subsequence() {
        let f = files(&["docs/main-notes.md", "src/main.rs", "crates/app/mod.rs"]);
        let got = filter(&f, "main");
        assert_eq!(got[0], "src/main.rs"); // filename prefix
        assert_eq!(got[1], "docs/main-notes.md"); // path substring
        let got = filter(&f, "camod");
        assert_eq!(got, vec!["crates/app/mod.rs".to_string()]); // subsequence
    }

    #[test]
    fn filter_empty_query_lists_everything_and_misses_are_dropped() {
        let f = files(&["a.rs", "b.rs"]);
        assert_eq!(filter(&f, "").len(), 2);
        assert!(filter(&f, "zzz").is_empty());
    }

    #[test]
    fn accept_replaces_the_trailing_token() {
        assert_eq!(accept("hey @sr", "src/main.rs"), "hey @src/main.rs ");
        assert_eq!(accept("look at @", "a.txt"), "look at @a.txt ");
    }

    fn tmp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("crew-mention-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn expand_appends_mentioned_file_contents() {
        let dir = tmp("expand");
        std::fs::write(dir.join("note.txt"), "hello world").unwrap();
        let out = expand("summarize @note.txt please", &dir);
        assert!(out.starts_with("summarize @note.txt please"));
        assert!(out.contains("--- file: note.txt ---\nhello world\n--- end file ---"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expand_skips_oversize_binary_and_missing() {
        let dir = tmp("caps");
        std::fs::write(dir.join("big.txt"), vec![b'a'; MAX_FILE_BYTES + 1]).unwrap();
        std::fs::write(dir.join("bin.dat"), [0u8, 159, 146, 150]).unwrap();
        let out = expand("see @big.txt @bin.dat @gone.txt", &dir);
        assert!(out.contains("--- file: big.txt skipped: too large ---"));
        assert!(out.contains("--- file: bin.dat skipped: binary ---"));
        assert!(!out.contains("gone.txt ---")); // unresolvable token left alone
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expand_ignores_the_leading_selector_and_dedups() {
        let dir = tmp("lead");
        std::fs::write(dir.join("a.txt"), "A").unwrap();
        // Leading token is the @agent selector even if it happens to be a path.
        let out = expand("@a.txt do it", &dir);
        assert_eq!(out, "@a.txt do it");
        let out = expand("x @a.txt and @a.txt", &dir);
        assert_eq!(out.matches("--- file: a.txt ---").count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
