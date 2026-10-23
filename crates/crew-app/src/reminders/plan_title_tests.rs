use super::tests::{f, open};
use super::{plan, Step};

/// A title alone is not the same thing to do: a new "pay rent" on the phone
/// was ticked off against last month's done one (and a crew todo ticked done
/// by an old reminder of the same name), and neither showed anywhere.
#[test]
fn a_title_pairs_only_with_one_as_done_as_it_is() {
    let todos = [(1, f("pay rent", true, None)), (2, open("call bo"))];
    let rems = [
        ("r1".to_string(), open("Pay rent")),
        ("r2".into(), f("call bo", true, None)),
    ];
    let p = plan(&todos, &rems, &[]);
    assert!(p.pairs.is_empty(), "{:?}", p.pairs);
    assert_eq!(
        p.steps,
        [
            Step::NewRem {
                todo: 2,
                f: open("call bo")
            },
            Step::NewTodo {
                rem: "r1".into(),
                f: open("Pay rent")
            },
        ],
        "each new one shows on the other side; nothing is ticked"
    );
}

/// As done as each other, a title still pairs, so turning the sync on twice
/// makes nothing twice.
#[test]
fn a_done_pair_by_title_is_still_one_pair() {
    let todos = [(1, f("filed taxes", true, None))];
    let rems = [("r1".to_string(), f("Filed taxes", true, None))];
    let p = plan(&todos, &rems, &[]);
    assert_eq!(p.pairs.len(), 1);
    assert!(
        p.steps.iter().all(|s| matches!(s, Step::SetRem { .. })),
        "{:?}",
        p.steps
    );
}
