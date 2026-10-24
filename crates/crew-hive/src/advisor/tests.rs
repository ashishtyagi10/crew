//! The advisor on its own: a fake call answers each brief and keeps it.
use std::sync::Mutex;

use super::*;

/// The failure of a build that did not build.
const BROKEN: &str = "exit 101\n   Compiling crew v0.1.0\nerror[E0308]: mismatched types\n  \
                      --> src/a.rs:3:5\nerror: could not compile";

/// What one fake advice call cost.
const SPENT: Spent = Spent {
    input: 40,
    output: 9,
    micros_usd: 120,
};

type Reply = Result<&'static str, &'static str>;

/// A surface whose `sys:run` fails on exit 101, as the session's does.
struct Runs;

impl Tools for Runs {
    fn hint(&self) -> String {
        "TOOLS: sys:run".into()
    }
    fn call(&self, _s: &str, _t: &str, args: &str) -> Result<String, String> {
        Ok(match args.contains("build") {
            true => BROKEN.into(),
            false => "exit 0\nfine".into(),
        })
    }
    fn failed(&self, s: &str, t: &str, out: &str) -> bool {
        (s, t) == ("sys", "run") && out.starts_with("exit 101")
    }
}

/// What a turn's advisor did: the briefs it was given, the lines it said,
/// and what it cost.
#[derive(Default)]
struct Seen {
    briefs: Mutex<Vec<String>>,
    said: Mutex<Vec<String>>,
    spent: Mutex<Vec<Spent>>,
}

/// An advisor answering `reply` to every brief, `finds` counting how often
/// it was looked up (`None`: there is none to find).
fn advisor(reply: Option<Reply>) -> (Advisor, Arc<Seen>, Arc<Mutex<u32>>) {
    let seen = Arc::new(Seen::default());
    let finds = Arc::new(Mutex::new(0));
    let (keep, count) = (Arc::clone(&seen), Arc::clone(&finds));
    let find: Find = Arc::new(move || {
        *count.lock().unwrap() += 1;
        let keep = Arc::clone(&keep);
        let reply = reply?;
        Some(Arc::new(move |brief: String| -> Asked {
            keep.briefs.lock().unwrap().push(brief);
            Box::pin(async move {
                reply
                    .map(|r| (r.to_string(), SPENT))
                    .map_err(str::to_string)
            })
        }) as Ask)
    });
    let (said, spent) = (Arc::clone(&seen), Arc::clone(&seen));
    let hooks = Hooks {
        say: Arc::new(move |l| said.said.lock().unwrap().push(l.to_string())),
        spent: Arc::new(move |s| spent.spent.lock().unwrap().push(s)),
    };
    (
        Advisor::new("make it build", Arc::new(Runs), find, hooks),
        seen,
        finds,
    )
}

fn run(cmd: &str) -> ToolCall {
    ToolCall {
        server: "sys".into(),
        tool: "run".into(),
        args: format!("{{\"cmd\": \"{cmd}\"}}"),
    }
}

/// One round of `cmd`, run on [`Runs`]: what the turn carries after it.
async fn round(a: &Advisor, cmd: &str) -> String {
    let call = run(cmd);
    let out = Runs.call(&call.server, &call.tool, &call.args).unwrap();
    let ran = (!Runs.failed(&call.server, &call.tool, &out), out);
    a.read(&[call], &[ran]).await
}

#[tokio::test]
async fn a_failed_run_is_read_once_and_its_advice_heads_a_section() {
    let (a, seen, _) = advisor(Some(Ok(
        "- E0308 at src/a.rs:3: the caller wants &str\n\n- fix \
                                     the call, then rerun cargo build\n- third\n- fourth",
    )));
    let added = round(&a, "cargo build").await;
    assert_eq!(
        added,
        format!(
            "{HEAD}\n- E0308 at src/a.rs:3: the caller wants &str\n- fix the call, then \
             rerun cargo build\n- third\n\n"
        )
    );
    let briefs = seen.briefs.lock().unwrap();
    assert_eq!(briefs.len(), 1);
    let b = &briefs[0];
    assert!(b.contains("THE REQUEST:\nmake it build"), "{b}");
    assert!(
        b.contains("1. sys:run {\"cmd\": \"cargo build\"} \u{2192} failed"),
        "{b}"
    );
    assert!(b.contains("THE COMMAND THAT FAILED: cargo build"), "{b}");
    assert!(
        b.contains("error[E0308]") && !b.contains("Compiling"),
        "cut: {b}"
    );
    assert_eq!(*seen.spent.lock().unwrap(), [SPENT], "the read is billed");
    let said = seen.said.lock().unwrap();
    assert_eq!(
        *said,
        ["advised \u{b7} E0308 at src/a.rs:3: the caller wants &str"]
    );
}

#[tokio::test]
async fn a_round_with_no_failure_asks_nothing_and_finds_nothing() {
    let (a, seen, finds) = advisor(Some(Ok("unused")));
    assert_eq!(round(&a, "cargo test").await, "");
    assert!(seen.briefs.lock().unwrap().is_empty());
    assert_eq!(*finds.lock().unwrap(), 0, "no lookup before a failure");
}

#[tokio::test]
async fn a_turn_asks_at_most_twice_and_keeps_the_newest_advice() {
    let (a, seen, finds) = advisor(Some(Ok("rerun with the fix")));
    for _ in 0..3 {
        let added = round(&a, "cargo build").await;
        assert_eq!(added, format!("{HEAD}\n- rerun with the fix\n\n"));
    }
    let briefs = seen.briefs.lock().unwrap();
    assert_eq!(briefs.len(), PER_TURN as usize);
    assert!(
        briefs[1].contains("2. sys:run"),
        "the calls so far: {}",
        briefs[1]
    );
    assert_eq!(*finds.lock().unwrap(), 1, "looked up once a turn");
}

#[tokio::test]
async fn an_advisor_that_errors_or_says_nothing_adds_nothing() {
    let (a, seen, _) = advisor(Some(Err("timed out after 30s")));
    assert_eq!(round(&a, "cargo build").await, "");
    assert_eq!(seen.briefs.lock().unwrap().len(), 1);
    assert!(seen.spent.lock().unwrap().is_empty(), "nothing billed");
    let (a, seen, _) = advisor(Some(Ok("\n```\n")));
    assert_eq!(round(&a, "cargo build").await, "", "said nothing");
    assert!(seen.said.lock().unwrap().is_empty());
}

#[tokio::test]
async fn with_none_to_find_a_failure_adds_nothing() {
    let (a, _, finds) = advisor(None);
    assert_eq!(round(&a, "cargo build").await, "");
    assert_eq!(round(&a, "cargo build").await, "");
    assert_eq!(*finds.lock().unwrap(), 1, "not looked for again");
}
