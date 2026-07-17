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

/// Spawn `agent` on `task` (which also marks it Running and maps the agent) and
/// credit it `input`/`output` tokens — the only path that reaches
/// `token_totals`, since `TokenDelta` is attributed via the agent→task map.
fn spawn_tok(p: &mut ChatPane, agent: u64, task: u64, input: u32, output: u32) {
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: crew_hive::AgentId(agent),
        task: TaskId(task),
    });
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: crew_hive::AgentId(agent),
        input,
        output,
    });
}

fn line(p: &ChatPane, cols: u16, now_ms: u64) -> String {
    let cells = block_cells(p, cols, 10, now_ms);
    let mut v: Vec<_> = cells.iter().filter(|c| c.row == 10).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn no_swarm_no_rows() {
    let p = pane();
    assert_eq!(swarm_rows(&p, 40), 0);
    assert!(block_cells(&p, 80, 5, 0).is_empty());
}

#[test]
fn a_live_run_claims_exactly_one_row_whatever_the_plan_size() {
    // The block used to grow a row per task and cap at 8. It now says what
    // crew is doing, which is always one thing.
    assert_eq!(swarm_rows(&pane_with_swarm(1), 40), 1);
    assert_eq!(swarm_rows(&pane_with_swarm(5), 40), 1);
    assert_eq!(swarm_rows(&pane_with_swarm(20), 40), 1);
}

#[test]
fn the_line_names_the_running_task_and_counts_the_plan() {
    let mut p = pane_with_swarm(5);
    run(&mut p, 2);
    let l = line(&p, 80, 0);
    assert!(l.contains("task-2"), "{l}");
    // The counter now lives inside the Claude-style parenthetical.
    assert!(l.contains("(0/5)"), "{l}");
}

#[test]
fn only_the_running_task_is_named_not_the_whole_plan() {
    let mut p = pane_with_swarm(3);
    run(&mut p, 1);
    let l = line(&p, 80, 0);
    assert!(l.contains("task-1"), "{l}");
    assert!(!l.contains("task-0"), "{l}");
    assert!(!l.contains("task-2"), "{l}");
    // One row only.
    assert!(block_cells(&p, 80, 10, 0).iter().all(|c| c.row == 10));
}

#[test]
fn the_oldest_running_task_wins_and_the_rest_are_counted() {
    // Parallel agents: naming the newest would make the line flicker as
    // tasks start and stop, so the oldest holds it.
    let mut p = pane_with_swarm(4);
    run(&mut p, 0);
    std::thread::sleep(std::time::Duration::from_millis(2));
    run(&mut p, 1);
    std::thread::sleep(std::time::Duration::from_millis(2));
    run(&mut p, 2);
    let l = line(&p, 80, 0);
    assert!(l.contains("task-0"), "oldest should hold the line: {l}");
    assert!(l.contains("+2"), "two others run alongside it: {l}");
}

#[test]
fn a_lone_running_task_gets_no_plus_suffix() {
    let mut p = pane_with_swarm(3);
    run(&mut p, 0);
    let l = line(&p, 80, 0);
    assert!(!l.contains('+'), "{l}");
}

#[test]
fn nothing_running_shows_a_working_line_with_the_counter() {
    // The gap between the plan arriving (all Pending) and the first spawn.
    let p = pane_with_swarm(5);
    let l = line(&p, 80, 0);
    assert!(l.contains("Working"), "{l}");
    assert!(l.contains("(0/5)"), "{l}");
    assert!(!l.contains("task-"), "{l}");
}

/// A digit immediately followed by `s` — the elapsed column's shape. Probing
/// for this instead of a literal substring keeps these tests from depending
/// on word choice (`!l.contains('s')` breaks the moment `WORKING` gains an
/// 's') or on the machine being fast enough to still read "0s" (`l.contains
/// ("0s")` is one slow tick from a false failure).
fn has_elapsed_pattern(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    chars
        .windows(2)
        .any(|w| w[0].is_ascii_digit() && w[1] == 's')
}

#[test]
fn the_working_line_carries_no_elapsed() {
    // Elapsed derives from a running task's `started`; there isn't one.
    let p = pane_with_swarm(2);
    let l = line(&p, 80, 5_000);
    assert!(l.contains("Working"), "{l}");
    assert!(
        !has_elapsed_pattern(&l),
        "no elapsed without a running task: {l}"
    );
}

#[test]
fn running_task_with_nonzero_now_shows_elapsed() {
    let mut p = pane_with_swarm(2);
    run(&mut p, 0);
    let l = line(&p, 80, 5_000);
    assert!(has_elapsed_pattern(&l), "{l}");
}

#[test]
fn running_task_with_zero_now_shows_no_elapsed() {
    // now_ms == 0 is the test frame: deterministic, no elapsed.
    // (Don't assert `!l.contains('s')` here — the title "task-0" has one.)
    let mut p = pane_with_swarm(2);
    run(&mut p, 0);
    let l = line(&p, 80, 0);
    assert!(!l.contains("0s"), "{l}");
}

#[test]
fn the_counter_survives_a_pane_too_narrow_for_elapsed() {
    // Width rule: elapsed drops below ELAPSED_MIN_COLS, the counter never
    // does — it's the whole point of the line.
    let mut p = pane_with_swarm(5);
    run(&mut p, 0);
    let l = line(&p, ELAPSED_MIN_COLS - 1, 5_000);
    assert!(l.contains("0/5"), "{l}");
    assert!(!l.contains("0s"), "elapsed should have dropped: {l}");
}

#[test]
fn the_plus_suffix_survives_a_title_clamp() {
    // +N is the only signal that parallel work exists, so the title yields
    // columns to it rather than the reverse.
    let mut p = pane();
    let tasks = (0..3)
        .map(|i| TaskSpec {
            id: TaskId(i),
            title: "a-very-long-task-title-that-will-not-fit".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
            specialty: String::new(),
            expertise: String::new(),
        })
        .collect();
    p.absorb_hive_plan(tasks);
    run(&mut p, 0);
    run(&mut p, 1);
    let l = line(&p, 30, 0);
    assert!(l.contains("+1"), "{l}");
    assert!(l.contains("0/3"), "{l}");
}

/// Expands each cell to its full display-width extent (`col..col+char_w`)
/// and asserts none overlap another, and none extend past `cols`. A plain
/// `col` dedup — what this replaces — can't see a wide glyph straddling its
/// neighbour: it occupies `col` AND `col+1` while emitting a single
/// `CellView`, so two cells at adjacent `col`s can still be a real overlap.
fn assert_no_collisions(cells: &[CellView], cols: u16, ctx: &str) {
    let mut ranges: Vec<(u16, u16)> = cells
        .iter()
        .map(|c| (c.col, c.col + crate::chatwidth::char_w(c.c) as u16))
        .collect();
    ranges.sort_unstable();
    for w in ranges.windows(2) {
        assert!(
            w[0].1 <= w[1].0,
            "{ctx}: cells overlap at cols={cols}: {:?} vs {:?} (all: {:?})",
            w[0],
            w[1],
            ranges
        );
    }
    assert!(
        ranges.iter().all(|&(_, end)| end <= cols),
        "{ctx}: a cell escaped the pane at cols={cols}: {ranges:?}"
    );
}

fn cjk_plan(n: u64) -> Vec<TaskSpec> {
    (0..n)
        .map(|i| TaskSpec {
            id: TaskId(i),
            title: "研究研究研究".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
            specialty: String::new(),
            expertise: String::new(),
        })
        .collect()
}

#[test]
fn cells_never_collide_or_leave_the_pane() {
    // Covers narrow-to-wide panes (from 0, not just 8 — there's no minimum
    // pane-width guard upstream), ASCII and CJK titles, and with/without the
    // `+N` suffix. `reserve`/`next_start`-style arithmetic used to be able to
    // disagree; the fix must make overlap structurally impossible instead.
    let cases: [(&str, ChatPane); 4] = [
        ("ascii, no suffix", {
            let mut p = pane_with_swarm(5);
            run(&mut p, 0);
            p
        }),
        ("ascii, +N suffix", {
            let mut p = pane_with_swarm(5);
            run(&mut p, 0);
            run(&mut p, 1);
            p
        }),
        ("CJK, no suffix", {
            let mut p = pane();
            p.absorb_hive_plan(cjk_plan(5));
            run(&mut p, 0);
            p
        }),
        ("CJK, +N suffix", {
            let mut p = pane();
            p.absorb_hive_plan(cjk_plan(5));
            run(&mut p, 0);
            run(&mut p, 1);
            p
        }),
    ];
    for (label, p) in &cases {
        for cols in 0..=80u16 {
            let cells = block_cells(p, cols, 10, 5_000);
            assert_no_collisions(&cells, cols, label);
        }
    }
}

#[test]
fn wide_glyph_title_never_overruns_the_counter() {
    // Finding-1 regression: cols=8, a single-task counter "0/1" (len 3), and
    // a CJK title. `avail` computes to 0 here, and `fit_end`'s stall guard
    // used to force one glyph through regardless, straddling into the
    // counter column.
    let mut p = pane();
    p.absorb_hive_plan(cjk_plan(1));
    run(&mut p, 0);
    let cells = block_cells(&p, 8, 10, 5_000);
    assert_no_collisions(&cells, 8, "CJK title at cols=8");
}

#[test]
fn swarm_rows_and_block_cells_agree_across_widths() {
    // Budget and draw must agree: if the row isn't drawn, it isn't claimed.
    // Mirrors chatprog_tests::a_pane_too_narrow_for_a_legible_bar_drops_the_row,
    // which guards the identical invariant for the progress bar. This is the
    // Finding-2 guard: nothing previously asserted `swarm_rows(pane, cols)`
    // and `!block_cells(pane, cols, ..).is_empty()` stay in lockstep as cols
    // shrinks below the `line_fits` floor.
    let running = {
        let mut p = pane_with_swarm(5);
        run(&mut p, 0);
        p
    };
    let working = pane_with_swarm(5);
    for (label, p) in [("running task", &running), ("Working… line", &working)] {
        for cols in 0..=80u16 {
            assert_eq!(
                swarm_rows(p, cols) == 1,
                !block_cells(p, cols, 10, 5_000).is_empty(),
                "{label}: claimed row and drawn row disagree at cols={cols}"
            );
        }
    }
}

#[test]
fn spinner_frames_are_all_single_column() {
    // `PREFIX_END`'s "col is now exactly PREFIX_END" reasoning (see
    // `block_cells`) assumes the spinner glyph is one display column wide.
    // If a wide glyph ever landed in `update::SPINNER`, `col` would come out
    // as PREFIX_END + 1 and quietly invalidate the avail/title_limit
    // non-underflow floor. Cheap to assert directly rather than trust it.
    for &c in crate::update::SPINNER.iter() {
        assert_eq!(
            crate::chatwidth::char_w(c),
            1,
            "spinner frame {c:?} is not 1 display column wide"
        );
    }
}

#[test]
fn wide_glyph_titles_advance_by_display_width() {
    // Each CJK glyph occupies 2 display columns; the char after one must
    // land exactly 2 columns later, not 1 (a char-count advance would
    // overlap the glyph's second cell). This is a property about *where*
    // cells land, not just that they don't collide — a naive "advance by 1"
    // implementation could still emit distinct, in-pane `col`s and pass a
    // collision check while getting every position wrong.
    let mut p = pane();
    p.absorb_hive_plan(vec![TaskSpec {
        id: TaskId(0),
        title: "日本x".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
        specialty: String::new(),
        expertise: String::new(),
    }]);
    run(&mut p, 0);
    let cells = block_cells(&p, 80, 5, 0);
    let ja0 = cells.iter().find(|c| c.c == '日').unwrap();
    let ja1 = cells.iter().find(|c| c.c == '本').unwrap();
    let x = cells.iter().find(|c| c.c == 'x').unwrap();
    assert_eq!(ja1.col, ja0.col + 2);
    assert_eq!(x.col, ja1.col + 2);
}

// --- Claude-style status line: parenthetical, elapsed format, tokens ---

#[test]
fn elapsed_formats_minutes_past_sixty_seconds() {
    assert_eq!(fmt_elapsed_short(0), "0s");
    assert_eq!(fmt_elapsed_short(12), "12s");
    assert_eq!(fmt_elapsed_short(59), "59s");
    assert_eq!(fmt_elapsed_short(60), "1m 0s");
    assert_eq!(fmt_elapsed_short(252), "4m 12s");
}

#[test]
fn the_parenthetical_wraps_elapsed_count_and_parallel() {
    // Claude-style: `{title}… (elapsed · done/total · +N)`. Two tasks run, so
    // the oldest holds the line and the other shows as +1.
    let mut p = pane_with_swarm(5);
    run(&mut p, 0);
    run(&mut p, 1);
    let l = line(&p, 80, 5_000);
    assert!(l.contains('('), "has a parenthetical: {l}");
    assert!(has_elapsed_pattern(&l), "elapsed inside: {l}");
    assert!(l.contains("0/5"), "count inside: {l}");
    assert!(l.contains("+1"), "parallel inside: {l}");
    assert!(l.trim_end().ends_with(')'), "closes the paren: {l}");
    // Order within the paren: elapsed, then count, then parallel.
    let open = l.find('(').unwrap();
    let paren = &l[open..];
    let count_at = paren.find("0/5").unwrap();
    let plus_at = paren.find("+1").unwrap();
    assert!(count_at < plus_at, "count precedes parallel: {paren}");
}

#[test]
fn tokens_render_split_with_arrows_at_the_right_edge() {
    let mut p = pane_with_swarm(3);
    spawn_tok(&mut p, 7, 0, 1_200, 3_400);
    let cells = block_cells(&p, 80, 10, 5_000);
    let l: String = {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == 10).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert!(l.contains("\u{2191}1.2k"), "up arrow + input tokens: {l}");
    assert!(
        l.contains("\u{2193}3.4k"),
        "down arrow + output tokens: {l}"
    );
    // Right-aligned: the up-arrow sits in the right half, past the words.
    let up = cells.iter().find(|c| c.c == '\u{2191}').unwrap();
    assert!(up.col > 40, "tokens hug the right edge: col {}", up.col);
    let down = cells.iter().find(|c| c.c == '\u{2193}').unwrap();
    // The pair ends flush with the pane's right margin.
    assert!(
        down.col > up.col,
        "output follows input: {} {}",
        up.col,
        down.col
    );
}

#[test]
fn tokens_drop_before_the_counter_on_a_narrow_pane() {
    // Tokens are the first sacrifice under width pressure; the counter is the
    // last. A pane wide enough for the words but not the tokens keeps the
    // counter and drops the ↑↓ pair entirely.
    let mut p = pane_with_swarm(3);
    spawn_tok(&mut p, 7, 0, 1_200, 3_400);
    let l = line(&p, 22, 5_000);
    assert!(l.contains("0/3"), "counter survives: {l}");
    assert!(!l.contains('\u{2191}'), "tokens dropped: {l}");
}

#[test]
fn the_words_are_narrower_than_a_wide_pane() {
    // The whole point of the redesign: the bar mirrors these words, so they
    // must not span the pane. A short title on an 80-col pane leaves slack.
    let mut p = pane_with_swarm(3);
    run(&mut p, 0);
    let w = words_width(&p, 80, 5_000).unwrap();
    assert!(w < 80, "words claim {w} of 80 columns");
    // And `words_width` equals the block's left extent (the non-token cells).
    let cells = block_cells(&p, 80, 10, 5_000);
    let left_extent = cells
        .iter()
        .filter(|c| c.c != '\u{2191}' && c.c != '\u{2193}')
        .map(|c| c.col + crate::chatwidth::char_w(c.c) as u16 - 1)
        .max()
        .unwrap();
    assert_eq!(w, left_extent, "words_width tracks the drawn left text");
}
