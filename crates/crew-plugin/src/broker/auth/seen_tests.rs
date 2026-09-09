use super::*;

/// The whole rule as one table: a flip out→in pins, in→out clears a pin
/// that named this CLI (and only that one), first sightings only record,
/// and a probe that could not answer changes nothing at all.
#[test]
fn a_flip_is_a_choice_a_first_sighting_is_not() {
    let table: &[(
        &str,
        Option<&str>,
        Option<&str>,
        CliAuth,
        Option<&str>,
        Option<Option<&str>>,
    )] = &[
        // (case, pinned, last, now, record, pin)
        (
            "first seen signed in: record only",
            Some("dashscope"),
            None,
            CliAuth::SignedIn,
            Some("in"),
            None,
        ),
        (
            "first seen signed out: record only",
            None,
            None,
            CliAuth::SignedOut,
            Some("out"),
            None,
        ),
        (
            "still signed in: nothing",
            Some("dashscope"),
            Some("in"),
            CliAuth::SignedIn,
            None,
            None,
        ),
        (
            "out → in: pins claude-code over dashscope",
            Some("dashscope"),
            Some("out"),
            CliAuth::SignedIn,
            Some("in"),
            Some(Some("claude-code")),
        ),
        (
            "in → out with its own pin: clears it",
            Some("claude-code"),
            Some("in"),
            CliAuth::SignedOut,
            Some("out"),
            Some(None),
        ),
        (
            "in → out with another pin: leaves it",
            Some("dashscope"),
            Some("in"),
            CliAuth::SignedOut,
            Some("out"),
            None,
        ),
        (
            "in → out unpinned: records only",
            None,
            Some("in"),
            CliAuth::SignedOut,
            Some("out"),
            None,
        ),
        (
            "timeout while signed in: no sign-out",
            Some("claude-code"),
            Some("in"),
            CliAuth::Unknown,
            None,
            None,
        ),
        (
            "uninstalled while signed in: no verdict",
            Some("claude-code"),
            Some("in"),
            CliAuth::Absent,
            None,
            None,
        ),
    ];
    for (case, pinned, last, now, record, pin) in table {
        let (r, p) = edits("claude-code", *pinned, *last, *now);
        assert_eq!(r, *record, "{case}: record");
        assert_eq!(p, pin.map(|o| o.map(str::to_string)), "{case}: pin");
    }
}

/// The verdict half alone: only the two definitive states record, and
/// only a change from the opposite recorded state is a change.
#[test]
fn only_definitive_verdicts_record() {
    assert_eq!(change(None, CliAuth::Unknown), (None, Change::None));
    assert_eq!(change(Some("in"), CliAuth::Absent), (None, Change::None));
    assert_eq!(
        change(Some("out"), CliAuth::SignedIn),
        (Some("in"), Change::SignedIn)
    );
    assert_eq!(
        change(Some("in"), CliAuth::SignedOut),
        (Some("out"), Change::SignedOut)
    );
    assert_eq!(
        change(Some("in"), CliAuth::SignedIn),
        (Some("in"), Change::None)
    );
}
