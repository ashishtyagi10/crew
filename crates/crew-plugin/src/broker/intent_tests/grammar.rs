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

#[test]
fn classify_sends_the_task_and_the_grammar_to_the_model() {
    let seen = std::sync::Mutex::new(String::new());
    let call = |p: &str| {
        *seen.lock().unwrap() = p.to_string();
        Ok("SHAPE: plan".to_string())
    };
    assert_eq!(
        classify_with("refactor the config parser", &call),
        Some(Shape::Plan)
    );
    let p = seen.lock().unwrap();
    assert!(p.contains("refactor the config parser"), "{p}");
    assert!(
        p.contains("SHAPE: <reply|fan|loop|plan|swarm|commit|review|standup|resume>"),
        "{p}"
    );
}

#[test]
fn classify_call_error_is_none() {
    let call = |_: &str| Err("boom".to_string());
    assert_eq!(classify_with("x", &call), None);
}

#[test]
fn classify_off_grammar_reply_is_none() {
    let call = |_: &str| Ok("I'd fan out for this one".to_string());
    assert_eq!(classify_with("x", &call), None);
}
