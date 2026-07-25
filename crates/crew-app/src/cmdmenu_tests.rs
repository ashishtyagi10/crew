use super::*;

#[test]
fn card_legend_is_the_given_title() {
    let matches = crate::suggest::menu_items("/s");
    let cells = menu_card("files", &matches, 0, 40, menu_rows(matches.len()));
    // The legend on the top border spells the title.
    let row0: String = {
        let mut cs: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
        cs.sort_by_key(|c| c.col);
        cs.iter().map(|c| c.c).collect()
    };
    assert!(row0.contains("files"));
}

#[test]
fn card_has_fieldset_border_legend_and_command_text() {
    let matches = crate::suggest::menu_items("/s");
    assert!(matches.len() >= 2); // /settings, /shell
    let cells = menu_card("commands", &matches, 0, 40, menu_rows(matches.len()));
    assert!(cells.iter().any(|c| c.c == '╭')); // fieldset corner
    assert!(cells.iter().any(|c| c.c == 'c')); // "commands" legend / text
    assert!(cells.iter().any(|c| c.c == 's')); // command text present
    assert!(cells.iter().any(|c| c.c == '›')); // selection marker present
}

#[test]
fn card_bg_uniform_no_highlight_bar() {
    let matches = crate::suggest::menu_items("/s");
    let cells = menu_card("commands", &matches, 0, 40, menu_rows(matches.len()));
    // No selection bar that could wash out text: every cell background is
    // uniform (the theme page_bg), so the description stays legible on any row.
    let bg = crew_theme::theme().page_bg;
    assert!(
        cells.iter().all(|c| c.bg == bg),
        "menu background must be uniform (no highlight bar)"
    );
}

#[test]
fn selected_row_is_bold_and_marked() {
    let matches = crate::suggest::menu_items("/"); // every command
    let cells = menu_card("commands", &matches, 0, 40, menu_rows(matches.len()));
    // Selected row is interior row 0 → card row 1: marked by `›`, and its
    // glyphs are bold (the only visual cue, never an obscuring background).
    assert!(cells.iter().any(|c| c.c == '›' && c.row == 1));
    assert!(cells.iter().any(|c| c.row == 1 && c.bold));
    // A non-selected row (card row 2) is not bold.
    assert!(cells.iter().filter(|c| c.row == 2).all(|c| !c.bold));
}

#[test]
fn empty_matches_render_nothing() {
    assert!(menu_card("commands", &[], 0, 40, 5).is_empty());
    assert!(menu_cells(&[], 0, 40, 5).is_empty());
}

#[test]
fn menu_rows_caps_long_lists() {
    assert_eq!(menu_rows(3), 5); // short list: exact fit
    assert_eq!(menu_rows(50), MAX_ROWS as u16 + 2); // long list: capped
}

#[test]
fn long_list_scrolls_to_selection() {
    let all = crate::suggest::menu_items("/"); // every command
    assert!(all.len() > MAX_ROWS, "need a list longer than the cap");
    let rows = menu_rows(all.len());
    assert_eq!(rows as usize, MAX_ROWS + 2); // height is capped
                                             // selecting the last command still renders it (the list scrolled): the
                                             // selection marker is drawn within the capped popup.
    let cells = menu_cells(&all, all.len() - 1, 40, rows);
    assert!(cells.iter().any(|c| c.c == '›'));
}

#[test]
fn header_rows_are_dim_and_unmarked() {
    let items = vec![
        crate::suggest::MenuItem {
            label: "anthropic".into(),
            desc: String::new(),
            fill: String::new(),
            submit: false,
            header: true,
        },
        crate::suggest::MenuItem {
            label: "Claude Sonnet 5".into(),
            desc: "claude-sonnet-5".into(),
            fill: "claude-sonnet-5".into(),
            submit: true,
            header: false,
        },
    ];
    // Selection sits on the model row (interior row 1 → card row 2).
    let cells = menu_card("models", &items, 1, 40, menu_rows(items.len()));
    assert!(cells.iter().any(|c| c.c == '\u{203a}' && c.row == 2));
    // The header row (card row 1) carries no selection marker...
    assert!(cells
        .iter()
        .filter(|c| c.row == 1)
        .all(|c| c.c != '\u{203a}'));
    // ...and is bold despite not being selected — that's the section styling.
    // (Excludes the card's own side border, which is drawn separately by
    // `titled_card` and is never bold regardless of row content.)
    assert!(cells
        .iter()
        .filter(|c| c.row == 1 && !c.c.is_whitespace() && c.c != '│')
        .all(|c| c.bold));
}
