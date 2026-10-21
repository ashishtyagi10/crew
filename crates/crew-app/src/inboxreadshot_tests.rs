//! Off-screen renders of the inbox READER: an HTML thread (a receipt, a
//! Gmail reply with its history folded, a newsletter), and a thread with a
//! carried picture drawn, a web one named, and two attachments. Split from
//! `inboxshot_tests` for the line cap; same command, same PNGs.
use crate::inbox::api::Mail;
use crate::inbox::Reading;
use crate::inboxshot_tests::{ready, rows, take};

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn inbox_shot_reader_screens() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let html = |name: &str, from: &str| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/inbox/html/corpus");
        let rich = crate::inbox::rich::Rich::of_html(
            &std::fs::read_to_string(dir.join(format!("{name}.html"))).unwrap(),
            None,
        );
        Mail {
            from: from.into(),
            date: "Tue, 20 Oct 2026 09:02:00 -0400".into(),
            text: rich.whole(),
            rich,
            ..Mail::default()
        }
    };
    let mut rich = ready();
    rich.reading = Some(Reading {
        mails: Some(vec![
            html("receipt", "Shop Co <orders@shop.io>"),
            html("gmail_reply", "Bob Park <bo@y.org>"),
            html("newsletter", "Shop Co <news@shop.io>"),
        ]),
        ..Reading::new(rows()[0].clone())
    });
    take("rich", &rich);
    let mut pics = ready();
    let html = "<p>Q4 is up 12% on Q3. The chart:</p><img src='cid:chart@x' alt='Q4 chart' height='240'>\
                <p>And the team photo, from the web:</p><img src='https://x.io/team.jpg' alt='Team'>\
                <p>Thanks, Ann</p>";
    let rich = crate::inbox::rich::Rich::of_html(html, None);
    pics.reading = Some(Reading {
        mails: Some(vec![Mail {
            id: "m1".into(),
            from: "Ann Lee <ann@x.org>".into(),
            date: "Tue, 20 Oct 2026 09:02:00 -0400".into(),
            text: rich.whole(),
            rich,
            attached: vec![
                crate::inbox::body::Part {
                    name: "q4-numbers.xlsx".into(),
                    size: 48_210,
                    ..Default::default()
                },
                crate::inbox::body::Part {
                    name: "roadmap-q4.pdf".into(),
                    size: 1_902_331,
                    ..Default::default()
                },
            ],
            ..Mail::default()
        }]),
        ..Reading::new(rows()[0].clone())
    });
    let (w, h) = (96u32, 48u32);
    let px = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let bar = x % 24 < 16 && y >= h - (x / 24 + 1) * 10;
            match bar {
                true => [60, 120 + (x / 24) as u8 * 30, 220, 255],
                false => [0, 0, 0, 0],
            }
        })
        .collect();
    let bm = crate::viewpane::bitmap::Bitmap {
        w,
        h,
        px,
        src: (w, h),
    };
    pics.pics.set("m1|cid:chart@x".into(), Some(bm));
    take("pictures", &pics);
}
