use super::*;

use crate::broker::recall::block::{block, RECALL_CAP};
use crate::broker::recall::node::Rel;
use crate::broker::recall::query::turns;
use crate::broker::recall::Recall;

const BASE: u64 = 1_700_000_000_000;
const PRICING: usize = 3;
const PRICE_FILE: usize = 7;
const FLAKY: usize = 35;

/// Sixty turns a minute apart, recorded the way a session records them
/// (topics, co-occurrence, the spine). 10..60 are about `test`: each with a
/// word of its own (`case12`), one of two areas, and from 30 on the `cost` of
/// a retry; 35 is also `flaky`. 3 is qwen-flash's pricing, 7 names the price
/// table's file, and the rest tidy UI cards.
fn sixty() -> Recall {
    let ui = [
        "weather", "clock", "font", "theme", "palette", "cursor", "scroll", "border",
    ];
    let mut ui = ui.iter();
    let mut r = Recall::default();
    for i in 0..60usize {
        let (asked, answered) = match i {
            PRICING => (
                "what is the pricing for qwen-flash".to_string(),
                "qwen-flash is 0.05 in and 0.40 out per million tokens",
            ),
            PRICE_FILE => (
                "where is the price table".to_string(),
                "in crates/crew-hive/src/pricing.rs, at the top",
            ),
            0..=9 => (format!("tidy the {} card", ui.next().unwrap()), "done"),
            _ => (
                format!(
                    "the {}{} test for case{i}",
                    if i == FLAKY { "flaky " } else { "" },
                    ["scheduler", "router"][i % 2]
                ),
                if i >= 30 {
                    "fixed, the test cost a retry"
                } else {
                    "fixed"
                },
            ),
        };
        r.record(&asked, answered);
        let mut n = r.g.node(r.prev.unwrap()).unwrap().clone();
        n.last_ms = BASE + i as u64 * 60_000; // the clock above is one millisecond
        r.g.put(n);
    }
    r
}

/// What `text` recalls, as positions in the sixty, strongest first.
fn recalled(r: &Recall, text: &str) -> Vec<usize> {
    turns(&r.g, text, &[], 4)
        .iter()
        .map(|h| ((r.g.node(h.id).unwrap().last_ms - BASE) / 60_000) as usize)
        .collect()
}

#[test]
fn a_common_word_beside_a_rare_one_brings_back_none_of_its_own_turns() {
    let r = sixty();
    let got = recalled(&r, "fix the flaky test");
    assert_eq!(got.first(), Some(&FLAKY), "{got:?}");
    // The turns either side of it on the spine come with it; none of the
    // other forty-seven `test` turns does.
    assert!(
        got.iter().all(|i| i.abs_diff(FLAKY) <= 1),
        "recalled for `test` alone: {got:?}"
    );
}

#[test]
fn a_rare_word_outranks_a_common_one_the_old_turn_never_used() {
    let r = sixty();
    let got = recalled(&r, "what does qwen-flash cost");
    assert_eq!(got.first(), Some(&PRICING), "{got:?}");
    assert!(
        got.iter().all(|i| i.abs_diff(PRICING) <= 1),
        "recalled for `cost` alone: {got:?}"
    );
}

#[test]
fn a_named_file_recalls_the_turn_that_named_it() {
    let r = sixty();
    let got = recalled(&r, "is the test for crates/crew-hive/src/pricing.rs slow");
    assert_eq!(got.first(), Some(&PRICE_FILE), "{got:?}");
    assert!(got.iter().all(|i| i.abs_diff(PRICE_FILE) <= 1), "{got:?}");
}

#[test]
fn a_request_of_only_common_words_recalls_nothing_and_carries_no_block() {
    let r = sixty();
    for text in [
        "rerun the test please",
        "is it fixed yet",
        "the scheduler again",
    ] {
        assert_eq!(recalled(&r, text), Vec::<usize>::new(), "{text:?}");
        let b = block(&r.g, text, &[], BASE, RECALL_CAP).map(|b| b.text);
        assert_eq!(b, None, "{text:?} still put a block in front of the task");
    }
}

/// Turns `0..d` for each `(word, d)`, so the words' turns overlap from 0.
fn mentions(words: &[(&str, usize)]) -> Graph {
    let mut g = Graph::default();
    for (w, d) in words {
        let t = g.touch(Kind::Topic, w, w, 0);
        for i in 0..*d {
            let turn = g.touch(Kind::Turn, &i.to_string(), &format!("turn {i}"), i as u64);
            g.link(turn, t, Rel::Mentions);
        }
    }
    g
}

#[test]
fn one_word_recalls_on_its_own_up_to_sixteen_turns_and_not_past_them() {
    for (d, want) in [(1, true), (4, true), (16, true), (17, false), (50, false)] {
        let g = mentions(&[("alpha", d)]);
        let got = turns(&g, "alpha", &[], 4).len();
        assert_eq!(got > 0, want, "a word in {d} turns recalled {got}");
    }
}

#[test]
fn two_words_each_in_sixty_four_turns_recall_the_turns_that_have_both() {
    for (d, want) in [(64, true), (65, false)] {
        let g = mentions(&[("alpha", d), ("beta", d)]);
        let got = turns(&g, "alpha beta", &[], 4).len();
        assert_eq!(got > 0, want, "two words in {d} turns each recalled {got}");
    }
}

#[test]
fn one_word_counts_once_however_many_paths_reach_a_turn() {
    // `alpha` is in 20 turns (0.10 each, under the floor); three of them also
    // say `beta`, which came up with `alpha` three times. Uncapped, those
    // three heard `alpha` again through `beta` (+0.08) and cleared the floor
    // on one common word.
    let mut g = mentions(&[("alpha", 20), ("beta", 3)]);
    let a = g.find(Kind::Topic, "alpha").unwrap();
    let b = g.find(Kind::Topic, "beta").unwrap();
    for _ in 0..3 {
        g.link(a, b, Rel::With);
    }
    let got: Vec<f32> = turns(&g, "alpha", &[], 4).iter().map(|h| h.score).collect();
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn the_floor_is_the_worked_number_its_comment_gives() {
    assert_eq!(rarity(SPECIFIC), 0.25);
    assert!((floor() - 0.1125).abs() < 1e-6, "{}", floor());
    assert!((trace() - 0.050_625).abs() < 1e-6, "{}", trace());
    // The second hop the comment promises: rare word, then a word in two.
    let hop = DECAY * DECAY * rarity(2);
    assert!(hop > floor(), "{hop}");
}
