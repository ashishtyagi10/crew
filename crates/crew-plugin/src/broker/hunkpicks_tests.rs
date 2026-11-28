use super::*;

fn keep(v: &[u32]) -> Picks {
    Picks::Keep(v.to_vec())
}

#[test]
fn numbers_with_spaces_or_commas_keep_those_hunks() {
    assert_eq!(parse_picks("1 3", 3), keep(&[1, 3]));
    assert_eq!(parse_picks("1,3", 3), keep(&[1, 3]));
    assert_eq!(parse_picks(" 3 ,1, 1 ", 3), keep(&[1, 3]));
}

#[test]
fn a_minus_keeps_all_but_those() {
    assert_eq!(parse_picks("-2", 3), keep(&[1, 3]));
    assert_eq!(parse_picks("-1 -3", 3), keep(&[2]));
    assert_eq!(parse_picks("-1,3", 3), keep(&[2]));
}

#[test]
fn words_are_not_picks() {
    for s in ["", "ship it", "1 and 3", "1 -2", "1.5", "-", "- 2x"] {
        assert_eq!(parse_picks(s, 3), Picks::NotPicks, "{s:?}");
    }
    assert_eq!(parse_picks("1", 0), Picks::NotPicks);
}

#[test]
fn out_of_range_says_which_and_how_many() {
    let Picks::Bad(why) = parse_picks("1 7", 3) else {
        panic!()
    };
    assert!(why.contains("no hunk 7") && why.contains("has 3"), "{why}");
    assert!(matches!(parse_picks("0", 3), Picks::Bad(_)));
    assert!(matches!(parse_picks("-9", 3), Picks::Bad(_)));
}

#[test]
fn excluding_everything_is_refused_with_a_pointer_to_esc() {
    let Picks::Bad(why) = parse_picks("-1 -2", 2) else {
        panic!()
    };
    assert!(why.contains("Esc"), "{why}");
}

#[test]
fn the_hunk_count_is_read_from_the_first_line() {
    assert_eq!(
        declared("edit a.rs \u{2014} 3 changes \u{b7} ok\n[1] @@ 1 @@"),
        3
    );
    assert_eq!(declared("edit a.rs \u{2014} 1 change \u{b7} ok"), 1);
    assert_eq!(
        declared("write a.rs \u{2014} new file, 4 lines \u{b7} ok"),
        0
    );
    assert_eq!(declared("run `ls` \u{2014} it cannot be undone"), 0);
    assert_eq!(declared("edit a.rs"), 0);
}
