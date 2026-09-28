//! The relay reaches the task's checklist through the session's own tool
//! surface: `sys:todo` writes the list of the task the surface was built for,
//! every surface built for that task shares it, and the next task starts
//! with none.
use super::*;
use crate::broker::toolcall::ToolRunner;

const LIST: &str = r#"{"items": [{"text": "add the command", "status": "in_progress"},
                               {"text": "write the docs", "status": "pending"}]}"#;

#[test]
fn sys_todo_writes_the_tasks_list_and_every_surface_of_the_task_sees_it() {
    let task = Session::new().snapshot_with_cancel(Arc::new(AtomicBool::new(false)));
    let hop = task.tools_with_sys(true).expect("the sys surface is on");
    let echo = hop.call("sys", "todo", LIST).unwrap();
    assert_eq!(
        echo,
        "\u{25b6} add the command\n\u{2610} write the docs\n0 of 2 done"
    );
    let next_hop = task.tools_with_sys(true).unwrap();
    assert_eq!(
        next_hop.checklist().unwrap().section(),
        format!("YOUR CHECKLIST:\n{echo}\n\n")
    );
    assert_eq!(task.todo.writes(), 1);
}

#[test]
fn a_fresh_task_starts_with_an_empty_list() {
    let pane = Session::new();
    let first = pane.snapshot_with_cancel(Arc::new(AtomicBool::new(false)));
    first
        .tools_with_sys(true)
        .unwrap()
        .call("sys", "todo", LIST)
        .unwrap();
    let second = pane.snapshot_with_cancel(Arc::new(AtomicBool::new(false)));
    let t = second.tools_with_sys(true).unwrap();
    assert_eq!(t.checklist().unwrap().section(), "");
    assert_eq!(t.checklist().unwrap().unfinished(), None);
    assert!(
        first.todo.unfinished().is_some(),
        "the first task's list stands"
    );
}

#[test]
fn a_bad_list_is_an_error_the_agent_reads_and_the_list_is_kept() {
    let t = SessionTools::for_test(Arc::new(Mutex::new(Default::default())), true);
    t.call("sys", "todo", LIST).unwrap();
    let two = LIST.replace("pending", "in_progress");
    let e = t.call("sys", "todo", &two).unwrap_err();
    assert!(e.contains("at most ONE item may be in_progress"), "{e}");
    assert!(t
        .checklist()
        .unwrap()
        .section()
        .contains("\u{2610} write the docs"));
}

#[test]
fn the_list_is_never_a_repeat_and_is_off_with_the_sys_surface() {
    let on = SessionTools::for_test(Arc::new(Mutex::new(Default::default())), true);
    assert!(!on.repeatable("sys", "todo"), "a second list is a new list");
    assert!(on.repeatable("sys", "read_file"));
    let off = SessionTools::for_test(Arc::new(Mutex::new(Default::default())), false);
    assert!(off.checklist().is_none());
}
