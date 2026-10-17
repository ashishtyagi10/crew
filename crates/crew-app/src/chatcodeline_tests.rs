//! The code line: a row above the composer for as long as a turn runs, a
//! word decoding out of glyphs, a streak that moves only while work does.
use super::*;
use crate::chatcodeword::{LOCK_MS, SCRAMBLE_MS};

const COLS: u16 = 60;
const SINCE: u64 = 1_000_000;
const WORD: &str = "deciphering";

fn frame(last_work: u64) -> Frame {
    let (cols, row, since, tool, level) = (COLS, 5, SINCE, false, MotionLevel::Full);
    Frame {
        cols,
        row,
        since,
        last_work,
        tool,
        level,
        word: (WORD, SINCE),
    }
}

/// The row as text, one char a column.
fn text(cells: &[CellView], cols: u16) -> String {
    let mut row = vec![' '; cols as usize];
    for c in cells {
        row[c.col as usize] = c.c;
    }
    row.into_iter().collect()
}

/// Where the word's whole cycle is held, `ms` in.
fn hold(ms: u64) -> u64 {
    let n = WORD.chars().count() as u64;
    SINCE + SCRAMBLE_MS + n * LOCK_MS + ms
}

/// The bold cells past the word: the streaks' heads.
fn heads(cells: &[CellView]) -> Vec<u16> {
    let past = INSET + WORD.chars().count() as u16;
    let mut v: Vec<u16> = cells
        .iter()
        .filter(|c| c.bold && c.col > past)
        .map(|c| c.col)
        .collect();
    v.sort_unstable();
    v
}

#[test]
fn the_word_decodes_out_of_glyphs_then_holds() {
    let w = WORD;
    let early = cells_at(&frame(SINCE + 100), SINCE + 100);
    assert!(!text(&early, COLS).contains(w), "still glyphs at 100 ms");
    let now = hold(200);
    let held = text(&cells_at(&frame(now), now), COLS);
    assert_eq!(&held[1..1 + w.len()], w, "{held:?}");
}

#[test]
fn the_clock_ends_a_column_short_of_the_card() {
    let now = SINCE + 6_400;
    let row = text(&cells_at(&frame(now), now), COLS);
    assert!(row.trim_end().ends_with("6s"), "{row:?}");
    assert_eq!(row.trim_end().chars().count(), COLS as usize - 1);
}

#[test]
fn a_stalled_turn_holds_its_word_and_dims() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let dist = |a: (u8, u8, u8)| {
        let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
        d(a.0, t.ink.0) + d(a.1, t.ink.1) + d(a.2, t.ink.2)
    };
    let w = WORD;
    // Mid-scramble on the clock, five cycles in, and nothing for 9 s.
    let cycle = crate::chatcodeword::cycle(w.chars().count() as u64, |b| b);
    let now = SINCE + 5 * cycle + 100;
    let stalled = cells_at(&frame(now - STALL_MS - 1_000), now);
    assert_eq!(&text(&stalled, COLS)[1..1 + w.len()], w, "the word is held");
    // Against a line as quiet but not yet stalled, holding the same word.
    let calm = hold(0) + 5 * cycle;
    let calm = cells_at(&frame(calm - STALL_MS + 1_000), calm);
    let first = |cs: &[CellView]| cs.iter().find(|c| c.col == INSET).unwrap().fg;
    assert!(
        dist(first(&stalled)) > dist(first(&calm)),
        "and dimmer than a quiet one that has not stalled"
    );
}

#[test]
fn the_streak_runs_while_work_flows_and_crawls_when_stalled() {
    let walk = |stalled: bool| {
        let distinct: std::collections::BTreeSet<Vec<u16>> = (0..10)
            .map(|k| hold(100) + k * STEP_MS)
            .map(|now| {
                let last = if stalled { now - STALL_MS - 1 } else { now };
                heads(&cells_at(&frame(last), now))
            })
            .collect();
        distinct.len()
    };
    assert!(walk(false) >= 8, "lively: a cell a step");
    assert!(walk(true) <= 4, "stalled: a quarter of the pace");
}

#[test]
fn a_running_tool_adds_a_second_streak() {
    let most = |tool: bool| {
        (0..40)
            .map(|k| hold(k * STEP_MS))
            .map(|now| heads(&cells_at(&Frame { tool, ..frame(now) }, now)).len())
            .max()
            .unwrap()
    };
    assert_eq!((most(false), most(true)), (1, 2));
}

#[test]
fn every_cell_stays_on_its_row_inside_the_card() {
    for cols in [MIN_COLS, 30, COLS, 120] {
        for k in 0..60u64 {
            let now = SINCE + k * 137;
            let f = Frame {
                cols,
                tool: k % 2 == 0,
                ..frame(now - (k % 3) * STALL_MS)
            };
            let cells = cells_at(&f, now);
            let mut seen: Vec<u16> = cells.iter().map(|c| c.col).collect();
            assert!(cells
                .iter()
                .all(|c| c.row == f.row && c.col >= INSET && c.col < cols - 1));
            let n = seen.len();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(
                seen.len(),
                n,
                "two cells share a column at cols={cols} k={k}"
            );
        }
    }
    assert!(cells_at(
        &Frame {
            cols: MIN_COLS - 1,
            ..frame(SINCE)
        },
        SINCE
    )
    .is_empty());
}

#[test]
fn motion_off_is_one_still_sentence() {
    let off = |now: u64| {
        let f = Frame {
            level: MotionLevel::Off,
            ..frame(now)
        };
        text(&cells_at(&f, now), COLS)
    };
    let want = format!(" smith \u{b7} {} \u{b7} 6s", WORD);
    assert_eq!(off(SINCE + 6_200).trim_end(), want);
    assert_eq!(off(SINCE + 6_200), off(SINCE + 6_900), "nothing moves");
}

#[test]
fn work_lights_the_word_for_a_beat() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let now = hold(300);
    let word_ink = |last: u64| {
        let cells = cells_at(&frame(last), now);
        cells.iter().find(|c| c.col == INSET).unwrap().fg
    };
    assert_eq!(word_ink(now), t.ink, "an event this frame: the page's ink");
    assert_ne!(
        word_ink(now - 2_000),
        t.ink,
        "two quiet seconds later: muted"
    );
}

#[test]
fn a_new_phase_word_decodes_from_when_it_went_up_not_from_the_turns_start() {
    let up = SINCE + 10_000;
    let f = Frame {
        word: ("tracing", up),
        ..frame(up)
    };
    let n = "tracing".len() as u64;
    let scrambling = text(&cells_at(&f, up + 10), COLS);
    assert!(!scrambling.contains("tracing"), "{scrambling:?}");
    let held = up + SCRAMBLE_MS + n * LOCK_MS + 100;
    assert!(text(&cells_at(&f, held), COLS).contains("tracing"));
}
