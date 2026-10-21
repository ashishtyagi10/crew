//! Phase 0's corpus: synthetic mails in the shapes real mailers send (none
//! of them real mail), each read against the markdown it should become.
//! `corpus/<name>.html` in, `corpus/<name>.md` expected; a quoted history
//! follows a `===== quote =====` line. `CREW_BLESS=1` rewrites the
//! expectations from what the converter says now — read the diff before
//! keeping it.
use super::to_markdown;

const NAMES: &[&str] = &[
    "apple_reply",
    "calendar",
    "cl_brief",
    "cl_cart",
    "cl_dark",
    "cl_launch",
    "cl_order",
    "cl_shop",
    "deceptive",
    "forward",
    "gcal_buttons",
    "github",
    "gmail_reply",
    "gmail_sig",
    "marketing",
    "newsletter",
    "notice",
    "outlook_reply",
    "pm_comment",
    "pm_receipt",
    "pm_reset",
    "receipt",
    "related",
    "structure",
];

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/inbox/html/corpus")
}

fn shown(name: &str) -> String {
    let html = std::fs::read_to_string(dir().join(format!("{name}.html"))).unwrap();
    // As the reader shows a mail: pictures from the web drawn (item 4).
    let c = to_markdown(&html, true);
    let foot = c
        .foot
        .map(|f| format!("\n\n===== small print =====\n\n{f}"));
    match c.quote {
        Some(q) => format!("{}\n\n===== quote =====\n\n{q}\n", c.body),
        None => format!("{}{}\n", c.body, foot.unwrap_or_default()),
    }
}

#[test]
fn every_corpus_mail_reads_as_its_expected_markdown() {
    let bless = std::env::var_os("CREW_BLESS").is_some();
    let mut wrong = Vec::new();
    for name in NAMES {
        let got = shown(name);
        let path = dir().join(format!("{name}.md"));
        if bless {
            std::fs::write(&path, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&path).unwrap_or_default();
        if got != want {
            wrong.push(format!("── {name}: got\n{got}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn what_nobody_sees_never_reaches_the_markdown() {
    let all: String = NAMES.iter().map(|n| shown(n)).collect();
    for hidden in [
        "IGNORE PREVIOUS",
        "Big savings",
        "shapedefaults",
        "color:red",
        "Weekly",
        "\u{200c}",
    ] {
        assert!(!all.contains(hidden), "{hidden:?} leaked");
    }
    assert!(!all.contains("javascript"), "a script link became a link");
}

#[test]
fn links_keep_their_addresses_and_a_lie_is_named() {
    let d = shown("deceptive");
    assert!(
        d.contains(
            "[paypal.com](<https://paypa1-secure-login.xyz/a>) \u{27f6} paypa1-secure-login.xyz"
        ),
        "{d}"
    );
    assert!(
        !d.contains("signin>) \u{27f6}"),
        "an honest link is not flagged: {d}"
    );
    let n = shown("newsletter");
    assert!(
        n.contains("[`  Shop now  `](<https://click.shop.io/c?u=1&s=2>)"),
        "{n}"
    );
}

#[test]
fn replies_fold_their_history_and_a_forward_does_not_lose_its_mail() {
    for name in ["gmail_reply", "outlook_reply", "apple_reply"] {
        let html = std::fs::read_to_string(dir().join(format!("{name}.html"))).unwrap();
        // As the reader shows a mail: pictures from the web drawn (item 4).
        let c = to_markdown(&html, true);
        assert!(c.quote.is_some(), "{name} found no history");
        assert!(!c.body.is_empty(), "{name} has no reply above its history");
    }
    let fwd = to_markdown(
        &std::fs::read_to_string(dir().join("forward.html")).unwrap(),
        false,
    );
    assert!(
        super::forwarded(fwd.quote.as_deref().unwrap_or("")),
        "{fwd:?}"
    );
}

#[test]
fn a_huge_newsletter_converts_quickly_and_is_cut_at_the_cap() {
    let one = std::fs::read_to_string(dir().join("newsletter.html")).unwrap();
    let big = one.repeat(super::CAP / one.len() + 40);
    let t = std::time::Instant::now();
    let c = to_markdown(&big, false);
    assert!(c.cut);
    // Debug build; the goal's 50 ms is for a release build.
    assert!(t.elapsed().as_millis() < 1500, "{:?}", t.elapsed());
}

#[test]
fn pictures_the_mail_carries_are_drawn_and_the_webs_are_words_while_hidden() {
    let html = std::fs::read_to_string(dir().join("related.html")).unwrap();
    let bare = to_markdown(&html, false);
    // A picture written into the mail is drawn by name, carried beside it.
    let inline = bare.inline[0].0.clone();
    assert_eq!(bare.pics, ["cid:chart@x", inline.as_str()]);
    assert_eq!(bare.web_left, 2, "the banner and the logo; never the pixel");
    assert!(
        bare.body.contains("![Q4 chart](<cid:chart@x#rows=19>)"),
        "300 px at 16 a row: {}",
        bare.body
    );
    assert!(
        bare.body
            .contains("[\u{25a3} Read the report](<https://x.io/report>)"),
        "{}",
        bare.body
    );
    let web = to_markdown(&html, true);
    assert_eq!(
        web.pics,
        [
            "cid:chart@x",
            "https://cdn.x.io/banner.png",
            "https://cdn.x.io/logo.png",
            inline.as_str()
        ]
    );
    assert!(
        web.body.contains(
            "![Read the report](<https://cdn.x.io/banner.png#rows=4&go=https://x.io/report>)"
        ),
        "{}",
        web.body
    );
    assert!(
        !web.body.contains("\u{2197}"),
        "the picture is the link, with no line under it: {}",
        web.body
    );
    assert!(
        !web.body.contains("p.gif") && !web.body.contains("base64"),
        "{}",
        web.body
    );
}

#[test]
fn read_with_its_pictures_hidden_no_mail_names_a_picture_its_caption_names() {
    for name in NAMES {
        let html = std::fs::read_to_string(dir().join(format!("{name}.html"))).unwrap();
        let said: Vec<String> = (to_markdown(&html, false).body.lines())
            .filter(|l| !l.trim().is_empty())
            .map(|l| super::plain_of(l).trim().to_lowercase())
            .collect();
        let echo = |a: &str, b: &str| a.strip_prefix('\u{25a3}').is_some_and(|a| a.trim() == b);
        for w in said.windows(2) {
            assert!(!echo(&w[0], &w[1]) && !echo(&w[1], &w[0]), "{name}: {w:?}");
        }
    }
}
