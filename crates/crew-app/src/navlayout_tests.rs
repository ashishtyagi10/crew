use super::*;

/// A nav on a short window: the pane list is served first, and the LOG takes
/// what is left — down to nothing rather than pushing a pane row off.
#[test]
fn the_pane_list_is_served_before_the_log() {
    let fixed = fixed_rows(true);
    // Exactly enough for the fixed sections and a 4-pane block, no more.
    let rows = fixed + 1 + crate::crewpie::ROWS + 4;
    let l = layout(rows, true, 40, 4);
    assert_eq!(l.log_lines, 0, "no room for a LOG at all");
    assert_eq!(l.panes_top, fixed, "and the list starts right where it can");
}

/// The whole reason this module exists: a tall window used to leave a third of
/// the nav empty under a LOG stuck at five lines.
#[test]
fn a_tall_nav_spends_its_slack_on_the_log() {
    let short = layout(fixed_rows(true) + 12, true, 64, 1);
    let tall = layout(fixed_rows(true) + 40, true, 64, 1);
    assert!(
        tall.log_lines > short.log_lines,
        "{} vs {}",
        tall.log_lines,
        short.log_lines
    );
    assert_eq!(tall.log_lines, LOG_MAX, "up to the cap");
}

/// …but never past the cap, and never past what there is to show.
#[test]
fn the_log_stops_at_the_cap_and_at_the_entries_it_has() {
    let huge = layout(fixed_rows(false) + 400, false, 64, 0);
    assert_eq!(huge.log_lines, LOG_MAX);
    let sparse = layout(fixed_rows(false) + 400, false, 3, 0);
    assert_eq!(sparse.log_lines, 3, "three entries, three lines");
}

/// A LOG too small to be worth its own rule is dropped, and the pane list
/// closes the gap rather than sitting under an empty section.
#[test]
fn a_log_below_the_minimum_is_dropped_entirely() {
    let l = layout(fixed_rows(false) + 30, false, 1, 1);
    assert_eq!(l.log_lines, 0);
    assert_eq!(l.log_block(), 0);
    assert_eq!(l.panes_top, l.log_top, "no rows reserved for it");
}

/// Every section below the LOG moves with it — the invariant the four
/// hand-summed copies of these offsets used to have to keep by hand.
#[test]
fn panes_top_is_always_log_top_plus_the_log_block() {
    for rows in 0..80u16 {
        for git in [false, true] {
            for len in [0usize, 1, 5, 64] {
                for panes in [0usize, 1, 9] {
                    let l = layout(rows, git, len, panes);
                    assert_eq!(l.panes_top, l.log_top + l.log_block(), "{rows} {len}");
                    assert!(l.log_lines <= len, "never more lines than entries");
                }
            }
        }
    }
}

/// GIT is the one section above the LOG that comes and goes, and everything
/// below it moves by exactly its block when it does.
#[test]
fn a_git_repo_pushes_everything_below_it_down_one_block() {
    let rows = fixed_rows(true) + 30;
    let (no, yes) = (layout(rows, false, 64, 2), layout(rows, true, 64, 2));
    assert_eq!(yes.log_top, no.log_top + CARD_BLOCK);
}
