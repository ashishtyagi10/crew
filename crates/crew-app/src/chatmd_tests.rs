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
    assert_eq!(out[0][1].fg, crate::chatink::marker_fg(), "the bullet");
    assert_eq!(out[0][3].fg, fg, "the item text");
}

#[test]
fn quote_bar_is_the_marker_colour_and_quoted_text_is_muted() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("> hi there", 40, (9, 9, 9));
    assert_eq!(row_text(&out[0]), " ▎ hi there");
    assert_eq!(out[0][1].fg, crate::chatink::marker_fg(), "the bar");
    assert_eq!(out[0][3].fg, crate::chatink::quote_fg(), "quoted text");
}

#[test]
fn inline_code_inside_a_quote_is_still_code_coloured() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("> use `let` here", 40, (9, 9, 9));
    let coded: Vec<_> = out[0]
        .iter()
        .filter(|c| c.bg == Some(crate::chatink::code_bg()))
        .collect();
    assert_eq!(coded.len(), 3, "l-e-t: {}", row_text(&out[0]));
    assert!(coded.iter().all(|c| c.fg == crate::chatink::code_fg()));
}

#[test]
fn fenced_code_inside_a_quote_still_renders_as_code() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("> ```\n> x = 1\n> ```", 40, (9, 9, 9));
    let code_cells: Vec<_> = out
        .iter()
        .flatten()
        .filter(|c| c.bg == Some(crate::chatink::code_bg()))
        .collect();
    assert!(
        !code_cells.is_empty(),
        "a quoted fence still gets a code card"
    );
    assert!(code_cells.iter().all(|c| c.fg == crate::chatink::code_fg()));
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
    let _guard = crate::app::theme_test_guard();
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

/// End to end through the real pipeline: a fenced block renders with the
/// comment, the string and the keyword each in their own colour, and the
/// keyword in bold. This is the assertion that would have failed before the
/// tokenizer existed, when every cell in a code block shared one colour.
#[test]
fn a_fenced_block_is_syntax_coloured() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("```rust\nlet s = \"hi\"; // note\n```", 60, (9, 9, 9));
    let code: Vec<&crate::chatbody::CardCell> = out
        .iter()
        .flatten()
        .filter(|c| c.bg == Some(crate::chatink::code_bg()))
        .collect();
    let colour_of = |ch: char| code.iter().find(|c| c.c == ch).map(|c| c.fg);
    // 'l' of `let` (keyword), 'h' of "hi" (string), 'n' of `note` (comment).
    let kw = colour_of('l').expect("keyword cell");
    let st = colour_of('h').expect("string cell");
    let cm = colour_of('n').expect("comment cell");
    assert_eq!(
        kw,
        crate::chatink::token_fg(crate::md::syntax::Token::Keyword)
    );
    assert_eq!(st, crate::chatink::token_fg(crate::md::syntax::Token::Str));
    assert_eq!(
        cm,
        crate::chatink::token_fg(crate::md::syntax::Token::Comment)
    );
    assert_ne!(st, cm, "string and comment share a colour");
    // Keywords are marked by weight, so they survive a single-phosphor theme.
    let kw_cell = code.iter().find(|c| c.c == 'l').unwrap();
    assert!(kw_cell.bold, "keywords draw bold");
}

/// A whole reply rendered the way the pane renders it, dumped as a colour MAP
/// so the highlighting is inspectable rather than merely asserted. Each cell
/// becomes one letter: K keyword, S string, C comment, c plain code, · prose.
///
/// This exists because eighteen releases of colour work were verified by
/// contrast arithmetic and never by looking at the output. The map is the
/// closest thing to looking that does not need a GPU, a window, or a key
/// press — everything below it (the CRT pass) has its own headless test.
fn colour_map(text: &str, width: usize) -> String {
    use crate::md::syntax::Token;
    let out = lines(text, width, crew_theme::theme().ink);
    let code_bg = Some(crate::chatink::code_bg());
    let class = |c: &crate::chatbody::CardCell| {
        if c.c == ' ' {
            return ' ';
        }
        if c.bg != code_bg {
            return '\u{00b7}';
        }
        match c.fg {
            f if f == crate::chatink::token_fg(Token::Keyword) && c.bold => 'K',
            f if f == crate::chatink::token_fg(Token::Str) => 'S',
            f if f == crate::chatink::token_fg(Token::Comment) => 'C',
            _ => 'c',
        }
    };
    out.iter()
        .map(|l| l.iter().map(class).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The end-to-end shape of a highlighted reply, pinned. Comments, strings and
/// keywords must each occupy their own cells — and prose outside the fence
/// must claim none of them.
#[test]
fn a_reply_with_code_renders_a_distinct_colour_for_each_class() {
    let _guard = crate::app::theme_test_guard();
    let reply = "Here it is:\n\n```rust\nlet name = \"world\"; // greet\n```\n\nDone.";
    let map = colour_map(reply, 34);
    // Prose lines carry no code cells at all.
    let prose: Vec<&str> = map.lines().filter(|l| l.contains('\u{00b7}')).collect();
    assert!(
        prose.iter().all(|l| !l.contains('K') && !l.contains('S')),
        "prose picked up code colours:\n{map}"
    );
    // The code line carries all three classes plus plain code.
    let code = map
        .lines()
        .find(|l| l.contains('K'))
        .unwrap_or_else(|| panic!("no keyword cell anywhere:\n{map}"));
    for (class, what) in [
        ('K', "keyword"),
        ('S', "string"),
        ('C', "comment"),
        ('c', "plain"),
    ] {
        assert!(
            code.contains(class),
            "no {what} cells on the code line:\n{map}"
        );
    }
    // Order: keyword, then string, then comment — left to right, as written.
    let pos = |ch: char| code.find(ch).unwrap();
    assert!(pos('K') < pos('S'), "keyword after string:\n{map}");
    assert!(pos('S') < pos('C'), "string after comment:\n{map}");
}
