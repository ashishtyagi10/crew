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

/// Whether the `sys` tools are in read-only mode (CREW_SYS_MODE in {readonly, ro}).
pub(crate) fn read_only() -> bool {
    read_only_from(std::env::var("CREW_SYS_MODE").ok().as_deref())
}

/// Pure gate for read-only mode.
pub(crate) fn read_only_from(v: Option<&str>) -> bool {
    matches!(v, Some("readonly") | Some("ro"))
}

/// The block message when a MUTATING tool is used in read-only mode, else None.
fn read_only_block(tool: &str, read_only: bool) -> Option<String> {
    if read_only && matches!(tool, "run" | "write_file") {
        Some(format!(
            "sys:{tool} blocked \u{2014} CREW_SYS_MODE=readonly (set CREW_SYS_MODE=full to enable)"
        ))
    } else {
        None
    }
}

/// Human label for the current sys mode.
pub(crate) fn mode_label() -> &'static str {
    if read_only() {
        "read-only"
    } else {
        "full"
    }
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
            "read a UTF-8 text file, 64 KB per call: {\"path\": \"README.md\", \"offset\": 0}",
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
    if let Some(blocked) = read_only_block(tool, read_only()) {
        return Err(blocked);
    }
    let args = args.trim();
    let v: serde_json::Value = if args.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(args).map_err(|e| format!("arguments are not valid JSON: {e}"))?
    };
    match tool {
        "run" => super::sysrun::run(str_arg(&v, "cmd")?),
        "read_file" => read_file(str_arg(&v, "path")?, offset_arg(&v)),
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

/// The optional `"offset"` byte argument, defaulting to 0.
fn offset_arg(v: &serde_json::Value) -> usize {
    v.get("offset").and_then(|n| n.as_u64()).unwrap_or(0) as usize
}

/// True if `idx` doesn't split a UTF-8 codepoint in `bytes` (mirrors
/// `str::is_char_boundary` without requiring a validated `&str` up front).
fn is_utf8_boundary(bytes: &[u8], idx: usize) -> bool {
    match bytes.get(idx) {
        None => idx == bytes.len(),
        Some(&b) => (b as i8) >= -0x40,
    }
}

fn read_file(path: &str, offset: usize) -> Result<String, String> {
    use std::io::{Read, Seek, SeekFrom};
    // Bound the I/O itself: at most CAP+1 bytes, so a huge/never-EOF file can't blow up memory or hang.
    let mut f = std::fs::File::open(path).map_err(|e| format!("read {path}: {e}"))?;
    let total = f.metadata().map_err(|e| format!("read {path}: {e}"))?.len() as usize;
    if offset > 0 {
        f.seek(SeekFrom::Start(offset as u64))
            .map_err(|e| format!("read {path}: {e}"))?;
    }
    let mut buf = Vec::new();
    f.take(CAP as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|e| format!("read {path}: {e}"))?;
    if buf.is_empty() && offset > 0 {
        return Ok(format!(
            "\u{2026} (offset {offset} is at or past the end \u{2014} file is {total} bytes)"
        ));
    }
    let hit_cap = buf.len() > CAP; // pre-trim: more file remains beyond this read
    let start = (0..=3.min(buf.len())) // offset may land mid-codepoint; skip <=3B to a boundary
        .find(|&i| is_utf8_boundary(&buf, i))
        .unwrap_or(0);
    let buf = &buf[start..];
    if hit_cap {
        let end = buf.len().min(CAP);
        let floor = end.saturating_sub(3); // boundary within 3 bytes; binary may lack one
        let cut = (floor..=end).rev().find(|&i| is_utf8_boundary(buf, i));
        let cut = cut.ok_or_else(|| {
            format!("read {path}: not valid UTF-8: no character boundary near the 64 KB cap")
        })?;
        let text = std::str::from_utf8(&buf[..cut])
            .map_err(|e| format!("read {path}: not valid UTF-8: {e}"))?;
        return Ok(format!(
            "{text}\n\u{2026} (truncated at 64 KB \u{2014} file is {total} bytes; \
             continue with {{\"offset\": {}}})",
            offset + start + cut
        ));
    }
    std::str::from_utf8(buf)
        .map(str::to_owned)
        .map_err(|e| format!("read {path}: not valid UTF-8: {e}"))
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
        // A broken symlink or a permissions error shouldn't fail the whole
        // listing — note the entry as unstat-able and keep going.
        lines.push(match std::fs::metadata(entry.path()) {
            Ok(meta) if meta.is_dir() => format!("{name}/"),
            Ok(meta) => format!("{name} ({} B)", meta.len()),
            Err(_) => format!("{name} (?)"),
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
