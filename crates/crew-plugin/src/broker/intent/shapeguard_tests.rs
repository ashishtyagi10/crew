//! The router's git and session shapes stand only on the message's own words.
use super::*;
use crate::broker::intent::decision::{decide_in, Routing};
use crate::broker::intent::world::World;

/// The router's reply for `task`, through the real read path.
fn routed(task: &str, reply: &str) -> Routing {
    let call = |_: &str| -> Result<String, String> { Ok(reply.to_string()) };
    decide_in(task, &World::default(), Some(&call))
}

fn shape_of(task: &str, said: &str) -> Shape {
    routed(task, &format!("SHAPE: {said}")).decision().shape
}

/// The live misroute of 2026-09-28, reply for reply.
#[test]
fn a_file_edit_the_router_called_a_commit_is_a_reply_and_the_line_says_so() {
    let task = "append the line '// probe marker' at the very end of \
                crates/crew-theme/src/lib.rs, nothing else";
    let r = routed(task, "SHAPE: commit\nWHY: Simple file modification needed");
    assert_eq!(r.decision().shape, Shape::Reply, "{r:?}");
    assert_eq!(
        r.line(),
        "routing: reply \u{2014} the router said commit (Simple file modification \
         needed), but nothing here asks for one"
    );
    let bare = routed(task, "SHAPE: commit");
    assert_eq!(
        bare.line(),
        "routing: reply \u{2014} the router said commit, but nothing here asks for one"
    );
}

#[test]
fn a_commit_asked_for_stands() {
    for task in [
        "commit this and push",
        "write a commit message",
        "Commit.",
        "can you COMMIT my work",
    ] {
        assert_eq!(shape_of(task, "commit"), Shape::Commit, "{task}");
    }
    // Word-bounded: talking ABOUT commits is not asking for one.
    for task in ["show my recent commits", "recommit is a word?"] {
        assert_eq!(shape_of(task, "commit"), Shape::Reply, "{task}");
    }
}

#[test]
fn a_review_stands_on_its_words_and_a_file_named_review_is_not_one() {
    for task in [
        "review my diff",
        "look over my changes before I push",
        "check my changes",
        "anything wrong in the diff?",
        "is this PR ready",
        // The word is there, so the router's reading stands: the guard only
        // removes a shape nothing asked for, it never second-guesses one.
        "fix the review comments",
    ] {
        assert_eq!(shape_of(task, "review"), Shape::Review, "{task}");
    }
    for task in [
        "fix the bug in review.rs",
        "fix the bug in crates/crew-plugin/src/broker/review.rs",
        "why does `review.rs` sort best-first?",
        "plan the sprint",
    ] {
        assert_eq!(shape_of(task, "review"), Shape::Reply, "{task}");
    }
}

#[test]
fn a_standup_and_a_resume_stand_only_on_their_words() {
    for task in [
        "what did I ship this week",
        "what did i do yesterday",
        "draft my stand-up",
        "draft my stand up",
    ] {
        assert_eq!(shape_of(task, "standup"), Shape::Standup, "{task}");
    }
    assert_eq!(shape_of("write the release notes", "standup"), Shape::Reply);
    assert_eq!(
        shape_of("explain relationship types", "standup"),
        Shape::Reply,
        "ship inside a longer word is not the word"
    );
    for task in [
        "pick up where we left off",
        "resume",
        "back to our last session",
    ] {
        assert_eq!(shape_of(task, "resume"), Shape::Resume, "{task}");
    }
    assert_eq!(
        shape_of("resume.rs is too long, split it", "resume"),
        Shape::Reply
    );
}

#[test]
fn every_other_shape_passes_untouched_whatever_the_message_says() {
    for said in ["reply", "fan", "loop", "plan", "goal", "swarm"] {
        let r = routed("append a line to lib.rs", &format!("SHAPE: {said}\nWHY: w"));
        assert_eq!(r.decision().shape.name(), said);
        assert_eq!(r.line(), format!("routing: {said} \u{2014} w"));
    }
}

/// An overruled pick is a reply in full: the skill line the router gave is
/// read for it, as for any reply.
#[test]
fn an_overruled_shape_is_dispatched_as_a_plain_reply() {
    let world = World {
        skills: vec![("pdf".into(), "p".into()), ("xlsx".into(), "x".into())],
        ..World::default()
    };
    let call = |_: &str| -> Result<String, String> { Ok("SHAPE: commit\nSKILLS: none".into()) };
    let r = decide_in("fix the typo in README.md", &world, Some(&call));
    let d = r.decision();
    assert_eq!(d.shape, Shape::Reply);
    assert_eq!(d.hints.skills, Some(vec![]));
    assert_eq!(d.label(), "reply");
}

#[test]
fn path_like_tokens_are_files_and_sentence_ends_are_not() {
    assert!(path_like("review.rs"));
    assert!(path_like("`review.rs`,"));
    assert!(path_like("src/commit"));
    assert!(path_like("C:\\review"));
    assert!(!path_like("commit."));
    assert!(!path_like("\"review\""));
    assert_eq!(words("Stand-up, PR's!", false), ["stand", "up", "pr", "s"]);
}

/// The prompt's half of the same fix: the router is told before it is
/// overruled, so the line above is the exception, not the routine.
#[test]
fn the_prompt_rules_file_edits_out_of_commit_and_skills_out_of_names() {
    let p = crate::broker::intent::classify::prompt("hi", &World::default());
    assert!(
        p.contains("editing, adding to, fixing or writing files\n"),
        "{p}"
    );
    assert!(p.contains("never commit"), "{p}");
    assert!(
        p.contains("ONLY when the message asks for exactly that"),
        "{p}"
    );
    let g = crate::broker::intent::skillhint::GRAMMAR;
    assert!(g.contains("Most messages need none"), "{g}");
    assert!(g.contains("its DESCRIPTION"), "{g}");
}
