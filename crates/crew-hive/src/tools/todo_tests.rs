use super::*;
use crate::tools::Tools;

fn items(list: &[(&str, &str)]) -> String {
    let items: Vec<Value> = list
        .iter()
        .map(|(text, status)| serde_json::json!({"text": text, "status": status}))
        .collect();
    serde_json::json!({ "items": items }).to_string()
}

#[test]
fn a_list_is_echoed_one_mark_a_line_with_the_count_under_it() {
    let list = Checklist::default();
    let shown = list
        .write(&items(&[
            ("add the command", "done"),
            ("add a test", "in_progress"),
            ("write the docs", "pending"),
        ]))
        .unwrap();
    assert_eq!(
        shown,
        "\u{2611} add the command\n\u{25b6} add a test\n\u{2610} write the docs\n1 of 3 done"
    );
    assert_eq!(list.section(), format!("YOUR CHECKLIST:\n{shown}\n\n"));
    assert_eq!(list.writes(), 1);
}

#[test]
fn two_items_in_progress_are_refused_and_the_old_list_stands() {
    let list = Checklist::default();
    list.write(&items(&[("a", "pending")])).unwrap();
    let e = list
        .write(&items(&[
            ("a", "in_progress"),
            ("b", "done"),
            ("c", "in_progress"),
        ]))
        .unwrap_err();
    assert!(e.contains("items 1, 3 are all in_progress"), "{e}");
    assert!(e.contains("at most ONE item may be in_progress"), "{e}");
    assert_eq!(
        list.section(),
        "YOUR CHECKLIST:\n\u{2610} a\n0 of 1 done\n\n"
    );
    assert_eq!(list.writes(), 1, "a refused write is not a write");
}

#[test]
fn thirteen_items_are_refused_and_twelve_are_not() {
    let twelve: Vec<(String, &str)> = (1..=12).map(|n| (format!("step {n}"), "pending")).collect();
    let as_refs = |v: &[(String, &str)]| -> String {
        items(&v.iter().map(|(t, s)| (t.as_str(), *s)).collect::<Vec<_>>())
    };
    let list = Checklist::default();
    assert!(list.write(&as_refs(&twelve)).is_ok());
    let mut thirteen = twelve.clone();
    thirteen.push(("step 13".into(), "pending"));
    let e = list.write(&as_refs(&thirteen)).unwrap_err();
    assert!(e.contains("13 items") && e.contains("at most 12"), "{e}");
}

#[test]
fn an_item_past_120_chars_is_refused_by_number() {
    let long = "x".repeat(121);
    let e = Checklist::default()
        .write(&items(&[("fine", "pending"), (&long, "pending")]))
        .unwrap_err();
    assert!(
        e.contains("item 2 is 121 chars") && e.contains("120"),
        "{e}"
    );
    assert!(Checklist::default()
        .write(&items(&[(&"x".repeat(120), "pending")]))
        .is_ok());
}

#[test]
fn a_malformed_list_says_what_is_wrong() {
    let list = Checklist::default();
    let e = list.write("{}").unwrap_err();
    assert!(
        e.contains("missing array argument \u{201c}items\u{201d}"),
        "{e}"
    );
    let e = list.write(&items(&[("a", "doing")])).unwrap_err();
    assert!(
        e.contains("item 1 has status \"doing\"") && e.contains("in_progress"),
        "{e}"
    );
    let e = list.write(&items(&[("  ", "pending")])).unwrap_err();
    assert!(e.contains("item 1 has no"), "{e}");
    assert!(list
        .write("not json")
        .unwrap_err()
        .contains("not valid JSON"));
}

#[test]
fn todowrites_words_are_read_and_a_missing_status_is_pending() {
    let list = Checklist::default();
    let args = r#"{"items": [{"content": "a", "status": "completed"}, {"text": "b\n  c"}]}"#;
    assert_eq!(
        list.write(args).unwrap(),
        "\u{2611} a\n\u{2610} b c\n1 of 2 done"
    );
}

#[test]
fn an_empty_list_clears_and_heads_no_prompt() {
    let list = Checklist::default();
    assert_eq!(list.section(), "", "a list nobody wrote heads nothing");
    list.write(&items(&[("a", "pending")])).unwrap();
    assert_eq!(list.write(r#"{"items": []}"#).unwrap(), "checklist cleared");
    assert_eq!(list.section(), "");
    assert_eq!(list.unfinished(), None);
}

#[test]
fn the_answer_names_what_was_left_and_is_untouched_otherwise() {
    let list = Checklist::default();
    assert_eq!(finished("answer".into(), None), "answer");
    assert_eq!(finished("answer".into(), Some(&list)), "answer");
    list.write(&items(&[
        ("add the command", "done"),
        ("add a test", "in_progress"),
        ("write the docs", "pending"),
    ]))
    .unwrap();
    let line = "checklist: 1 of 3 done \u{2014} not done: add a test, write the docs";
    assert_eq!(list.unfinished().as_deref(), Some(line));
    assert_eq!(
        finished("answer\n".into(), Some(&list)),
        format!("answer\n\n{line}")
    );
    assert_eq!(finished("  ".into(), Some(&list)), line);
    list.write(&items(&[("add the command", "done")])).unwrap();
    assert_eq!(finished("answer".into(), Some(&list)), "answer");
}

/// A surface with a checklist, which also answers one other tool.
struct Surface(Checklist);

impl Tools for Surface {
    fn hint(&self) -> String {
        "hint".into()
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        match (server, tool) {
            ("sys", "todo") => self.0.write(args),
            _ => Ok(format!("{server}:{tool} ran")),
        }
    }
    fn checklist(&self) -> Option<&Checklist> {
        Some(&self.0)
    }
}

struct Bare;

impl Tools for Bare {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        Ok(String::new())
    }
}

#[test]
fn each_task_of_a_shared_surface_keeps_a_list_of_its_own() {
    let shared: Arc<dyn Tools> = Arc::new(Surface(Checklist::default()));
    let (one, two) = (per_task(Arc::clone(&shared)), per_task(Arc::clone(&shared)));
    one.call("sys", "todo", &items(&[("mine", "pending")]))
        .unwrap();
    assert!(one.checklist().unwrap().section().contains("mine"));
    assert_eq!(
        two.checklist().unwrap().section(),
        "",
        "a fresh task starts empty"
    );
    assert_eq!(
        shared.checklist().unwrap().section(),
        "",
        "the shared list is untouched"
    );
    assert_eq!(one.call("fs", "read", "{}").unwrap(), "fs:read ran");
    assert_eq!(one.hint(), "hint");
    let bare: Arc<dyn Tools> = Arc::new(Bare);
    assert!(
        Arc::ptr_eq(&per_task(Arc::clone(&bare)), &bare),
        "a surface without lists is left as it was"
    );
}
