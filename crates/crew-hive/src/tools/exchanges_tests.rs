use super::*;
use crate::tools::Tools;

/// `n` numbered lines of 50 chars each, then a last line naming `tag`.
fn body(tag: &str, n: usize) -> String {
    let mut lines: Vec<String> = (0..n)
        .map(|i| format!("{tag} line {i:03}: {}", "x".repeat(50 - tag.len() - 11)))
        .collect();
    lines.push(format!("END OF {tag}"));
    lines.join("\n")
}

fn log(results: &[&str]) -> Exchanges {
    let mut log = Exchanges::default();
    for (i, r) in results.iter().enumerate() {
        log.push(Exchange::new(
            &format!("reading {i}"),
            "fs:read",
            "{}",
            r.to_string(),
        ));
    }
    log
}

#[test]
fn two_exchanges_are_shown_exactly_as_they_always_were() {
    let (a, b) = (body("a", 100), body("b", 100));
    assert_eq!(
        log(&[&a, &b]).render(),
        format!(
            "YOUR MESSAGE:\nreading 0\nCALLED fs:read {{}}\nRESULT:\n{a}\n\n\
             YOUR MESSAGE:\nreading 1\nCALLED fs:read {{}}\nRESULT:\n{b}"
        )
    );
}

#[test]
fn a_third_shortens_the_first_to_its_head_cut_on_a_line_end() {
    let (a, b, c) = (body("a", 100), body("b", 100), body("c", 100));
    let shown = log(&[&a, &b, &c]).render();
    // Seven 51-char lines (newline included) fit in 400; the eighth would not.
    let head: Vec<&str> = a.lines().take(7).collect();
    let total = a.chars().count();
    assert_eq!(total, 5_108);
    let first = format!(
        "YOUR MESSAGE:\nreading 0\nCALLED fs:read {{}}\nRESULT:\n{}\n\
         \u{2026} (result shortened \u{2014} 5,108 chars; the call can be made again to see it all)",
        head.join("\n")
    );
    assert!(shown.starts_with(&format!("{first}\n\n")), "{shown}");
    assert!(!shown.contains("END OF a"), "the old tail is gone");
    assert!(
        shown.contains(&b) && shown.contains(&c),
        "the last two are whole"
    );
}

#[test]
fn a_short_result_stays_whole_however_old() {
    let (b, c) = (body("b", 100), body("c", 100));
    let shown = log(&["no matches", &b, &c]).render();
    assert!(shown.contains("RESULT:\nno matches\n\n"), "{shown}");
    assert!(!shown.contains("shortened"), "{shown}");
}

/// A surface where every call only looks.
struct Reads;

impl Tools for Reads {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        unreachable!("nothing runs here")
    }
    fn repeatable(&self, _s: &str, _t: &str) -> bool {
        true
    }
}

fn read(path: &str) -> ToolCall {
    ToolCall {
        server: "fs".into(),
        tool: "read".into(),
        args: format!("{{\"path\":\"{path}\"}}"),
    }
}

#[test]
fn a_repeat_points_back_only_while_its_result_is_shown_whole() {
    let long = body("a", 100);
    let mut seen = Seen::default();
    for (round, path) in [(1, "a"), (2, "b"), (3, "c")] {
        seen.ran(&Reads, &read(path), round, true);
    }
    let three = log(&[&long, "short b", &long]);
    // The next prompt holds four exchanges: round 3 is one of the last two.
    assert_eq!(three.repeat(&seen, &read("c")), Some(3));
    // Round 2 is older but short, so it is still there whole.
    assert_eq!(three.repeat(&seen, &read("b")), Some(2));
    // Round 1 is shortened: the pointer would be false, so it must run.
    assert_eq!(three.repeat(&seen, &read("a")), None);
    assert_eq!(seen.check(&read("a")), Some(1), "Seen alone would point");
}

#[test]
fn counts_are_grouped_in_threes() {
    assert_eq!(grouped(5), "5");
    assert_eq!(grouped(812), "812");
    assert_eq!(grouped(5_812), "5,812");
    assert_eq!(grouped(1_234_567), "1,234,567");
}

/// Three reads made by one reply are one round: all three reach the prompt
/// that answers them whole, and they age together.
#[test]
fn the_calls_of_one_round_are_whole_together_and_shortened_together() {
    let (a, b, c, d) = (
        body("a", 100),
        body("b", 100),
        body("c", 100),
        body("d", 100),
    );
    let mut three = Exchanges::default();
    three.push_round(
        [&a, &b, &c]
            .iter()
            .map(|r| Exchange::new("", "fs:read", "{}", r.to_string()))
            .collect(),
    );
    assert!(!three.render().contains("shortened"), "{}", three.render());
    assert_eq!(three.next_entry(), 4);
    three.push(Exchange::new("", "fs:read", "{}", d.clone()));
    assert!(
        !three.render().contains("shortened"),
        "two rounds are whole"
    );
    three.push(Exchange::new("", "fs:read", "{}", d));
    assert_eq!(three.render().matches("shortened").count(), 3);
}

/// `Seen` records entries; the pointer names the round the entry was made
/// in, and an entry not pushed yet is in the round being built.
#[test]
fn a_repeat_of_one_call_in_a_round_points_at_that_round() {
    let long = body("a", 100);
    let mut seen = Seen::default();
    for (entry, path) in [(1, "a"), (2, "b"), (3, "c"), (4, "d")] {
        seen.ran(&Reads, &read(path), entry, true);
    }
    let mut log = log(&[&long]);
    log.push_round(vec![
        Exchange::new("", "fs:read", "{}", long.clone()),
        Exchange::new("", "fs:read", "{}", long.clone()),
    ]);
    assert_eq!(log.repeat(&seen, &read("c")), Some(2), "entry 3 is round 2");
    assert_eq!(
        log.repeat(&seen, &read("a")),
        None,
        "round 1 is shortened next"
    );
    assert_eq!(
        log.repeat(&seen, &read("d")),
        Some(3),
        "entry 4 is being built"
    );
}
