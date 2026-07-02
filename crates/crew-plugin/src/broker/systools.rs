//! Built-in `sys` tools: shell + file access every agent gets with zero
//! config, on the same `@tool` surface as MCP (`@tool sys:run {"cmd": …}`).
//! Bounded (timeout, 64 KB captures), non-interactive, visible as hops.
//! `CREW_SYS_TOOLS=0` disables; so does the mock provider, keeping scripted
//! broker tests deterministic. Relative paths resolve against the broker cwd
//! — a convention, not a sandbox.
use crate::mcp::McpTool;

/// Capture cap per stream / per file read.
pub(crate) const CAP: usize = 64 * 1024;

/// Whether the `sys` surface is on (env wrapper over [`enabled_from`]).
pub(crate) fn enabled() -> bool {
    enabled_from(
        std::env::var("CREW_SYS_TOOLS").ok().as_deref(),
        std::env::var("CREW_BROKER_MOCK_REPLY").is_ok(),
    )
}

/// Pure gate: on unless `CREW_SYS_TOOLS=0` or the mock provider is active.
pub(crate) fn enabled_from(sys_tools: Option<&str>, mock: bool) -> bool {
    !mock && sys_tools != Some("0")
}

/// The four `sys` tool descriptors, in the shape the TOOLS hint renders.
pub(crate) fn tools() -> Vec<McpTool> {
    let mk = |name: &str, desc: &str| McpTool {
        server: "sys".into(),
        name: name.into(),
        description: desc.into(),
    };
    vec![
        mk(
            "run",
            "run a shell command, non-interactive: {\"cmd\": \"zip -r out.zip docs/\"}",
        ),
        mk(
            "read_file",
            "read a UTF-8 text file: {\"path\": \"README.md\"}",
        ),
        mk(
            "write_file",
            "create/overwrite a text file: {\"path\": …, \"content\": …}",
        ),
        mk(
            "list_dir",
            "list a directory (default .): {\"path\": \"src\"}",
        ),
    ]
}

/// Dispatch one `sys` call. Errors return to the agent as `ERROR: …`.
pub(crate) fn call(tool: &str, args: &str) -> Result<String, String> {
    let args = args.trim();
    let v: serde_json::Value = if args.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(args).map_err(|e| format!("arguments are not valid JSON: {e}"))?
    };
    match tool {
        "run" => super::sysrun::run(str_arg(&v, "cmd")?),
        "read_file" => read_file(str_arg(&v, "path")?),
        "write_file" => write_file(str_arg(&v, "path")?, str_arg(&v, "content")?),
        "list_dir" => list_dir(v.get("path").and_then(|p| p.as_str()).unwrap_or(".")),
        other => Err(format!(
            "unknown sys tool \u{201c}{other}\u{201d} \u{2014} available: run, read_file, write_file, list_dir"
        )),
    }
}

/// A required string argument, with an agent-readable error.
fn str_arg<'a>(v: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    v.get(key)
        .and_then(|s| s.as_str())
        .ok_or_else(|| format!("missing string argument \u{201c}{key}\u{201d}"))
}

fn read_file(path: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    if text.len() > CAP {
        let cut = if text.is_char_boundary(CAP) {
            CAP
        } else {
            let mut cut = CAP;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            cut
        };
        return Ok(format!("{}\n\u{2026} (truncated at 64 KB)", &text[..cut]));
    }
    Ok(text)
}

fn write_file(path: &str, content: &str) -> Result<String, String> {
    std::fs::write(path, content).map_err(|e| format!("write {path}: {e}"))?;
    Ok(format!("wrote {} bytes to {path}", content.len()))
}

/// Entries at most, so a huge directory can't flood the prompt.
const MAX_ENTRIES: usize = 500;

fn list_dir(path: &str) -> Result<String, String> {
    let rd = std::fs::read_dir(path).map_err(|e| format!("list {path}: {e}"))?;
    let mut lines: Vec<String> = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.metadata().map_err(|e| format!("stat {name}: {e}"))?;
        lines.push(if meta.is_dir() {
            format!("{name}/")
        } else {
            format!("{name} ({} B)", meta.len())
        });
    }
    lines.sort();
    let n = lines.len();
    lines.truncate(MAX_ENTRIES);
    if n > MAX_ENTRIES {
        lines.push(format!("\u{2026} {} more entries", n - MAX_ENTRIES));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
#[path = "systools_tests.rs"]
mod tests;
