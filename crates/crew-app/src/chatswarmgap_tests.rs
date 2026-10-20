//! The live swarm block sits a card gap under the transcript (its status line
//! used to hang off the user's last sentence and read as its second line), and
//! the token trailer ends where the task rows under it end.
use super::*;
use crate::chat::ChatPane;
use crate::chatlayout::Message;
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

fn pane(said: Option<&str>, tasks: u64) -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.messages = said
        .map(|text| Message {
            sender: "user".into(),
            text: text.into(),
            ts: String::new(),
            meta: String::new(),
            usage: None,
            expanded: false,
        })
        .into_iter()
        .collect();
    p.absorb_hive_plan(
        (0..tasks)
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
            .collect(),
    );
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Running,
    });
    p
}

fn rows_of(p: &ChatPane, cols: u16, rows: u16) -> Vec<String> {
    let mut grid = vec![vec![' '; cols as usize]; rows as usize];
    for c in crate::chatview::cells(p, cols, rows) {
        if let Some(cell) = grid
            .get_mut(c.row as usize)
            .and_then(|r| r.get_mut(c.col as usize))
        {
            *cell = c.c;
        }
    }
    grid.into_iter().map(|r| r.into_iter().collect()).collect()
}

#[test]
fn a_transcript_buys_the_block_one_card_gap() {
    let gap = pane(None, 3).view().gap_rows as u16;
    let bare = swarm_rows(&pane(None, 3), 60);
    assert_eq!(
        swarm_rows(&pane(Some("compare the samplers"), 3), 60),
        bare + gap
    );
}

#[test]
fn the_status_line_sits_a_gap_under_the_last_message() {
    let p = pane(Some("compare the samplers"), 3);
    let gap = p.view().gap_rows;
    assert!(gap >= 1, "fixture: the density keeps a card gap");
    let lines = rows_of(&p, 60, 24);
    let status = lines
        .iter()
        .position(|l| l.contains("task-0"))
        .expect("status line");
    let said = lines
        .iter()
        .rposition(|l| l.contains("compare the samplers"));
    assert_eq!(said, Some(status - gap - 1), "{lines:#?}");
    for row in &lines[status - gap..status] {
        assert!(row.trim().is_empty(), "the gap row holds {row:?}");
    }
}

#[test]
fn a_squeezed_block_gives_up_the_gap_before_its_status_line() {
    let p = pane(Some("compare the samplers"), 3);
    let body = swarm_rows(&pane(None, 3), 60);
    let drawn = block_in(&p, 60, 10, body);
    assert_eq!(drawn.iter().map(|c| c.row).min(), Some(10));
    let roomy = block_in(&p, 60, 10, body + p.view().gap_rows as u16);
    assert_eq!(
        roomy.iter().map(|c| c.row).min(),
        Some(10 + p.view().gap_rows as u16)
    );
}

#[test]
fn the_token_trailer_ends_where_the_task_rows_end() {
    let mut p = pane(Some("compare the samplers"), 3);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: crew_hive::AgentId(1),
        task: TaskId(0),
    });
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: crew_hive::AgentId(1),
        input: 4200,
        output: 610,
    });
    let cols = 100;
    let cells = block_cells(&p, cols, 10, 0);
    let last = |row: u16| {
        cells
            .iter()
            .filter(|c| c.row == row && c.c != ' ')
            .map(|c| c.col)
            .max()
    };
    let line: String = {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == 10).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert!(
        line.trim_end().ends_with(')'),
        "fixture: a trailer shows: {line:?}"
    );
    assert_eq!(last(10), Some(cols - 2), "{line:?}");
    assert!(last(11).is_some_and(|c| c <= cols - 2));
}
