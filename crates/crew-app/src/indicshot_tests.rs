//! Off-screen renders of Hindi, and the other scripts that join their
//! letters, on every surface crew draws text on: a chat reply, the `/inbox`
//! reader, a terminal and the input bar, on the light page and on glass.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew indic_shot -- --ignored`
use crew_render::PaneScene;
use crew_term::{GridSize, HeadlessTerm, TermModel};

use crate::chatlayout::Message;
use crate::inbox::api::Mail;
use crate::inbox::Reading;
use crate::inputbar::InputBar;
use crate::session::to_cellviews;
use crate::shotgpu_tests::shot_at;

/// The words the goal names: a plain word, a conjunct, a nasal mark over a
/// vowel sign that sits before its letter, a reph-less ligature, a stacked
/// `dv`, and a vowel sign under a conjunct.
pub(crate) const WORDS: &str = "भारत क्ष हिंदी श्री द्वारा कृपया";
/// A sentence of news, the shape the digest that showed the bug was in.
pub(crate) const NEWS: &str =
    "नई दिल्ली: भारत ने आज अंतरिक्ष में नया उपग्रह भेजा, जिसे वैज्ञानिकों ने ऐतिहासिक बताया।";
/// Hindi and English on one line, which must share a baseline and the grid.
pub(crate) const MIXED: &str = "crew v0.27 में हिंदी text अब जुड़कर दिखता है, cursor समेत।";
/// Bengali, Tamil, Gujarati and Thai: the other scripts the run path covers.
pub(crate) const OTHERS: [&str; 4] = [
    "বাংলা ভাষা খুব সুন্দর",
    "தமிழ் மொழி அழகானது",
    "ગુજરાતી ભાષા સરસ છે",
    "ภาษาไทยสวยงามมาก",
];

/// Run `shoot` once per page the goal is judged on, tagging each file with
/// the theme. Stops quietly where there is no GPU.
fn each_theme(mut shoot: impl FnMut(&str) -> Option<Vec<u8>>) {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in [
        crew_theme::ThemeId::PaperLight,
        crew_theme::ThemeId::GlassNight,
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        if shoot(&format!("{id:?}").to_lowercase()).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            break;
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

fn message(sender: &str, text: String) -> Message {
    Message {
        sender: sender.into(),
        text,
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn indic_shot_chat() {
    let mut p = crate::chatshot_tests::live_pane();
    p.messages = vec![
        message("user", "आज की खबरें हिंदी में बताओ".into()),
        message(
            "smith",
            format!(
                "**मुख्य समाचार**\n\n{NEWS}\n\n{WORDS}\n\n{MIXED}\n\n\
                 [पूरी खबर पढ़ें](https://example.invalid/news)\n\n{}",
                OTHERS.join("\n\n")
            ),
        ),
    ];
    each_theme(|t| {
        shot_at(
            &format!("indic-chat-{t}"),
            700,
            620,
            13.0,
            "crew",
            |c, r, a| crate::chatview::art(&p, c, r, a),
        )
    });
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn indic_shot_reader() {
    let html = format!(
        "<h2>आज की मुख्य खबरें</h2><p><b>{NEWS}</b></p><p>{WORDS}</p><p>{MIXED}</p>\
         <p><a href='https://example.invalid/news'>पूरी खबर पढ़ें</a></p>"
    );
    let rich = crate::inbox::rich::Rich::of_html(&html, None);
    let mut p = crate::inboxshot_tests::ready();
    p.reading = Some(Reading {
        mails: Some(vec![Mail {
            from: "समाचार <news@example.invalid>".into(),
            date: "Wed, 21 Oct 2026 07:30:00 +0530".into(),
            text: rich.whole(),
            rich,
            ..Mail::default()
        }]),
        ..Reading::new(crate::inboxshot_tests::rows()[0].clone())
    });
    // `take` shoots both pages itself.
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    crate::inboxshot_tests::take("indic", &p);
}

/// What a shell prints: the line the goal names, the sentence, the mixed
/// line, and a table a program drew AROUND Hindi, which has to stay square.
fn term_body() -> String {
    format!(
        "$ printf 'भारत क्ष हिंदी\\n'\r\nभारत क्ष हिंदी\r\n{NEWS}\r\n{MIXED}\r\n\
         ┌──────┬──────┐\r\n│ भारत │ ok   │\r\n│ हिंदी │ done │\r\n└──────┴──────┘\r\n\
         {}\r\n$ ",
        OTHERS.join("  ")
    )
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn indic_shot_terminal() {
    let body = term_body();
    each_theme(|t| {
        shot_at(
            &format!("indic-term-{t}"),
            700,
            320,
            13.0,
            "zsh",
            |c, r, _| {
                let mut term = HeadlessTerm::new(GridSize { cols: c, rows: r });
                term.feed(body.as_bytes());
                (to_cellviews(&term.cells(true)), Vec::new())
            },
        )
    });
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn indic_shot_input_bar() {
    let bar = InputBar {
        text: "echo हिंदी क्ष श्री कृपया".into(),
        focused: true,
        cwd: "~/code/crew".into(),
        ..Default::default()
    };
    each_theme(|t| {
        let (w, h, pad) = (560u32, 72u32, 12.0f32);
        let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
            let bw = w as f32 - 2.0 * pad;
            vec![PaneScene {
                cells: bar.cells((bw / cw).floor() as u16, 3, None, None, Some("crew")),
                x: pad,
                y: (h as f32 - 3.0 * ch) / 2.0,
                w: bw,
                h: 3.0 * ch,
                focused: true,
                glass: true,
                ..Default::default()
            }]
        })?;
        crate::shotdraw_tests::write_png(&format!("indic-input-{t}"), &px, w, h);
        Some(px)
    });
}
