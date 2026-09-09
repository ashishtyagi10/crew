//! The four `lsp` tools as the model sees them, and the text they answer
//! with. Descriptions are deliberately rich in the words a code task uses —
//! symbol, definition, references, type, error, diagnostic — because the
//! retrieval picker (`broker/toolpick.rs`) scores a tool by its first line
//! and may leave these out of a large catalog otherwise.
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::mcp::McpTool;
use crew_lsp::{Diagnostic, Location};

pub(super) const NAMES: [&str; 4] = ["hover", "definition", "references", "diagnostics"];

/// `{file, line, col}` — one-based, the way an editor and a compiler print
/// them; `file` is relative to the project (the broker's cwd) or absolute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub file: String,
    pub line: u32,
    pub col: u32,
}

impl Args {
    pub fn parse(args: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(if args.trim().is_empty() { "{}" } else { args })
            .map_err(|e| format!("tool arguments are not valid JSON: {e}"))?;
        let file = v
            .get("file")
            .and_then(Value::as_str)
            .filter(|f| !f.trim().is_empty())
            .ok_or("missing string argument \"file\"")?
            .to_string();
        let num = |k: &str| {
            v.get(k)
                .and_then(Value::as_u64)
                .map_or(1, |n| n.max(1) as u32)
        };
        Ok(Self {
            file,
            line: num("line"),
            col: num("col"),
        })
    }
}

/// `file` as an absolute path, against the broker's cwd — the project.
pub(super) fn absolute(file: &str) -> PathBuf {
    let p = Path::new(file);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    std::env::current_dir().map_or_else(|_| p.to_path_buf(), |d| d.join(p))
}

pub(super) fn ext_of(path: &Path) -> String {
    path.extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_else(|| "extensionless".into())
}

/// `path` as the agent would write it: relative to `root` when it is inside.
fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn schema(position: bool) -> Value {
    let mut props = json!({
        "file": {"type": "string", "description": "the source file, relative to the project root or absolute"},
    });
    let mut required = vec!["file"];
    if position {
        props["line"] =
            json!({"type": "integer", "minimum": 1, "description": "1-based line of the symbol"});
        props["col"] =
            json!({"type": "integer", "minimum": 1, "description": "1-based column of the symbol"});
        required.extend(["line", "col"]);
    }
    json!({"type": "object", "properties": props, "required": required})
}

/// The tool list, in the order the model reads it.
pub(super) fn descriptors() -> Vec<McpTool> {
    let mk = |name: &str, desc: &str, position: bool| McpTool {
        server: "lsp".into(),
        name: name.into(),
        description: desc.into(),
        input_schema: schema(position),
    };
    vec![
        mk(
            "hover",
            "language server hover: the type, signature and docs of the symbol at a position \
             in a source file, resolved by the compiler-grade server rather than guessed from \
             text: {\"file\": \"src/main.rs\", \"line\": 12, \"col\": 8}",
            true,
        ),
        mk(
            "definition",
            "language server go-to-definition: where the symbol (function, type, variable, \
             import) at a position is declared, as file:line:col, across the whole project: \
             {\"file\": \"src/main.rs\", \"line\": 12, \"col\": 8}",
            true,
        ),
        mk(
            "references",
            "language server find-references: every use of the symbol at a position across \
             the project (callers, usages, implementations), one file:line:col per line: \
             {\"file\": \"src/main.rs\", \"line\": 12, \"col\": 8}",
            true,
        ),
        mk(
            "diagnostics",
            "language server diagnostics for one source file: the compiler's errors and \
             warnings (type errors, unresolved names, unused variables) as \
             file:line:col \u{2014} severity: message, without running a build: \
             {\"file\": \"src/main.rs\"}",
            false,
        ),
    ]
}

pub(super) fn hover_text(root: &Path, path: &Path, a: &Args, hover: Option<String>) -> String {
    match hover {
        Some(h) => format!("{}:{}:{} \u{2014} {h}", rel(root, path), a.line, a.col),
        None => format!(
            "no hover information at {}:{}:{}",
            rel(root, path),
            a.line,
            a.col
        ),
    }
}

pub(super) fn locations_text(
    tool: &str,
    root: &Path,
    path: &Path,
    a: &Args,
    locs: &[Location],
) -> String {
    if locs.is_empty() {
        return format!(
            "no {tool} found for {}:{}:{}",
            rel(root, path),
            a.line,
            a.col
        );
    }
    locs.iter()
        .map(|l| {
            let file = l.path().map_or_else(|| l.uri.clone(), |p| rel(root, &p));
            format!(
                "{file}:{}:{}",
                l.range.start.line + 1,
                l.range.start.character + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn diagnostics_text(root: &Path, path: &Path, list: &[Diagnostic]) -> String {
    let file = rel(root, path);
    if list.is_empty() {
        return format!("no diagnostics for {file}");
    }
    list.iter()
        .map(|d| {
            let src = d
                .source
                .as_deref()
                .map(|s| format!(" [{s}]"))
                .unwrap_or_default();
            format!(
                "{file}:{}:{} \u{2014} {}{src}: {}",
                d.range.start.line + 1,
                d.range.start.character + 1,
                d.severity.label(),
                d.message.lines().next().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
