use super::*;

fn scan(chunks: &[&[u8]]) -> Option<PathBuf> {
    let mut s = OscScanner::default();
    for c in chunks {
        s.feed(c);
    }
    s.take_cwd()
}

fn marks(bytes: &[u8]) -> Vec<ShellMark> {
    let mut s = OscScanner::default();
    s.feed(bytes);
    s.take_shell()
}

/// The four semantic marks, in both terminations a shell may use.
#[test]
fn the_semantic_prompt_marks_are_read() {
    assert_eq!(marks(b"\x1b]133;A\x07"), vec![ShellMark::Prompt]);
    assert_eq!(marks(b"\x1b]133;C\x1b\\"), vec![ShellMark::OutputStart]);
    assert_eq!(marks(b"\x1b]133;D;1\x07"), vec![ShellMark::Done(Some(1))]);
    assert_eq!(marks(b"\x1b]133;D\x07"), vec![ShellMark::Done(None)]);
}

/// `B` says the command LINE begins, which is where the user is typing —
/// crew has nothing to do with that, so it is not queued.
#[test]
fn the_command_line_mark_is_ignored() {
    assert!(marks(b"\x1b]133;B\x07").is_empty());
    assert!(marks(b"\x1b]133;Z\x07").is_empty());
    assert!(marks(b"\x1b]133;\x07").is_empty());
}

/// Shell integrations append their own parameters after the status; the
/// status is the first field and the rest is theirs.
#[test]
fn extra_parameters_after_the_status_are_ignored() {
    assert_eq!(
        marks(b"\x1b]133;D;130;aid=7\x07"),
        vec![ShellMark::Done(Some(130))]
    );
    assert_eq!(marks(b"\x1b]133;A;cl=m\x07"), vec![ShellMark::Prompt]);
}

/// A whole command's worth of marks, in order, and drained once.
#[test]
fn marks_queue_in_order_and_drain_once() {
    let mut s = OscScanner::default();
    s.feed(b"\x1b]133;A\x07prompt$ \x1b]133;C\x07output\r\n\x1b]133;D;0\x07");
    assert_eq!(
        s.take_shell(),
        vec![
            ShellMark::Prompt,
            ShellMark::OutputStart,
            ShellMark::Done(Some(0))
        ]
    );
    assert!(s.take_shell().is_empty(), "draining takes them");
}

/// Split across `feed` chunks, like every other sequence here.
#[test]
fn a_mark_split_across_reads_is_still_read() {
    let mut s = OscScanner::default();
    for c in [&b"\x1b]13"[..], b"3;D;", b"7\x07"] {
        s.feed(c);
    }
    assert_eq!(s.take_shell(), vec![ShellMark::Done(Some(7))]);
}

/// A shell replaying a long scrollback can emit hundreds before anyone
/// drains them. The queue keeps the newest rather than growing.
#[test]
fn the_queue_is_bounded_and_keeps_the_newest() {
    let mut s = OscScanner::default();
    for i in 0..(MAX_MARKS + 10) {
        s.feed(format!("\x1b]133;D;{i}\x07").as_bytes());
    }
    let got = s.take_shell();
    assert_eq!(got.len(), MAX_MARKS);
    assert_eq!(
        got.last(),
        Some(&ShellMark::Done(Some(MAX_MARKS as i32 + 9)))
    );
}

/// OSC 133 must not disturb the sequences that were already read.
#[test]
fn the_other_sequences_still_work_around_it() {
    let mut s = OscScanner::default();
    s.feed(b"\x1b]133;A\x07\x1b]7;file://host/tmp\x07\x1b]133;D;0\x07");
    assert_eq!(s.take_cwd(), Some(PathBuf::from("/tmp")));
    assert_eq!(s.take_shell().len(), 2);
}

#[test]
fn parses_bel_terminated_report() {
    let cwd = scan(&[b"\x1b]7;file://host/Users/me/code\x07"]);
    assert_eq!(cwd, Some(PathBuf::from("/Users/me/code")));
}

#[test]
fn parses_st_terminated_report() {
    let cwd = scan(&[b"\x1b]7;file://host/tmp\x1b\\"]);
    assert_eq!(cwd, Some(PathBuf::from("/tmp")));
}

#[test]
fn empty_host_is_fine() {
    let cwd = scan(&[b"\x1b]7;file:///var/log\x07"]);
    assert_eq!(cwd, Some(PathBuf::from("/var/log")));
}

#[test]
fn percent_decodes_spaces() {
    let cwd = scan(&[b"\x1b]7;file://h/Users/me/My%20Code\x07"]);
    assert_eq!(cwd, Some(PathBuf::from("/Users/me/My Code")));
}

#[test]
fn reassembles_a_split_sequence() {
    // The report is delivered across three feed() chunks.
    let cwd = scan(&[b"\x1b]7;file://host/Use", b"rs/me/co", b"de\x07"]);
    assert_eq!(cwd, Some(PathBuf::from("/Users/me/code")));
}

#[test]
fn ignores_other_osc_sequences() {
    // OSC 0 (title) must not be mistaken for a cwd report.
    assert_eq!(scan(&[b"\x1b]0;some title\x07"]), None);
    assert_eq!(scan(&[b"\x1b]2;another\x07"]), None);
}

#[test]
fn take_is_one_shot_until_it_changes() {
    let mut s = OscScanner::default();
    s.feed(b"\x1b]7;file://h/a\x07");
    assert_eq!(s.take_cwd(), Some(PathBuf::from("/a")));
    // No new report → nothing to take.
    assert_eq!(s.take_cwd(), None);
    // Same dir reported again → still nothing (no change).
    s.feed(b"\x1b]7;file://h/a\x07");
    assert_eq!(s.take_cwd(), None);
    // A real change is reported.
    s.feed(b"\x1b]7;file://h/b\x07");
    assert_eq!(s.take_cwd(), Some(PathBuf::from("/b")));
    assert_eq!(s.cwd(), Some(Path::new("/b")));
}

#[test]
fn unterminated_payload_does_not_grow_without_bound() {
    let mut s = OscScanner::default();
    s.feed(b"\x1b]7;file://h/");
    s.feed(&vec![b'a'; MAX_PAYLOAD + 100]);
    // Aborted past the cap; no cwd captured, buffer released.
    assert_eq!(s.take_cwd(), None);
    assert!(s.buf.is_empty());
}
