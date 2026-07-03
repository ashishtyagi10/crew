//! `/export`: write the crew pane's transcript to a timestamped Markdown file
//! in the working directory (à la OpenCode's `/export`), so a session's
//! multi-agent conversation survives the pane. Handled app-side — the
//! transcript lives in the pane, not the broker.
use std::path::PathBuf;

use crate::chat::ChatPane;
use crate::chatlayout::Message;

/// Intercept composer submissions the pane answers locally. Returns `true`
/// when `text` was consumed (nothing should be sent to the broker).
pub(crate) fn intercept(pane: &mut ChatPane, text: &str) -> bool {
    if text.trim() != "/export" {
        return false;
    }
    let note = match export_transcript(&pane.channel, &pane.messages) {
        Ok(path) => format!("transcript exported \u{2192} {}", path.display()),
        Err(e) => format!("export failed: {e}"),
    };
    let ts = chrono::Local::now().timestamp_millis().to_string();
    pane.messages.push(Message {
        sender: "crew".into(),
        text: note,
        ts,
        meta: String::new(),
    });
    true
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
        "crew".to_string()
    } else {
        format!("crew \u{00b7} {channel}")
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
mod tests {
    use super::*;

    fn msg(sender: &str, text: &str, ts: &str, meta: &str) -> Message {
        Message {
            sender: sender.into(),
            text: text.into(),
            ts: ts.into(),
            meta: meta.into(),
        }
    }

    #[test]
    fn markdown_has_title_and_a_section_per_message() {
        let msgs = [
            msg("user", "build it", "", ""),
            msg("planner", "plan:\n1. do", "", "4.2s"),
        ];
        let md = transcript_markdown("general", &msgs, &chrono::Local::now());
        assert!(md.starts_with("# crew \u{00b7} general\n"), "got: {md}");
        assert!(md.contains("\n## user\n\nbuild it\n"), "got: {md}");
        assert!(md.contains("\n## planner \u{00b7} 4.2s\n"), "got: {md}");
        assert!(md.contains("plan:\n1. do\n"), "got: {md}");
    }

    #[test]
    fn empty_channel_titles_plain_crew_and_counts_messages() {
        let md = transcript_markdown("", &[], &chrono::Local::now());
        assert!(md.starts_with("# crew\n"), "got: {md}");
        assert!(md.contains("0 message(s)"), "got: {md}");
    }

    #[test]
    fn local_time_parses_epoch_millis_and_rejects_garbage() {
        assert!(local_time("1750000000000").is_some());
        assert_eq!(local_time(""), None);
        assert_eq!(local_time("not-a-ts"), None);
    }

    #[test]
    fn task_tagged_meta_exports_the_stripped_latency_not_the_tag() {
        let msgs = [msg("coder", "done", "", "task:2 \u{00b7} 0.0s")];
        let md = transcript_markdown("general", &msgs, &chrono::Local::now());
        assert!(md.contains("0.0s"), "got: {md}");
        assert!(!md.contains("task:"), "tag must not leak into export: {md}");
    }
}
