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
fn busy_pane_draws_a_sweep_on_the_bottom_border() {
    let busy = Bar {
        busy: Some(0),
        ..bar(true)
    };
    let cells = pane_card(38, 10, &busy);
    // heavy rule glyphs ride the bottom border (row = interior + 1) when busy…
    let bottom = 10 + 1;
    assert!(cells.iter().any(|c| c.c == '━' && c.row == bottom));
    // …and never when idle.
    assert!(!pane_card(38, 10, &bar(true)).iter().any(|c| c.c == '━'));
}

#[test]
fn tiny_pane_yields_no_card() {
    // Interior so small the card can't be drawn → empty (degenerate tile).
    assert!(pane_card(1, 0, &bar(true)).is_empty());
}
