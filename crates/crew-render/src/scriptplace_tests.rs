use super::*;
use crate::celltext::build_pane_text;
use crate::celltext::FontParams;
use crate::scriptrun::tests::{cells, drawable, params};

/// `text` on row 1 of a 60-column pane: its snapped buffer, runs and areas.
fn laid(text: &str) -> (Buffer, Vec<RunSpec>, Vec<RunArea>) {
    let p = params();
    let mut fs = crate::embedfont::font_system();
    let cs = cells(text, 1);
    let (w, h) = (60.0 * p.cell_w, 2.0 * p.line_height);
    let (buf, specs) = build_pane_text(&mut fs, &cs, 60, 2, w, h, &p);
    let areas = place(&mut fs, &buf, &specs, &p);
    (buf, specs, areas)
}

#[test]
fn a_phrase_is_laid_whole_at_its_natural_size_with_even_gaps_on_the_rows_baseline() {
    if !drawable(&mut crate::embedfont::font_system(), "भारत") {
        return;
    }
    let p = params();
    let (buf, specs, areas) = laid("ok भारत ने आज अंतरिक्ष में नया उपग्रह भेजा");
    let latin = buf.layout_runs().find(|r| r.line_i == 1).expect("row 1");
    assert_eq!(
        areas[0].dx,
        3.0 * p.cell_w,
        "the phrase starts at its first cell"
    );
    let mut gaps = Vec::new();
    for (i, (s, a)) in specs.iter().zip(&areas).enumerate() {
        let run = a.buf.layout_runs().next().expect("shaped");
        assert!(
            (run.line_y + a.dy - latin.line_y).abs() <= 0.5,
            "{}: off the baseline",
            s.text
        );
        assert!(
            run.glyphs.iter().all(|g| g.font_size == p.font_size),
            "{} shrank",
            s.text
        );
        let drift = a.dx - f32::from(s.col) * p.cell_w;
        assert!(
            drift.abs() <= 2.0 * p.cell_w,
            "{} is {drift}px off its cells",
            s.text
        );
        if let Some(next) = areas.get(i + 1) {
            gaps.push(next.dx - (a.dx + width(&a.buf)));
        }
    }
    let (lo, hi) = gaps
        .iter()
        .fold((f32::MAX, f32::MIN), |(l, h), &g| (l.min(g), h.max(g)));
    assert!(hi - lo <= 1.5, "gaps differ: {gaps:?}");
    assert!(
        lo >= GAP_MIN * p.cell_w - 1.0 && hi <= GAP_MAX * p.cell_w + 1.0,
        "{gaps:?}"
    );
    let (s, a) = (specs.last().unwrap(), areas.last().unwrap());
    let end = f32::from(s.col) * p.cell_w + s.room(p.cell_w);
    assert!(
        a.dx + width(&a.buf) <= end + 1.0,
        "the last word stays in its room"
    );
}

#[test]
fn a_phrase_too_tight_to_lay_whole_puts_each_word_in_its_own_room() {
    if !drawable(&mut crate::embedfont::font_system(), "ভাষা") {
        return;
    }
    let p = params();
    // Bengali: each `ভাষা` is two cells and three of ink, and a letter
    // straight after the last leaves it no blanks to spill into.
    let (_, specs, areas) = laid("ভাষা ভাষা ভাষাx");
    for (s, a) in specs.iter().zip(&areas) {
        assert_eq!(a.dx, f32::from(s.col) * p.cell_w);
        assert!(width(&a.buf) <= s.room(p.cell_w) + 0.01, "{}", s.text);
    }
}

#[test]
fn a_run_wider_than_its_room_is_shaped_smaller_to_fit() {
    if !drawable(&mut crate::embedfont::font_system(), "क्ष्म") {
        return;
    }
    let p = params();
    let mut fs = crate::embedfont::font_system();
    let spec = RunSpec {
        row: 0,
        col: 0,
        cols: 1,
        text: "क्ष्म".into(),
        fg: (0, 0, 0),
        weight: 400,
        italic: false,
        light: false,
        gamma: 0,
        spare: 0,
    };
    let (buf, _) = shaped(&mut fs, &spec, &p, spec.room(p.cell_w));
    let w = width(&buf);
    assert!(w > 0.0 && w <= p.cell_w + 0.01, "{w} in {}", p.cell_w);
    let size = buf.layout_runs().next().unwrap().glyphs[0].font_size;
    assert!(size < p.font_size, "{size}");
    let (free, _) = shaped(&mut fs, &spec, &p, f32::INFINITY);
    assert!(width(&free) > p.cell_w, "no limit, no shrinking");
}

#[test]
fn bold_is_taken_from_the_family_that_draws_the_body() {
    if !drawable(&mut crate::embedfont::font_system(), "भारत") {
        return;
    }
    let p = FontParams {
        weight: 500,
        ..params()
    };
    let mut fs = crate::embedfont::font_system();
    let face = |fs: &mut FontSystem, weight: u16| {
        let spec = RunSpec {
            row: 0,
            col: 0,
            cols: 4,
            text: "भारत".into(),
            fg: (0, 0, 0),
            weight,
            italic: false,
            light: false,
            gamma: 0,
            spare: 0,
        };
        let (buf, _) = shaped(fs, &spec, &p, f32::INFINITY);
        let g = buf.layout_runs().next().unwrap().glyphs[0].clone();
        let f = fs.db().face(g.font_id).unwrap();
        (f.families[0].0.clone(), f.weight.0)
    };
    let (body, bold) = (face(&mut fs, 500), face(&mut fs, 700));
    assert_eq!(body.0, bold.0, "one typeface");
    assert!(bold.1 > body.1, "{body:?} {bold:?}");
}

#[test]
fn a_runs_rules_follow_its_ink_across_its_phrase_and_the_blanks_after() {
    let area = |col, cols, spare, dx, ink| RunArea {
        buf: Arc::new(Buffer::new_empty(glyphon::Metrics::new(14.0, 18.0))),
        dx,
        dy: 0.0,
        ink,
        row: 0,
        col,
        cols,
        spare,
    };
    // Two words of a phrase at 8px cells: cells 16..48 inked at 16..36, a
    // blank, cells 56..80 inked at 42..60, then two blanks.
    let warp = Warp::of(&[area(2, 4, 1, 16.0, 20.0), area(7, 3, 2, 42.0, 18.0)], 8.0);
    assert_eq!(
        warp.span(0, 16.0, 8.0),
        (16.0, 5.0),
        "a word's cell, scaled to its ink"
    );
    assert_eq!(
        warp.span(0, 48.0, 8.0),
        (36.0, 6.0),
        "the blank between, to the space between"
    );
    assert_eq!(
        warp.span(0, 80.0, 8.0),
        (60.0, 8.0),
        "a blank after, moved with the word"
    );
    assert_eq!(
        warp.span(0, 200.0, 8.0),
        (200.0, 8.0),
        "anything else stays"
    );
    assert_eq!(warp.span(1, 16.0, 8.0), (16.0, 8.0), "another row stays");
}
