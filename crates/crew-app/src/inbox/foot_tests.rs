use super::{lines, said_lines, width, Key, GAP, LIST, MORE, READER};
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
            // As many lines as a pane of 16 rows or more spares.
            let got = lines(keys, room, usize::from(super::LINES));
            let shown: Vec<Key> = got.iter().flatten().copied().collect();
            assert!(
                got.iter().all(|l| width(l) <= room),
                "{room}: {:?}",
                said(&got)
            );
            if room >= 58 {
                let keys: Vec<Key> = keys.iter().copied().filter(|&k| k != GAP).collect();
                let shown: Vec<Key> = shown.into_iter().filter(|&k| k != GAP).collect();
                assert_eq!(shown, keys, "{room}: every key, in order");
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
    assert_eq!(h, lines(READER, 68, usize::from(super::LINES)).len() as u16);
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

#[test]
fn a_long_note_wraps_whole_so_its_end_says_what_to_press() {
    let note = "u again unsubscribes from Dainik Bhaskar \u{b7} one click, nothing is sent from your account";
    let got = said_lines(note, 48);
    assert!(
        got.len() == 2 && got.iter().all(|l| l.chars().count() <= 48),
        "{got:?}"
    );
    assert!(got[0].starts_with("u again"));
    assert_eq!(got.join(" "), note, "every word, none cut");
    assert!(said_lines("", 48).is_empty());
    let flood = said_lines(&"word ".repeat(80), 20);
    assert_eq!(flood.len(), 3);
    assert!(flood[2].ends_with('\u{2026}'), "{flood:?}");
    let mut p = crate::inboxshot_tests::ready();
    p.reading = Some(crate::inbox::Reading::new(Thread::default()));
    let bare = p.foot_h(50, 30);
    p.note = note.into();
    assert_eq!(
        p.foot_h(50, 30),
        bare + 2,
        "the note takes the lines it needs"
    );
}

/// While a search is typed the foot shows the search's keys: the list's
/// stayed, and "e archive" typed an e.
#[test]
fn the_foot_shows_the_search_s_keys_while_it_is_typed() {
    use crate::inbox::worker::Worker;
    use crate::inbox::{InboxPane, Stage};
    let ((tx, _), (_, rx)) = (std::sync::mpsc::channel(), std::sync::mpsc::channel());
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = vec![Thread::default()];
    p.press(&winit::keyboard::Key::Character("/".into()), 80, 24);
    let text: String = p.cells(80, 24).iter().map(|c| c.c).collect();
    assert!(text.contains("cancel") && text.contains("erase"), "{text}");
    assert!(!text.contains("archive"), "{text}");
}

/// An empty list offers what an empty list can use: refresh first.
#[test]
fn an_empty_list_offers_refresh_search_undo_and_gmail_only() {
    use crate::inbox::worker::Worker;
    use crate::inbox::{InboxPane, Stage};
    let ((tx, _), (_, rx)) = (std::sync::mpsc::channel(), std::sync::mpsc::channel());
    let p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    let text: String = p.cells(80, 24).iter().map(|c| c.c).collect();
    assert!(text.contains("refresh") && text.contains("gmail"), "{text}");
    assert!(
        !text.contains("archive") && !text.contains("reply"),
        "{text}"
    );
}
