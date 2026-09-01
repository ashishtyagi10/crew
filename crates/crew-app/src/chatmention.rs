//! `@file` mentions in the crew composer: detect the trailing `@token` being
//! typed (the message-leading token stays the `@agent` selector), fuzzy-filter
//! the file index against it, and splice the accepted path back into the
//! input. Pure string-in/string-out, like `chatcomplete`.

pub(crate) use crate::mentionexpand::*;
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

#[cfg(test)]
#[path = "chatmention_tests.rs"]
mod tests;
