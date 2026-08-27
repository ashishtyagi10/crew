use super::*;

fn text_of(s: &str, reveal: bool) -> String {
    prepare(s, reveal).text
}

/// The bug this exists for: a tab has zero display width, so `place_row`
/// skipped it and a tab-indented file drew with no indentation at all.
#[test]
fn a_tab_becomes_the_columns_it_stands_for() {
    assert_eq!(text_of("\tx", false), format!("{}x", " ".repeat(TAB_STOP)));
    assert_eq!(
        text_of("\t\tx", false),
        format!("{}x", " ".repeat(TAB_STOP * 2))
    );
}

/// A tab advances to the next STOP, it does not insert a fixed number of
/// spaces — which is the whole difference between a tab and an indent, and
/// what makes a table of tab-separated columns line up.
#[test]
fn a_tab_advances_to_the_next_stop_not_by_a_fixed_width() {
    // One character in, the tab covers the remaining seven columns.
    assert_eq!(
        text_of("a\tb", false),
        format!("a{}b", " ".repeat(TAB_STOP - 1))
    );
    // Exactly on a stop, it covers a whole one.
    let full = "a".repeat(TAB_STOP);
    assert_eq!(
        text_of(&format!("{full}\tb"), false),
        format!("{full}{}b", " ".repeat(TAB_STOP))
    );
    // Every prefix length lands the following character on a stop boundary.
    for n in 0..(TAB_STOP * 2) {
        let out = text_of(&format!("{}\tX", "a".repeat(n)), false);
        let at = out
            .chars()
            .position(|c| c == 'X')
            .expect("the marker survived");
        assert_eq!(at % TAB_STOP, 0, "{n} leading chars put X at column {at}");
    }
}

/// A wide glyph is two columns, and the stop is measured in columns. Two
/// CJK glyphs put the cursor at column 4, so the tab covers the remaining
/// four — a character count would have said eight and drawn the text one
/// stop too far in.
#[test]
fn the_stop_is_measured_in_columns_not_characters() {
    let out = text_of("\u{4e16}\u{754c}\tX", false); // two 2-wide glyphs = 4 columns
    let chars: Vec<char> = out.chars().collect();
    assert_eq!(
        chars.iter().position(|c| *c == 'X'),
        Some(2 + (TAB_STOP - 4))
    );
}

/// A carriage return has no width either, so it was never drawn. With the
/// reveal off it is simply dropped rather than left in the text to mislead
/// whatever measures the line.
#[test]
fn a_carriage_return_is_dropped_unless_it_is_being_shown() {
    assert_eq!(text_of("x\r", false), "x");
    assert_eq!(text_of("x\r", true), "x\u{240d}");
}

#[test]
fn revealing_marks_the_tab_once_and_pads_the_rest() {
    let out = text_of("\tx", true);
    assert!(out.starts_with('\u{2192}'), "the arrow leads: {out:?}");
    assert_eq!(out.chars().filter(|c| *c == '\u{2192}').count(), 1);
    assert_eq!(out.chars().count(), TAB_STOP + 1, "still eight columns");
}

/// Only TRAILING spaces are marked: a line of pure indentation still reads as
/// one, and the spaces between words are not the problem anyone is looking
/// for.
#[test]
fn only_trailing_spaces_are_marked() {
    assert_eq!(text_of("a b  ", true), "a b\u{b7}\u{b7}");
    assert_eq!(text_of("a b  ", false), "a b  ");
    assert_eq!(
        text_of("    ", true),
        "\u{b7}\u{b7}\u{b7}\u{b7}",
        "a blank line is all trailing"
    );
}

/// The marks are reported per character, aligned to the text `prepare`
/// returns — recolouring by glyph instead would dim a `·` that was genuinely
/// in the file.
#[test]
fn the_marks_line_up_with_the_text_they_describe() {
    let p = prepare("a\tb  \nplain", true);
    for (line, marks) in p.text.split('\n').zip(&p.marks) {
        assert_eq!(line.chars().count(), marks.len(), "{line:?}");
    }
    let first: Vec<char> = p.text.split('\n').next().unwrap().chars().collect();
    for (i, m) in p.marks[0].iter().enumerate() {
        assert_eq!(
            *m,
            first[i] != 'a' && first[i] != 'b',
            "column {i} ({:?}) marked {m}",
            first[i]
        );
    }
    assert!(p.marks[1].iter().all(|m| !m), "a clean line marks nothing");
}

/// With the reveal off nothing is marked — including the spaces a tab
/// expanded into, which are indentation and not a finding.
#[test]
fn nothing_is_marked_when_nothing_is_revealed() {
    let p = prepare("a\tb  \r", false);
    assert!(p.marks.iter().flatten().all(|m| !m));
}

#[test]
fn dimming_recolours_exactly_the_marked_characters() {
    let ink = (200, 200, 200);
    let muted = (90, 90, 90);
    let mut paints = vec![vec![(ink, true); 4]];
    dim(&mut paints, &[vec![false, true, false, true]], muted);
    assert_eq!(
        paints[0],
        vec![(ink, true), (muted, false), (ink, true), (muted, false)]
    );
}

/// A paint array and a mark array of different lengths must not panic — the
/// tokenizer's losslessness is enforced by tests rather than by the type
/// system, and this runs on the winit thread mid-frame.
#[test]
fn a_mismatched_pair_stops_rather_than_panicking() {
    let mut paints = vec![vec![((1, 1, 1), false); 2]];
    dim(&mut paints, &[vec![true; 9]], (9, 9, 9));
    assert_eq!(paints[0].len(), 2);
    dim(&mut paints, &[], (9, 9, 9));
}
