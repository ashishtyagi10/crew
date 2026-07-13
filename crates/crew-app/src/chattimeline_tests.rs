use super::timeline_block;

fn item(title: &str, span: Option<(u64, u64)>) -> (String, Option<(u64, u64)>) {
    (title.into(), span)
}

#[test]
fn two_overlapping_tasks_render_exact_bars() {
    // research 0..3200, merge 3000..12400 over a 12400ms run (BAR_W = 20):
    // research: floor(0)=0 .. ceil(3200*20/12400)=6 → 6 blocks, 14 dots
    // merge: floor(3000*20/12400)=4 .. 20 → 4 dots, 16 blocks
    let block = timeline_block(&[
        item("research", Some((0, 3_200))),
        item("merge", Some((3_000, 12_400))),
    ])
    .unwrap();
    let lines: Vec<&str> = block.lines().collect();
    assert_eq!(lines[0], "```");
    assert_eq!(lines[1], "timeline \u{00b7} 12.4s");
    assert_eq!(lines[2], "research  \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{b7}\u{b7}\u{b7}\u{b7}\u{b7}\u{b7}\u{b7}\u{b7}\u{b7}\u{b7}\u{b7}\u{b7}\u{b7}\u{b7}");
    assert_eq!(lines[3], "merge     \u{b7}\u{b7}\u{b7}\u{b7}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}");
    assert_eq!(lines[4], "```");
}

#[test]
fn fewer_than_two_timed_spans_yield_none() {
    assert!(timeline_block(&[]).is_none());
    assert!(timeline_block(&[item("solo", Some((0, 5_000)))]).is_none());
    // A second task that never started doesn't count toward the minimum.
    assert!(timeline_block(&[item("a", Some((0, 5_000))), item("b", None)]).is_none());
}

#[test]
fn zero_length_run_yields_none() {
    assert!(timeline_block(&[item("a", Some((0, 0))), item("b", Some((0, 0)))]).is_none());
}

#[test]
fn never_started_tasks_are_omitted_from_the_rows() {
    let block = timeline_block(&[
        item("a", Some((0, 1_000))),
        item("skipped", None),
        item("b", Some((500, 2_000))),
    ])
    .unwrap();
    assert!(!block.contains("skipped"));
    assert!(block.contains('a'));
    assert!(block.contains('b'));
}

#[test]
fn brief_task_still_gets_one_filled_cell() {
    // 10ms of a 10s run floors AND ceils to cell 0..1 — never an empty bar.
    let block =
        timeline_block(&[item("blip", Some((0, 10))), item("long", Some((0, 10_000)))]).unwrap();
    let blip = block.lines().find(|l| l.starts_with("blip")).unwrap();
    assert_eq!(blip.matches('\u{2588}').count(), 1);
}

#[test]
fn late_start_near_the_edge_stays_in_the_bar() {
    // start offset ≈ total: floor would land on cell 20 (past the bar);
    // clamped to 19 with the minimum 1-cell fill.
    let block = timeline_block(&[
        item("tail", Some((9_999, 10_000))),
        item("run", Some((0, 10_000))),
    ])
    .unwrap();
    let tail = block.lines().find(|l| l.starts_with("tail")).unwrap();
    let bar: String = tail.chars().skip_while(|c| *c != ' ').collect();
    assert_eq!(bar.matches('\u{2588}').count(), 1);
    assert!(tail.ends_with('\u{2588}'), "{tail}");
}

#[test]
fn cjk_titles_clip_by_display_width_and_pad_align() {
    // 8 CJK chars = 16 display columns → clipped to 14 columns (7 chars).
    let block = timeline_block(&[
        item("研究研究研究研究", Some((0, 1_000))),
        item("ok", Some((0, 2_000))),
    ])
    .unwrap();
    let lines: Vec<&str> = block.lines().collect();
    let cjk = lines.iter().find(|l| l.starts_with('研')).unwrap();
    let ok = lines.iter().find(|l| l.starts_with("ok")).unwrap();
    assert_eq!(cjk.chars().filter(|c| "研究".contains(*c)).count(), 7);
    // Both bars start at the same display column.
    let bar_col = |l: &str| -> usize {
        let cut = l.find(['\u{2588}', '\u{b7}']).unwrap();
        crate::chatwidth::str_w(&l[..cut])
    };
    assert_eq!(bar_col(cjk), bar_col(ok));
}
