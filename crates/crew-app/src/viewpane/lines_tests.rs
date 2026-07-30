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
            meta: None,
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
            meta: None,
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
    // Fix 6: the old assertion was only `add != del`, so red additions with
    // green deletions — the colours swapped — would still pass. Assert
    // against the exact theme slot each side draws from, per `diff_lines`'s
    // own mapping ('+' -> ansi[2], '-' -> ansi[1]) — this is the only test
    // inspecting rendered ink for any non-markdown rung, which is why Fix 1
    // (Code/Data never syntax-colouring anything) went unnoticed for eleven
    // reviews.
    let t = crew_theme::theme();
    let ls = for_state(&ready(Format::Diff, "+added\n-gone\n"), false, 40);
    let add = ls[0].iter().find(|c| c.c == 'a').unwrap().fg;
    let del = ls[1].iter().find(|c| c.c == 'g').unwrap().fg;
    assert_eq!(add, t.ansi[2], "an addition draws from ansi[2]");
    assert_eq!(del, t.ansi[1], "a deletion draws from ansi[1]");
}

#[test]
fn a_wrapped_added_diff_line_keeps_its_colour_on_the_continuation_row() {
    // Fix 7: `diff_lines` used to read `chars.first()` of each WRAPPED row
    // as if it were the `+`/`-` marker, so a wrapped added line lost its
    // colour after the first row (the continuation's first char is body
    // text, not a marker). 30 columns leaves a 24-char body width (30 minus
    // the 6-column gutter), so this line wraps into at least two rows.
    let long = format!("+{}", "a".repeat(50));
    let ls = for_state(&ready(Format::Diff, &long), false, 30);
    assert!(
        ls.len() >= 2,
        "expected the line to wrap: got {} rows",
        ls.len()
    );
    let t = crew_theme::theme();
    let row1_a = ls[1].iter().find(|c| c.c == 'a').unwrap().fg;
    assert_eq!(
        row1_a, t.ansi[2],
        "a continuation row of an added line is still added"
    );
}

#[test]
fn a_wrapped_diff_row_blanks_its_gutter_like_numbered_does() {
    // Fix 7: `diff_lines` used to reprint the line number on every wrapped
    // row instead of blanking continuations the way `numbered` does.
    let long = format!("+{}", "a".repeat(50));
    let ls = for_state(&ready(Format::Diff, &long), false, 30);
    assert!(
        ls.len() >= 2,
        "expected the line to wrap: got {} rows",
        ls.len()
    );
    let gutter: String = ls[1].iter().take(GUTTER_W).map(|c| c.c).collect();
    assert_eq!(
        gutter,
        " ".repeat(GUTTER_W),
        "a wrapped continuation row's gutter should be blank, got {gutter:?}"
    );
}

#[test]
fn a_keyword_is_coloured_differently_from_a_plain_identifier() {
    // Fix 1: `Code`/`Data` used to reach `numbered`, which painted every
    // character `ink` regardless of what the lexer would have called it —
    // `md::syntax::tokenize` was never called from `viewpane/` at all.
    let t = crew_theme::theme();
    let ls = for_state(
        &ready(Format::Code { lang: "rust" }, "let x = 1;\n"),
        false,
        60,
    );
    let kw = ls[0].iter().find(|c| c.c == 'l').unwrap(); // "let"
    let ident = ls[0].iter().find(|c| c.c == 'x').unwrap(); // the identifier
    assert_eq!(
        kw.fg,
        crate::chatink::token_fg(crate::md::syntax::Token::Keyword),
        "a keyword draws from chatink's derived keyword slot"
    );
    assert!(kw.bold, "chatmd's own convention marks a keyword by weight");
    assert_eq!(
        ident.fg, t.ink,
        "a plain identifier stays on the pane's ink"
    );
    assert_ne!(
        kw.fg, ident.fg,
        "a keyword indistinguishable from plain code delivers no colouring"
    );
}

#[test]
fn a_data_rung_is_syntax_coloured_too() {
    // The brief names both `Code` and `Data` as the rungs that silently did
    // nothing — this covers `Data` specifically so a fix that only wires up
    // `Code` still fails here.
    let ls = for_state(
        &ready(Format::Data { lang: "json" }, "{\"a\": null}\n"),
        false,
        60,
    );
    let kw = ls[0].iter().find(|c| c.c == 'n').unwrap(); // "null"
    assert_eq!(
        kw.fg,
        crate::chatink::token_fg(crate::md::syntax::Token::Keyword)
    );
}

#[test]
fn zero_width_never_panics() {
    let _ = for_state(&ready(Format::Code { lang: "" }, "x\n"), false, 0);
}

/// Low item 2: `row_paint` is the guard between `numbered`'s indexing and a
/// panic if `tokenize` ever stopped covering a line exactly once. Exercised
/// directly, because faking that regression through the real tokenizer would
/// mean breaking `tokenize` itself, which is exactly what its own lossless
/// tests exist to prevent.
#[test]
fn row_paint_is_none_rather_than_panicking_when_a_row_runs_short() {
    let one_entry = vec![((1, 2, 3), false)];
    let paints = vec![one_entry];
    // The normal case: asking for exactly what line 1 has, at offset 0.
    assert!(row_paint(&paints, 1, 0, 1).is_some());
    // A row shorter than requested — the scenario a broken `tokenize` would
    // cause: the wrapped chunk is longer than the paint this line actually
    // has.
    assert!(
        row_paint(&paints, 1, 0, 2).is_none(),
        "asking past the end of a short row must not panic"
    );
    // A line number with no paint vector at all.
    assert!(row_paint(&paints, 5, 0, 1).is_none());
    // `n` of 0 would underflow a direct `n - 1`; must stay `None`, not panic.
    assert!(row_paint(&paints, 0, 0, 1).is_none());
}

#[test]
fn a_huge_line_count_is_capped_and_announced_in_a_banner() {
    // Fix 5: `for_state` used to run the full per-rung render (tokenizing,
    // markdown layout, ...) over however much text `load` handed it — up to
    // the full 8 MB byte cap — on the winit thread, on every distinct `cols`
    // during a resize drag. Double the cap, well past any real "how many
    // rows fit on screen" concern.
    let total = MAX_RENDER_LINES * 2;
    let body = vec!["x"; total].join("\n");
    let ls = for_state(&ready(Format::Code { lang: "" }, &body), false, 60);
    let banner = text(&ls[0]);
    assert!(
        banner.contains(&format!("first {MAX_RENDER_LINES} of {total} lines")),
        "banner names the cap and the real count: {banner}"
    );
    assert!(banner.contains(" o "), "offers the escape: {banner}");
    // The numbered rung emits exactly one row per source line at this width
    // (no wrapping: each line is one 'x'), so the banner plus capped rows is
    // the whole output, not merely "no more than the cap".
    assert_eq!(
        ls.len() - 1,
        MAX_RENDER_LINES,
        "rendered rows must stop at the cap, not merely approach it"
    );
}

// The exactly-at-the-cap "must not be announced as truncated" boundary is
// covered by `rendercap`'s own fast, pure-function tests
// (`text_at_or_under_the_cap_is_returned_unchanged`) rather than repeated
// here through the full render pipeline.

#[test]
fn banner_says_at_least_when_the_text_was_already_byte_capped() {
    // Low item 3: `cap_render_lines` counts lines in `loaded.text`, which is
    // already the 8 MB VIEW cap's slice whenever `loaded.truncated` is
    // `Some` — so an exact "of N lines" would be naming the slice's count as
    // if it were the file's. When both caps fire, the banner must hedge.
    let total = MAX_RENDER_LINES * 2;
    let body = vec!["x"; total].join("\n");
    let state = LoadState::Ready {
        format: Format::Code { lang: "" },
        loaded: Loaded {
            text: body,
            truncated: Some(41_000_000),
            meta: None,
        },
    };
    let ls = for_state(&state, false, 60);
    // ls[0] is the byte-cap banner (`loaded.truncated`), ls[1] the line-cap
    // banner (`capped_from`) — see `truncation_is_announced_in_a_banner_row`
    // and `a_huge_line_count_is_capped_and_announced_in_a_banner` for each in
    // isolation.
    let banner = text(&ls[1]);
    assert!(
        banner.contains(&format!(
            "first {MAX_RENDER_LINES} of at least {total} lines"
        )),
        "must not claim an exact count for text already byte-capped: {banner}"
    );
}

/// The companion to the hedge above: when only the line cap fires (the text
/// was never byte-truncated), the count IS exact and must stay unhedged —
/// otherwise a mutation that always says "at least" would pass the test
/// above while making every ordinary huge-file banner needlessly vague.
#[test]
fn banner_states_an_exact_count_when_the_text_was_not_byte_capped() {
    let total = MAX_RENDER_LINES * 2;
    let body = vec!["x"; total].join("\n");
    let ls = for_state(&ready(Format::Code { lang: "" }, &body), false, 60);
    let banner = text(&ls[0]);
    assert!(
        banner.contains(&format!("first {MAX_RENDER_LINES} of {total} lines")),
        "an exact count when there was no byte cap: {banner}"
    );
    assert!(
        !banner.contains("at least"),
        "must not hedge when the text was not byte-capped: {banner}"
    );
}

#[test]
fn the_line_cap_bounds_a_markdown_render_too() {
    // The hazard the brief names explicitly: for markdown, `for_state` used
    // to run the full `md::render` over the whole text regardless of how
    // many lines that was. Confirm the cap applies before the markdown
    // renderer runs, not just on the gutter rungs.
    let body = vec!["line"; MAX_RENDER_LINES * 2].join("\n");
    let ls = for_state(&ready(Format::Markdown, &body), false, 60);
    let banner = text(&ls[0]);
    assert!(
        banner.contains(&format!("first {MAX_RENDER_LINES}")),
        "markdown rung must also be capped: {banner}"
    );
}
