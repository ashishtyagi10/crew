//! `lsp:symbols` — "where is `ToolBudget` defined", answered by the language
//! server's own index (`workspace/symbol`) instead of a grep that also finds
//! comments, strings and call sites. Kept apart from the position tools: it
//! takes a query rather than a place, and `file` only picks the server.
use std::path::PathBuf;

use serde_json::{json, Value};

use super::{tools, LspHost, TIMEOUT};
use crate::mcp::McpTool;
use crew_lsp::symbols::rank_key;
use crew_lsp::{servers, Symbol};

/// Lines in one answer; the rest is a count, so a broad query stays cheap.
const MAX_LINES: usize = 30;

pub(super) fn descriptor() -> McpTool {
    McpTool {
        server: "lsp".into(),
        name: "symbols".into(),
        description: "language server symbol search: find where a symbol (class, struct, \
            function, type, trait, method, constant) is defined by name across the whole \
            project \u{2014} where is it, which file and line \u{2014} one `kind name \u{2014} \
            file:line` per line, best match first, without grepping: \
            {\"query\": \"ToolBudget\", \"file\": \"src/main.rs\"} (file is any source file \
            of the project, to pick the language)"
            .into(),
        input_schema: json!({"type": "object", "properties": {
            "query": {"type": "string", "description": "the symbol name, or part of it"},
            "file": {"type": "string", "description": "any source file in the project, to choose the language server; optional when one is already running"},
        }, "required": ["query"]}),
    }
}

/// `(query, file)` from the tool's arguments.
pub(super) fn parse(args: &str) -> Result<(String, Option<String>), String> {
    let v: Value = serde_json::from_str(if args.trim().is_empty() { "{}" } else { args })
        .map_err(|e| format!("tool arguments are not valid JSON: {e}"))?;
    let text = |k: &str| {
        v.get(k)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let query = text("query").ok_or("missing string argument \"query\"")?;
    Ok((query, text("file")))
}

/// The answer: deduped, ranked, cut at [`MAX_LINES`], each path relative to
/// the root of the server that gave it (two servers can be rooted apart).
pub(super) fn text(query: &str, mut found: Vec<(PathBuf, Symbol)>) -> String {
    if found.is_empty() {
        return format!(
            "no symbols match \"{query}\" (a server still indexing a big project answers \
            with nothing yet \u{2014} try again in a few seconds)"
        );
    }
    // Two servers (or two roots) can index the same file and name it twice.
    let mut seen = std::collections::HashSet::new();
    found.retain(|(_, s)| seen.insert((s.uri.clone(), s.name.clone(), s.line)));
    found.sort_by_key(|(_, s)| rank_key(query, s));
    let line = |(root, s): &(PathBuf, Symbol)| {
        let path = crew_lsp::uri::to_path(&s.uri).map_or_else(
            || s.uri.clone(),
            |p| p.strip_prefix(root).unwrap_or(&p).to_string_lossy().into(),
        );
        match s.line {
            Some(l) => format!("{} {} \u{2014} {path}:{}", s.kind, s.name, l + 1),
            None => format!("{} {} \u{2014} {path}", s.kind, s.name),
        }
    };
    let mut out: Vec<String> = found.iter().take(MAX_LINES).map(line).collect();
    if found.len() > MAX_LINES {
        out.push(format!("+{} more", found.len() - MAX_LINES));
    }
    out.join("\n")
}

impl LspHost {
    /// Ask `file`'s server (started if need be), or every running one.
    pub(super) fn symbols(&mut self, args: &str) -> Result<String, String> {
        let (query, file) = parse(args)?;
        let keys: Vec<(String, PathBuf)> = match file {
            Some(f) => {
                let path = tools::absolute(&f);
                let lang = servers::lang_of(&path).ok_or_else(|| {
                    format!("no language server for {} files", tools::ext_of(&path))
                })?;
                let root = crew_lsp::root::for_file(&path);
                self.client(lang, &root)?;
                vec![(lang.to_string(), root)]
            }
            None if self.clients.is_empty() => {
                return Err(
                    "no language server is running yet \u{2014} pass \"file\" (any \
                    source file in the project) so crew starts the right one; `/lsp` lists \
                    the servers and which are installed"
                        .into(),
                );
            }
            None => self.clients.keys().cloned().collect(),
        };
        let (mut found, mut first_err) = (Vec::new(), None);
        for key in keys {
            let Some(c) = self.clients.get_mut(&key) else {
                continue;
            };
            match c.workspace_symbols(&query, TIMEOUT) {
                Ok(list) => found.extend(list.into_iter().map(|s| (key.1.clone(), s))),
                Err(e) if e.contains("no reply within") => {
                    // Still indexing: keep the client — dropping it would kill
                    // the very index the retry waits for.
                    first_err.get_or_insert(format!(
                        "the {} language server is still indexing the project \u{2014} \
                        try lsp:symbols again in a few seconds",
                        key.0
                    ));
                }
                Err(e) => {
                    // Like the other tools: a failed request drops the client.
                    self.clients.remove(&key);
                    first_err.get_or_insert(e);
                }
            }
        }
        match first_err {
            Some(e) if found.is_empty() => Err(e),
            _ => Ok(text(&query, found)),
        }
    }
}

#[cfg(test)]
#[path = "symbols_tests.rs"]
mod tests;
