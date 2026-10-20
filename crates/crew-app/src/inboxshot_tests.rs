//! Off-screen renders of the `/inbox` pane: the setup page, the list and an
//! opened thread, on the Ink Reader light page and on glass.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew inbox_shot -- --ignored`
use std::sync::mpsc;

use crate::chartshot_tests::shot;
use crate::inbox::api::Mail;
use crate::inbox::grant::Client;
use crate::inbox::wire::Thread;
use crate::inbox::worker::Worker;
use crate::inbox::{InboxPane, Reading, Stage};

const H: i64 = 3_600_000;

fn rows() -> Vec<Thread> {
    let now = chrono::Utc::now().timestamp_millis();
    let t = |id: &str,
             from: &str,
             subject: &str,
             snippet: &str,
             unread: bool,
             ago: i64,
             count: usize| Thread {
        id: id.into(),
        from: from.into(),
        subject: subject.into(),
        snippet: snippet.into(),
        unread,
        starred: id == "b",
        at_ms: now - ago,
        count,
    };
    vec![
        t(
            "a",
            "Ann Lee <ann@x.org>",
            "Q4 roadmap review",
            "Can we move the review to Thursday? I need one more day for the numbers",
            true,
            H / 6,
            3,
        ),
        t(
            "b",
            "GitHub <noreply@github.com>",
            "[crew] Release v0.27.28",
            "Release succeeded for 6 targets in 10m 26s",
            true,
            2 * H,
            1,
        ),
        t(
            "c",
            "Bo Park <bo@y.org>",
            "Lunch Friday?",
            "Sure — the place on 5th at 12:30 works for me",
            false,
            5 * H,
            2,
        ),
        t(
            "d",
            "Google Cloud <cloud@google.com>",
            "Your Gmail API client is ready",
            "You created an OAuth client for crew-inbox",
            false,
            30 * H,
            1,
        ),
        t(
            "e",
            "Cy Diaz <cy@z.org>",
            "Re: the glass theme",
            "White text on the frost reads well on my monitor now",
            false,
            4 * 24 * H,
            6,
        ),
    ]
}

fn ready() -> InboxPane {
    let (tx, _) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = rows();
    p.unread = Some(2);
    p
}

fn take(name: &str, p: &InboxPane) {
    for id in [
        crew_theme::ThemeId::PaperLight,
        crew_theme::ThemeId::GlassNight,
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let tag = format!("inbox-{name}-{}", format!("{id:?}").to_lowercase());
        let px = shot(&tag, &p.legend(), |cols, rows, _| {
            (p.cells(cols, rows), Vec::new())
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
    take("list", &ready());
    let mut reading = ready();
    reading.reading = Some(Reading {
        thread: rows()[0].clone(),
        mails: Some(vec![
            Mail { from: "Ann Lee <ann@x.org>".into(), date: "Tue, 20 Oct 2026 09:02:00 -0400".into(), text: "Hi all,\n\nCan we move the Q4 roadmap review to Thursday? I need one more day to finish the numbers for the platform team, and Bo is out on Wednesday anyway.\n\nThanks,\nAnn".into(), files: vec!["roadmap-q4.pdf".into()] },
            Mail { from: "Bo Park <bo@y.org>".into(), date: "Tue, 20 Oct 2026 09:40:00 -0400 (EDT)".into(), text: "Thursday works for me.".into(), files: vec![] },
        ]),
        scroll: 0,
    });
    take("reader", &reading);
}
