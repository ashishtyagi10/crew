//! Off-screen renders of the `/inbox` pane on a quarter tile (380 px): the
//! other inbox shots are all 760 px wide, so nothing showed how the list,
//! the reader and a new mail hold up where most panes actually live.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew inbox_quarter_shot -- --ignored`
use crate::inbox::api::Mail;
use crate::inbox::Reading;
use crate::inboxshot_tests::{ready, rows};
use crate::inboxsplitshot_tests::both;

const W: u32 = 380;
const H: u32 = 430;

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn inbox_quarter_shot_screens() {
    let mut list = ready();
    list.sel = 2;
    both("quarter-list", W, H, &list);
    let mut reading = ready();
    reading.reading = Some(Reading {
        mails: Some(vec![Mail {
            from: "Ann Lee <ann@x.org>".into(),
            date: "Tue, 20 Oct 2026 09:02:00 -0400".into(),
            text: "Hi all,\n\nCan we move the Q4 roadmap review to Thursday? I need one more day to finish the numbers.\n\nThanks,\nAnn".into(),
            files: vec!["roadmap-q4.pdf".into()],
            ..Mail::default()
        }]),
        ..Reading::new(rows()[0].clone())
    });
    both("quarter-reader", W, H, &reading);
    let mut new = ready();
    new.open_new();
    let c = new.compose.as_mut().unwrap();
    c.to.put("Ann Lee <ann@x.org>", true);
    c.subject.put("Q4 roadmap", true);
    c.body.put("Hi Ann,\n\nThursday works.", false);
    c.on = crate::inbox::compose::On::Body;
    both("quarter-compose-new", W, H, &new);
}
