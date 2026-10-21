use std::sync::mpsc;

use super::super::api::Mail;
use super::super::body::Part;
use super::super::rich::Rich;
use super::super::wire::Thread;
use super::super::worker::{Ask, Got, Worker};
use super::super::{InboxPane, Reading, Stage};
use crate::viewpane::bitmap::Bitmap;

fn pane() -> (InboxPane, mpsc::Receiver<Ask>) {
    let (tx, asks) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.reading = Some(Reading::new(Thread {
        id: "t1".into(),
        ..Thread::default()
    }));
    (p, asks)
}

fn mail(html: &str, parts: Vec<Part>) -> Mail {
    Mail {
        id: "m1".into(),
        rich: Rich::of_html(html, None),
        parts,
        ..Mail::default()
    }
}

fn text(p: &InboxPane, cols: u16, rows: u16) -> String {
    let mut grid = vec![vec![' '; cols as usize]; rows as usize];
    for c in p.cells(cols, rows) {
        grid[c.row as usize][c.col as usize] = c.c;
    }
    grid.iter()
        .map(|r| r.iter().collect::<String>() + "\n")
        .collect()
}

#[test]
fn a_thread_asks_for_the_pictures_it_carries_and_only_those() {
    let (mut p, asks) = pane();
    let parts = vec![
        Part {
            cid: Some("chart@x".into()),
            att: Some("A1".into()),
            mime: "image/png".into(),
            ..Part::default()
        },
        Part {
            name: "photo.jpg".into(),
            att: Some("A2".into()),
            mime: "image/jpeg".into(),
            ..Part::default()
        },
        Part {
            cid: Some("unused@x".into()),
            att: Some("A3".into()),
            mime: "image/png".into(),
            ..Part::default()
        },
    ];
    let m = mail("<p>See</p><img src='cid:chart@x' alt='chart'>", parts);
    p.take(Got::Opened("t1".into(), vec![m]));
    let keys: Vec<String> = asks
        .try_iter()
        .filter_map(|a| match a {
            Ask::Picture { key, .. } => Some(key),
            _ => None,
        })
        .collect();
    assert_eq!(keys, ["m1|cid:chart@x", "m1|att:A2"]);
    assert!(!p.busy, "a picture is not a load the list waits on");
}

#[test]
fn a_decoded_picture_is_painted_in_its_box_and_one_still_coming_says_so() {
    let (mut p, _asks) = pane();
    let m = mail(
        "<p>See</p><img src='cid:a@x' alt='A' height='60'><img src='cid:b@x' alt='B'>",
        vec![],
    );
    p.reading.as_mut().unwrap().mails = Some(vec![m]);
    let bm = Bitmap {
        w: 4,
        h: 2,
        px: vec![[200, 0, 0, 255]; 8],
        src: (4, 2),
    };
    p.pics.set("m1|cid:a@x".into(), Some(bm));
    let (_, paint) = p.art(80, 60, 2.0);
    assert!(!paint.is_empty(), "the decoded picture is drawn");
    assert!(
        paint.iter().all(|q| q.y + q.h <= 59.0),
        "never over the foot row"
    );
    assert!(
        text(&p, 80, 60).contains("loading picture"),
        "{}",
        text(&p, 80, 60)
    );
}

#[test]
fn the_webs_pictures_come_with_the_mail_i_hides_them_and_a_failed_one_says_what_it_was() {
    let (mut p, _asks) = pane();
    let html = "<p>Hi</p><a href='https://x.io/r'><img src='http://127.0.0.1:9/b.png' alt='Read the report' height='60'></a>";
    p.reading.as_mut().unwrap().mails = Some(vec![mail(html, vec![])]);
    assert!(text(&p, 80, 30).contains("1 picture from the web hidden"));
    assert_eq!(p.fetch_web(), 1, "fetched as the thread opens");
    assert!(p.reading.as_ref().unwrap().web);
    assert_eq!(
        p.show_web(),
        "pictures from the web hidden \u{b7} i shows them"
    );
    let hint = text(&p, 80, 30);
    let (at, body) = (hint.find("hidden").unwrap(), hint.find("Hi").unwrap());
    assert!(
        at < body,
        "the hint is under the sender, over the words: {hint}"
    );
    assert_eq!(
        p.show_web(),
        "showing 1 picture from the web \u{b7} i hides it"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !p.pics.drain() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let shown = text(&p, 80, 30);
    assert!(shown.contains("\u{25a3} Read the report"), "{shown}");
    assert!(
        shown.contains("\u{2197} Read the report"),
        "the link survives: {shown}"
    );
}
