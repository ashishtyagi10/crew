use super::*;

fn bar(focused: bool) -> Bar<'static> {
    Bar {
        index: Some(2),
        title: "shell",
        focused,
        scroll: 37,
        total: 0,
        activity: true,
        bell: true,
        broadcast: false,
        min_btn: false,
        focus_t: 1.0,
        assemble_t: 1.0,
        git: None,
        ticks: &[],
        hits: &[],
        progress: None,
        unread: 0,
        doc: false,
    }
}

#[test]
fn card_has_rounded_border_and_legend() {
    let cells = pane_card(38, 10, &bar(true));
    let has = |ch: char| cells.iter().any(|c| c.c == ch);
    // fieldset frame, not a filled title bar
    assert!(has('╭') && has('╮') && has('╰') && has('╯'));
    // legend on the top border: index then title
    assert!(cells.iter().any(|c| c.c == '2' && c.row == 0));
    assert!(cells.iter().any(|c| c.c == 's' && c.row == 0)); // "shell"
}

#[test]
fn legend_wears_the_pane_signature_hue() {
    // Focused: the title glyph on the top border takes the title-derived hue
    // (same hash the roster uses), so a pane and its roster row match.
    let hue = crate::chatroster::agent_color("shell");
    assert!(
        pane_card(38, 10, &bar(true))
            .iter()
            .any(|c| c.c == 's' && c.row == 0 && c.fg == hue),
        "focused legend should be the pane's signature hue"
    );
    // Unfocused: the same hue, dimmed toward legend_off (still identifiable).
    let dim = crate::anim::lerp_rgb(hue, crew_theme::theme().legend_off, 0.55);
    assert!(
        pane_card(38, 10, &bar(false))
            .iter()
            .any(|c| c.c == 's' && c.row == 0 && c.fg == dim),
        "unfocused legend recedes to a dimmed hue"
    );
}

#[test]
fn status_glyphs_ride_the_top_border() {
    let cells = pane_card(38, 10, &bar(true));
    let on_top =
        |ch: char, fg: (u8, u8, u8)| cells.iter().any(|c| c.c == ch && c.row == 0 && c.fg == fg);
    // `⇡37` — the scrollback count. Its COLOUR is the buffer position (see
    // `panescroll::position_fg`, and the test there holding it identical to the
    // thumb's), not a fixed role, so this only asks that it is drawn at all.
    assert!(cells.iter().any(|c| c.c == '⇡' && c.row == 0));
    assert!(on_top('●', crew_theme::theme().activity));
    assert!(on_top('!', crew_theme::theme().bell));
}

#[test]
fn broadcast_marker_shown_only_when_set() {
    let b = Bar {
        broadcast: true,
        ..bar(true)
    };
    assert!(pane_card(38, 10, &b)
        .iter()
        .any(|c| c.c == '»' && c.fg == crew_theme::theme().broadcast));
    assert!(!pane_card(38, 10, &bar(true)).iter().any(|c| c.c == '»'));
}

#[test]
fn no_scroll_indicator_at_bottom() {
    let b = Bar {
        scroll: 0,
        total: 0,
        activity: false,
        bell: false,
        ..bar(true)
    };
    assert!(!pane_card(38, 10, &b).iter().any(|c| c.c == '⇡'));
}

#[test]
fn border_colour_differs_by_focus() {
    let corner = |foc| {
        pane_card(38, 10, &bar(foc))
            .into_iter()
            .find(|c| c.c == '╭')
            .map(|c| c.fg)
            .unwrap()
    };
    assert_ne!(corner(true), corner(false));
}

#[test]
fn focused_legend_is_bold_unfocused_is_not() {
    let bold_legend = |foc| {
        pane_card(38, 10, &bar(foc))
            .into_iter()
            .any(|c| c.c == 's' && c.row == 0 && c.bold)
    };
    assert!(bold_legend(true), "focused legend should be bold");
    assert!(!bold_legend(false), "unfocused legend stays regular");
}

#[test]
fn tiny_pane_yields_no_card() {
    // Interior so small the card can't be drawn → empty (degenerate tile).
    assert!(pane_card(1, 0, &bar(true)).is_empty());
}

#[test]
fn border_buttons_draw_minus_then_x_and_shift_status_glyphs() {
    let b = Bar {
        min_btn: true,
        ..bar(true)
    };
    let cells = pane_card(38, 10, &b);
    // The buttons: [-] at cols 32..=34, [x] at cols 35..=37 (cols = 38 + 2 = 40)
    let at = |col: u16| cells.iter().find(|c| c.row == 0 && c.col == col).unwrap().c;
    assert_eq!(at(32), '[');
    assert_eq!(at(33), '-');
    assert_eq!(at(34), ']');
    assert_eq!(at(35), '[');
    assert_eq!(at(36), 'x');
    assert_eq!(at(37), ']');
    // Status glyphs still render, stepping further left of the buttons.
    let scroll_col = cells
        .iter()
        .find(|c| c.c == '⇡' && c.row == 0)
        .map(|c| c.col)
        .unwrap();
    assert!(scroll_col < 32, "scroll indicator left of the buttons");
}

#[test]
fn border_buttons_absent_when_disabled_on_a_wide_card() {
    // min_btn: false on a card well above BTNS_COLS (13) draws neither
    // button — the pair is gated on min_btn, not just on width.
    assert!(!pane_card(38, 10, &bar(true))
        .iter()
        .any(|c| (c.c == '-' || c.c == 'x') && c.row == 0));
}

#[test]
fn border_buttons_absent_when_narrow() {
    let b = Bar {
        min_btn: true,
        ..bar(true)
    };
    // A card narrower than 13 cells (11 interior) has no room for the button pair.
    let cells = pane_card(9, 10, &b);
    assert!(
        !cells.is_empty(),
        "the card drew nothing, so absence proves nothing"
    );
    assert!(
        !cells
            .iter()
            .any(|c| (c.c == '-' || c.c == 'x') && c.row == 0),
        "no buttons at 11 card cols"
    );
}

#[test]
fn close_rect_covers_the_corner_button_and_min_rect_sits_left_of_it() {
    use crate::layout::Rect;
    let r = Rect {
        x: 0.0,
        y: 0.0,
        w: 300.0,
        h: 100.0,
    };
    let close = close_btn_rect(r, 10.0, 20.0).unwrap();
    let min = min_btn_rect(r, 10.0, 20.0).unwrap();
    // cw=10, ch=20, w=300 → interior cols = 30-2 = 28, card cols = 30
    // [x] at cols 25..=27 (off=5), [-] at cols 22..=24 (off=8)
    assert_eq!(close.w, 30.0); // 3 cells * 10
    assert_eq!(min.w, 30.0); // 3 cells * 10
                             // [x] takes the corner slot; [-] sits directly left of it.
    assert_eq!(close.x, 250.0); // col 25 = 25*10 = 250
    assert_eq!(min.x, 220.0); // col 22 = 22*10 = 220
    assert!(close.x > min.x);
}

#[test]
fn buttons_rect_none_when_too_narrow() {
    use crate::layout::Rect;
    // A card narrower than 13 cells (BTNS_COLS) has no room → None.
    let narrow = Rect {
        x: 0.0,
        y: 0.0,
        w: 110.0, // 11 card cols * 10 cw
        h: 300.0,
    };
    assert!(min_btn_rect(narrow, 10.0, 20.0).is_none());
    assert!(close_btn_rect(narrow, 10.0, 20.0).is_none());
}

// --- focus brackets ---------------------------------------------------------

use crate::palette::accent;

/// Count border cells drawn in the accent — the focus brackets. Row 0 is
/// excluded: a focused legend is bold too, and a title whose signature hue
/// happened to equal the accent would otherwise be counted as a bracket.
fn bracket_cells(cells: &[crew_render::CellView]) -> usize {
    cells
        .iter()
        .filter(|c| c.row > 0 && c.fg == accent() && c.bold)
        .count()
}

/// The brackets are the focus announcement: an unfocused card must not carry
/// any, or every pane reads as active.
#[test]
fn only_the_focused_card_gets_brackets() {
    let bar = |focused| Bar {
        index: None,
        title: "t",
        focused,
        scroll: 0,
        total: 0,
        activity: false,
        bell: false,
        broadcast: false,
        min_btn: false,
        focus_t: 1.0,
        assemble_t: 1.0,
        git: None,
        ticks: &[],
        hits: &[],
        progress: None,
        unread: 0,
        doc: false,
    };
    assert_eq!(
        bracket_cells(&crate::panecard::pane_card(40, 20, &bar(false))),
        0
    );
    assert!(bracket_cells(&crate::panecard::pane_card(40, 20, &bar(true))) > 0);
}

/// They grow: mid-animation there is strictly less bracket than at rest. This
/// is what makes the mark read as arriving rather than blinking on.
#[test]
fn brackets_grow_with_progress() {
    let at = |t: f32| {
        bracket_cells(&crate::panecard::pane_card(
            40,
            20,
            &Bar {
                index: None,
                title: "t",
                focused: true,
                scroll: 0,
                total: 0,
                activity: false,
                bell: false,
                broadcast: false,
                min_btn: false,
                focus_t: t,
                assemble_t: 1.0,
                git: None,
                ticks: &[],
                hits: &[],
                progress: None,
                unread: 0,
                doc: false,
            },
        ))
    };
    let (start, mid, end) = (at(0.0), at(0.5), at(1.0));
    assert_eq!(start, 0, "nothing drawn before the animation begins");
    assert!(mid > 0 && mid < end, "0 < {mid} < {end}");
}

/// A two-row strip thumbnail has no room for a bracket, and lighting its whole
/// side would read as a focused-everywhere frame.
#[test]
fn tiny_cards_get_no_brackets() {
    let cells = crate::panecard::pane_card(
        10,
        1,
        &Bar {
            index: None,
            title: "t",
            focused: true,
            scroll: 0,
            total: 0,
            activity: false,
            bell: false,
            broadcast: false,
            min_btn: false,
            focus_t: 1.0,
            assemble_t: 1.0,
            git: None,
            ticks: &[],
            hits: &[],
            progress: None,
            unread: 0,
            doc: false,
        },
    );
    assert_eq!(bracket_cells(&cells), 0);
}

/// Brackets never touch row 0: the legend and the `[-][x]` buttons live there,
/// and decoration must not overwrite information.
#[test]
fn brackets_leave_the_legend_row_alone() {
    let cells = crate::panecard::pane_card(
        40,
        20,
        &Bar {
            index: Some(2),
            title: "shell",
            focused: true,
            scroll: 0,
            total: 0,
            activity: false,
            bell: false,
            broadcast: false,
            min_btn: true,
            focus_t: 1.0,
            assemble_t: 1.0,
            git: None,
            ticks: &[],
            hits: &[],
            progress: None,
            unread: 0,
            doc: false,
        },
    );
    assert!(
        !cells
            .iter()
            .any(|c| c.row == 0 && c.fg == accent() && c.bold),
        "a bracket landed on the legend row"
    );
}

// --- assemble ---------------------------------------------------------------

fn card_at(assemble_t: f32) -> Vec<crew_render::CellView> {
    crate::panecard::pane_card(
        40,
        20,
        &Bar {
            index: None,
            title: "build",
            focused: false,
            scroll: 0,
            total: 0,
            activity: false,
            bell: false,
            broadcast: false,
            min_btn: false,
            focus_t: 0.0,
            assemble_t,
            git: None,
            ticks: &[],
            hits: &[],
            progress: None,
            unread: 0,
            doc: false,
        },
    )
}

/// The frame draws itself in: strictly more of it exists as the animation runs,
/// and the finished card is the same one crew has always drawn.
#[test]
fn the_card_assembles_monotonically() {
    let counts: Vec<usize> = [0.0, 0.25, 0.5, 0.75, 1.0]
        .into_iter()
        .map(|t| card_at(t).len())
        .collect();
    for w in counts.windows(2) {
        assert!(w[0] <= w[1], "frame shrank mid-assemble: {counts:?}");
    }
    assert!(
        counts[0] < counts[4],
        "nothing was hidden at t=0: {counts:?}"
    );
}

/// It grows *out of the corners*, which is what reads as drawn rather than
/// faded: early on, the corner cell exists and the middle of the top edge
/// does not.
#[test]
fn assembly_starts_at_the_corners() {
    let early = card_at(0.15);
    let has = |col: u16, row: u16| early.iter().any(|c| c.col == col && c.row == row);
    assert!(has(0, 0), "the top-left corner should be drawn first");
    assert!(
        !has(21, 0),
        "the middle of the top edge should not exist yet"
    );
}

/// A pane you cannot name is worse than one that simply appeared, so the legend
/// — which rides the top border beside the left corner — survives from the
/// first frames.
#[test]
fn the_legend_survives_assembly() {
    let early = card_at(0.2);
    let top: String = {
        let mut cells: Vec<_> = early.iter().filter(|c| c.row == 0).collect();
        cells.sort_by_key(|c| c.col);
        cells.iter().map(|c| c.c).collect()
    };
    assert!(
        top.contains("build"),
        "legend lost during assembly: {top:?}"
    );
}

// --- busy scan --------------------------------------------------------------

/// The scan is the "this pane is working" signal on the glass. An idle pane
/// must not carry one: a surface that sweeps forever would repaint forever.
#[test]
fn only_a_working_pane_carries_a_scan() {
    let _g = crate::app::motion_test_guard();
    use crate::pane::{Pane, PaneContent};
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let idle = Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Far(crate::farpane::FarPane::new(std::env::temp_dir())),
        grid: crew_term::GridSize { cols: 40, rows: 12 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 200.0,
        },
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
    };
    assert!(!crate::paneview::pane_busy(&idle), "fixture must be idle");
    let scenes = crate::paneview::build_scenes(
        &[idle],
        Some(0),
        false,
        None,
        None,
        1.0,
        10.0,
        16.0,
        &Default::default(),
    );
    let card = scenes.iter().find(|s| s.glass).expect("a card");
    assert!(card.scan < 0.0, "an idle card must not sweep");
}
