use super::*;
use crate::viewpane::detect::{Extractor, Format, Opaque};
use crate::viewpane::load::Loaded;

fn text(l: &CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

fn ready(format: Format, body: &str) -> LoadState {
    LoadState::Ready {
        format,
        loaded: Loaded {
            text: body.into(),
            truncated: None,
        },
    }
}

#[test]
fn code_lines_carry_a_numbered_gutter() {
    let ls = for_state(
        &ready(Format::Code { lang: "rust" }, "fn a() {}\nfn b() {}\n"),
        false,
        40,
    );
    assert!(text(&ls[0]).starts_with("    1 "), "got {:?}", text(&ls[0]));
    assert!(text(&ls[1]).starts_with("    2 "));
}

#[test]
fn a_wrapped_row_does_not_reprint_its_line_number() {
    let long = "x".repeat(60);
    let ls = for_state(&ready(Format::Code { lang: "" }, &long), false, 20);
    assert!(text(&ls[0]).starts_with("    1 "));
    assert!(
        text(&ls[1]).starts_with("      "),
        "continuation gutter is blank, got {:?}",
        text(&ls[1])
    );
}

#[test]
fn truncation_is_announced_in_a_banner_row() {
    // A cap that bites silently reads as "this is the whole file", which is a
    // lie about the source.
    let state = LoadState::Ready {
        format: Format::Code { lang: "" },
        loaded: Loaded {
            text: "head\n".into(),
            truncated: Some(41_000_000),
        },
    };
    let ls = for_state(&state, false, 60);
    let banner = text(&ls[0]);
    // Ordered, not two unordered substring checks: "8 MB" and "39" swapped
    // still both appear ("showing first 39 MB of 8 MB"), which is backwards
    // and actively misleading — it tells the reader a 39 MB slice was taken
    // from an 8 MB file.
    assert!(
        banner.contains("first 8 MB of 39 MB"),
        "banner names what is shown before what exists: {banner}"
    );
    assert!(banner.contains(" o "), "offers the escape: {banner}");
}

#[test]
fn an_extract_says_it_is_an_extract() {
    let ls = for_state(
        &ready(
            Format::Extract {
                via: Extractor::PdfToText,
            },
            "page one\n",
        ),
        false,
        60,
    );
    let banner = text(&ls[0]);
    assert!(banner.contains("text extract"), "got {banner}");
    assert!(banner.contains("o "), "offers the OS app: {banner}");
}

#[test]
fn a_missing_extractor_names_what_to_install() {
    let state = ready(
        Format::Opaque {
            why: Opaque::NoExtractor(Extractor::PdfToText),
        },
        "",
    );
    let ls = for_state(&state, false, 60);
    let card: String = ls.iter().map(text).collect::<Vec<_>>().join("\n");
    assert!(card.contains("poppler"), "names the install: {card}");
}

#[test]
fn a_loading_pane_draws_a_skeleton_not_an_empty_page() {
    let ls = for_state(
        &LoadState::Loading {
            rx: std::sync::mpsc::channel().1,
        },
        false,
        40,
    );
    // `!ls.is_empty()` alone is satisfied by `vec![CardLine::new()]` — one
    // visually blank row — which shows the user nothing while still
    // technically being a non-empty outer Vec. Assert actual visible text.
    let visible: String = ls.iter().flatten().map(|c| c.c).collect();
    assert!(
        visible.trim().contains("loading"),
        "a loading pane shows something, not a blank row: {visible:?}"
    );
}

#[test]
fn a_failure_is_drawn_in_the_pane() {
    let ls = for_state(&LoadState::Failed("gone.txt: not found".into()), false, 40);
    let card: String = ls.iter().map(text).collect::<Vec<_>>().join("\n");
    assert!(card.contains("gone.txt"), "got {card}");
}

#[test]
fn raw_mode_shows_markdown_source_verbatim() {
    let ls = for_state(&ready(Format::Markdown, "# Heading\n"), true, 40);
    assert!(text(&ls[0]).contains("# Heading"), "raw keeps the hash");
}

#[test]
fn diff_ink_differs_between_added_and_removed() {
    let ls = for_state(&ready(Format::Diff, "+added\n-gone\n"), false, 40);
    let add = ls[0].iter().find(|c| c.c == 'a').unwrap().fg;
    let del = ls[1].iter().find(|c| c.c == 'g').unwrap().fg;
    assert_ne!(
        add, del,
        "a diff that colours both sides alike is not a diff"
    );
}

#[test]
fn zero_width_never_panics() {
    let _ = for_state(&ready(Format::Code { lang: "" }, "x\n"), false, 0);
}
