//! Older tool results are shortened in the relay's follow-up prompt, and a
//! repeat of a shortened read runs again instead of pointing at a result that
//! is no longer there (`crew_hive::tools::exchanges`). Each test runs a whole
//! turn against a scripted agent and reads the prompts it was really sent.
use std::sync::Mutex;
use std::time::Duration;

use super::*;
use crate::broker::tier::{tier_of, Tier};
use crate::Registry;

/// Replies in order, then `@done`; keeps every prompt it was dialed with.
struct Scripted(Mutex<Vec<String>>, Mutex<Vec<String>>);

impl Adapter for Scripted {
    fn name(&self) -> &str {
        "planner"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        self.1.lock().unwrap().push(body.to_string());
        Ok(self
            .0
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_else(|| "@done".into()))
    }
}

/// Answers every read with [`file`] for its path, and keeps the paths it ran.
struct Files(Mutex<Vec<String>>);

impl ToolRunner for Files {
    fn hint(&self) -> String {
        "TOOLS: sys:read_file".into()
    }
    fn call(&self, _server: &str, _tool: &str, args: &str) -> Result<String, String> {
        let v: serde_json::Value = serde_json::from_str(args).map_err(|e| e.to_string())?;
        let path = v["path"].as_str().unwrap_or_default().to_string();
        self.0.lock().unwrap().push(path.clone());
        Ok(file(&path))
    }
    fn repeatable(&self, server: &str, tool: &str) -> bool {
        tier_of(server, tool) == Tier::Read
    }
}

/// 5,203 chars under the relay's 6,000 clip, so what is sent is exactly this;
/// its last line names it, so a whole result can be told from a shortened one.
fn file(name: &str) -> String {
    let mut lines: Vec<String> = (0..98)
        .map(|i| format!("{name} line {i:02}: {}", "x".repeat(40)))
        .collect();
    lines.push(format!("END OF {name}"));
    lines.join("\n")
}

fn read(name: &str) -> String {
    format!("reading {name}\n@tool sys:read_file {{\"path\":\"{name}\"}}")
}

/// Run a turn of up to 8 tool rounds that reads `names` in order and then
/// answers; hand back the reads that ran and every follow-up prompt.
fn turn(names: &[&str]) -> (Vec<String>, Vec<String>) {
    let runner = std::sync::Arc::new(Files(Mutex::new(Vec::new())));
    let b = Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5))
        .with_tools(runner.clone())
        .with_tool_rounds(8);
    let mut then: Vec<String> = names[1..].iter().map(|n| read(n)).collect();
    then.push("answer\n@done".into());
    then.reverse();
    let agent = Scripted(Mutex::new(then), Mutex::new(Vec::new()));
    b.run_tools(
        &agent,
        "base prompt",
        read(names[0]),
        &mut RunStats::default(),
        &mut Usage::default(),
        &Envelope::new("user", "planner", "t1", "task"),
        &HopStream::noop(),
        &mut |_| {},
    );
    let ran = runner.0.lock().unwrap().clone();
    let dialed = agent.1.lock().unwrap().clone();
    (ran, dialed)
}

/// The follow-up as it was built before anything was shortened: every read
/// in `names` whole, with `left` calls to go.
fn whole(names: &[&str], left: u32) -> String {
    let exchanges: Vec<String> = names
        .iter()
        .map(|n| {
            format!(
                "YOUR MESSAGE:\nreading {n}\nCALLED sys:read_file {{\"path\":\"{n}\"}}\n\
                 RESULT:\n{}",
                file(n)
            )
        })
        .collect();
    format!(
        "base prompt\n\nTOOL EXCHANGES THIS TURN:\n{}\n\nYou may make {left} more tool \
         call(s) this turn. Continue the task using these results. You may call another \
         tool, or answer and end with your routing line (`@next <agent>` or `@done`).",
        exchanges.join("\n\n")
    )
}

/// The one exchange in `prompt` that read `name`.
fn exchange<'a>(prompt: &'a str, name: &str) -> &'a str {
    prompt
        .split("YOUR MESSAGE:\n")
        .find(|c| c.starts_with(&format!("reading {name}\n")))
        .unwrap_or_else(|| panic!("no exchange for {name} in:\n{prompt}"))
}

/// Less than three whole results: the shortened prompt carries two and two
/// heads, the whole one all four.
const BOUND: usize = 3 * 5_203;

#[test]
fn the_fourth_prompt_carries_the_last_two_results_whole_and_the_first_two_shortened() {
    assert_eq!(file("f1").chars().count(), 5_203);
    let (ran, dialed) = turn(&["f1", "f2", "f3", "f4"]);
    assert_eq!(ran, ["f1", "f2", "f3", "f4"]);
    let fourth = &dialed[3];
    let note = "\n\u{2026} (result shortened \u{2014} 5,203 chars; the call can be made again \
                to see it all)";
    for old in ["f1", "f2"] {
        let e = exchange(fourth, old);
        assert!(e.contains(note), "{old} not shortened:\n{e}");
        assert!(
            e.contains(&format!("{old} line 00: ")),
            "{old} lost its head"
        );
        assert!(!e.contains(&format!("END OF {old}")), "{old} kept its tail");
    }
    for new in ["f3", "f4"] {
        assert!(
            exchange(fourth, new).contains(&file(new)),
            "{new} not whole"
        );
    }
    // Every call's own line survives, shortened or not.
    for n in ["f1", "f2", "f3", "f4"] {
        assert!(fourth.contains(&format!("CALLED sys:read_file {{\"path\":\"{n}\"}}")));
    }
    let before = whole(&["f1", "f2", "f3", "f4"], 4);
    eprintln!(
        "fourth follow-up: {} chars, whole: {}",
        fourth.len(),
        before.len()
    );
    assert!(before.len() > BOUND, "whole is {}", before.len());
    assert!(fourth.len() < BOUND, "shortened is {}", fourth.len());
}

#[test]
fn a_turn_of_two_reads_is_sent_exactly_as_before() {
    let (_, dialed) = turn(&["f1", "f2"]);
    assert_eq!(dialed.len(), 2);
    assert_eq!(dialed[0], whole(&["f1"], 7));
    assert_eq!(dialed[1], whole(&["f1", "f2"], 6));
}

#[test]
fn a_repeat_of_a_shortened_read_runs_and_a_fresh_one_still_points() {
    let (ran, dialed) = turn(&["A", "B", "C", "C", "A", "A"]);
    // C straight after C is still whole in the prompt the pointer goes into.
    assert!(
        dialed[3].contains("same call as round 3"),
        "no pointer:\n{}",
        dialed[3]
    );
    // A's round-1 result is shortened by then, so the repeat runs, whole.
    assert_eq!(ran, ["A", "B", "C", "A"], "A must run again, C must not");
    assert!(!dialed[4].contains("same call as round 1"));
    assert!(exchange(&dialed[4], "A").contains("\u{2026} (result shortened"));
    assert!(
        dialed[4].contains(&file("A")),
        "the run's result is not whole"
    );
    // And A once more points at round 5, where it is whole.
    assert!(dialed[5].contains("same call as round 5"), "{}", dialed[5]);
}
