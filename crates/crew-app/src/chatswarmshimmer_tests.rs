//! The live status line's title shimmer: only the running task's title
//! sweeps; the counter, the placeholder and Motion Off stay flat muted.
use super::*;
use crate::chat::ChatPane;
use crate::motion::{set_level, MotionLevel};
use crate::shimmer::Color;
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

fn planned(title: &str) -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.absorb_hive_plan(vec![TaskSpec {
        id: TaskId(0),
        title: title.into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
        specialty: String::new(),
        expertise: String::new(),
    }]);
    p
}

fn running(title: &str) -> ChatPane {
    let mut p = planned(title);
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Running,
    });
    p
}

/// The line's cells sorted by column, and its text.
fn line(p: &ChatPane, now_ms: u64) -> (Vec<CellView>, String) {
    let mut v: Vec<CellView> = block_cells(p, 60, 10, now_ms)
        .into_iter()
        .filter(|c| c.row == 10)
        .collect();
    v.sort_by_key(|c| c.col);
    let text = v.iter().map(|c| c.c).collect();
    (v, text)
}

/// The colours under `word`, in order.
fn fgs(cells: &[CellView], text: &str, word: &str) -> Vec<Color> {
    let at = text
        .find(word)
        .unwrap_or_else(|| panic!("{word:?} not in {text:?}"));
    let start = text[..at].chars().count();
    cells[start..start + word.chars().count()]
        .iter()
        .map(|c| c.fg)
        .collect()
}

#[test]
fn the_running_title_shimmers_and_the_counter_does_not() {
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Full);
    let p = running("write the tests");
    let (cells, text) = line(&p, 700);
    let title = fgs(&cells, &text, "write the tests");
    assert!(
        title.iter().any(|c| *c != title[0]),
        "the title must carry the sweep: {title:?}"
    );
    let muted = crew_theme::theme().text_muted;
    let counter = fgs(&cells, &text, "0/1");
    assert!(
        counter.iter().all(|c| *c == muted),
        "counters stay muted: {counter:?}"
    );
}

#[test]
fn the_title_is_flat_muted_at_off() {
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Off);
    let p = running("write the tests");
    let muted = crew_theme::theme().text_muted;
    for now in [0, 700, 1_400] {
        let (cells, text) = line(&p, now);
        let title = fgs(&cells, &text, "write the tests");
        assert!(title.iter().all(|c| *c == muted), "now={now}: {title:?}");
    }
}

#[test]
fn the_working_placeholder_never_shimmers() {
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Full);
    let p = planned("write the tests");
    let (cells, text) = line(&p, 700);
    let muted = crew_theme::theme().text_muted;
    let words = fgs(&cells, &text, "Working");
    assert!(words.iter().all(|c| *c == muted), "not a task: {words:?}");
}
