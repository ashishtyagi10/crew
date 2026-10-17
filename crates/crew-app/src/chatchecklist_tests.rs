use crew_hive::tools::todowire::{Step, StepState};

use super::{tally, window, Row};

/// `d` done steps, then the active one, then `p` pending.
fn plan(d: usize, p: usize) -> Vec<Step> {
    let step = |i: usize, state| Step {
        text: format!("step {i}"),
        state,
    };
    let mut v: Vec<Step> = (0..d).map(|i| step(i, StepState::Done)).collect();
    v.push(step(d, StepState::Active));
    v.extend((0..p).map(|i| step(d + 1 + i, StepState::Pending)));
    v
}

#[test]
fn a_list_that_fits_is_drawn_whole() {
    let rows = window(&plan(1, 1), 5);
    assert_eq!(rows, [Row::Step(0), Row::Step(1), Row::Step(2)]);
}

#[test]
fn a_long_list_keeps_the_step_it_is_on_and_the_one_before() {
    // 4 done, active at 4, 5 pending: ten steps in five rows.
    let rows = window(&plan(4, 5), 5);
    assert_eq!(
        rows,
        [
            Row::Done(3),
            Row::Step(3),
            Row::Step(4),
            Row::Step(5),
            Row::More(4)
        ]
    );
}

#[test]
fn early_in_a_long_list_there_is_no_head_row() {
    let rows = window(&plan(0, 9), 5);
    assert_eq!(rows[0], Row::Step(0));
    assert_eq!(rows[4], Row::More(6));
    assert_eq!(rows.len(), 5);
}

#[test]
fn late_in_a_long_list_the_spare_rows_go_to_earlier_steps() {
    // Active is the last of ten: no tail, and the window reaches back to
    // fill all five rows rather than leaving two empty.
    let rows = window(&plan(9, 0), 5);
    assert_eq!(
        rows,
        [
            Row::Done(6),
            Row::Step(6),
            Row::Step(7),
            Row::Step(8),
            Row::Step(9)
        ]
    );
}

#[test]
fn a_skipped_step_above_the_window_is_not_called_done() {
    let mut steps = plan(5, 4);
    steps[1].state = StepState::Pending;
    assert_eq!(window(&steps, 5)[0], Row::Earlier(4));
}

#[test]
fn with_no_room_to_count_the_window_is_the_step_and_what_follows() {
    assert_eq!(window(&plan(6, 3), 2), [Row::Step(6), Row::Step(7)]);
    assert_eq!(window(&plan(6, 3), 1), [Row::Step(6)]);
    // At the end of the list the count fits beside the last step.
    assert_eq!(window(&plan(9, 0), 2), [Row::Done(9), Row::Step(9)]);
}

#[test]
fn every_window_shows_the_step_it_is_on_and_fills_its_rows() {
    for n in 1..14 {
        for at in 0..n {
            let steps = plan(at, n - 1 - at);
            for r in 1..=6 {
                let rows = window(&steps, r);
                assert_eq!(rows.len(), r.min(n), "n={n} at={at} r={r}");
                assert!(
                    rows.contains(&Row::Step(at)),
                    "n={n} at={at} r={r}: {rows:?}"
                );
            }
        }
    }
}

#[test]
fn the_tally_counts_done_over_all() {
    assert_eq!(tally(&plan(2, 2)), "2/5");
}
