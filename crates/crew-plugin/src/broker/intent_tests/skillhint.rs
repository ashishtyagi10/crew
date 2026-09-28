//! The routing call chooses the playbooks, and no arm asks again.
use super::*;
use crate::broker::intent::skillhint::{block, parse, seed, GRAMMAR};
use crate::broker::intent::world::World;

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_skills_line_anywhere_after_the_shape_is_read_in_roster_order() {
    let roster = names(&["deploy", "review", "ship-notes"]);
    let reply = "SHAPE: reply\nWHY: notes\nSKILLS: ship-notes, Deploy";
    assert_eq!(
        parse(reply, &roster),
        Some(names(&["deploy", "ship-notes"]))
    );
    assert_eq!(parse("SHAPE: reply\nSKILLS: none", &roster), Some(vec![]));
    // Unknown names are no decision — the decider still gets its say.
    assert_eq!(parse("SHAPE: reply\nSKILLS: poetry", &roster), None);
    assert_eq!(parse("SHAPE: reply\nWHY: x", &roster), None);
    // The first line is the shape's, never a skills line.
    assert_eq!(parse("SKILLS: deploy", &roster), None);
}

#[test]
fn the_prompt_offers_skills_only_when_there_is_a_choice() {
    let bare = classify::prompt("hi", &World::default());
    assert!(!bare.contains("SKILLS:"), "{bare}");
    let world = World {
        skills: vec![
            ("deploy".into(), "ship safely".into()),
            ("review".into(), "review code".into()),
        ],
        ..World::default()
    };
    let p = classify::prompt("hi", &world);
    assert!(p.contains(GRAMMAR.trim()), "{p}");
    assert!(p.contains("  deploy \u{2014} ship safely"), "{p}");
    assert_eq!(block(&[]), None);
}

#[test]
fn the_router_s_choice_rides_on_the_decision_for_shapes_that_frame_skills() {
    let world = World {
        skills: vec![("deploy".into(), "d".into()), ("review".into(), "r".into())],
        ..World::default()
    };
    let call = |_: &str| -> Result<String, String> { Ok("SHAPE: reply\nSKILLS: review".into()) };
    let Routing::Chosen(d) = decide_in("look at my diff", &world, Some(&call)) else {
        panic!("chosen")
    };
    assert_eq!(d.hints.skills, Some(names(&["review"])));
    // Shown the roster and silent about it: that is "none", not a second call.
    let silent = |_: &str| -> Result<String, String> { Ok("SHAPE: reply\nWHY: q".into()) };
    let Routing::Chosen(d) = decide_in("what is 2+2", &world, Some(&silent)) else {
        panic!("chosen")
    };
    assert_eq!(d.hints.skills, Some(vec![]));
    // No roster shown, no choice made: the decider keeps its say.
    let Routing::Chosen(d) = decide_in("what is 2+2", &World::default(), Some(&silent)) else {
        panic!("chosen")
    };
    assert_eq!(d.hints.skills, None);
    // A fan frames no skills, so its decision carries none.
    let fan = |_: &str| -> Result<String, String> { Ok("SHAPE: fan\nSKILLS: review".into()) };
    let Routing::Chosen(d) = decide_in("everyone: ideas", &world, Some(&fan)) else {
        panic!("chosen")
    };
    assert_eq!(d.hints.skills, None);
}

/// Seeded, the arm's own pick is the router's answer — `by_model`, with no
/// chooser in reach (the mock provider has none), so it came from the memo.
#[test]
fn a_seeded_choice_is_what_the_arm_picks() {
    let _g = testenv::mock("ok");
    let dir = std::path::PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    std::fs::create_dir_all(dir.join(".crew/skills")).unwrap();
    for (n, d) in [
        ("ship-notes", "write release notes"),
        ("triage", "sort bugs"),
    ] {
        std::fs::write(
            dir.join(format!(".crew/skills/{n}.md")),
            format!("---\nname: {n}\ndescription: {d}\n---\nbody"),
        )
        .unwrap();
    }
    let task = "what changed since the last tag? (seed test)";
    seed(task, &names(&["triage"]));
    let skills = crate::broker::skills::load();
    let pick = crate::broker::skillchoice::decider().pick(task, &skills);
    let got: Vec<&str> = pick.skills.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(got, vec!["triage"]);
    assert!(pick.by_model, "the router's choice, from the memo");
}
