//! `msg_rows_budget` is the single source both `chatview::cells` and
//! `placed_lines` call for how many message rows the transcript gets, and it
//! must reserve the swarm block's row by the pane's *width* (`cols`), since
//! that's what `chatswarmview::swarm_rows`/`block_cells` key their agreement
//! on — not by `rows`, which happens to share the same `u16` type and so a
//! mixed-up argument compiles silently.
use super::*;
use crate::chat::ChatPane;
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

fn pane() -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

fn pane_with_swarm(n: u64) -> ChatPane {
    let mut p = pane();
    let tasks = (0..n)
        .map(|i| TaskSpec {
            id: TaskId(i),
            title: format!("task-{i}"),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
            specialty: String::new(),
            expertise: String::new(),
        })
        .collect();
    p.absorb_hive_plan(tasks);
    p
}

fn run(p: &mut ChatPane, id: u64) {
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(id),
        state: TaskState::Running,
    });
}

#[test]
fn msg_rows_budget_reserves_the_swarm_row_by_width_not_height() {
    // Finding-1 regression: `msg_rows_budget` used to pass `rows` into
    // `swarm_rows`'s `cols` parameter. Pick cols/rows far enough apart that
    // the mix-up is observable: cols=80 comfortably fits the live line
    // (PREFIX_END + "0/5".len() == 6), rows=5 does not if it leaked into the
    // width check.
    let mut p = pane_with_swarm(5);
    run(&mut p, 0);
    let (cols, rows) = (80u16, 5u16);
    assert_ne!(cols, rows, "fixture must distinguish width from height");

    let by_width = crate::chatswarmview::swarm_rows(&p, cols);
    let by_height = crate::chatswarmview::swarm_rows(&p, rows);
    assert_eq!(by_width, 1, "fixture must actually claim a row at cols=80");
    assert_ne!(
        by_width, by_height,
        "fixture must make swarm_rows(pane, cols) and swarm_rows(pane, rows) \
         disagree, or this test can't distinguish the two parameters"
    );

    let top = p.status_rows(cols, rows);
    let bottom = crate::chatinput::composer_rows(&p.input, cols, rows);
    let queued = crate::chatqueue::queued_rows(&p);
    let prog = crate::chatprog::progress_rows(&p, cols);
    let want = rows.saturating_sub(top + bottom + by_width + queued + prog);

    assert_eq!(
        msg_rows_budget(&p, cols, rows),
        want,
        "msg_rows_budget must reserve swarm_rows(pane, cols), not swarm_rows(pane, rows)"
    );
}

/// A pane too short for every surface must DROP the ones that don't fit, not
/// pile them onto one row.
///
/// The anchors each floored at `.max(top)`, so on a squeezed pane `bar_row`,
/// `indicator_row` and `block_start` all collapsed to the SAME row and
/// last-write-wins hid whichever drew first. The grants decide once, in
/// priority order, so the sum can never exceed the space between the header
/// and the composer.
#[test]
fn a_short_pane_drops_surfaces_instead_of_piling_them() {
    let mut p = pane_with_swarm(4);
    run(&mut p, 0);
    p.queued.push_back("later".into());

    for rows in 0..=14u16 {
        for cols in [20u16, 40, 80] {
            let g = grants(&p, cols, rows);
            // The header and composer are chrome and are never dropped, so on
            // a pane too small even for those they can exceed `rows` — that is
            // outside what grants arbitrates. What grants DOES guarantee: the
            // droppable surfaces plus the transcript never exceed the space
            // between them.
            let body = rows.saturating_sub(g.top).saturating_sub(g.bottom);
            let claimed = g.swarm + g.queued + g.prog + g.msg;
            assert!(
                claimed <= body,
                "grants claim {claimed} of {body} body rows at cols={cols} rows={rows}"
            );
            // Priority: the bar is the first to go — the status line carries
            // the done/total counter the bar's label used to.
            if g.prog == 1 {
                assert_eq!(g.swarm, 1, "bar kept while the status line was dropped");
            }
            if g.queued == 1 {
                assert_eq!(
                    g.swarm, 1,
                    "indicator kept while the status line was dropped"
                );
            }
        }
    }
}

/// Every granted surface gets a row of its own.
#[test]
fn granted_surfaces_never_share_a_row() {
    let mut p = pane_with_swarm(4);
    run(&mut p, 0);
    p.queued.push_back("later".into());

    for rows in 0..=14u16 {
        for cols in [20u16, 40, 80] {
            let g = grants(&p, cols, rows);
            let mut used: Vec<u16> = Vec::new();
            let bar = rows.saturating_sub(g.bottom + g.prog);
            let ind = rows.saturating_sub(g.bottom + g.prog + g.queued);
            let blk = rows.saturating_sub(g.bottom + g.prog + g.queued + g.swarm);
            if g.prog == 1 {
                used.push(bar);
            }
            if g.queued == 1 {
                used.push(ind);
            }
            if g.swarm == 1 {
                used.push(blk);
            }
            let n = used.len();
            used.sort_unstable();
            used.dedup();
            assert_eq!(
                n,
                used.len(),
                "two surfaces share a row at cols={cols} rows={rows}"
            );
            // …and none of them lands on the header or the composer.
            for r in &used {
                assert!(
                    *r >= g.top && *r < rows.saturating_sub(g.bottom),
                    "surface at row {r} escaped the body at cols={cols} rows={rows} (top={}, bottom={})",
                    g.top,
                    g.bottom
                );
            }
        }
    }
}
