use super::*;

#[test]
fn empty_focused_shows_placeholder() {
    let bar = InputBar {
        focused: true,
        ..Default::default()
    };
    let cells = bar.cells(40, 3, None);
    // the placeholder hint renders in the faint placeholder colour
    assert!(cells.iter().any(|c| c.c == '/' && c.fg == PLACEHOLDER));
    // once there's text, the placeholder is gone
    let typed = InputBar {
        text: "ls".into(),
        focused: true,
        ..Default::default()
    };
    assert!(!typed.cells(40, 3, None).iter().any(|c| c.fg == PLACEHOLDER));
}

#[test]
fn cells_focused_shows_accent_prompt_and_text() {
    let bar = InputBar {
        text: "ls".into(),
        focused: true,
        ..Default::default()
    };
    let cells = bar.cells(40, 3, None);
    assert!(cells.iter().any(|c| c.c == '>'));
    assert!(cells.iter().any(|c| c.c == 'l'));
    assert!(cells.iter().any(|c| c.c == 's'));
    let prompt = cells.iter().find(|c| c.c == '>').unwrap();
    assert_eq!(prompt.fg, ACCENT);
    // a block cursor is shown while focused with no suggestion
    assert!(cells.iter().any(|c| c.c == '█'));
}

#[test]
fn cells_long_text_follows_cursor_tail() {
    let text = format!("S{}E", "x".repeat(80));
    let bar = InputBar {
        text,
        focused: true,
        ..Default::default()
    };
    let cells = bar.cells(20, 3, None);
    assert!(cells.iter().any(|c| c.c == 'E'));
    assert!(cells.iter().any(|c| c.c == '█'));
    assert!(!cells.iter().any(|c| c.c == 'S'));
}

#[test]
fn cells_shows_dim_ghost_suggestion() {
    let bar = InputBar {
        text: "/se".into(),
        focused: true,
        ..Default::default()
    };
    let cells = bar.cells(40, 3, None);
    assert!(cells.iter().any(|c| c.c == 't' && c.fg == DIM));
    assert!(!cells.iter().any(|c| c.c == '█'));
}

#[test]
fn cells_unfocused_has_no_cursor() {
    let bar = InputBar {
        text: "ls".into(),
        focused: false,
        ..Default::default()
    };
    assert!(!bar.cells(40, 3, None).iter().any(|c| c.c == '█'));
}

#[test]
fn cells_unfocused_prompt_is_dim() {
    let bar = InputBar {
        text: String::new(),
        focused: false,
        ..Default::default()
    };
    let prompt = bar
        .cells(40, 3, None)
        .into_iter()
        .find(|c| c.c == '>')
        .unwrap();
    assert_eq!(prompt.fg, DIM);
}

#[test]
fn history_up_down_recalls_entries() {
    let mut bar = InputBar {
        focused: true,
        history: vec!["one".into(), "two".into(), "three".into()],
        ..Default::default()
    };
    bar.history_prev(); // newest
    assert_eq!(bar.text, "three");
    bar.history_prev();
    assert_eq!(bar.text, "two");
    bar.history_next();
    assert_eq!(bar.text, "three");
    bar.history_next(); // past newest → clears
    assert_eq!(bar.text, "");
    assert_eq!(bar.hist_pos, None);
}

#[test]
fn broadcast_prompt_is_magenta() {
    let bar = InputBar {
        focused: true,
        broadcast: true,
        ..Default::default()
    };
    let cells = bar.cells(40, 3, None);
    assert!(cells.iter().any(|c| c.c == '»' && c.fg == BROADCAST));
}

#[test]
fn cells_show_cwd_legend_on_top_border() {
    let bar = InputBar {
        text: String::new(),
        focused: true,
        cwd: "/code/crew".into(),
        ..Default::default()
    };
    let cells = bar.cells(40, 3, None);
    // the cwd legend rides the top border (row 0) in the accent colour
    assert!(cells
        .iter()
        .any(|c| c.c == 'w' && c.row == 0 && c.fg == ACCENT));
    // the card has rounded corners and the prompt is on the interior row
    assert!(cells.iter().any(|c| c.c == '╭'));
    assert!(cells.iter().any(|c| c.c == '>' && c.row == 1));
}

#[test]
fn cells_show_status_on_bottom_border() {
    let bar = InputBar {
        focused: true,
        ..Default::default()
    };
    let cells = bar.cells(40, 3, Some("copied 4 lines"));
    // status characters appear on the bottom border row in the amber colour
    assert!(cells
        .iter()
        .any(|c| c.c == 'c' && c.row == 2 && c.fg == STATUS_FG));
    // without a status, the bottom row carries no amber text
    let plain = bar.cells(40, 3, None);
    assert!(!plain.iter().any(|c| c.fg == STATUS_FG));
}

#[test]
fn cells_tiny_returns_empty() {
    assert!(InputBar::default().cells(3, 3, None).is_empty());
    assert!(InputBar::default().cells(40, 0, None).is_empty());
}
