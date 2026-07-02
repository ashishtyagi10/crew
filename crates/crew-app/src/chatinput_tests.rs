use super::*;

fn agents(names: &[&str]) -> Vec<AgentInfo> {
    names
        .iter()
        .map(|n| AgentInfo {
            name: (*n).into(),
            role: String::new(),
            model: String::new(),
        })
        .collect()
}

fn row_text(cells: &[CellView], row: u16) -> String {
    // Bucket by column with last-write-wins, as the renderer's grid does, and
    // preserve gaps so left/right alignment is visible.
    let mut line: Vec<char> = Vec::new();
    for c in cells.iter().filter(|c| c.row == row) {
        let col = c.col as usize;
        if line.len() <= col {
            line.resize(col + 1, ' ');
        }
        line[col] = c.c;
    }
    line.into_iter().collect::<String>().trim_end().to_string()
}

#[test]
fn tall_pane_gets_a_bordered_card() {
    let cells = composer_cells("hi", &agents(&["planner", "coder"]), 80, 10);
    // Top border (row 7): rounded corners with the agent chips as the legend.
    let top = row_text(&cells, 7);
    assert!(top.starts_with('\u{256d}'), "top: {top}"); // ╭
    assert!(top.ends_with('\u{256e}'), "top: {top}"); // ╮
    assert!(
        top.contains("@planner") && top.contains("@coder"),
        "top: {top}"
    );
    // Interior (row 8): side borders around the prompt.
    let mid = row_text(&cells, 8);
    assert!(mid.starts_with("\u{2502} \u{276f} hi"), "mid: {mid}"); // │ ❯ hi
    assert!(mid.ends_with('\u{2502}'), "mid: {mid}"); // │
                                                      // Bottom border (row 9): key hints ride the border, right-aligned.
    let bot = row_text(&cells, 9);
    assert!(bot.starts_with('\u{2570}'), "bot: {bot}"); // ╰
    assert!(bot.ends_with('\u{256f}'), "bot: {bot}"); // ╯
    assert!(bot.contains("Enter send \u{00b7} Esc close"), "bot: {bot}");
}

#[test]
fn short_pane_gets_prompt_only() {
    let cells = composer_cells("hi", &agents(&["planner"]), 60, 5);
    assert!(cells.iter().all(|c| c.row == 4));
    assert!(row_text(&cells, 4).starts_with("\u{276f} hi"));
}

#[test]
fn valid_mention_is_highlighted_in_agent_colour() {
    let a = agents(&["coder"]);
    let cells = composer_cells("@coder fix", &a, 60, 10);
    let ink = crew_theme::theme().ink;
    let at = |col: u16| cells.iter().find(|c| c.row == 8 && c.col == col).unwrap();
    // Card interior: `│ ❯ @coder fix` — the mention starts at col 4.
    assert_ne!(at(4).fg, ink, "@ of the mention takes the agent colour");
    assert!(at(4).bold && at(9).bold, "mention renders bold");
    assert_eq!(at(11).fg, ink, "text after the mention stays ink");
}

#[test]
fn unknown_mention_stays_plain() {
    let cells = composer_cells("@ghost hi", &agents(&["coder"]), 60, 10);
    let ink = crew_theme::theme().ink;
    assert!(cells
        .iter()
        .filter(|c| c.row == 8 && c.col >= 4 && c.c != '\u{258f}' && c.c != '\u{2502}')
        .all(|c| c.fg == ink));
}

#[test]
fn caret_follows_the_input() {
    let cells = composer_cells("ab", &[], 60, 10);
    let caret = cells.iter().find(|c| c.c == '\u{258f}').unwrap();
    assert_eq!((caret.col, caret.row), (6, 8));
}

#[test]
fn everything_clips_to_width() {
    let cells = composer_cells(
        "a very long input line that overflows",
        &agents(&["planner", "coder", "reviewer"]),
        12,
        10,
    );
    assert!(cells.iter().all(|c| c.col < 12));
}
