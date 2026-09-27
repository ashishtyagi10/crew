use super::*;
use crate::chatlayout::Message;

fn pane() -> ChatPane {
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    let long = "the atlas grows on the first frame after a theme switch, and the \
                ice-pixel canvas, and the gantt's now-rule. The second is the one that \
                matters, because the first only happens once and the third is cosmetic.";
    p.messages = (0..16)
        .map(|i| Message {
            sender: if i % 2 == 0 {
                "user".into()
            } else {
                "scout".into()
            },
            text: long.into(),
            ts: String::new(),
            meta: String::new(),
            usage: None,
            expanded: false,
        })
        .collect();
    p.input = "/d".into();
    crate::chatpalette::after_edit(&mut p.palette, "/d", None, Vec::new);
    p
}

#[test]
fn the_rows_a_popup_stands_on_are_blank_beside_it() {
    let _g = crate::app::theme_test_guard();
    let p = pane();
    // Wide enough that the palette card (its descriptions set its width)
    // leaves transcript beside it.
    let (cols, rows) = (180u16, 30u16);
    let (top, bottom, from) = band(&p, cols, rows).expect("the palette is open");
    // The band is where the pop-up actually stands: `above_composer` on the
    // card rect (content plus the frame's cell each side), one cell out.
    let (cw, ch) = (8.0, 16.0);
    let r = crate::layout::Rect {
        x: 0.0,
        y: 0.0,
        w: f32::from(cols + 2) * cw,
        h: f32::from(rows + 2) * ch,
    };
    let card = p.popup(cols + 2).unwrap();
    let y = crate::popupplace::above_composer(&p, r, cw, ch, f32::from(card.rows) * ch);
    assert_eq!((y / ch) as u16, top + 1, "card top, in rect rows");
    let beside = |cells: &[CellView]| {
        cells
            .iter()
            .filter(|c| c.row >= top && c.row < bottom && c.col >= from && c.c != ' ')
            .count()
    };
    // Unfocused, nothing is drawn over the pane and nothing is cleared…
    let (plain, _) = art(&p, false, crew_term::GridSize { cols, rows }, 2.0);
    assert!(
        beside(&plain) > 0,
        "the transcript runs beside the card's rows"
    );
    // …focused, the pop-up stands there and its rows are its own.
    let (shown, _) = art(&p, true, crew_term::GridSize { cols, rows }, 2.0);
    assert_eq!(beside(&shown), 0);
    // Rows above the band and the composer below it are untouched.
    let row = |cells: &[CellView], r: u16| cells.iter().filter(|c| c.row == r).count();
    assert_eq!(
        row(&plain, top.saturating_sub(1)),
        row(&shown, top.saturating_sub(1))
    );
    assert_eq!(
        row(&plain, bottom),
        row(&shown, bottom),
        "the composer keeps its row"
    );
}

#[test]
fn with_nothing_open_there_is_no_band() {
    let mut p = pane();
    p.palette = None;
    p.input.clear();
    assert!(band(&p, 100, 30).is_none());
}
