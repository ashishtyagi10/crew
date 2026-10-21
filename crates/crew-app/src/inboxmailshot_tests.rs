//! Off-screen renders of real-world mail in the reader (goal
//! 2026-10-21-mail-looks-right, item 3): each corpus fixture opened as the
//! reader shows it, its pictures from the web drawn, at a few depths, on the
//! light Ink page and on glass. The pictures are stand-ins of each one's own
//! shape (a placeholder address says its size), since a shot runs offline.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew inbox_mail_shot -- --ignored`
use crate::inbox::api::Mail;
use crate::inbox::rich::Rich;
use crate::inbox::Reading;
use crate::inboxshot_tests::{ready, rows};
use crate::viewpane::bitmap::Bitmap;

/// What each fixture is, and the depths it is shot at.
const MAILS: &[(&str, &str, &[usize])] = &[
    ("marketing", "Acme <news@acme.example>", &[0, 40, 90]),
    (
        "pm_receipt",
        "Acme Billing <billing@acme.example>",
        &[0, 30],
    ),
    ("pm_reset", "Acme <no-reply@acme.example>", &[0]),
    ("pm_comment", "Acme <notify@acme.example>", &[0]),
    ("notice", "HTMLemail <hello@htmlemail.example>", &[0]),
    ("newsletter", "Shop Co <news@shop.io>", &[0]),
    ("cl_shop", "Shopping <hello@shop.example>", &[0, 45]),
    ("cl_order", "Northbound <orders@northbound.example>", &[0]),
    ("cl_brief", "Maya Chen <brief@brief.example>", &[0, 30]),
    ("cl_dark", "Pulse <alex@pulse.example>", &[0]),
    ("cl_cart", "Shop <cart@shop.example>", &[0]),
];

/// A picture of the size its address names (`…/310`, `…/200x50`), else 4:3:
/// a soft gradient with a darker edge, so its box and fit can be judged.
fn stand_in(src: &str) -> Bitmap {
    let size = src.rsplit('/').next().unwrap_or("");
    let mut dims = size.split('x').map(|n| n.parse::<u32>().ok());
    let (w, h) = match (dims.next().flatten(), dims.next().flatten()) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, w),
        _ => (400, 300),
    };
    let (bw, bh) = ((w / 4).clamp(8, 400), (h / 4).clamp(8, 400));
    let px = (0..bw * bh)
        .map(|i| {
            let (x, y) = (i % bw, i / bw);
            let edge = x < 2 || y < 2 || x + 2 >= bw || y + 2 >= bh;
            let t = (x * 255 / bw) as u8;
            match edge {
                true => [70, 90, 140, 255],
                false => [120 + t / 3, 150, 210 - t / 4, 255],
            }
        })
        .collect();
    Bitmap {
        w: bw,
        h: bh,
        px,
        src: (w, h),
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn inbox_mail_shot_corpus() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/inbox/html/corpus");
    for &(name, from, depths) in MAILS {
        let html = std::fs::read_to_string(dir.join(format!("{name}.html"))).unwrap();
        let rich = Rich::of_html(&html, None);
        let mut p = ready();
        let web: Vec<String> = crate::inbox::pics::web_pics(&rich).cloned().collect();
        for src in web.iter().chain(rich.pics.iter()) {
            p.pics.set(src.clone(), Some(stand_in(src)));
        }
        let mail = Mail {
            id: "m1".into(),
            from: from.into(),
            date: "Wed, 21 Oct 2026 09:02:00 -0400".into(),
            text: rich.whole(),
            rich,
            ..Mail::default()
        };
        for &depth in depths {
            p.reading = Some(Reading {
                mails: Some(vec![mail.clone()]),
                web: true,
                scroll: depth,
                ..Reading::new(rows()[0].clone())
            });
            p.sync_sizes();
            for id in [
                crew_theme::ThemeId::PaperLight,
                crew_theme::ThemeId::GlassNight,
            ] {
                crew_theme::set_theme(id);
                crate::palette::set_accent(crew_theme::theme().accent_default);
                let tag = format!("mail-{name}-{depth}-{}", format!("{id:?}").to_lowercase());
                let legend = p.legend();
                let px = crate::shotgpu_tests::shot_at(&tag, 760, 900, 13.0, &legend, |c, r, a| {
                    p.art(c, r, a)
                });
                if px.is_none() {
                    eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
                    return;
                }
            }
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
