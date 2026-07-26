use super::*;

/// The characters of one card line, in order.
fn row_text(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

/// Render `text` through the same path a chat message body takes.
fn lines(text: &str, width: usize, fg: Color) -> Vec<CardLine> {
    map_lines(crate::md::render_chat(text, width), width, fg)
}

#[test]
fn fenced_code_takes_the_code_colour() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("```rust\nfn x() {}\n```", 40, (9, 9, 9));
    // 0 = "╭─ rust" chrome, 1 = code content, 2 = "╰─" chrome.
    assert_eq!(row_text(&out[1]), " fn x() {}");
    let cell = &out[1][1];
    assert_eq!(cell.fg, crew_theme::theme().ansi[6]);
    assert_eq!(cell.bg, Some(crate::chatink::code_bg()));
}

#[test]
fn code_chrome_stays_muted() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("```rust\nfn x() {}\n```", 40, (9, 9, 9));
    assert_eq!(row_text(&out[0]), " ╭─ rust");
    assert_eq!(out[0][1].fg, crew_theme::theme().text_muted);
    assert_eq!(out[0][1].bg, None);
}

#[test]
fn inline_code_is_coloured_but_surrounding_prose_is_not() {
    let _guard = crate::app::theme_test_guard();
    let fg = (9, 9, 9);
    let out = lines("use `let` now", 40, fg);
    // " use let now" — index 0 is the indent cell, 5..8 is "let".
    assert_eq!(row_text(&out[0]), " use let now");
    assert_eq!(out[0][5].fg, crew_theme::theme().ansi[6]);
    assert_eq!(out[0][5].bg, Some(crate::chatink::code_bg()));
    assert_eq!(out[0][1].fg, fg);
    assert_eq!(out[0][1].bg, None);
}

#[test]
fn headings_are_ink_and_bold_at_every_level() {
    let _guard = crate::app::theme_test_guard();
    for src in ["# One", "## Two", "### Three", "###### Six"] {
        let out = lines(src, 40, (9, 9, 9));
        let cell = &out[0][1];
        assert_eq!(cell.fg, crew_theme::theme().ink, "{src}");
        assert!(cell.bold, "{src}");
    }
}

#[test]
fn links_keep_the_link_colour() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("go to [site](https://s.io) now", 60, (9, 9, 9));
    let cell = out[0]
        .iter()
        .find(|c| c.link.is_some())
        .expect("a link cell");
    assert_eq!(cell.fg, crate::chatink::link_color());
    assert!(cell.bold);
}

#[test]
fn list_bullet_is_the_marker_colour_and_its_text_is_not() {
    let _guard = crate::app::theme_test_guard();
    let fg = (9, 9, 9);
    let out = lines("- one", 40, fg);
    assert_eq!(row_text(&out[0]), " • one");
    assert_eq!(out[0][1].fg, crew_theme::theme().ansi[3], "the bullet");
    assert_eq!(out[0][3].fg, fg, "the item text");
}

#[test]
fn quote_bar_is_the_marker_colour_and_quoted_text_is_muted() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("> hi there", 40, (9, 9, 9));
    assert_eq!(row_text(&out[0]), " ▎ hi there");
    assert_eq!(out[0][1].fg, crew_theme::theme().ansi[3], "the bar");
    assert_eq!(out[0][3].fg, crew_theme::theme().text_muted, "quoted text");
}

#[test]
fn inline_code_inside_a_quote_is_still_code_coloured() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("> use `let` here", 40, (9, 9, 9));
    let theme = crew_theme::theme();
    let coded: Vec<_> = out[0]
        .iter()
        .filter(|c| c.bg == Some(crate::chatink::code_bg()))
        .collect();
    assert_eq!(coded.len(), 3, "l-e-t: {}", row_text(&out[0]));
    assert!(coded.iter().all(|c| c.fg == theme.ansi[6]));
}

#[test]
fn fenced_code_inside_a_quote_still_renders_as_code() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("> ```\n> x = 1\n> ```", 40, (9, 9, 9));
    let theme = crew_theme::theme();
    let code_cells: Vec<_> = out
        .iter()
        .flatten()
        .filter(|c| c.bg == Some(crate::chatink::code_bg()))
        .collect();
    assert!(
        !code_cells.is_empty(),
        "a quoted fence still gets a code card"
    );
    assert!(code_cells.iter().all(|c| c.fg == theme.ansi[6]));
}

#[test]
fn the_bar_of_a_quoted_fence_is_a_marker_not_code() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("> ```\n> x = 1\n> ```", 40, (9, 9, 9));
    // Every line of the quoted block carries the bar, including the actual
    // `Code` row ("x = 1") — not just the CodeHeader/CodeFooter chrome rows,
    // whose cells have `bg: None` regardless of whether `marker` or `kind`
    // wins in `span_style`. Only the Code row can distinguish the two.
    let bar = out
        .iter()
        .find(|l| row_text(l).contains("x = 1"))
        .expect("the code line");
    assert_eq!(
        bar[1].fg,
        crew_theme::theme().ansi[3],
        "the bar stays a marker"
    );
    assert_eq!(bar[1].bg, None, "the bar takes no code tint");
}

#[test]
fn a_link_inside_a_list_item_keeps_the_link_colour() {
    let out = lines("- see [site](https://s.io)", 60, (9, 9, 9));
    let cell = out[0]
        .iter()
        .find(|c| c.link.is_some())
        .expect("a link cell");
    assert_eq!(cell.fg, crate::chatink::link_color());
    assert_ne!(
        cell.fg,
        crew_theme::theme().ansi[3],
        "not the marker colour"
    );
}
