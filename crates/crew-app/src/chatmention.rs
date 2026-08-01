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

/// One row of the attach picker: a rostered agent, a skill playbook, or a
/// file from the pane's cwd index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MentionEntry {
    Agent { name: String, role: String },
    Skill { name: String, desc: String },
    File(String),
}

impl MentionEntry {
    /// The text the query filters on (and the row label's body).
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Agent { name, .. } | Self::Skill { name, .. } => name,
            Self::File(p) => p,
        }
    }
    /// What `accept` splices after the `@`.
    pub(crate) fn token(&self) -> String {
        match self {
            Self::Agent { name, .. } => name.clone(),
            Self::Skill { name, .. } => format!("skill:{name}"),
            Self::File(p) => p.clone(),
        }
    }
    /// Dim hint after the label — the row's kind (files stay unadorned,
    /// matching the old files-only popup).
    pub(crate) fn desc(&self) -> String {
        match self {
            Self::Agent { role, .. } => format!("agent \u{b7} {role}"),
            Self::Skill { desc, .. } => format!("skill \u{b7} {desc}"),
            Self::File(_) => String::new(),
        }
    }
    /// Section rank: agents, then skills, then files.
    fn section(&self) -> u8 {
        match self {
            Self::Agent { .. } => 0,
            Self::Skill { .. } => 1,
            Self::File(_) => 2,
        }
    }
}

/// Everything the picker offers, scanned once when the popup opens:
/// roster agents, skills (user + project via crew-plugin), cwd files.
pub(crate) fn scan_entries(
    cwd: &std::path::Path,
    agents: &[crew_plugin::AgentInfo],
) -> Vec<MentionEntry> {
    let mut out: Vec<MentionEntry> = agents
        .iter()
        .map(|a| MentionEntry::Agent {
            name: a.name.clone(),
            role: a.role.clone(),
        })
        .collect();
    out.extend(
        crew_plugin::skills_list(cwd)
            .into_iter()
            .map(|s| MentionEntry::Skill {
                name: s.name,
                desc: s.description,
            }),
    );
    out.extend(
        crate::fileindex::scan(cwd)
            .into_iter()
            .map(MentionEntry::File),
    );
    out
}

/// Entries matching `query`, section-major (agents, then skills, then
/// files), and within a section: filename-prefix, then path-substring, then
/// path-subsequence matches; ties break shorter-label-first. Capped.
pub(crate) fn filter(entries: &[MentionEntry], query: &str) -> Vec<MentionEntry> {
    let q = query.to_lowercase();
    let mut scored: Vec<(u8, u8, &MentionEntry)> = entries
        .iter()
        .filter_map(|e| rank(e.label(), &q).map(|r| (e.section(), r, e)))
        .collect();
    scored.sort_by(|(sa, ra, ea), (sb, rb, eb)| {
        (sa, ra, ea.label().len(), ea.label()).cmp(&(sb, rb, eb.label().len(), eb.label()))
    });
    scored.truncate(MAX_MATCHES);
    scored.into_iter().map(|(_, _, e)| e.clone()).collect()
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

use crate::chatkeys::ChatInput;

/// The open mention popup: the scanned index (kept while typing narrows the
/// query), the current matches, and the selected row.
pub(crate) struct MentionState {
    pub entries: Vec<MentionEntry>,
    pub matches: Vec<MentionEntry>,
    pub sel: usize,
}

/// Whether the popup consumed a key (navigation/accept/close) or the
/// composer should handle it normally.
pub(crate) enum MentionKey {
    Consumed,
    Forward,
}

/// Popup-first key routing: while open, arrows move, Tab/Enter accept the
/// selection into `input`, Escape closes the popup (not the pane).
pub(crate) fn popup_key(
    mention: &mut Option<MentionState>,
    input: &mut String,
    key: &ChatInput,
) -> MentionKey {
    let Some(m) = mention else {
        return MentionKey::Forward;
    };
    match key {
        ChatInput::Up => m.sel = m.sel.saturating_sub(1),
        ChatInput::Down => m.sel = (m.sel + 1).min(m.matches.len().saturating_sub(1)),
        ChatInput::Complete | ChatInput::Enter => {
            if let Some(entry) = m.matches.get(m.sel) {
                *input = accept(input, &entry.token());
            }
            *mention = None;
        }
        ChatInput::Close => *mention = None,
        _ => return MentionKey::Forward,
    }
    MentionKey::Consumed
}

/// Sync the popup to the input after an edit: open it (scanning once) when a
/// mention is being typed, refilter while it is, close it when the token
/// ends or nothing matches.
pub(crate) fn after_edit(
    mention: &mut Option<MentionState>,
    input: &str,
    scan: impl FnOnce() -> Vec<MentionEntry>,
) {
    let Some(q) = pending_mention(input) else {
        *mention = None;
        return;
    };
    let m = mention.get_or_insert_with(|| MentionState {
        entries: scan(),
        matches: Vec::new(),
        sel: 0,
    });
    m.matches = filter(&m.entries, q);
    m.sel = m.sel.min(m.matches.len().saturating_sub(1));
    if m.matches.is_empty() {
        *mention = None;
    }
}

/// Half-open char-index ranges of every non-leading `@token` in the input —
/// the composer tints them so mentions read as chips while typing. A quoted
/// `@"…"` token is one chip through its closing quote (same rule as
/// [`tokens`]).
pub(crate) fn spans(input: &str) -> Vec<(usize, usize)> {
    let chars: Vec<char> = input.chars().collect();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        let quoted_end = (chars[i] == '@' && chars.get(i + 1) == Some(&'"'))
            .then(|| chars[i + 2..].iter().position(|c| *c == '"'))
            .flatten()
            .map(|q| i + 2 + q + 1);
        match quoted_end {
            Some(e) => i = e,
            None => {
                while i < chars.len() && !chars[i].is_whitespace() {
                    i += 1;
                }
            }
        }
        if start > 0 && chars[start] == '@' && i - start > 1 {
            spans.push((start, i));
        }
    }
    spans
}

/// Largest file inlined into a message; bigger mentions become a skip note
/// instead of blowing up the agents' context.
pub(crate) const MAX_FILE_BYTES: usize = 64 * 1024;

/// Whitespace-separated tokens of `text`, except that a token opening with
/// `@"` runs to its closing quote (spaces and all) — the form `filedrop`
/// mints for paths containing whitespace. Quotes stay on the yielded token
/// (`expand` strips them); an unterminated quote falls back to the plain
/// whitespace-delimited token, so ordinary text tokenizes exactly as before.
fn tokens(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let Some(off) = text[start..].find(|c: char| !c.is_whitespace()) else {
            break;
        };
        start += off;
        let quoted_end = text[start..]
            .strip_prefix("@\"")
            .and_then(|rest| rest.find('"').map(|q| start + 2 + q + 1));
        let end = quoted_end.unwrap_or_else(|| {
            text[start..]
                .find(char::is_whitespace)
                .map_or(text.len(), |w| start + w)
        });
        out.push(&text[start..end]);
        start = end;
    }
    out
}

/// Expand mentions in an outgoing message: every mention token gets its
/// referent appended — file contents as a `--- file ---` block, `@skill:`
/// playbooks as a `--- skill ---` block. Token 0 is the routing selector
/// only while it names rostered agents (every `+` segment); otherwise it
/// expands like any other token, so attachments picked at the leading
/// position aren't silently dropped. Never blocks sending.
pub(crate) fn expand(text: &str, cwd: &std::path::Path, agent_names: &[String]) -> String {
    let mut out = text.to_string();
    let mut seen: Vec<&str> = Vec::new();
    let mut skills: Option<Vec<crew_plugin::Skill>> = None;
    for (i, tok) in tokens(text).into_iter().enumerate() {
        let Some(rel) = tok.strip_prefix('@') else {
            continue;
        };
        // The quoted form: the payload between the quotes is the path,
        // verbatim — whitespace and all.
        let rel = rel
            .strip_prefix('"')
            .and_then(|r| r.strip_suffix('"'))
            .unwrap_or(rel);
        if i == 0
            && !rel.is_empty()
            && rel
                .split('+')
                .all(|s| agent_names.iter().any(|a| a.eq_ignore_ascii_case(s)))
        {
            continue; // the @agent routing selector
        }
        if rel.is_empty() || seen.contains(&rel) {
            continue;
        }
        if let Some(name) = rel.strip_prefix("skill:") {
            let list = skills.get_or_insert_with(|| crew_plugin::skills_list(cwd));
            if let Some(s) = list.iter().find(|s| s.name == name) {
                seen.push(rel);
                out.push_str(&skill_attachment(s));
            }
            continue;
        }
        // The WHOLE token first: a file may legitimately be named `x:10`,
        // and its own name outranks an idiom about it.
        let (base, range) = if cwd.join(rel).is_file() {
            (rel, None)
        } else {
            crate::mentionrange::split(rel)
        };
        let path = cwd.join(base);
        if !path.is_file() {
            continue;
        }
        seen.push(rel);
        out.push_str(&attachment(rel, base, &path, range));
    }
    out
}

/// One mention's appended block: contents, or a one-line skip note.
///
/// `range` selects lines (`@src/main.rs:120-180`). The size cap is applied to
/// what is actually attached, not to the file on disk — attaching forty lines
/// of a 200 KB module is the whole point of having ranges, so refusing it
/// because of the file's total size would defeat them.
fn attachment(
    rel: &str,
    base: &str,
    path: &std::path::Path,
    range: Option<crate::mentionrange::Range>,
) -> String {
    // Whole-file mentions gate on metadata BEFORE any read: a multi-GB drop
    // must not be slurped onto the winit thread just to be told it's too
    // big. Range mentions still read the file — the cap applies to what is
    // attached, which is the whole point of ranges (see below).
    if range.is_none() {
        if let Ok(md) = std::fs::metadata(path) {
            if md.len() > MAX_FILE_BYTES as u64 {
                return too_large(base, base);
            }
        }
    }
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return format!("\n\n--- file: {rel} skipped: {e} ---"),
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return format!("\n\n--- file: {rel} skipped: binary ---");
    };
    let (body, header) = match range {
        None => (text, base.to_string()),
        Some(r) => match crate::mentionrange::slice(&text, r) {
            Some(sel) => (sel, format!("{base} {}", crate::mentionrange::label(r))),
            None => {
                return format!(
                    "\n\n--- file: {base} skipped: {} is past the end ---",
                    crate::mentionrange::label(r)
                )
            }
        },
    };
    if body.len() > MAX_FILE_BYTES {
        return too_large(&header, base);
    }
    format!("\n\n--- file: {header} ---\n{body}\n--- end file ---")
}

/// The over-cap skip note, naming the way out. A cap that only says no
/// leaves the user with a file they cannot attach at all, which is what
/// ranges are for.
fn too_large(header: &str, base: &str) -> String {
    format!(
        "\n\n--- file: {header} skipped: too large \u{2014} attach part of it with              @{base}:<start>-<end> ---"
    )
}

/// One skill mention's appended block: the playbook body, or a skip note.
fn skill_attachment(s: &crew_plugin::Skill) -> String {
    if s.body.len() > MAX_FILE_BYTES {
        return format!("\n\n--- skill: {} skipped: too large ---", s.name);
    }
    format!(
        "\n\n--- skill: {} ---\n{}\n--- end skill ---",
        s.name, s.body
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(paths: &[&str]) -> Vec<MentionEntry> {
        paths
            .iter()
            .map(|p| MentionEntry::File(p.to_string()))
            .collect()
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
        let e = entries(&["docs/main-notes.md", "src/main.rs", "crates/app/mod.rs"]);
        let got = filter(&e, "main");
        assert_eq!(got[0].label(), "src/main.rs"); // filename prefix
        assert_eq!(got[1].label(), "docs/main-notes.md"); // path substring
        let got = filter(&e, "camod");
        assert_eq!(
            got.iter().map(|m| m.label()).collect::<Vec<_>>(),
            vec!["crates/app/mod.rs"]
        ); // subsequence
    }

    #[test]
    fn filter_empty_query_lists_everything_and_misses_are_dropped() {
        let e = entries(&["a.rs", "b.rs"]);
        assert_eq!(filter(&e, "").len(), 2);
        assert!(filter(&e, "zzz").is_empty());
    }

    #[test]
    fn filter_sections_agents_then_skills_then_files() {
        let mut e = entries(&["review-checklist.md"]);
        e.push(MentionEntry::Agent {
            name: "reviewer".into(),
            role: "reviews".into(),
        });
        e.push(MentionEntry::Skill {
            name: "review".into(),
            desc: "playbook".into(),
        });
        let got = filter(&e, "rev");
        let labels: Vec<&str> = got.iter().map(|m| m.label()).collect();
        assert_eq!(labels, vec!["reviewer", "review", "review-checklist.md"]);
    }

    #[test]
    fn tokens_by_kind() {
        assert_eq!(
            MentionEntry::Agent {
                name: "coder".into(),
                role: String::new()
            }
            .token(),
            "coder"
        );
        assert_eq!(
            MentionEntry::Skill {
                name: "deploy".into(),
                desc: String::new()
            }
            .token(),
            "skill:deploy"
        );
        assert_eq!(
            MentionEntry::File("src/main.rs".into()).token(),
            "src/main.rs"
        );
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
    fn spans_cover_non_leading_at_tokens() {
        assert_eq!(spans("hey @a.rs now"), vec![(4, 9)]);
        assert_eq!(spans("@coder fix @src/x.rs"), vec![(11, 20)]); // leading selector excluded
        assert!(spans("plain text").is_empty());
        assert!(spans("hey @").is_empty()); // bare '@' is not a mention yet
    }

    #[test]
    fn spans_cover_a_quoted_mention_as_one_chip() {
        // `see @"a b.txt" now` — the chip runs from '@' through the closing
        // quote, whitespace inside and all.
        assert_eq!(spans("see @\"a b.txt\" now"), vec![(4, 14)]);
        // An unterminated quote falls back to the whitespace-delimited token.
        assert_eq!(spans("see @\"a now"), vec![(4, 7)]);
    }

    #[test]
    fn a_quoted_mention_round_trips_a_path_with_spaces() {
        // The exact loop a Finder drop takes: `filedrop::mention_token` mints
        // the quoted form, and `expand` must resolve it at send.
        let dir = tmp("quoted");
        std::fs::write(dir.join("my notes.txt"), "space content").unwrap();
        let tok = crate::filedrop::mention_token(&dir.join("my notes.txt"), &dir);
        assert_eq!(tok, "@\"my notes.txt\" ");
        let out = expand(&format!("summarize {tok}please"), &dir, &[]);
        assert!(
            out.contains("--- file: my notes.txt ---\nspace content\n--- end file ---"),
            "{out}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expand_appends_mentioned_file_contents() {
        let dir = tmp("expand");
        std::fs::write(dir.join("note.txt"), "hello world").unwrap();
        let out = expand("summarize @note.txt please", &dir, &[]);
        assert!(out.starts_with("summarize @note.txt please"));
        assert!(out.contains("--- file: note.txt ---\nhello world\n--- end file ---"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `@path:120-180` attaches those lines only. The case it exists for is
    /// the file too big to attach whole — the one you most want to point an
    /// agent at one function of.
    #[test]
    fn expand_attaches_only_the_named_lines() {
        let dir = tmp("ranges");
        let body: String = (1..=10).map(|i| format!("line{i}\n")).collect();
        std::fs::write(dir.join("f.txt"), &body).unwrap();

        let out = expand("look at @f.txt:3-5", &dir, &[]);
        assert!(out.contains("--- file: f.txt lines 3-5 ---"), "{out}");
        assert!(
            out.contains("line3\nline4\nline5\n--- end file ---"),
            "{out}"
        );
        assert!(!out.contains("line2"), "attached more than asked: {out}");
        assert!(!out.contains("line6"), "attached more than asked: {out}");

        // A single line reads as one.
        let out = expand("look at @f.txt:7", &dir, &[]);
        assert!(out.contains("--- file: f.txt line 7 ---\nline7\n"), "{out}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The cap applies to what is ATTACHED, not to the file on disk —
    /// refusing forty lines because the module is 200 KB would defeat the
    /// whole feature.
    #[test]
    fn a_range_makes_an_oversize_file_attachable() {
        let dir = tmp("range-big");
        let mut body = String::from("the interesting line\n");
        body.push_str(&"x".repeat(MAX_FILE_BYTES + 1));
        std::fs::write(dir.join("big.txt"), &body).unwrap();

        let out = expand("see @big.txt:1", &dir, &[]);
        assert!(out.contains("--- file: big.txt line 1 ---"), "{out}");
        assert!(out.contains("the interesting line"), "{out}");
        assert!(!out.contains("too large"), "{out}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A range past the end says so rather than attaching an empty block that
    /// reads, to an agent, as an empty file.
    #[test]
    fn a_range_past_the_end_says_so() {
        let dir = tmp("range-past");
        std::fs::write(dir.join("f.txt"), "one\ntwo\n").unwrap();
        let out = expand("see @f.txt:50-60", &dir, &[]);
        assert!(out.contains("lines 50-60 is past the end"), "{out}");
        assert!(!out.contains("--- end file ---"), "{out}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file whose NAME ends in something range-shaped is still that file.
    #[test]
    fn a_file_actually_named_with_a_colon_wins() {
        let dir = tmp("range-name");
        std::fs::write(dir.join("odd:10"), "the real file\n").unwrap();
        let out = expand("see @odd:10", &dir, &[]);
        assert!(out.contains("--- file: odd:10 ---"), "{out}");
        assert!(out.contains("the real file"), "{out}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The size gate runs BEFORE any read: a multi-GB drop must not be
    /// slurped onto the winit thread just to be told it's too big. Pinned via
    /// permissions — the file is stat-able but unreadable, so any read
    /// attempt would surface "Permission denied" instead of the size note.
    #[cfg(unix)]
    #[test]
    fn an_oversize_file_is_skipped_by_metadata_before_any_read() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tmp("statfirst");
        let path = dir.join("huge.txt");
        let f = std::fs::File::create(&path).unwrap();
        f.set_len(MAX_FILE_BYTES as u64 + 1).unwrap(); // sparse: over the cap
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o200)).unwrap();
        let out = expand("see @huge.txt", &dir, &[]);
        assert!(out.contains("huge.txt skipped: too large"), "{out}");
        assert!(
            !out.contains("skipped: Permission"),
            "the file was read before the size gate: {out}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expand_skips_oversize_binary_and_missing() {
        let dir = tmp("caps");
        std::fs::write(dir.join("big.txt"), vec![b'a'; MAX_FILE_BYTES + 1]).unwrap();
        std::fs::write(dir.join("bin.dat"), [0u8, 159, 146, 150]).unwrap();
        let out = expand("see @big.txt @bin.dat @gone.txt", &dir, &[]);
        // The cap now names the way out: a file too big to attach whole is
        // exactly the file you want to attach part of.
        assert!(out.contains("big.txt skipped: too large"), "{out}");
        assert!(out.contains("@big.txt:<start>-<end>"), "{out}");
        assert!(out.contains("--- file: bin.dat skipped: binary ---"));
        assert!(!out.contains("gone.txt ---")); // unresolvable token left alone
        let _ = std::fs::remove_dir_all(&dir);
    }

    use crate::chatkeys::ChatInput;

    fn open(matches: &[&str]) -> Option<MentionState> {
        Some(MentionState {
            entries: entries(matches),
            matches: entries(matches),
            sel: 0,
        })
    }

    #[test]
    fn popup_navigates_accepts_and_closes() {
        let mut m = open(&["a.rs", "b.rs"]);
        let mut input = "see @".to_string();
        assert!(matches!(
            popup_key(&mut m, &mut input, &ChatInput::Down),
            MentionKey::Consumed
        ));
        assert_eq!(m.as_ref().unwrap().sel, 1);
        assert!(matches!(
            popup_key(&mut m, &mut input, &ChatInput::Enter),
            MentionKey::Consumed
        ));
        assert_eq!(input, "see @b.rs ");
        assert!(m.is_none()); // accept closes

        let mut m = open(&["a.rs"]);
        assert!(matches!(
            popup_key(&mut m, &mut input, &ChatInput::Close),
            MentionKey::Consumed
        ));
        assert!(m.is_none()); // Esc closes the popup, not the pane
    }

    #[test]
    fn popup_forwards_when_closed_and_on_edits() {
        let mut m: Option<MentionState> = None;
        let mut input = String::new();
        assert!(matches!(
            popup_key(&mut m, &mut input, &ChatInput::Enter),
            MentionKey::Forward
        ));
        let mut m = open(&["a.rs"]);
        assert!(matches!(
            popup_key(&mut m, &mut input, &ChatInput::Char('x')),
            MentionKey::Forward
        ));
    }

    #[test]
    fn after_edit_opens_refilters_and_closes() {
        let mut m: Option<MentionState> = None;
        // Typing "@" after a word opens the popup with the scanned files.
        after_edit(&mut m, "see @", || entries(&["a.rs", "b.md"]));
        assert_eq!(m.as_ref().unwrap().matches.len(), 2);
        // Narrowing the query refilters WITHOUT rescanning (scan would panic).
        after_edit(&mut m, "see @a", || unreachable!("no rescan while open"));
        assert_eq!(
            m.as_ref().unwrap().matches,
            vec![MentionEntry::File("a.rs".to_string())]
        );
        // No match → closed; token ended → stays closed.
        after_edit(&mut m, "see @zzz", || unreachable!());
        assert!(m.is_none());
        after_edit(&mut m, "see @a.rs ", || entries(&["a.rs"]));
        assert!(m.is_none());
    }

    #[test]
    fn expand_ignores_the_leading_selector_and_dedups() {
        let dir = tmp("lead");
        std::fs::write(dir.join("a.txt"), "A").unwrap();
        // Leading token is the @agent selector even if it happens to be a path,
        // as long as it names a rostered agent.
        let out = expand("@a.txt do it", &dir, &["a.txt".to_string()]);
        assert_eq!(out, "@a.txt do it");
        let out = expand("x @a.txt and @a.txt", &dir, &[]);
        assert_eq!(out.matches("--- file: a.txt ---").count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expand_attaches_a_leading_non_agent_mention() {
        let dir = tmp("leadfile");
        std::fs::write(dir.join("a.txt"), "A").unwrap();
        // roster contains planner only: leading @a.txt is a mention, not routing.
        let out = expand("@a.txt summarize", &dir, &["planner".to_string()]);
        assert!(out.contains("--- file: a.txt ---"), "{out}");
        // rostered leading selector still skipped, including multi-target
        let out = expand("@planner do it @a.txt", &dir, &["planner".to_string()]);
        assert!(out.starts_with("@planner do it @a.txt"));
        assert_eq!(out.matches("--- file: a.txt ---").count(), 1);
        let out = expand("@Planner do it", &dir, &["planner".to_string()]);
        assert_eq!(out, "@Planner do it"); // roster match is case-insensitive, like broker routing
        let out = expand(
            "@planner+coder go",
            &dir,
            &["planner".to_string(), "coder".to_string()],
        );
        assert_eq!(out, "@planner+coder go");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expand_attaches_skill_playbooks_and_leaves_unknown_skills_alone() {
        let dir = tmp("skilltok");
        let sk = dir.join(".crew/skills");
        std::fs::create_dir_all(&sk).unwrap();
        std::fs::write(sk.join("deploy.md"), "---\ndescription: d\n---\nship it").unwrap();
        let out = expand("use @skill:deploy now", &dir, &[]);
        assert!(
            out.contains("--- skill: deploy ---\nship it\n--- end skill ---"),
            "{out}"
        );
        // dedup + unknown left alone
        let out = expand("x @skill:deploy @skill:deploy @skill:ghost", &dir, &[]);
        assert_eq!(out.matches("--- skill: deploy ---").count(), 1);
        assert!(!out.contains("--- skill: ghost"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
