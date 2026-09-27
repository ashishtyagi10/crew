use super::*;

fn rows(lines: &[&str], cols: usize) -> Vec<Vec<char>> {
    lines
        .iter()
        .map(|l| {
            let mut v: Vec<char> = l.chars().collect();
            v.resize(cols, ' ');
            v
        })
        .collect()
}

#[test]
fn a_path_the_pane_wrapped_is_one_target_at_its_first_cell() {
    let r = rows(&["see crates/crew-app/src/md/", "fold.rs:25 for it"], 27);
    let got = found(&r, &[true, false]);
    let path = got.iter().find(|t| t.3 == Kind::Path).expect("a path");
    assert_eq!(path.2, "crates/crew-app/src/md/fold.rs:25");
    assert_eq!((path.0, path.1), (0, 4), "labelled where it starts");
}

#[test]
fn without_the_wrap_flag_the_rows_stay_apart() {
    let r = rows(&["see crates/crew-app/src/md/", "fold.rs:25 for it"], 27);
    let got = found(&r, &[false, false]);
    assert!(
        got.iter()
            .all(|t| t.2 != "crates/crew-app/src/md/fold.rs:25"),
        "{got:?}"
    );
}

#[test]
fn a_target_starting_on_a_continuation_row_is_placed_on_it() {
    let r = rows(&["aaaaaaaaaaaaaa", "aa src/main.rs"], 14);
    let got = found(&r, &[true, false]);
    let path = got.iter().find(|t| t.3 == Kind::Path).expect("a path");
    assert_eq!((path.0, path.1, path.2.as_str()), (1, 3, "src/main.rs"));
}
