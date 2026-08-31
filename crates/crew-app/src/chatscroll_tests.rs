use super::*;
use crate::chat::ChatPane;
use crew_hive::{AgentKind, ModelTier, TaskId, TaskSpec};
use crew_plugin::Plugin;

#[test]
fn scroll_clamp_accounts_for_the_live_swarm_block() {
    // A long transcript on a live 8-task swarm run: msg_rows_budget
    // reserves rows for the block, so the drawn window is shorter than
    // `rows - top - bottom` alone suggests. If the scroll clamp doesn't
    // account for the block too, the top of the transcript becomes
    // unreachable — max scroll stops short of the first line.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    for i in 0..100 {
        p.messages.push(crate::chatlayout::Message {
            sender: "agent smith".into(),
            text: format!("message number {i}"),
            ts: String::new(),
            meta: String::new(),
            usage: None,
            expanded: false,
        });
    }
    let tasks = (0..8)
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

    let (cols, rows) = (80u16, 30u16);
    p.scroll(1_000_000, cols, rows);
    let lines = crate::chatplace::placed_lines(&p, cols, rows);
    let visible: String = lines
        .iter()
        .flat_map(|(_, l)| l.iter().map(|c| c.c))
        .collect();
    assert!(
        visible.contains("message number 0"),
        "max scroll should reach the very first transcript line even \
         while a live swarm block is open; visible window: {visible:?}"
    );
}

#[test]
fn thumb_geometry_is_proportional_and_anchored() {
    assert_eq!(thumb(5, 10, 0), None, "fits — no thumb");
    assert_eq!(thumb(10, 10, 0), None, "exactly fits — no thumb");
    assert_eq!(thumb(100, 10, 0), Some((0, 1)), "top of the list");
    let (top, len) = thumb(100, 10, 90).expect("overflowing");
    assert_eq!(top + len, 10, "bottom-anchored at max scroll");
    let (top, len) = thumb(100, 10, 45).expect("overflowing");
    assert!(top > 0 && top + len < 10, "mid-scroll sits mid-track");
}

#[test]
fn pill_is_right_aligned_and_gated_on_unread() {
    assert!(new_pill_cells(0, 80, 5).is_empty());
    let cells = new_pill_cells(3, 80, 5);
    let text: String = cells.iter().map(|c| c.c).collect();
    assert_eq!(text, "\u{2193} 3 new");
    assert_eq!(cells.last().unwrap().col, 78); // one column in from the edge
    assert!(cells.iter().all(|c| c.row == 5));
}

#[test]
fn pill_hides_when_too_narrow() {
    assert!(new_pill_cells(3, 6, 0).is_empty());
}
