//! Turning `@file` mentions into what actually gets sent: the tokens in a
//! line, the file each names, the attachment its contents become, and the
//! refusal when one is too big to carry.
//!
//! Split from [`crate::chatmention`] for the line cap, along the line between
//! the popup you pick from and what the pick becomes.

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
pub(crate) fn tokens(text: &str) -> Vec<&str> {
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
pub(crate) fn attachment(
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
pub(crate) fn too_large(header: &str, base: &str) -> String {
    format!(
        "\n\n--- file: {header} skipped: too large \u{2014} attach part of it with              @{base}:<start>-<end> ---"
    )
}

/// One skill mention's appended block: the playbook body, or a skip note.
pub(crate) fn skill_attachment(s: &crew_plugin::Skill) -> String {
    if s.body.len() > MAX_FILE_BYTES {
        return format!("\n\n--- skill: {} skipped: too large ---", s.name);
    }
    format!(
        "\n\n--- skill: {} ---\n{}\n--- end skill ---",
        s.name, s.body
    )
}
