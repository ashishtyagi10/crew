//! `/export`: write the crew pane's transcript to a timestamped Markdown file
//! in the working directory (à la OpenCode's `/export`), so a session's
//! multi-agent conversation survives the pane. Handled app-side — the
//! transcript lives in the pane, not the broker.
use std::path::{Path, PathBuf};

use crate::chat::ChatPane;
use crate::chatlayout::Message;

/// Intercept composer submissions the pane answers locally. Returns `true`
/// when `text` was consumed (nothing should be sent to the broker).
pub(crate) fn intercept(pane: &mut ChatPane, text: &str) -> bool {
    if text.trim() != "/export" {
        return false;
    }
    let n = pane.messages.len();
    // An export of nothing is not an export. Writing it anyway left 68-byte
    // "0 message(s)" files wherever crew happened to be running — 54 of them
    // reached this repo's history before anyone noticed, because the command
    // reported success every time.
    let note = if n == 0 {
        "nothing to export \u{2014} this pane has no messages yet".to_string()
    } else {
        match export_transcript(&pane.channel, &pane.messages) {
            Ok(path) => success_note(n, &path),
            Err(e) => format!("export failed: {e}"),
        }
    };
    let ts = chrono::Local::now().timestamp_millis().to_string();
    pane.messages.push(Message {
        sender: "agent smith".into(),
        text: note,
        ts,
        meta: String::new(),
        usage: None,
        expanded: false,
    });
    true
}

/// The `/export` success echo: the exported message count (pluralized) and
/// the file it was written to, so the user knows what landed on disk.
fn success_note(n: usize, path: &Path) -> String {
    let plural = if n == 1 { "" } else { "s" };
    format!(
        "transcript exported ({n} message{plural}) \u{2192} {}",
        path.display()
    )
}

/// Write the transcript and return the file's path. The file lands in the
/// current working directory as `crew-transcript-YYYYmmdd-HHMMSS.md`.
fn export_transcript(channel: &str, messages: &[Message]) -> Result<PathBuf, String> {
    let now = chrono::Local::now();
    let name = format!("crew-transcript-{}.md", now.format("%Y%m%d-%H%M%S"));
    let path = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join(name);
    std::fs::write(&path, transcript_markdown(channel, messages, &now))
        .map_err(|e| e.to_string())?;
    Ok(path)
}

/// The transcript as Markdown: a title, the export date, then one `## sender`
/// section per message with its wall-clock time and latency when known.
pub(crate) fn transcript_markdown(
    channel: &str,
    messages: &[Message],
    now: &chrono::DateTime<chrono::Local>,
) -> String {
    let title = if channel.is_empty() {
        "agent smith".to_string()
    } else {
        format!("agent smith \u{00b7} {channel}")
    };
    let mut out = format!(
        "# {title}\n\nExported {} \u{00b7} {} message(s)\n",
        now.format("%Y-%m-%d %H:%M:%S"),
        messages.len()
    );
    for m in messages {
        let mut head = format!("\n## {}", m.sender);
        if let Some(t) = local_time(&m.ts) {
            head.push_str(&format!(" \u{00b7} {t}"));
        }
        let meta = crate::chattime::strip_task_tag(&m.meta);
        if !meta.is_empty() {
            head.push_str(&format!(" \u{00b7} {}", meta));
        }
        out.push_str(&head);
        out.push_str("\n\n");
        out.push_str(m.text.trim_end());
        out.push('\n');
    }
    out
}

/// An epoch-milliseconds string as a local `HH:MM:SS` (None when unparseable).
fn local_time(ts: &str) -> Option<String> {
    let ms: i64 = ts.parse().ok()?;
    let utc = chrono::DateTime::from_timestamp_millis(ms)?;
    Some(
        utc.with_timezone(&chrono::Local)
            .format("%H:%M:%S")
            .to_string(),
    )
}

#[cfg(test)]
#[path = "chatexport_tests.rs"]
mod tests;
