use crate::inboxglance::Card;

fn rows(card: &Card) -> Vec<String> {
    let cells = super::cells(card, 30);
    let n = cells.iter().map(|c| c.row).max().map_or(0, |r| r + 1);
    (0..n)
        .map(|r| {
            let mut line = vec![' '; 30];
            for c in cells.iter().filter(|c| c.row == r) {
                line[c.col as usize] = c.c;
            }
            line.into_iter().collect::<String>().trim_end().to_string()
        })
        .collect()
}

#[test]
fn the_card_says_the_count_and_the_newest_unread() {
    let _g = crate::app::theme_test_guard();
    let card = Card::Count {
        unread: 12,
        newest: Some(("Ann Lee".into(), "Q4 roadmap review".into())),
    };
    assert_eq!(super::block(&card), 4);
    let r = rows(&card);
    assert!(r[0].contains("INBOX"), "{r:?}");
    assert!(r[1].contains("\u{25cf} 12 unread threads"), "{r:?}");
    assert!(r[2].contains("Ann Lee \u{b7} Q4 roadmap"), "{r:?}");
    assert!(
        super::cells(&card, 30).iter().all(|c| c.col < 29),
        "nothing in the margin column"
    );
}

#[test]
fn quiet_states_take_one_row_and_signed_out_takes_none() {
    let _g = crate::app::theme_test_guard();
    for (card, said) in [
        (
            Card::Count {
                unread: 0,
                newest: None,
            },
            "nothing unread",
        ),
        (
            Card::Count {
                unread: 1,
                newest: None,
            },
            "1 unread thread",
        ),
        (Card::Checking, "checking"),
        (
            Card::Problem("signed out \u{2014} /inbox".into()),
            "signed out",
        ),
    ] {
        assert_eq!(super::block(&card), 3, "{card:?}");
        assert!(rows(&card)[1].contains(said), "{card:?}: {:?}", rows(&card));
    }
    let stale = Card::Count {
        unread: 0,
        newest: Some(("Ann".into(), "old".into())),
    };
    assert_eq!(super::block(&stale), 3, "nothing unread names no newest");
    assert_eq!(super::block(&Card::Off), 0);
    assert!(super::cells(&Card::Off, 30).is_empty());
}
