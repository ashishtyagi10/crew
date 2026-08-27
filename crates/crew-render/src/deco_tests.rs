use super::*;

const W: f32 = 10.0;
const H: f32 = 20.0;

fn ys(r: &[Rect]) -> Vec<f32> {
    r.iter().map(|q| q.1).collect()
}

/// Which pixel columns a set of rects paints, as integers.
fn painted(r: &[Rect]) -> Vec<i32> {
    let mut out = Vec::new();
    for (x, _, w, _) in r {
        let mut px = *x;
        while px < x + w {
            out.push(px as i32);
            px += 1.0;
        }
    }
    out
}

#[test]
fn an_undecorated_cell_draws_nothing() {
    assert!(rects(&Deco::NONE, 0.0, 0.0, W, H).is_empty());
}

#[test]
fn the_single_rule_sits_in_the_bottom_of_the_cell_and_spans_it() {
    let r = rects(&Deco::underline(DecoLine::Single), 0.0, 100.0, W, H);
    assert_eq!(r.len(), 1);
    let (x, y, w, h) = r[0];
    assert_eq!((x, w), (0.0, W));
    assert!(y > 100.0 + H * 0.6, "rule at {y} is not in the bottom");
    assert!(y + h <= 100.0 + H, "rule at {y}+{h} leaves the cell");
}

#[test]
fn the_double_rule_is_two_separated_lines_both_inside_the_cell() {
    let r = rects(&Deco::underline(DecoLine::Double), 0.0, 0.0, W, H);
    assert_eq!(r.len(), 2);
    let t = thickness(H);
    let gap = (r[1].1 - r[0].1).abs();
    assert!(gap >= 2.0 * t, "lines {gap}px apart are one thick line");
    assert!(r.iter().all(|q| q.1 + q.3 <= H && q.1 >= 0.0));
}

#[test]
fn the_dotted_rule_paints_about_half_the_cell_in_alternating_runs() {
    let r = rects(&Deco::underline(DecoLine::Dotted), 0.0, 0.0, W, H);
    let ink: f32 = r.iter().map(|q| q.2).sum();
    assert!(
        (ink - W / 2.0).abs() <= 1.0,
        "dotted painted {ink} of {W} columns"
    );
    assert!(r.len() >= 2, "one run is a solid rule, not a dotted one");
    assert!(ys(&r).windows(2).all(|w| w[0] == w[1]));
}

/// The pattern is a property of the pane, not of the cell: cell two continues
/// where cell one stopped. With `on + off = 5` and a cell 9 wide the two never
/// line up, so a phase taken from the cell's own left edge fails here.
#[test]
fn the_dashed_pattern_runs_unbroken_across_a_cell_boundary() {
    let w = 9.0;
    let d = Deco::underline(DecoLine::Dashed);
    let mut got = painted(&rects(&d, 0.0, 0.0, w, H));
    got.extend(painted(&rects(&d, w, 0.0, w, H)));
    let t = thickness(H);
    let period = 5.0 * t;
    let want: Vec<i32> = (0..(2.0 * w) as i32)
        .filter(|px| (*px as f32).rem_euclid(period) < 3.0 * t)
        .collect();
    assert_eq!(got, want);
}

/// Same argument for the squiggle: equal phases must give equal heights even
/// when they fall in different cells.
#[test]
fn the_squiggle_keeps_its_phase_across_a_cell_boundary() {
    let d = Deco::underline(DecoLine::Curly);
    let first = rects(&d, 0.0, 0.0, W, H);
    let second = rects(&d, W, 0.0, W, H);
    let period = (W * 0.66).round().max(4.0);
    for (i, q) in second.iter().enumerate() {
        let px = W + i as f32;
        let same_phase = px - period * (px / period).floor();
        let want = first
            .iter()
            .find(|f| (f.0 - same_phase).abs() < 0.001)
            .unwrap_or_else(|| panic!("no pixel at phase {same_phase}"));
        assert_eq!(q.1, want.1, "pixel {px} broke the wave");
    }
}

#[test]
fn the_squiggle_waves_above_and_below_the_rule_it_replaces() {
    let curly = rects(&Deco::underline(DecoLine::Curly), 0.0, 0.0, W, H);
    let flat = rects(&Deco::underline(DecoLine::Single), 0.0, 0.0, W, H)[0].1;
    let hi = ys(&curly).into_iter().fold(f32::MAX, f32::min);
    let lo = ys(&curly).into_iter().fold(f32::MIN, f32::max);
    assert!(
        hi < flat && lo > flat,
        "wave {hi}..{lo} never crosses {flat}"
    );
    assert!(lo + thickness(H) <= H, "the wave dips out of the cell");
}

#[test]
fn a_struck_underlined_cell_draws_both_rules() {
    let d = Deco {
        line: DecoLine::Single,
        strike: true,
        color: None,
    };
    let r = rects(&d, 0.0, 0.0, W, H);
    assert_eq!(r.len(), 2);
    let strike = r[1].1;
    assert!(
        (strike - (H - thickness(H)) / 2.0).abs() <= 1.0,
        "strike at {strike} is not through the middle of {H}"
    );
    assert!(strike < r[0].1, "the strike is below the underline");
}

#[test]
fn strikeout_alone_draws_only_the_strike() {
    let d = Deco {
        strike: true,
        ..Deco::NONE
    };
    assert_eq!(rects(&d, 0.0, 0.0, W, H).len(), 1);
}

#[test]
fn the_rule_takes_sgr58s_colour_when_there_is_one_and_the_text_colour_otherwise() {
    let fg = (10, 20, 30);
    assert_eq!(color(&Deco::NONE, fg), fg);
    let tinted = Deco {
        color: Some((200, 0, 0)),
        ..Deco::underline(DecoLine::Curly)
    };
    assert_eq!(color(&tinted, fg), (200, 0, 0));
}

#[test]
fn the_rule_thickens_with_the_cell_and_never_vanishes() {
    assert!(thickness(40.0) > thickness(14.0));
    assert!(thickness(1.0) >= 1.0);
}

fn mark(shape: CursorShape) -> CursorMark {
    CursorMark {
        shape,
        color: (255, 255, 255),
    }
}

#[test]
fn the_filled_block_draws_no_rule_because_it_is_drawn_by_inverting_the_cell() {
    for shape in [CursorShape::None, CursorShape::Block] {
        assert!(
            cursor_rects(&mark(shape), 0.0, 0.0, W, H).is_empty(),
            "{shape:?}"
        );
    }
    assert!(!mark(CursorShape::Block).is_rule());
    assert!(mark(CursorShape::Beam).is_rule());
}

/// A bar sits on the leading edge and is thin — a bar as wide as the cell is a
/// block, and one drawn a third of the way in points at the wrong character.
#[test]
fn the_bar_is_a_thin_full_height_rule_on_the_leading_edge() {
    let r = cursor_rects(&mark(CursorShape::Beam), 7.0, 3.0, W, H);
    assert_eq!(r.len(), 1);
    let (x, y, w, h) = r[0];
    assert_eq!((x, y, h), (7.0, 3.0, H));
    assert!(
        (2.0..W / 3.0).contains(&w),
        "a bar {w} wide in a cell {W} wide"
    );
}

#[test]
fn the_underline_cursor_spans_the_cell_at_its_foot_and_is_thicker_than_a_text_rule() {
    let r = cursor_rects(&mark(CursorShape::Underline), 0.0, 0.0, W, H);
    assert_eq!(r.len(), 1);
    let (x, y, w, h) = r[0];
    assert_eq!((x, w), (0.0, W));
    assert_eq!(y + h, H, "the cursor rule sits on the cell's foot");
    assert!(h > thickness(H), "it reads the same as an underlined word");
}

/// The point of the outline is the hole in it. Four edges, and nothing over
/// the glyph in the middle.
#[test]
fn the_outline_is_four_edges_around_an_empty_middle() {
    let r = cursor_rects(&mark(CursorShape::Hollow), 0.0, 0.0, W, H);
    assert_eq!(r.len(), 4);
    let covers = |px: f32, py: f32| {
        r.iter()
            .any(|(x, y, w, h)| px >= *x && px < x + w && py >= *y && py < y + h)
    };
    assert!(covers(0.5, 0.5), "the top-left corner is not drawn");
    assert!(
        covers(W - 0.5, H - 0.5),
        "the bottom-right corner is not drawn"
    );
    assert!(covers(W / 2.0, 0.5), "the top edge is not drawn");
    assert!(!covers(W / 2.0, H / 2.0), "the outline is filled in");
}
