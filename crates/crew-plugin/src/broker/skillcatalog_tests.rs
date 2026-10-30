//! An agent that opens files itself gets the skill catalog and picks its own
//! playbook inside its turn; one that cannot still gets the playbooks the
//! decider picked for it.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::catalog;
use crate::broker::relay::relay_turn;
use crate::broker::skills::Skill;
use crate::broker::testenv;
use crate::broker::{Adapter, Broker, Registry};

fn skill(name: &str, description: &str) -> Skill {
    Skill {
        name: name.into(),
        description: description.into(),
        body: "the playbook".into(),
        origin: "user",
        path: PathBuf::from(format!("/skills/{name}/SKILL.md")),
        dir: Some(PathBuf::from(format!("/skills/{name}"))),
    }
}

#[test]
fn the_catalog_names_each_skill_its_line_and_its_file_then_the_task() {
    let long = "x".repeat(140);
    let skills = [
        skill("pdf", "Read and write PDF files.\nMore."),
        skill("docx", &long),
    ];
    let f = catalog("make a pdf of notes.md", &skills);
    let lines: Vec<&str> = f.body.lines().collect();
    assert!(lines[0].starts_with("Skills on hand"), "{lines:?}");
    assert_eq!(
        lines[1],
        "- pdf \u{2014} Read and write PDF files. (/skills/pdf/SKILL.md)"
    );
    assert!(
        lines[2].contains("\u{2026} (/skills/docx/SKILL.md)"),
        "clipped"
    );
    assert!(f.body.ends_with("TASK:\nmake a pdf of notes.md"));
    assert!(f.applied.is_empty(), "the agent decides; nothing applied");
    assert_eq!(catalog("t", &[]).body, "t", "no skills, no frame");
}

/// Records every prompt, and says whether it reads files itself.
struct Agent {
    reads: bool,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Adapter for Agent {
    fn name(&self) -> &str {
        "claude"
    }
    fn probe(&self) -> bool {
        true
    }
    fn reads_files(&self) -> bool {
        self.reads
    }
    fn call(&self, body: &str, _t: std::time::Duration) -> Result<String, String> {
        self.seen.lock().unwrap().push(body.to_string());
        Ok("ok\n@done".into())
    }
}

fn first_prompt(reads: bool) -> String {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let agent = Agent {
        reads,
        seen: Arc::clone(&seen),
    };
    let broker = Broker::new(
        Registry::new(vec![Box::new(agent)]),
        6,
        std::time::Duration::from_secs(1),
    );
    let tick = crate::broker::tick::noop_tick_emit();
    relay_turn(
        &broker,
        "claude",
        "run a review",
        "run a review",
        "t1",
        &tick,
        &mut |_| Ok(()),
    )
    .unwrap();
    let first = seen.lock().unwrap()[0].clone();
    first
}

#[test]
fn a_relay_to_an_agent_that_reads_files_carries_the_catalog_not_a_playbook() {
    let _g = testenv::mock("unused");
    let dir = PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    std::fs::create_dir_all(dir.join(".crew/skills")).unwrap();
    std::fs::write(
        dir.join(".crew/skills/review.md"),
        "Check unsafe blocks first.",
    )
    .unwrap();
    let reads = first_prompt(true);
    assert!(
        reads.contains("Skills on hand \u{2014} playbooks"),
        "{reads}"
    );
    assert!(reads.contains("review.md)"), "the file to read: {reads}");
    assert!(!reads.contains("follow this playbook"), "nothing inlined");
    let told = first_prompt(false);
    assert!(
        told.contains("follow this playbook"),
        "picked for it: {told}"
    );
}
