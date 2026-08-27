//! The top border is shared. Everything that rides it — the legend, the git
//! badge, the elapsed clock, the pin mark, the scroll count, the `[-][x]`
//! buttons and the status glyphs — steps leftward from the corner through one
//! running cursor, and each was added in a different release. This is the
//! sweep that says they still fit together — including the newest of them,
//! the name of the command you are scrolled back into, which is the only one
//! whose width is a whole phrase rather than a badge.
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
        at_cmd: Some("cargo build \u{2717}101"),
        fail_rows: &[],
        cmd_rows: &[],
        err_rows: &[],
        unread: 128,
        doc: false,
    };
    pane_card(cols, 12, &bar)
}

/// The card's top border as one string, blanks included.
fn border(git: &GitInfo, cols: u16) -> String {
    let mut cells: Vec<_> = loaded(git, cols)
        .into_iter()
        .filter(|c| c.row == 0)
        .collect();
    cells.sort_by_key(|c| c.col);
    cells.iter().map(|c| c.c).collect()
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
    let tokens = [
        "2m14",
        "\u{25cf}12",
        "\u{2191}3",
        "99+",
        "\u{21e1}240",
        "\u{2576} cargo build \u{2717}101",
    ];
    let mut seen_at = [0u16; 6];
    // The width each token first fit at, so "it never comes back" can be
    // asserted rather than merely claimed in a comment.
    let mut first_at = [None::<u16>; 6];
    for cols in 20..=160u16 {
        let text = border(&git, cols);
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
                first_at[i].get_or_insert(cols);
            } else if let Some(first) = first_at[i] {
                // The budget only ever grows with the card, so a token that
                // fit at a NARROWER width and does not fit here means
                // something else grew into its room — the bug this sweep
                // exists to catch, and the one its own doc comment claimed
                // without ever checking.
                panic!("{cols}: `{tok}` fit at {first} and is gone here \u{2014} {text}");
            }
        }
    }
    // …and every one of them does fit somewhere, or the sweep proves nothing.
    for (i, tok) in tokens.iter().enumerate() {
        assert!(seen_at[i] > 0, "`{tok}` never fits at any width");
    }
}

/// The tokens keep their order across the border. Each steps leftward from
/// the corner through one running cursor, so their left-to-right order is
/// fixed by the code that draws them; a token that has jumped its neighbour
/// is a cursor that was not advanced.
#[test]
fn the_border_tokens_keep_their_order() {
    let _g = crate::app::theme_test_guard();
    let git = info();
    // Drawn right to left — the scroll count first, from the corner, and the
    // branch last — so read left to right they run: branch, command name,
    // clock, pin, unread, status glyphs, scroll count.
    let order = [
        "\u{2191}3",
        "\u{2576} cargo build",
        "2m14",
        "99+",
        "\u{21e1}240",
    ];
    for cols in 20..=160u16 {
        let text = border(&git, cols);
        let mut last = 0usize;
        for tok in order {
            let Some(at) = text.find(tok) else { continue };
            assert!(
                at >= last,
                "{cols}: `{tok}` is left of the token before it \u{2014} {text}"
            );
            last = at;
        }
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

/// Nothing may sit flush against the pane's name.
///
/// Every other pair of neighbours on this border is separated by a cell of
/// frame, because the leftward cursor steps by two. The two tokens that take
/// "whatever is left" — the command name and the branch — were floored at the
/// legend's last column PLUS ONE, so on a card just wide enough for one of
/// them the border read `claude╶ cargo build…`: the pane's name and the
/// command run together into one word.
#[test]
fn nothing_is_drawn_flush_against_the_legend() {
    let _g = crate::app::theme_test_guard();
    let git = info();
    for cols in 20..=160u16 {
        let text = border(&git, cols);
        let Some(at) = text.find("claude") else {
            continue;
        };
        let after: Option<char> = text[at + "claude".len()..].chars().next();
        assert!(
            matches!(after, None | Some(' ') | Some('\u{2500}')),
            "{cols}: `{after:?}` is flush against the legend \u{2014} {text}"
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
        let text = border(&git, cols);
        assert!(text.contains("7 crew"), "{cols}: {text}");
    }
}
