//! The router, measured live. `#[ignore]`d: it calls the real cheap-tier
//! provider, costs a few cents, and its answer is a number, not a pass.
//!
//! ```text
//! source ~/.zshenv
//! cargo test -p crew-plugin --lib router_eval -- --ignored --nocapture
//! ```
//!
//! Each labelled message goes through the real routing prompt and the real
//! read path (`decide_in`), in a world with a dirty tree and a skill roster
//! holding decoys — `theme-factory`, `doc-coauthoring`, `pdf` — whose names
//! turn up in messages that do not want them, as they did live on
//! 2026-09-28. Three scores: the shape the model said, the shape dispatched
//! after `shapeguard`, and the skills it named. It lives in the crate, not
//! in `tests/`, because the router's seams are `pub(crate)`.
use std::cell::RefCell;
use std::time::Instant;

use super::*;
use crate::broker::intent::world::World;

/// One labelled message per line: `shapes that are right | skills it must
/// name (and no others; - is none) | message`.
const CASES: &str = "\
reply swarm | - | append the line '// probe marker' at the very end of crates/crew-theme/src/lib.rs, nothing else
reply swarm | - | add a one-line doc comment above pub fn relative_luminance in crates/crew-theme/src/lib.rs saying what it returns
reply swarm | - | rename clip to clip_chars in crates/crew-plugin/src/broker/route.rs
reply swarm | - | fix the typo in README.md
reply swarm | - | fix the bug in review.rs where findings sort best-first
reply swarm | - | make the palette in crates/crew-theme/src/palette.rs a little warmer
reply swarm | - | add a pdf_export flag to the Config struct in config.rs
reply | - | what does crew-render do
reply | - | where is TASK_CAP defined
reply | - | how does the theme crossfade work?
reply | - | why is the doc-coauthoring skill listed twice in the picker?
commit | - | commit this
commit | - | write a commit message for my changes
review | - | review my diff
review | - | look over my changes before I push
standup | - | what did I ship this week
resume | - | pick up where we left off
swarm plan | - | add a /foo command with tests and docs
swarm plan | - | add a settings export: the command, a file format, docs and tests
fan | - | give me three different takes on a name for the router module
reply swarm | pdf | turn docs/guide.md into a PDF
reply loop plan swarm goal | doc-coauthoring | help me co-author a design doc for the new router, section by section";

/// The roster the router is shown: the three decoys beside one-liners
/// from a live install.
const SKILLS: &[(&str, &str)] = &[
    (
        "theme-factory",
        "Toolkit for styling artifacts with a theme",
    ),
    (
        "doc-coauthoring",
        "Guide users through a structured workflow for co-authoring documentation",
    ),
    ("pdf", "Create and edit PDF files"),
    (
        "frontend-design",
        "Guidance for distinctive, intentional visual design when building new UI",
    ),
    (
        "webapp-testing",
        "Toolkit for interacting with and testing local web applications using Playwright",
    ),
    (
        "xlsx",
        "Use this skill any time a spreadsheet file is the primary input or output",
    ),
    (
        "skill-creator",
        "Create new skills, modify and improve existing skills",
    ),
    (
        "internal-comms",
        "Resources to write all kinds of internal communications",
    ),
];

fn world() -> World {
    World {
        agents: testenv::TRIO.iter().map(|(n, _)| n.to_string()).collect(),
        roles: testenv::TRIO
            .iter()
            .map(|(n, r)| format!("{n} ({r})"))
            .collect(),
        dirty: Some(2),
        skills: SKILLS
            .iter()
            .map(|(n, d)| (n.to_string(), d.to_string()))
            .collect(),
        ..World::default()
    }
}

/// One cell of the table: the message cut to a column.
fn cut(s: &str, n: usize) -> String {
    match s.chars().count() > n {
        true => format!("{}…", s.chars().take(n - 1).collect::<String>()),
        false => s.to_string(),
    }
}

fn pct(n: usize, of: usize) -> String {
    format!("{n}/{of} ({:.0}%)", 100.0 * n as f64 / of as f64)
}

#[test]
#[ignore = "live: calls the cheap-tier router on the discovered provider"]
fn router_eval() {
    let Some(call) = classify::live_router() else {
        panic!("no live router: source ~/.zshenv for a provider key, and unset CREW_INTENT");
    };
    let tier = crew_hive::ModelTier::Cheap;
    let model = crate::broker::discover::provider_and_model_for(tier).map(|(_, m)| m);
    println!("\nrouter: {}", model.unwrap_or_default());
    let world = world();
    let names: Vec<String> = world.skills.iter().map(|(n, _)| n.clone()).collect();
    let (mut raw_ok, mut held_ok, mut skills_ok) = (0, 0, 0);
    let started = Instant::now();
    println!(
        "\n{:<52} {:<14} {:<8} {:<8} skills",
        "message", "want", "raw", "held"
    );
    for row in CASES.lines() {
        let mut cols = row.splitn(3, " | ");
        let want: Vec<&str> = cols.next().unwrap().split(' ').collect();
        let must: Vec<&str> = cols
            .next()
            .unwrap()
            .split(' ')
            .filter(|w| *w != "-")
            .collect();
        let task = cols.next().unwrap();
        let last = RefCell::new(String::new());
        let seen = |p: &str| {
            let r = call(p).map(|(reply, _)| reply);
            *last.borrow_mut() = r.clone().unwrap_or_else(|e| format!("ERR {e}"));
            r
        };
        let routing = decide_in(task, &world, Some(&seen));
        let reply = last.into_inner();
        let raw = parse_shape(&reply).map_or("?", Shape::name);
        let held = routing.decision().shape.name();
        let got = crate::broker::intent::skillhint::parse(&reply, &names).unwrap_or_default();
        let skill_ok = got.len() == must.len() && got.iter().all(|g| must.contains(&g.as_str()));
        raw_ok += usize::from(want.contains(&raw));
        held_ok += usize::from(want.contains(&held));
        skills_ok += usize::from(skill_ok);
        let mark = |ok: bool| if ok { " " } else { "✗" };
        println!(
            "{:<52} {:<14} {}{:<7} {}{:<7} {}{}",
            cut(task, 52),
            cut(&want.join("|"), 14),
            mark(want.contains(&raw)),
            raw,
            mark(want.contains(&held)),
            held,
            mark(skill_ok),
            if got.is_empty() {
                "-".to_string()
            } else {
                got.join(",")
            },
        );
        if !want.contains(&raw) || !want.contains(&held) || !skill_ok {
            println!("    ↳ {}", cut(&reply.replace('\n', " ⏎ "), 150));
        }
    }
    let n = CASES.lines().count();
    println!(
        "\nshape raw {} · shape guarded {} · skills {} · {:.1} s\n",
        pct(raw_ok, n),
        pct(held_ok, n),
        pct(skills_ok, n),
        started.elapsed().as_secs_f64()
    );
}
