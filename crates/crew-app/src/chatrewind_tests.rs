use super::fix::*;
use super::*;
use crew_plugin::PluginEvent;

#[test]
fn the_list_holds_only_messages_that_started_a_task_newest_first() {
    let (mut p, _) = history();
    key(&mut p, ChatInput::Rewind);
    let texts: Vec<&str> = p.rewind.picks().iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, ["now add tests", "fix the parser"]);
    let drawn = card_text(&p);
    assert!(drawn.contains("now add tests") && drawn.contains("fix the parser"));
    assert!(
        !drawn.contains("/diff") && !drawn.contains("tests added"),
        "{drawn}"
    );
}

#[test]
fn picking_a_row_asks_about_that_rows_task() {
    let (mut p, file) = history();
    key(&mut p, ChatInput::Rewind);
    key(&mut p, ChatInput::Down); // the older one
    key(&mut p, ChatInput::Enter);
    let lines = sent(&file, 1);
    assert_eq!(lines[0], r#"{"type":"rewind","task":1,"confirm":false}"#);
    assert!(p.rewind.sel.is_none(), "the list closes on a pick");
}

#[test]
fn the_confirmation_counts_files_and_names_outside_edits() {
    let (mut p, _) = history();
    key(&mut p, ChatInput::Rewind);
    key(&mut p, ChatInput::Down);
    key(&mut p, ChatInput::Enter);
    p.rewind_hears(&listed(
        1,
        &["a.rs", "b.rs", "notes.md"],
        &["notes.md"],
        false,
    ));
    let said = card_text(&p);
    assert!(
        said.contains("rewind to \u{201c}fix the parser\u{201d}?"),
        "{said}"
    );
    assert!(
        said.contains("these go back: a.rs, b.rs, notes.md (yours)")
            && said.contains("Enter rewinds \u{b7} Esc keeps"),
        "{said}"
    );
    assert!(!said.contains("outside") && !said.contains("more"));
}

#[test]
fn a_long_list_is_counted_and_an_uncertain_one_says_so() {
    let (mut p, _) = history();
    key(&mut p, ChatInput::Rewind);
    key(&mut p, ChatInput::Down);
    key(&mut p, ChatInput::Enter);
    let five = ["a", "b", "c", "d", "e"];
    let mut ev = listed(1, &five, &[], true);
    if let PluginEvent::Rewind { files, .. } = &mut ev {
        *files = 7;
    }
    p.rewind_hears(&ev);
    let said = card_text(&p);
    assert!(said.contains("a, b, c, d, e +2 more"), "{said}");
    assert!(
        said.contains("may include edits made outside crew"),
        "{said}"
    );
}

#[test]
fn yes_cuts_the_transcript_and_returns_the_text_to_the_composer() {
    let (mut p, file) = history();
    key(&mut p, ChatInput::Rewind);
    key(&mut p, ChatInput::Down);
    key(&mut p, ChatInput::Enter);
    p.rewind_hears(&answer(1, 2, false, false, ""));
    key(&mut p, ChatInput::Enter);
    assert_eq!(
        sent(&file, 2)[1],
        r#"{"type":"rewind","task":1,"confirm":true,"seen":"S"}"#,
        "yes carries the list it was said to"
    );
    // The broker says it is done.
    let mut done = answer(1, 2, false, true, "");
    if let PluginEvent::Rewind { pin, .. } = &mut done {
        *pin = 1;
    }
    p.rewind_hears(&done);
    assert_eq!(p.input, "fix the parser");
    assert!(p
        .messages
        .iter()
        .all(|m| m.ts != "100" && m.ts != "300" && m.ts != "301"));
    let note = &p.messages.last().unwrap().text;
    assert!(
        note.contains("rewound to") && note.contains("2 files back"),
        "{note}"
    );
    assert!(
        note.contains("your files before this are pinned") && note.contains("undo #1"),
        "{note}"
    );
    assert!(
        p.rewind.picks().is_empty(),
        "the rewound messages are not listed again"
    );
}

#[test]
fn a_message_that_changed_no_files_rewinds_the_conversation_without_asking() {
    let (mut p, file) = history();
    key(&mut p, ChatInput::Rewind);
    key(&mut p, ChatInput::Enter); // the newest
    p.rewind_hears(&answer(2, 0, false, false, ""));
    assert!(
        p.rewind.ask.is_none(),
        "no confirmation for a file step that does not exist"
    );
    assert_eq!(
        sent(&file, 2)[1],
        r#"{"type":"rewind","task":2,"confirm":true,"seen":"S"}"#
    );
    p.rewind_hears(&answer(2, 0, false, true, ""));
    assert_eq!(p.input, "now add tests");
    assert!(
        !p.messages.last().unwrap().text.contains("back"),
        "no file count to report"
    );
}

#[test]
fn a_running_task_refuses_it() {
    let (mut p, file) = history();
    p.absorb_task(9, true);
    key(&mut p, ChatInput::Rewind);
    assert!(p.rewind.sel.is_none());
    assert!(p
        .messages
        .last()
        .unwrap()
        .text
        .contains("stop it first (/stop)"));
    assert!(sent(&file, 1).is_empty());
}
