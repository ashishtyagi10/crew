//! The threads the `/inbox` shots draw: a dozen rows of the kinds an inbox
//! holds (a thread among several people, a robot, a receipt, a newsletter),
//! spread from minutes to a year old.
use crate::inbox::wire::{Sender, Thread};

const H: i64 = 3_600_000;

/// One row for the shots: `who` is every message's sender, oldest first
/// (`me` is the account's own), `ago` how long since the newest.
type Row<'a> = (&'a str, &'a [&'a str], &'a str, &'a str, bool, i64);

const ROWS: [Row; 12] = [
    (
        "a",
        &["Ann Lee <ann@x.org>", "me", "Bo Park <bo@y.org>"],
        "Q4 roadmap review",
        "Can we move the review to Thursday? I need one more day for the numbers",
        true,
        H / 6,
    ),
    (
        "b",
        &["GitHub <noreply@github.com>"],
        "[crew] Release v0.27.28",
        "Release succeeded for 6 targets in 10m 26s",
        true,
        2 * H,
    ),
    (
        "c",
        &["Bo Park <bo@y.org>", "me"],
        "Lunch Friday?",
        "Sure \u{2014} the place on 5th at 12:30 works for me",
        false,
        5 * H,
    ),
    (
        "d",
        &["Google Cloud <cloud@google.com>"],
        "Your Gmail API client is ready",
        "You created an OAuth client for crew-inbox",
        false,
        30 * H,
    ),
    (
        "e",
        &[
            "Cy Diaz <cy@z.org>",
            "Ann Lee <ann@x.org>",
            "me",
            "Cy Diaz <cy@z.org>",
            "Bo Park <bo@y.org>",
            "Cy Diaz <cy@z.org>",
        ],
        "Re: the glass theme",
        "White text on the frost reads well on my monitor now",
        false,
        4 * 24 * H,
    ),
    (
        "f",
        &["Stripe <receipts@stripe.com>"],
        "Your receipt from Fly.io #2291-4410",
        "Amount paid $14.20 \u{b7} Oct 14, 2026",
        false,
        6 * 24 * H,
    ),
    (
        "g",
        &["Dee Wu <dee@w.org>"],
        "Photos from the offsite",
        "Here are the ones from Saturday \u{2014} the lake ones came out great",
        true,
        8 * 24 * H,
    ),
    (
        "h",
        &["me", "Eli Ross <eli@r.org>"],
        "Re: contract renewal",
        "Signed copy attached, thanks for the quick turnaround",
        false,
        12 * 24 * H,
    ),
    (
        "i",
        &["Hacker Newsletter <hi@hackernewsletter.com>"],
        "Hacker Newsletter #712",
        "Rust in the kernel, a GPU terminal, and why mail is still hard",
        false,
        15 * 24 * H,
    ),
    (
        "j",
        &["Fay Kim <fay@k.org>", "me"],
        "Visa appointment",
        "Confirmed for the 3rd at 9:15, bring the printed form",
        false,
        40 * 24 * H,
    ),
    (
        "k",
        &["AWS <no-reply@aws.amazon.com>"],
        "Your September invoice",
        "Total due $3.81",
        false,
        50 * 24 * H,
    ),
    (
        "l",
        &["Gus Hale <gus@h.org>"],
        "Old friends dinner",
        "It has been too long \u{2014} are you around in November?",
        false,
        400 * 24 * H,
    ),
];

pub(crate) fn rows() -> Vec<Thread> {
    let now = chrono::Utc::now().timestamp_millis();
    ROWS.iter()
        .map(|&(id, who, subject, snippet, unread, ago)| {
            let senders: Vec<Sender> = who
                .iter()
                .map(|w| Sender {
                    from: match *w {
                        "me" => "Ash T <ash@mail.org>".into(),
                        w => w.into(),
                    },
                    sent: *w == "me",
                })
                .collect();
            Thread {
                id: id.into(),
                from: senders.last().map(|s| s.from.clone()).unwrap_or_default(),
                subject: subject.into(),
                snippet: snippet.into(),
                unread,
                starred: id == "b" || id == "h",
                at_ms: now - ago,
                count: who.len(),
                senders,
            }
        })
        .collect()
}
