//! Off-screen renders of the inbox list's last two phases (goal
//! 2026-10-20-inbox-list-look, phases 4 and 5): a thread read beside the list
//! on a wide pane, and the list mid-motion, with a hovered row, the
//! selection's wash on its way, new mail glowing and an archived thread
//! folding away, on the Ink Reader light page and on glass.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew inbox_split_shot -- --ignored`
use crate::inbox::api::Mail;
use crate::inbox::motion::Fold;
use crate::inbox::{InboxPane, Reading};
use crate::inboxshot_tests::{ready, rows};
use crate::shotgpu_tests::shot_pics;

fn both(name: &str, w: u32, h: u32, p: &InboxPane) {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in [
        crew_theme::ThemeId::PaperLight,
        crew_theme::ThemeId::GlassNight,
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let tag = format!("inbox-{name}-{}", format!("{id:?}").to_lowercase());
        let px = shot_pics(&tag, w, h, 13.0, &p.legend(), |c, r, a| {
            let (cells, paint) = p.art(c, r, a);
            (cells, paint, p.pictures(c, r, a))
        });
        if px.is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            break;
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn inbox_split_shot_wide_and_moving() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/inbox/html/corpus");
    let html = std::fs::read_to_string(dir.join("newsletter.html")).unwrap();
    let rich = crate::inbox::rich::Rich::of_html(&html, None);
    let mut wide = ready();
    wide.sel = 1;
    wide.reading = Some(Reading {
        mails: Some(vec![Mail {
            from: "Shop Co <news@shop.io>".into(),
            date: "Tue, 20 Oct 2026 09:02:00 -0400".into(),
            text: rich.whole(),
            rich,
            ..Mail::default()
        }]),
        ..Reading::new(rows()[1].clone())
    });
    both("split", 1320, 640, &wide);
    // Everything that moves, stopped mid-way: the selection gliding from
    // the first thread to the third, the fifth hovered, new mail glowing
    // on the first, and an archived thread folding where the fourth was.
    let now = crate::anim::now_ms();
    let mut moving = ready();
    moving.sel = 2;
    moving.hover = Some(5);
    moving.motion.glide = Some((0, now.saturating_sub(40)));
    moving.motion.fresh = Some((vec![moving.rows[0].id.clone()], now));
    moving.motion.fold = Some(Fold {
        at: 3,
        thread: moving.rows[3].clone(),
        start: now.saturating_sub(60),
    });
    both("moving", 760, 560, &moving);
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn inbox_split_shot_reply_box() {
    let mut p = ready();
    p.sel = 1;
    p.reading = Some(Reading {
        mails: Some(vec![Mail {
            from: "Bob Park <bo@y.org>".into(),
            date: "Tue, 20 Oct 2026 09:02:00 -0400".into(),
            rich: crate::inbox::rich::Rich::of_plain(
                "Hi Ann,\n\nFriday works. I'll bring the Q4 numbers and the forecast sheet.\n\nBob",
                None,
            ),
            ..Mail::default()
        }]),
        ..Reading::new(rows()[1].clone())
    });
    p.press(&winit::keyboard::Key::Character("r".into()), 90, 30);
    p.type_in("Thanks Bob, Friday at 10 then.\nI'll book the room.");
    both("reply", 760, 560, &p);
}

/// `R` and `f`: the box with its `to` and `cc` lines, the caret on cc, and
/// a forward carrying a file.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn inbox_split_shot_cc_and_forward() {
    let mut p = ready();
    p.sel = 1;
    p.email = "ann@x.org".into();
    let mut mail = Mail {
        id: "m1".into(),
        from: "Bob Park <bo@y.org>".into(),
        date: "Tue, 20 Oct 2026 09:02:00 -0400".into(),
        rich: crate::inbox::rich::Rich::of_plain("Hi Ann,\n\nFriday works.\n\nBob", None),
        attached: vec![crate::inbox::body::Part {
            name: "Q4 forecast.xlsx".into(),
            size: 48_000,
            ..Default::default()
        }],
        ..Mail::default()
    };
    mail.reply.subject = "Q4 review".into();
    mail.reply.to = "Ann Lee <ann@x.org>, Cy Dee <cy@y.org>".into();
    mail.reply.cc = "dana@y.org".into();
    let read = |p: &mut InboxPane| {
        p.compose = None;
        p.reading = Some(Reading {
            mails: Some(vec![mail.clone()]),
            ..Reading::new(rows()[1].clone())
        });
    };
    let key = |p: &mut InboxPane, k: &str| {
        p.press(&winit::keyboard::Key::Character(k.into()), 90, 30);
    };
    read(&mut p);
    key(&mut p, "R");
    p.type_in("Friday at 10 suits everyone.");
    p.press(
        &winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp),
        90,
        30,
    );
    p.type_in(", eve@y.org");
    both("reply-all", 760, 560, &p);
    read(&mut p);
    key(&mut p, "f");
    p.type_in("Kim <kim@z.org>");
    both("forward", 760, 560, &p);
}
