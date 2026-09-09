use crate::md::{render_chat, LineKind, MdLine};

fn flat(line: &MdLine) -> String {
    line.spans.iter().map(|s| s.text.as_str()).collect()
}

/// On main `ENABLE_FOOTNOTES` was off: `[^1]` rendered as the literal
/// characters `[^1]` and the definition line as prose `[^1]: the note`.
#[test]
fn a_reference_is_a_muted_mark_and_the_definition_trails_under_a_rule() {
    let lines = render_chat("text[^1] here\n\n[^1]: the note", 20);
    let rows: Vec<String> = lines.iter().map(flat).collect();
    let rule = "─".repeat(20);
    assert_eq!(
        rows,
        vec!["text[1] here", "", &rule, "1. the note"],
        "{rows:?}"
    );
    let mark = &lines[0].spans[1];
    assert!(mark.style.footnote && mark.text == "[1]", "{mark:?}");
    assert_eq!(lines[2].kind, LineKind::Rule);
    // `1. ` is a marker glyph, like a list's ordinal.
    assert!(lines[3].spans[0].style.marker);
    assert!(!lines[3].spans[1].style.footnote);
}

/// A definition written ABOVE the prose still lays out last.
#[test]
fn definitions_gather_at_the_end_wherever_the_source_put_them() {
    let lines = render_chat("[^a]: first\n\nbody[^a]\n\nmore", 30);
    let rows: Vec<String> = lines.iter().map(flat).collect();
    assert_eq!(rows[0], "body[a]");
    assert_eq!(rows[2], "more");
    assert_eq!(rows.last().map(String::as_str), Some("a. first"));
    assert_eq!(rows.iter().filter(|r| r.starts_with('─')).count(), 1);
}

/// A long definition wraps under a hanging indent the width of its label.
#[test]
fn a_long_definition_wraps_under_its_label() {
    let lines = render_chat("x[^1]\n\n[^1]: one two three four", 14);
    let rows: Vec<String> = lines.iter().map(flat).collect();
    assert_eq!(rows[3], "1. one two");
    assert_eq!(rows[4], "   three four");
}

#[test]
fn an_undefined_reference_renders_and_nothing_panics() {
    let rows: Vec<String> = render_chat("see[^nope] and [^]", 30)
        .iter()
        .map(flat)
        .collect();
    assert!(rows[0].contains("nope"), "{rows:?}");
    assert!(rows.iter().all(|r| !r.starts_with('─')), "{rows:?}");
}

/// A document that is ONLY definitions has nothing above the rule to
/// separate from, so it does not open with a blank row.
#[test]
fn notes_alone_do_not_lead_with_a_blank() {
    let lines = render_chat("[^1]: alone", 10);
    assert_eq!(lines[0].kind, LineKind::Rule);
    assert_eq!(flat(&lines[1]), "1. alone");
}
