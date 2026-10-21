use super::{lines, width, Key, LIST, MORE, READER};
use crate::inbox::wire::Thread;

fn said(ls: &[Vec<Key>]) -> Vec<String> {
    ls.iter()
        .map(|l| {
            l.iter()
                .map(|(k, w)| format!("{k} {w}"))
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect()
}

#[test]
fn every_key_shows_wrapped_whole_and_only_a_short_pane_points_at_the_rest() {
    for room in [30, 58, 78, 118] {
        for keys in [LIST, READER] {
            let got = lines(keys, room, 3);
            let shown: Vec<Key> = got.iter().flatten().copied().collect();
            assert!(
                got.iter().all(|l| width(l) <= room),
                "{room}: {:?}",
                said(&got)
            );
            if room >= 58 {
                assert_eq!(shown, keys.to_vec(), "{room}: every key, in order");
            }
        }
    }
    assert_eq!(lines(LIST, 400, 3).len(), 1, "one line when one holds them");
    let short = lines(READER, 40, 1);
    assert_eq!(short.len(), 1);
    assert_eq!(short[0].last(), Some(&MORE), "{:?}", said(&short));
    assert!(width(&short[0]) <= 40);
}

#[test]
fn the_pane_draws_above_the_foot_and_a_note_takes_a_line_of_its_own() {
    let mut p = crate::inboxshot_tests::ready();
    p.reading = Some(crate::inbox::Reading::new(Thread::default()));
    let h = p.foot_h(70, 30);
    assert_eq!(h, lines(READER, 68, 3).len() as u16);
    assert!(h >= 2, "the reader's keys take two lines at 70: {h}");
    p.note = "deleted \u{201c}x\u{201d} \u{b7} u undoes".into();
    assert_eq!(p.foot_h(70, 30), h + 1);
    let cells = p.cells(70, 30);
    let row = |r: u16| -> String {
        let mut line = vec![' '; 70];
        for c in cells.iter().filter(|c| c.row == r) {
            line[usize::from(c.col)] = c.c;
        }
        line.into_iter().collect::<String>().trim().to_string()
    };
    assert!(
        row(30 - h - 1).starts_with("deleted"),
        "{:?}",
        row(30 - h - 1)
    );
    assert!(row(29).len() > 4, "the keys' last line: {:?}", row(29));
    let all: String = (30 - h..30).map(row).collect::<Vec<_>>().join(" ");
    for (k, w) in READER {
        assert!(all.contains(&format!("{k} {w}")), "{k} {w} in {all:?}");
    }
}
