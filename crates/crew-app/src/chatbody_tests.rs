use super::*;

fn text(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

#[test]
fn newlines_split_prose_into_lines() {
    let lines = body_lines("one\ntwo", 40, (9, 9, 9), false);
    assert_eq!(lines.len(), 2);
    assert_eq!(text(&lines[0]), " one");
    assert_eq!(text(&lines[1]), " two");
}

/// The block is one padded rectangle: a language row, the code, a closing
/// blank row, every one of them the same width and every cell on the code
/// field. The corner glyphs it used to draw instead (`\u{256d}\u{2500} rust` over a
/// background that stopped at the end of each line) read as an unfinished
/// box, not a block.
#[test]
fn code_block_is_one_padded_field_with_its_language_on_top() {
    let lines = body_lines("see:\n```rust\nfn x() {}\n```", 40, (9, 9, 9), false);
    let all: Vec<String> = lines.iter().map(text).collect();
    assert_eq!(all[0], " see:");
    assert_eq!(all[1], " ", "a blank row separates prose from the field");
    assert_eq!(all[2], "  rust      ");
    assert_eq!(all[3], "  fn x() {} ");
    assert_eq!(all[4], "            ", "a blank row closes it");
    let bg = Some(crate::chatink::code_bg());
    for row in &lines[2..5] {
        assert!(
            row[1..].iter().all(|c| c.bg == bg),
            "every cell past the indent is on the field"
        );
        assert!(row[0].bg.is_none(), "the indent column keeps the page");
    }
}

#[test]
fn untagged_fence_is_labelled_code() {
    let lines = body_lines("```\nx\n```", 40, (9, 9, 9), false);
    assert_eq!(text(&lines[0]), "  code ");
}

#[test]
fn long_code_lines_hard_wrap_verbatim() {
    let lines = body_lines("```\nlet a = 1;\n```", 6, (9, 9, 9), false);
    // The field's own padding is part of the card's width, never past it.
    assert!(
        lines.iter().all(|l| l.len() <= 6),
        "{:?}",
        lines.iter().map(text).collect::<Vec<_>>()
    );
    // Every character — including the spaces — survives the wrap. Read
    // from inside the field: column 0 is the indent, then the pad, then
    // the code itself, which is the card's width less both pads.
    let code_w = 6 - 1 - crate::chatfield::PAD * 2;
    let joined: String = lines[1..lines.len() - 1]
        .iter()
        .map(|l| {
            l[1 + crate::chatfield::PAD..]
                .iter()
                .take(code_w)
                .map(|c| c.c)
                .collect::<String>()
        })
        .collect();
    assert_eq!(joined.trim_end(), "let a = 1;");
}

// -- Task 4: full markdown, not just fenced code -----------------------

#[test]
fn bold_survives_to_cardcells() {
    let lines = body_lines("**hi**", 40, (9, 9, 9), false);
    assert!(
        lines[0][1..].iter().all(|c| c.bold),
        "not all bold: {}",
        text(&lines[0])
    );
}

#[test]
fn heading_is_bold() {
    let lines = body_lines("# Title", 40, (9, 9, 9), false);
    assert_eq!(text(&lines[0]), " Title");
    assert!(lines[0][1..].iter().all(|c| c.bold));
}

#[test]
fn link_cells_carry_url() {
    let lines = body_lines("go to [site](https://s.io) now", 60, (9, 9, 9), false);
    let joined = text(&lines[0]);
    let start = joined.find("site").expect("site text present");
    for cell in &lines[0][start..start + "site".len()] {
        assert_eq!(cell.link.as_deref(), Some("https://s.io"));
    }
}

#[test]
fn bullet_list_renders() {
    let lines = body_lines("- one\n- two", 40, (9, 9, 9), false);
    assert_eq!(text(&lines[0]), " \u{2022} one");
    assert_eq!(text(&lines[1]), " \u{2022} two");
}

#[test]
fn numbered_list_with_fenced_code_renders_chrome() {
    let lines = body_lines(
        "1. First do X:\n\n   ```bash\n   cmd --flag\n   ```",
        40,
        (9, 9, 9),
        false,
    );
    let all: Vec<String> = lines.iter().map(text).collect();
    assert!(
        all.iter().any(|l| l.trim() == "bash"),
        "missing the fence's language row: {all:?}"
    );
    let cmd_row = all
        .iter()
        .position(|l| l.contains("cmd --flag"))
        .unwrap_or_else(|| panic!("missing verbatim code line: {all:?}"));
    assert!(
        lines[cmd_row][1].bg.is_some(),
        "code line should carry the code bg: {all:?}"
    );
}

#[test]
fn source_mode_stays_flat() {
    let fg = (9, 9, 9);
    let lines = body_lines("see `this`:\n```rust\nfn x() {}\n```", 40, fg, true);
    for line in &lines {
        for cell in line {
            assert_eq!(cell.fg, fg, "source mode must not colour cells");
            assert_eq!(cell.bg, None, "source mode must not tint cells");
        }
    }
}

#[test]
fn cjk_prose_rechunks_to_display_width_budget() {
    let text_in = "\u{6f22}\u{5b57}".repeat(30);
    let lines = body_lines(&text_in, 20, (9, 9, 9), false);
    assert!(!lines.is_empty());
    for l in &lines {
        let w: usize = l.iter().map(|c| crate::chatwidth::char_w(c.c)).sum();
        assert!(
            w <= 20,
            "line exceeds width budget ({w} > 20): {:?}",
            text(l)
        );
    }
}
