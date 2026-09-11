//! The `SHAPE:` first-line grammar and the injected-classifier seam.
use super::*;

#[test]
fn parses_every_shape_token() {
    for (line, shape) in [
        ("SHAPE: reply", Shape::Reply),
        ("SHAPE: fan", Shape::Fan),
        ("SHAPE: loop", Shape::Loop),
        ("SHAPE: plan", Shape::Plan),
        ("SHAPE: swarm", Shape::Swarm),
        ("SHAPE: commit", Shape::Commit),
        ("SHAPE: review", Shape::Review),
        ("SHAPE: standup", Shape::Standup),
        ("SHAPE: resume", Shape::Resume),
    ] {
        assert_eq!(parse_shape(line), Some(shape), "{line}");
    }
}

#[test]
fn grammar_is_case_insensitive_and_tolerates_padding() {
    assert_eq!(parse_shape("shape: FAN"), Some(Shape::Fan));
    assert_eq!(parse_shape("  SHAPE:  plan  "), Some(Shape::Plan));
    assert_eq!(parse_shape("\n\nSHAPE: loop"), Some(Shape::Loop));
}

#[test]
fn prose_after_the_first_line_is_tolerated() {
    assert_eq!(
        parse_shape("SHAPE: fan\nbecause the user wants everyone's take"),
        Some(Shape::Fan)
    );
}

#[test]
fn trailing_punctuation_on_the_token_is_tolerated() {
    assert_eq!(parse_shape("SHAPE: reply."), Some(Shape::Reply));
}

#[test]
fn garbage_parses_to_none_never_a_guess() {
    for bad in [
        "",
        "fan",
        "I would fan out here",
        "SHAPE:",
        "SHAPE: banana",
        "SHAPES: fan",
        "the shape is: fan",
    ] {
        assert_eq!(parse_shape(bad), None, "{bad:?}");
    }
}

// ── the optional second line ───────────────────────────────────────────────

#[test]
fn a_why_line_rides_along_with_the_shape() {
    assert_eq!(
        parse_decision("SHAPE: loop\nWHY: iterate until polished"),
        Some(Decision {
            shape: Shape::Loop,
            why: Some("iterate until polished".into())
        })
    );
}

#[test]
fn a_why_without_a_shape_is_none_never_a_guess() {
    assert_eq!(parse_decision("WHY: iterate until polished"), None);
    assert_eq!(parse_decision("WHY: many takes\nSHAPE: fan"), None);
}

#[test]
fn a_bad_second_line_keeps_the_shape_and_drops_the_why() {
    for reply in [
        "SHAPE: fan",
        "SHAPE: fan\nbecause everyone should weigh in",
        "SHAPE: fan\nWHY:",
        "SHAPE: fan\nWHY: .",
    ] {
        assert_eq!(
            parse_decision(reply),
            Some(Decision {
                shape: Shape::Fan,
                why: None
            }),
            "{reply:?}"
        );
    }
}

#[test]
fn the_why_line_is_as_tolerant_as_the_shape_line() {
    let d = parse_decision("  shape: PLAN.  \n  why:  needs sign-off first.  \nmore prose");
    assert_eq!(
        d,
        Some(Decision {
            shape: Shape::Plan,
            why: Some("needs sign-off first".into())
        })
    );
}

#[test]
fn a_rambling_why_is_cut_to_one_line() {
    let long = "x".repeat(200);
    let d = parse_decision(&format!("SHAPE: reply\nWHY: {long}")).unwrap();
    let why = d.why.unwrap();
    assert!(why.chars().count() < 100, "{}", why.chars().count());
    assert!(why.ends_with('\u{2026}'), "{why}");
}

// ── the injected-classifier seam ───────────────────────────────────────────

#[test]
fn decide_sends_the_task_and_both_grammar_lines_to_the_model() {
    let seen = std::sync::Mutex::new(String::new());
    let call = |p: &str| {
        *seen.lock().unwrap() = p.to_string();
        Ok("SHAPE: plan\nWHY: needs sign-off".to_string())
    };
    assert_eq!(
        decide("refactor the config parser", Some(&call)),
        Routing::Chosen(Decision {
            shape: Shape::Plan,
            why: Some("needs sign-off".into())
        })
    );
    let p = seen.lock().unwrap();
    assert!(p.contains("refactor the config parser"), "{p}");
    assert!(
        p.contains("SHAPE: <reply|fan|loop|plan|goal|swarm|commit|review|standup|resume>"),
        "{p}"
    );
    assert!(p.contains("WHY: <one short clause>"), "{p}");
}

#[test]
fn decide_keeps_a_call_error_apart_from_an_off_grammar_reply() {
    let err = |_: &str| Err("boom".to_string());
    assert_eq!(decide("x", Some(&err)), Routing::Failed("boom".into()));
    let prose = |_: &str| Ok("I'd fan out for this one".to_string());
    assert_eq!(decide("x", Some(&prose)), Routing::OffGrammar);
    assert_eq!(decide("x", None), Routing::Off);
}

#[test]
fn every_stop_dispatches_as_the_swarm() {
    for r in [
        Routing::Off,
        Routing::Failed("boom".into()),
        Routing::OffGrammar,
    ] {
        assert_eq!(r.shape(), Shape::Swarm, "{r:?}");
    }
}
