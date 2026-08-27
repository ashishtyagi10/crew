use super::*;

fn info(branch: &str, changed: usize, ahead: usize, behind: usize) -> GitInfo {
    GitInfo {
        branch: branch.into(),
        changed,
        ahead,
        behind,
    }
}

fn text(segs: &[Seg]) -> String {
    segs.iter()
        .map(|(s, _)| s.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn a_wide_border_shows_the_whole_state() {
    let segs = fit(&info("main", 3, 2, 1), 40).unwrap();
    assert_eq!(text(&segs), "main \u{25cf}3 \u{2191}2 \u{2193}1");
}

/// Nothing to report is nothing to draw: a clean repo is its branch, with no
/// tick, no zeroes and no punctuation waiting to be read.
#[test]
fn a_clean_repo_is_just_the_branch() {
    assert_eq!(text(&fit(&info("main", 0, 0, 0), 40).unwrap()), "main");
}

/// The badge is on a border shared with the legend and the status glyphs, so
/// the only thing it must never do is overrun its budget. Swept across every
/// width a card can be, for four repo states.
#[test]
fn the_badge_never_exceeds_the_columns_it_was_given() {
    for state in [
        info("main", 0, 0, 0),
        info("feat/ui4-git-badge-on-the-card", 12, 3, 4),
        info("a", 1, 0, 0),
        info("release/2026-08-26-long-name", 0, 99, 99),
    ] {
        for budget in 0..=60usize {
            if let Some(segs) = fit(&state, budget) {
                let w = width(&segs);
                assert!(w <= budget, "{} took {w} of {budget}", state.branch);
            }
        }
    }
}

/// Detail is dropped in one order — behind, ahead, dirty count, branch — so a
/// card that grows never shows LESS than it did a column narrower.
#[test]
fn detail_only_ever_grows_with_the_budget() {
    let state = info("main", 3, 2, 1);
    let mut last = 0;
    for budget in 0..=40usize {
        let w = fit(&state, budget).map(|s| width(&s)).unwrap_or(0);
        assert!(w >= last, "budget {budget} showed less than {budget} - 1");
        last = w;
    }
}

/// The dirty count is the reason to look at the badge at all; it must outlive
/// both arrows as the card narrows.
#[test]
fn the_dirty_count_survives_longer_than_the_arrows() {
    let state = info("main", 7, 2, 3);
    let at = |b: usize| fit(&state, b).map(|s| text(&s)).unwrap_or_default();
    assert_eq!(at(11), "main \u{25cf}7 \u{2191}2");
    assert_eq!(at(8), "main \u{25cf}7");
    assert_eq!(at(4), "main");
}

/// A branch too long for the card truncates rather than disappearing — you
/// still learn you are not on the branch you thought.
#[test]
fn a_long_branch_truncates_with_an_ellipsis_until_it_cannot() {
    let state = info("feat/very-long-branch", 0, 0, 0);
    assert_eq!(text(&fit(&state, 6).unwrap()), "feat/\u{2026}");
    assert_eq!(text(&fit(&state, 4).unwrap()), "fea\u{2026}");
    assert_eq!(fit(&state, 3), None, "two letters and a dot say nothing");
    assert_eq!(fit(&state, 0), None);
}

/// Drawing is right-aligned: the badge ends where it was told to end, and
/// never reaches the column the legend claimed.
#[test]
fn the_drawn_badge_ends_at_rx_and_stops_short_of_the_legend() {
    let _g = crate::app::theme_test_guard();
    let mut v = Vec::new();
    let next = draw(&mut v, 40, 10, &info("main", 3, 0, 0));
    let cols: Vec<u16> = v.iter().map(|c| c.col).collect();
    assert_eq!(cols.iter().copied().max(), Some(40));
    assert!(cols.iter().copied().min().unwrap() > 10);
    assert!(v.iter().all(|c| c.row == 0), "the badge left the border");
    assert!(next < cols.iter().copied().min().unwrap());
}

/// No room means no badge — not a squeezed one, and not a panic.
#[test]
fn a_narrow_card_draws_no_badge_at_all() {
    let _g = crate::app::theme_test_guard();
    let mut v = Vec::new();
    assert_eq!(draw(&mut v, 12, 10, &info("main", 3, 0, 0)), 12);
    assert!(v.is_empty());
}

/// On a real card: the badge lands on the top border, to the right of the
/// legend, and a card with no git answer draws exactly what it drew before.
#[test]
fn the_card_carries_the_badge_without_disturbing_its_legend() {
    let _g = crate::app::theme_test_guard();
    let state = info("main", 4, 0, 0);
    let bar = |git| crate::panecard::Bar {
        index: Some(1),
        title: "zsh",
        focused: false,
        scroll: 0,
        total: 0,
        activity: false,
        bell: false,
        broadcast: false,
        min_btn: false,
        assemble_t: 1.0,
        focus_t: 1.0,
        git,
        ticks: &[],
        hits: &[],
        unread: 0,
        doc: false,
    };
    let plain = crate::panecard::pane_card(60, 10, &bar(None));
    let badged = crate::panecard::pane_card(60, 10, &bar(Some(&state)));
    let row0 = |v: &[crew_render::CellView]| -> String {
        let mut cs: Vec<(u16, char)> = v
            .iter()
            .filter(|c| c.row == 0)
            .map(|c| (c.col, c.c))
            .collect();
        cs.sort_by_key(|(c, _)| *c);
        cs.into_iter().map(|(_, c)| c).collect()
    };
    let (before, after) = (row0(&plain), row0(&badged));
    assert!(after.contains("main"), "no branch on the border: {after}");
    assert!(after.contains("\u{25cf}4"), "no dirty count: {after}");
    assert!(!before.contains("main"), "the plain card grew a badge");
    assert!(
        after.contains("1 zsh"),
        "the legend was pushed off: {after}"
    );
    assert_eq!(
        before.chars().count(),
        after.chars().count(),
        "row width moved"
    );
}
