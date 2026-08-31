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
#[path = "chatmention_tests.rs"]
mod tests;
