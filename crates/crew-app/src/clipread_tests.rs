//! OSC 52 reads answered only when the setting allows it, only for the
//! focused pane, and never past the size cap — with a fake clipboard that
//! counts how often it was read, so "denied" also proves "not looked at".
use super::{answer, READ_CAP};
use crew_term::{ClipboardAsk, GridSize, HeadlessTerm, TermModel};

fn ask(query: &[u8]) -> ClipboardAsk {
    let mut t = HeadlessTerm::new(GridSize { cols: 80, rows: 24 });
    t.feed(query);
    t.take_clipboard_ask().expect("the query asks")
}

/// A clipboard holding `text` that counts its reads.
pub(super) fn fake<'a>(text: &str, reads: &'a mut usize) -> impl FnMut() -> Option<String> + 'a {
    let text = text.to_string();
    move || {
        *reads += 1;
        Some(text.clone())
    }
}

#[test]
fn allowed_and_focused_answers_in_base64() {
    let mut reads = 0;
    let got = answer(
        &ask(b"\x1b]52;c;?\x07"),
        true,
        true,
        &mut fake("hi", &mut reads),
    );
    assert_eq!(got.as_deref(), Some("\x1b]52;c;aGk=\x07"));
    assert_eq!(reads, 1);
}

/// tmux closes with ST, and gets ST back.
#[test]
fn an_st_query_is_answered_with_st() {
    let mut reads = 0;
    let got = answer(
        &ask(b"\x1b]52;c;?\x1b\\"),
        true,
        true,
        &mut fake("hi", &mut reads),
    );
    assert_eq!(got.as_deref(), Some("\x1b]52;c;aGk=\x1b\\"));
}

#[test]
fn denied_answers_nothing_and_never_reads() {
    let mut reads = 0;
    let got = answer(
        &ask(b"\x1b]52;c;?\x07"),
        false,
        true,
        &mut fake("hi", &mut reads),
    );
    assert_eq!(got, None);
    assert_eq!(reads, 0, "the clipboard was read while denied");
}

#[test]
fn an_unfocused_pane_gets_nothing_and_nothing_is_read() {
    let mut reads = 0;
    let got = answer(
        &ask(b"\x1b]52;c;?\x07"),
        true,
        false,
        &mut fake("hi", &mut reads),
    );
    assert_eq!(got, None);
    assert_eq!(reads, 0);
}

/// At the cap the whole clipboard goes; one byte past it nothing does —
/// never a silently cut paste.
#[test]
fn past_the_cap_answers_nothing() {
    let q = ask(b"\x1b]52;c;?\x07");
    let mut reads = 0;
    let at = "a".repeat(READ_CAP);
    assert!(answer(&q, true, true, &mut fake(&at, &mut reads)).is_some());
    let over = "a".repeat(READ_CAP + 1);
    assert_eq!(answer(&q, true, true, &mut fake(&over, &mut reads)), None);
}

/// No text on the clipboard (empty, or an image) is an empty answer, so a
/// waiting program is told rather than left to time out.
#[test]
fn no_text_answers_empty() {
    let got = answer(&ask(b"\x1b]52;c;?\x07"), true, true, &mut || None);
    assert_eq!(got.as_deref(), Some("\x1b]52;c;\x07"));
}

/// The setting fails closed: only `allow` opens it.
#[test]
fn only_allow_opens_the_gate() {
    let mut c = crate::config::CrewConfig::default();
    assert_eq!(c.clipboard_read, "deny");
    for (v, open) in [
        ("allow", true),
        (" Allow ", true),
        ("ask", false),
        ("yes", false),
    ] {
        c.clipboard_read = v.to_string();
        assert_eq!(c.clipboard_read_allowed(), open, "{v}");
        assert_eq!(
            c.clone().clamped().clipboard_read,
            ["deny", "allow"][usize::from(open)]
        );
    }
}
