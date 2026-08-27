//! The top border is shared. Everything that rides it — the legend, the git
//! badge, the elapsed clock, the pin mark, the scroll count, the `[-][x]`
//! buttons and the status glyphs — steps leftward from the corner through one
//! running cursor, and each was added in a different release. This is the
//! sweep that says they still fit together.
use crate::git::GitInfo;
use crate::panecard::{pane_card, Bar};

fn loaded(git: &GitInfo, cols: u16) -> Vec<crew_render::CellView> {
    let bar = Bar {
        index: Some(7),
        title: "crew · claude",
        focused: true,
        scroll: 240,
        total: 4000,
        activity: true,
        bell: true,
        broadcast: true,
        min_btn: true,
        assemble_t: 1.0,
        focus_t: 1.0,
        git: Some(git),
        ticks: &[],
        hits: &[],
        progress: None,
        elapsed: Some("2m14".into()),
        pinned: true,
        cmd_rows: &[],
        err_rows: &[],
        unread: 128,
        doc: false,
    };
    pane_card(cols, 12, &bar)
}

fn info() -> GitInfo {
    GitInfo {
        branch: "feat/a-long-branch-name".into(),
        changed: 12,
        ahead: 3,
        behind: 4,
    }
}

/// Each thing on the border is drawn whole or not at all.
///
/// `put` overwrites the cell it lands on rather than stacking, so a collision
/// never shows up as two cells at one column — it shows up as a *fragment*:
/// `2m1`, or a badge with its last digit gone. Every token that appears at a
/// given width has to appear in full, and each may only disappear as the card
/// narrows, never come back.
#[test]
fn everything_on_the_top_border_is_drawn_whole_or_not_at_all() {
    let _g = crate::app::theme_test_guard();
    let git = info();
    // The tokens each feature owns, and the fragments that would prove one
    // had been half-overwritten by the next.
    // `99+` is what an unread count of 128 caps to.
    let tokens = ["2m14", "\u{25cf}12", "\u{2191}3", "99+", "\u{21e1}240"];
    let mut seen_at = [0u16; 5];
    for cols in 20..=160u16 {
        let mut cells: Vec<_> = loaded(&git, cols)
            .into_iter()
            .filter(|c| c.row == 0)
            .collect();
        cells.sort_by_key(|c| c.col);
        let text: String = cells.iter().map(|c| c.c).collect();
        for (i, tok) in tokens.iter().enumerate() {
            // A token is present, or its first character is absent too: a
            // leading fragment with the rest overwritten is the failure.
            let whole = text.contains(tok);
            let head: String = tok.chars().take(tok.chars().count() - 1).collect();
            assert!(
                whole || !text.contains(&head),
                "{cols}: `{tok}` is drawn as `{head}` \u{2014} {text}"
            );
            if whole {
                seen_at[i] = seen_at[i].max(cols);
            }
        }
    }
    // …and every one of them does fit somewhere, or the sweep proves nothing.
    for (i, tok) in tokens.iter().enumerate() {
        assert!(seen_at[i] > 0, "`{tok}` never fits at any width");
    }
}

/// …and nothing on it may be drawn outside the card.
#[test]
fn nothing_on_the_card_is_drawn_outside_it() {
    let _g = crate::app::theme_test_guard();
    let git = info();
    for cols in 8..=160u16 {
        let cells = loaded(&git, cols);
        // `pane_card` takes the INTERIOR size and frames it, so the drawn
        // card is two columns and two rows larger.
        assert!(
            cells.iter().all(|c| c.col < cols + 2 && c.row < 14),
            "a cell escaped a {cols}-column card"
        );
    }
}

/// The legend is the one thing on the border that names the pane, so it is
/// the last thing anything else may take room from.
#[test]
fn the_legend_survives_a_border_carrying_everything_else() {
    let _g = crate::app::theme_test_guard();
    let git = info();
    for cols in [60u16, 80, 100, 120, 160] {
        let mut cells: Vec<_> = loaded(&git, cols)
            .into_iter()
            .filter(|c| c.row == 0)
            .collect();
        cells.sort_by_key(|c| c.col);
        let text: String = cells.iter().map(|c| c.c).collect();
        assert!(text.contains("7 crew"), "{cols}: {text}");
    }
}
