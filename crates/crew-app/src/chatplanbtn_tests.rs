use super::*;

fn text(row: &Row) -> String {
    let mut cells = row.cells.clone();
    cells.sort_by_key(|(c, _)| *c);
    let mut out = String::new();
    let mut at = cells.first().map_or(0, |(c, _)| *c);
    for (col, c) in cells {
        while at < col {
            out.push(' ');
            at += 1;
        }
        out.push(c.c);
        at += crate::chatwidth::char_w(c.c) as u16;
    }
    out
}

/// The cells inside `btn`'s span (caps excluded): the block and its label.
fn block_cells(row: &Row, btn: Btn) -> Vec<segment::Cell> {
    let span = &row.spans.iter().find(|(b, _)| *b == btn).unwrap().1;
    row.cells
        .iter()
        .filter(|(c, _)| (span.start + 1..span.end - 1).contains(c))
        .map(|(_, c)| *c)
        .collect()
}

#[test]
fn the_row_reads_run_then_discard_then_the_keys_with_plain_caps() {
    let _g = crate::app::theme_test_guard();
    let row = buttons(60, false, None, None).expect("fits");
    assert_eq!(
        text(&row),
        "\u{2590} \u{25b6} run \u{258c} \u{2590} \u{2717} discard \u{258c}  enter \u{00b7} esc"
    );
}

#[test]
fn the_nerd_font_row_wears_the_round_caps_and_the_play_icon() {
    let _g = crate::app::theme_test_guard();
    let row = buttons(60, true, None, None).expect("fits");
    let t = text(&row);
    assert!(t.starts_with('\u{e0b6}'), "left arc cap: {t:?}");
    assert!(
        t.contains("\u{f04b} run"),
        "nf-fa-play before the verb: {t:?}"
    );
    assert!(
        t.contains("\u{f00d} discard"),
        "nf-fa-times before discard: {t:?}"
    );
    assert!(
        t.contains("\u{e0b4}  enter"),
        "right arc cap then the hint: {t:?}"
    );
}

#[test]
fn spans_cover_their_badges_without_overlap_and_inside_the_pane() {
    let _g = crate::app::theme_test_guard();
    for cols in [24u16, 30, 40, 80] {
        let row = buttons(cols, false, None, None).expect("fits");
        let [(Btn::Run, run), (Btn::Discard, discard)] = row.spans.as_slice() else {
            panic!("two spans, run first: {:?}", row.spans);
        };
        assert!(
            run.end <= discard.start,
            "{cols}: run {run:?} overlaps {discard:?}"
        );
        assert!(
            discard.end <= cols,
            "{cols}: discard {discard:?} runs off the pane"
        );
        // Every cell of a badge sits inside its span, and nothing else does.
        let inside = |span: &Range<u16>, c: char| {
            row.cells
                .iter()
                .filter(|(col, _)| span.contains(col))
                .any(|(_, cell)| cell.c == c)
        };
        assert!(inside(run, '\u{25b6}') && !inside(run, '\u{2717}'));
        assert!(inside(discard, '\u{2717}') && !inside(discard, '\u{25b6}'));
        assert!(
            row.cells.iter().all(|(c, _)| *c < cols),
            "{cols}: a cell past the edge"
        );
    }
}

#[test]
fn the_hint_is_dropped_first_then_the_verbs_then_the_row() {
    let _g = crate::app::theme_test_guard();
    let at = |cols| text(&buttons(cols, false, None, None).expect("fits"));
    assert!(at(40).contains("enter"), "40 cols carries the hint");
    let narrow = at(30);
    assert!(
        !narrow.contains("enter"),
        "30 cols drops the hint: {narrow:?}"
    );
    assert!(
        narrow.contains("discard"),
        "but keeps the verbs: {narrow:?}"
    );
    let terse = at(14);
    assert!(
        !terse.contains("run"),
        "14 cols keeps the marks alone: {terse:?}"
    );
    assert!(terse.contains('\u{25b6}') && terse.contains('\u{2717}'));
    assert!(buttons(10, false, None, None).is_none(), "no room: no row");
}

#[test]
fn hovering_lifts_the_block_and_keeps_the_label_floored() {
    let _g = crate::app::theme_test_guard();
    let floor = crew_theme::contrast::text_floor();
    let rest = buttons(60, false, None, None).unwrap();
    let hover = buttons(60, false, Some(Btn::Run), None).unwrap();
    let (r, h) = (block_cells(&rest, Btn::Run), block_cells(&hover, Btn::Run));
    assert_ne!(r[0].bg, h[0].bg, "the hovered block changes colour");
    assert_eq!(
        block_cells(&rest, Btn::Discard)[0].bg,
        block_cells(&hover, Btn::Discard)[0].bg,
        "the other button is untouched"
    );
    for c in h.iter().filter(|c| c.c != ' ') {
        let ratio = crew_theme::contrast_ratio(c.fg, c.bg.unwrap());
        assert!(
            ratio >= floor,
            "hovered label at {ratio:.2} under the {floor} floor"
        );
    }
    // Spans do not move with the state: a hover cannot shift the hit target.
    assert_eq!(rest.spans, hover.spans);
}

#[test]
fn a_pressed_badge_inverts_and_still_reads() {
    let _g = crate::app::theme_test_guard();
    let rest = buttons(60, false, None, None).unwrap();
    let pressed = buttons(60, false, None, Some(Btn::Discard)).unwrap();
    let (r, p) = (
        block_cells(&rest, Btn::Discard),
        block_cells(&pressed, Btn::Discard),
    );
    assert_eq!(
        p[0].bg,
        Some(crew_theme::theme().ink),
        "block goes to the ink"
    );
    assert_ne!(p[0].bg, r[0].bg);
    let floor = crew_theme::contrast::text_floor();
    for c in p.iter().filter(|c| c.c != ' ') {
        assert!(crew_theme::contrast_ratio(c.fg, c.bg.unwrap()) >= floor);
    }
}

#[test]
fn the_row_is_budgeted_only_while_a_plan_is_pending() {
    let _g = crate::app::theme_test_guard();
    let mut p = crate::chat::tests::pane();
    assert_eq!(plan_rows(&p, 60), 0);
    p.plan_pending = true;
    assert_eq!(plan_rows(&p, 60), 1);
    assert_eq!(plan_rows(&p, 10), 0, "too narrow for the badges: no row");
}

#[test]
fn row_cells_paint_the_page_behind_the_caps_and_the_hint() {
    let _g = crate::app::theme_test_guard();
    let _plain = crate::glyphs::force(false);
    let mut p = crate::chat::tests::pane();
    p.plan_pending = true;
    let cells = row_cells(&p, 60, 7);
    let page = crew_theme::theme().page_bg;
    assert!(cells.iter().all(|c| c.row == 7));
    let cap = cells
        .iter()
        .find(|c| c.c == '\u{2590}')
        .expect("a left cap");
    assert_eq!(cap.bg, page, "a cap cell stands on the page");
    assert_eq!(
        cap.fg,
        crate::palette::accent(),
        "and is painted in the block's colour"
    );
    let hint = cells.iter().find(|c| c.c == 'e').expect("the hint");
    assert_eq!(hint.bg, page);
}
