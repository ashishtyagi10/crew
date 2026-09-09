//! On main the counter's digits were always muted: `3/7` became `4/7` with
//! nothing marking the frame it happened.
use super::*;
use crate::motion::{set_level, MotionLevel};
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;
use crew_render::CellView;

const COLS: u16 = 40;

fn pane_with_swarm(n: u64) -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
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

fn settle(p: &mut ChatPane, id: u64, state: TaskState) {
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(id),
        state,
    });
}

/// The status line's cells for `count`'s digits, in order.
fn count_fgs(p: &ChatPane, now: u64, count: &str) -> Vec<Color> {
    let cells: Vec<CellView> = crate::chatswarmview::block_cells(p, COLS, 3, now);
    let mut row: Vec<&CellView> = cells.iter().filter(|c| c.row == 3).collect();
    row.sort_by_key(|c| c.col);
    let text: String = row.iter().map(|c| c.c).collect();
    let at = text
        .rfind(count)
        .unwrap_or_else(|| panic!("{count:?} in {text:?}"));
    let at = text[..at].chars().count();
    row[at..at + count.len()].iter().map(|c| c.fg).collect()
}

/// The changed digit's ink is the page ink on the frame the count changes
/// and the muted words again by +FLASH_MS; the unchanged total never moves.
#[test]
fn a_settled_task_flashes_the_changed_digit_to_ink_then_back_to_muted() {
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Full);
    let th = crew_theme::theme();
    let (ink, muted) = (th.ink, th.text_muted);
    let mut p = pane_with_swarm(7);
    let t = 5_000;
    assert_eq!(
        count_fgs(&p, t, "0/7"),
        vec![muted; 3],
        "first sight: quiet"
    );
    settle(&mut p, 0, TaskState::Done);
    let at0 = count_fgs(&p, t, "1/7");
    assert_eq!(at0[0], ink, "the changed digit is the ink at +0");
    assert_eq!(&at0[1..], &[muted, muted], "the slash and total stay put");
    let mid = count_fgs(&p, t + FLASH_MS / 2, "1/7")[0];
    assert!(mid != ink && mid != muted, "easing back: {mid:?}");
    assert_eq!(count_fgs(&p, t + FLASH_MS, "1/7")[0], muted, "settled");
    assert!(!p.swarm.as_ref().unwrap().flash.live(t + FLASH_MS));
    // A failure settles too, and flashes the same way.
    settle(&mut p, 1, TaskState::Failed);
    assert_eq!(count_fgs(&p, t + 1_000, "2/7")[0], ink);
}

#[test]
fn off_never_flashes() {
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Off);
    let muted = crew_theme::theme().text_muted;
    let mut p = pane_with_swarm(3);
    count_fgs(&p, 1_000, "0/3");
    settle(&mut p, 0, TaskState::Done);
    assert_eq!(count_fgs(&p, 1_000, "1/3"), vec![muted; 3]);
    set_level(MotionLevel::Full);
}

#[test]
fn only_the_digits_that_changed_are_marked() {
    let _g = crate::app::motion_test_guard();
    set_level(MotionLevel::Full);
    let f = Flash::default();
    f.observe(9, 0);
    f.observe(10, 0);
    assert_eq!(f.changed(10), vec![true, true], "a digit grew: all of it");
    f.observe(11, 0);
    assert_eq!(f.changed(11), vec![false, true], "only the ones digit");
    f.observe(11, 100);
    assert!(f.mix(100) < 1.0, "a same-count observe does not restart");
    assert_eq!(f.mix(FLASH_MS), 0.0);
}
