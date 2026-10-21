//! Off-screen renders of the `/inbox` pane: the setup page, the list and an
//! opened thread, on the Ink Reader light page and on glass.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew inbox_shot -- --ignored`
use std::sync::mpsc;

use crate::chartshot_tests::shot;
use crate::inbox::api::Mail;
use crate::inbox::grant::Client;
use crate::inbox::worker::Worker;
use crate::inbox::{InboxPane, Reading, Stage};

pub(crate) use crate::inboxrows_tests::rows;

pub(crate) fn ready() -> InboxPane {
    let (tx, _) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = rows();
    p.unread = Some(2);
    p
}

pub(crate) fn take(name: &str, p: &InboxPane) {
    for id in [
        crew_theme::ThemeId::PaperLight,
        crew_theme::ThemeId::GlassNight,
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let tag = format!("inbox-{name}-{}", format!("{id:?}").to_lowercase());
        let px = shot(&tag, &p.legend(), |cols, rows, aspect| {
            p.art(cols, rows, aspect)
        });
        if px.is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn inbox_shot_screens() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let found = Some((
        "client_secret_1234-abc.apps.googleusercontent.com.json".into(),
        Client {
            id: "1".into(),
            secret: "s".into(),
        },
    ));
    take("setup", &InboxPane::with(Stage::Setup { found }));
    let mut list = ready();
    list.sel = 2;
    take("list", &list);
    list.density = crate::inbox::list::Density::One;
    take("list-one", &list);
    let mut reading = ready();
    reading.reading = Some(Reading {
        mails: Some(vec![
            Mail { from: "Ann Lee <ann@x.org>".into(), date: "Tue, 20 Oct 2026 09:02:00 -0400".into(), text: "Hi all,\n\nCan we move the Q4 roadmap review to Thursday? I need one more day to finish the numbers for the platform team, and Bo is out on Wednesday anyway.\n\nThanks,\nAnn".into(), files: vec!["roadmap-q4.pdf".into()], ..Mail::default() },
            Mail { from: "Bo Park <bo@y.org>".into(), date: "Tue, 20 Oct 2026 09:40:00 -0400 (EDT)".into(), text: "Thursday works for me.".into(), files: vec![], ..Mail::default() },
        ]),
        ..Reading::new(rows()[0].clone())
    });
    take("reader", &reading);
}
