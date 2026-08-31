use super::*;

fn msg(sender: &str, text: &str, ts: &str, meta: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: ts.into(),
        meta: meta.into(),
        usage: None,
        expanded: false,
    }
}

#[test]
fn markdown_has_title_and_a_section_per_message() {
    let msgs = [
        msg("user", "build it", "", ""),
        msg("planner", "plan:\n1. do", "", "4.2s"),
    ];
    let md = transcript_markdown("general", &msgs, &chrono::Local::now());
    assert!(
        md.starts_with("# agent smith \u{00b7} general\n"),
        "got: {md}"
    );
    assert!(md.contains("\n## user\n\nbuild it\n"), "got: {md}");
    assert!(md.contains("\n## planner \u{00b7} 4.2s\n"), "got: {md}");
    assert!(md.contains("plan:\n1. do\n"), "got: {md}");
}

#[test]
fn empty_channel_titles_plain_agent_smith_and_counts_messages() {
    let md = transcript_markdown("", &[], &chrono::Local::now());
    assert!(md.starts_with("# agent smith\n"), "got: {md}");
    assert!(md.contains("0 message(s)"), "got: {md}");
}

#[test]
fn local_time_parses_epoch_millis_and_rejects_garbage() {
    assert!(local_time("1750000000000").is_some());
    assert_eq!(local_time(""), None);
    assert_eq!(local_time("not-a-ts"), None);
}

#[test]
fn success_note_reports_the_message_count_pluralized() {
    let path = PathBuf::from("/tmp/crew-transcript-20260702-000000.md");
    let one = success_note(1, &path);
    assert!(one.contains("exported (1 message)"), "got: {one}");
    assert!(!one.contains("(1 messages)"), "no plural for one: {one}");
    let two = success_note(2, &path);
    assert!(two.contains("exported (2 messages)"), "got: {two}");
}

#[test]
fn task_tagged_meta_exports_the_stripped_latency_not_the_tag() {
    let msgs = [msg("coder", "done", "", "task:2 \u{00b7} 0.0s")];
    let md = transcript_markdown("general", &msgs, &chrono::Local::now());
    assert!(md.contains("0.0s"), "got: {md}");
    assert!(!md.contains("task:"), "tag must not leak into export: {md}");
}
/// `/export` on a pane with no messages must not touch the disk. Every one
/// of the 64 stray `crew-transcript-*.md` files this repo accumulated was a
/// 68-byte "0 message(s)" export, and 54 of them were committed — the
/// command reported success each time, so nothing ever looked wrong.
#[test]
fn exporting_an_empty_pane_writes_no_file() {
    let before = stray_transcripts();
    // An idle child stands in for the broker; only pane state is under test.
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    let mut pane = ChatPane::new(plugin, "crew".into());
    pane.messages.clear();
    assert!(intercept(&mut pane, "/export"));
    let note = &pane.messages.last().unwrap().text;
    assert!(note.contains("nothing to export"), "got: {note}");
    assert_eq!(
        stray_transcripts(),
        before,
        "an empty export still wrote a file"
    );
}

/// Transcripts land in the process's working directory, which for a test
/// binary is the crate root — so this counts what a stray export would drop
/// into the source tree.
fn stray_transcripts() -> usize {
    std::fs::read_dir(".")
        .map(|d| {
            d.filter_map(Result::ok)
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("crew-transcript-")
                })
                .count()
        })
        .unwrap_or(0)
}
