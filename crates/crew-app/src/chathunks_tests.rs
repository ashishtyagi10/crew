use std::time::{Duration, Instant};

use crew_plugin::Plugin;

use super::*;
use crate::chatkeys::ChatInput;

/// A pane whose "broker" writes every command it is sent to a file.
fn pane() -> (ChatPane, std::path::PathBuf) {
    let file = std::env::temp_dir().join(format!(
        "crew-chathunks-{}-{:?}.jsonl",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_file(&file);
    let sh = format!("cat > '{}'", file.display());
    let plugin = Plugin::spawn("sh", &["-c".to_string(), sh]).unwrap();
    (ChatPane::new(plugin, "crew".into()), file)
}

fn sent(file: &std::path::Path, want: usize) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        if text.lines().count() >= want || Instant::now() > deadline {
            return text.lines().map(str::to_string).collect();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

const Q: &str = "edit a.rs \u{2014} 3 changes \u{b7} the checkpoint can undo it\n\
    [1] @@ 2 @@\n- a\n+ A\n[2] @@ 4 @@\n- b\n+ B\n[3] @@ 6 @@\n+ c\n\
    Enter all \u{b7} Esc none \u{b7} type 1 3 to keep those";

fn type_and_send(p: &mut ChatPane, text: &str) {
    p.input = text.into();
    p.on_input(ChatInput::Enter, &std::env::temp_dir());
}

#[test]
fn the_rows_go_in_a_diff_fence_and_the_keys_stay_below_it() {
    let line = asked("a1", Q);
    assert!(
        line.starts_with("\u{26a0} allow this? edit a.rs \u{2014} 3 changes"),
        "{line}"
    );
    assert!(
        line.contains("\n````diff\n[1] @@ 2 @@\n- a\n+ A\n"),
        "{line}"
    );
    assert!(
        line.ends_with("\n````\nEnter all \u{b7} Esc none \u{b7} type 1 3 to keep those"),
        "{line}"
    );
    assert_eq!(
        asked("a2", "edit a.rs"),
        crate::spendask::asked("a2", "edit a.rs")
    );
}

#[test]
fn typed_numbers_allow_only_those_hunks() {
    for (typed, want) in [
        ("1 3", "[1,3]"),
        ("1,3", "[1,3]"),
        ("-2", "[1,3]"),
        ("2", "[2]"),
    ] {
        let (mut p, file) = pane();
        p.ask_user("a9".into(), Q.into());
        type_and_send(&mut p, typed);
        assert!(p.asking.is_empty(), "{typed} answered it");
        let lines = sent(&file, 1);
        let ok = |l: &String| {
            l.contains(r#""id":"a9""#)
                && l.contains(r#""granted":true"#)
                && l.contains(&format!(r#""hunks":{want}"#))
        };
        assert!(lines.iter().any(ok), "{typed}: {lines:?}");
        assert!(
            p.messages.last().unwrap().text.starts_with("allowed hunks"),
            "{typed}"
        );
        let _ = std::fs::remove_file(&file);
    }
}

#[test]
fn every_hunk_typed_out_is_a_plain_allow() {
    let (mut p, file) = pane();
    p.ask_user("a4".into(), Q.into());
    type_and_send(&mut p, "1 2 3");
    let lines = sent(&file, 1);
    assert!(
        lines
            .iter()
            .any(|l| l.contains(r#""granted":true"#) && !l.contains("hunks")),
        "{lines:?}"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_number_that_is_not_a_hunk_is_said_and_the_question_stays_open() {
    let (mut p, file) = pane();
    p.ask_user("a5".into(), Q.into());
    type_and_send(&mut p, "1 9");
    assert_eq!(p.asking.len(), 1, "still waiting");
    assert!(p.input.is_empty());
    assert!(p.messages.last().unwrap().text.contains("no hunk 9"));
    assert!(sent(&file, 1).is_empty(), "nothing was sent");
    let _ = std::fs::remove_file(&file);
}

#[test]
fn words_and_non_edit_questions_are_not_picks() {
    let (mut p, _) = pane();
    p.ask_user("a6".into(), Q.into());
    p.input = "why this one?".into();
    assert!(p.approval_key(&ChatInput::Enter).is_none(), "a message");
    let (mut p, _) = pane();
    p.ask_user("a7".into(), "run `ls` \u{2014} it cannot be undone".into());
    p.input = "1".into();
    assert!(
        p.approval_key(&ChatInput::Enter).is_none(),
        "no hunks to pick"
    );
    assert_eq!(p.asking.len(), 1);
}

#[test]
fn the_footer_asks_in_one_line() {
    let (mut p, _) = pane();
    p.ask_user("a8".into(), Q.into());
    assert_eq!(
        p.approval_footer().asking,
        Some("edit a.rs \u{2014} 3 changes \u{b7} the checkpoint can undo it")
    );
}

/// The question as the pane draws it, to LOOK at:
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew hunk_question_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn hunk_question_shot() {
    let _g = crate::app::theme_test_guard();
    let mut pane = crate::chatshot_tests::live_pane();
    pane.ask_user("a1".into(), Q.into());
    let Some(px) =
        crate::shotgpu_tests::shot_at("chat-hunks", 900, 760, 13.0, "crew", |c, r, a| {
            crate::chatview::art(&pane, c, r, a)
        })
    else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(crate::shotgpu_tests::ink(&px) > 4000);
}
