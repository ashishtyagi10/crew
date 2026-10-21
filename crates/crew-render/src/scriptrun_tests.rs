use super::*;
use crate::celltext::{build_pane_text, cell_metrics, FontParams, CELL_H_RATIO};
use crew_theme::marks::Marks;
use unicode_width::UnicodeWidthChar;

pub(crate) fn params() -> FontParams {
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 400,
        smooth: 0,
        gamma: 0,
        dark: true,
        body: ((255, 255, 255), (0, 0, 0)),
    }
}

/// `s` as a widget places it: one cell per character with a width, the
/// width-0 marks after it riding on it.
pub(crate) fn cells(s: &str, row: u16) -> Vec<CellView> {
    let mut out: Vec<CellView> = Vec::new();
    let mut col = 0;
    for c in s.chars() {
        match (UnicodeWidthChar::width(c).unwrap_or(0), out.last_mut()) {
            (0, Some(prev)) => drop(prev.marks.push(c)),
            (w, _) => {
                out.push(CellView {
                    col,
                    row,
                    c,
                    fg: (220, 220, 220),
                    ..Default::default()
                });
                col += w.max(1) as u16;
            }
        }
    }
    out
}

/// Whether this machine has a face that draws `s`. A runner without the
/// script's fonts skips the shaping tests rather than measure notdef boxes.
pub(crate) fn drawable(fs: &mut glyphon::FontSystem, s: &str) -> bool {
    let mut b = Buffer::new(fs, glyphon::Metrics::new(14.0, 18.0));
    b.set_text(
        fs,
        s,
        &glyphon::Attrs::new(),
        glyphon::Shaping::Advanced,
        None,
    );
    let drawn = b
        .layout_runs()
        .all(|r| r.glyphs.iter().all(|g| g.glyph_id != 0));
    if !drawn {
        eprintln!("no face draws {s:?} here \u{2014} skipping (this is a skip, not a pass)");
    }
    drawn
}

#[test]
fn only_the_scripts_that_join_their_letters_take_the_run_path() {
    for c in ['भ', 'ा', 'ক', 'த', 'ગ', 'ภ', 'ລ', 'က', 'ក', '॥'] {
        assert!(joins(c), "{c}");
    }
    for c in ['a', 'é', '中', '─', ' ', '😀', 'ע', 'ب'] {
        assert!(!joins(c), "{c}");
    }
}

#[test]
fn a_run_is_its_cells_with_their_marks_and_a_style_change_splits_it() {
    let mut cs = cells("क्ष ok हिंदी", 0);
    let mut runs = Runs::default();
    for c in &cs {
        runs.take(c, 400, true, 0);
    }
    let got: Vec<(u16, u16, &str)> = runs
        .0
        .iter()
        .map(|r| (r.col, r.cols, r.text.as_str()))
        .collect();
    assert_eq!(got, [(0, 2, "क्ष"), (6, 4, "हिंदी")]);
    // A bold letter mid-word is a run of its own, in its own weight.
    let mut runs = Runs::default();
    cs[1].bold = true;
    for (i, c) in cs.iter().enumerate() {
        runs.take(c, if i == 1 { 700 } else { 400 }, true, 0);
    }
    assert_eq!(runs.0[0].text, "क्");
    assert_eq!((runs.0[1].text.as_str(), runs.0[1].weight), ("ष", 700));
}

#[test]
fn joining_cells_are_spaces_in_the_snapped_buffer_and_come_back_as_runs() {
    let p = params();
    let mut fs = crate::embedfont::font_system();
    let cs = cells("ok भारत क्ष", 0);
    let (buf, specs) = build_pane_text(&mut fs, &cs, 20, 1, 20.0 * p.cell_w, p.line_height, &p);
    let line = buf.layout_runs().next().expect("a line").text.to_string();
    assert_eq!(line.trim_end(), "ok", "{line:?}");
    assert_eq!(specs.len(), 2);
    assert_eq!(
        (specs[0].col, specs[0].cols, specs[0].text.as_str()),
        (3, 4, "भारत")
    );
    assert_eq!(
        (specs[1].col, specs[1].cols, specs[1].text.as_str()),
        (8, 2, "क्ष")
    );
}

#[test]
fn latin_marks_stay_in_the_snapped_buffer_on_their_letter() {
    let p = params();
    let mut fs = crate::embedfont::font_system();
    let cs = vec![
        CellView {
            col: 0,
            c: 'e',
            marks: Marks::of(['\u{301}']),
            ..Default::default()
        },
        CellView {
            col: 1,
            c: 'x',
            ..Default::default()
        },
    ];
    let (buf, specs) = build_pane_text(&mut fs, &cs, 2, 1, 2.0 * p.cell_w, p.line_height, &p);
    assert!(specs.is_empty());
    let run = buf.layout_runs().next().unwrap();
    assert_eq!(run.text, "e\u{301}x");
    let x = run
        .glyphs
        .iter()
        .find(|g| &run.text[g.start..g.end] == "x")
        .unwrap()
        .x;
    assert_eq!(x, p.cell_w, "the mark takes no column");
}

#[test]
fn a_word_spills_into_the_blanks_after_it_before_it_shrinks() {
    let mut runs = Runs::default();
    for c in cells("ভাষা  ok", 0) {
        match c.c {
            ' ' => runs.blank(c.row, c.col),
            _ => drop(runs.take(&c, 400, false, 0)),
        }
    }
    // `ভাষা` is two cells: unicode-width gives its vowel signs none.
    let s = &runs.0[0];
    assert_eq!((s.cols, s.spare), (2, 2));
    assert_eq!(s.room(8.0), 16.0 + 16.0 - 4.0, "half a cell kept clear");
    let lone = RunSpec {
        spare: 0,
        ..s.clone()
    };
    assert_eq!(lone.room(8.0), 16.0);
    let far = RunSpec {
        spare: 9,
        ..s.clone()
    };
    assert_eq!(
        far.room(8.0),
        16.0 + 16.0 - 4.0,
        "at most its own width again"
    );
}

#[test]
fn punctuation_straight_after_a_word_goes_on_its_run() {
    let mut runs = Runs::default();
    for c in cells("दिल्ली: भेजा, |भारत| (ok)", 0) {
        runs.take(&c, 400, false, 0);
    }
    let texts: Vec<&str> = runs.0.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(texts, ["दिल्ली:", "भेजा,", "भारत"], "not a column rule");
    assert_eq!(runs.0[0].cols, 6, "द ि ल ल ी and the colon");
}
