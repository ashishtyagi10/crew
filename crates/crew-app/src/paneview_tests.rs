use super::*;

fn bar(focused: bool) -> Bar<'static> {
    Bar {
        index: Some(2),
        title: "shell",
        focused,
        scroll: 37,
        activity: true,
        bell: true,
        broadcast: false,
        busy: None,
        min_btn: false,
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
fn status_glyphs_ride_the_top_border() {
    let cells = pane_card(38, 10, &bar(true));
    let on_top =
        |ch: char, fg: (u8, u8, u8)| cells.iter().any(|c| c.c == ch && c.row == 0 && c.fg == fg);
    assert!(on_top('⇡', crew_theme::theme().status_fg)); // scrollback `⇡37`
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
fn busy_pane_rains_in_the_bottom_right_corner() {
    let busy = Bar {
        busy: Some(0),
        ..bar(true)
    };
    // Card 40×12: the rain patch is w=10 h=3, right/bottom-aligned inside the
    // border → cols 29..=38, rows 8..=10 (all interior, clear of the ring).
    let in_corner = |cells: &[CellView]| {
        cells
            .iter()
            .any(|c| (29..=38).contains(&c.col) && (8..=10).contains(&c.row) && c.c != ' ')
    };
    assert!(
        in_corner(&pane_card(38, 10, &busy)),
        "busy pane rains visible glyphs in the bottom-right interior corner"
    );
    assert!(
        !in_corner(&pane_card(38, 10, &bar(true))),
        "idle pane leaves the corner clear — no in-pane rain"
    );
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
