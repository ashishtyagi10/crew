//! The one-line names column against the subject on a quarter tile.
use super::tests::text;
use super::{cells, who_w, Ctx};
use crate::inbox::wire::Thread;

const NOW: i64 = 1_792_500_000_000;
const DAY: i64 = 86_400_000;

fn mail(from: &str, subject: &str, ago: i64) -> Thread {
    Thread {
        id: from.into(),
        from: from.into(),
        subject: subject.into(),
        at_ms: NOW - ago,
        count: 1,
        ..Thread::default()
    }
}

/// The 380 px inbox (38 list columns): `Google Cloud` held 12 of them and
/// left `Your Gmail A…` beside it. The subject now reads at least twice
/// the names' room, and a short sender is still whole.
#[test]
fn a_quarter_tiles_subject_outreads_its_sender() {
    let cols = 38;
    let rows = [
        mail(
            "Google Cloud <c@g.com>",
            "Your Gmail Account is ready",
            20 * DAY,
        ),
        mail("Cy Diaz <cy@x.org>", "Re: the glass theme", 3 * DAY),
    ];
    let x = Ctx {
        cols,
        two: false,
        me: "",
        now: NOW,
        who_w: who_w(&rows, cols, ""),
    };
    let mut out = Vec::new();
    for (r, th) in rows.iter().enumerate() {
        cells(&mut out, th, r as u16, false, &x);
    }
    let google = text(&out, 0, cols);
    assert!(google.contains("Your Gmail Acc"), "{google:?}");
    assert!(
        google.starts_with("   Google "),
        "a first name, not cut: {google:?}"
    );
    let cy = text(&out, 1, cols);
    assert!(cy.starts_with("   Cy Diaz "), "{cy:?}");
    assert!(
        x.who_w * 2 <= cols - x.who_w - 14,
        "names {} of {cols}",
        x.who_w
    );
}

/// A wide pane keeps the names whole: the cap only bites when room is short.
#[test]
fn a_wide_pane_names_its_senders_in_full() {
    let rows = [mail("Google Cloud <c@g.com>", "Your Gmail Account", DAY)];
    assert_eq!(who_w(&rows, 100, ""), 12);
}
