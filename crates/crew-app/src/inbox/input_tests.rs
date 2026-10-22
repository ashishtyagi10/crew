use crate::inbox::compose::On;
use crate::inbox::composefiles::What;
use crate::inbox::composekeys::tests::{on, reading, typed};

#[test]
fn the_wheel_scrolls_the_mail_while_a_reply_keeps_its_caret() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = reading();
    if let Some(m) = p.reading.as_mut().and_then(|r| r.mails.as_mut()) {
        m[0].text = "line\n\n".repeat(60);
    }
    typed(&mut p, "r");
    typed(&mut p, "Yes");
    p.scroll_wheel(-3, 80, 30);
    assert_eq!(p.reading.as_ref().unwrap().scroll, 3, "the mail moved");
    assert_eq!(on(&p), On::Body, "the caret stayed in the words");
    assert_eq!(p.compose.as_ref().unwrap().body.text, "Yes");
    p.scroll_wheel(10, 80, 30);
    assert_eq!(
        p.reading.as_ref().unwrap().scroll,
        0,
        "and back, no further"
    );
}

#[test]
fn the_pointer_finds_the_box_files_and_attach_and_nothing_else() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = reading();
    typed(&mut p, "r");
    let b = p.cards(80, 30).reply.expect("the box");
    let files = b.row + b.h - 2;
    assert_eq!(p.compose_spot_at(80, 30, files, b.col + 4), Some(What::Add));
    assert_eq!(
        p.compose_spot_at(80, 30, files, b.col + b.w - 3),
        None,
        "past its words"
    );
    assert_eq!(
        p.compose_spot_at(80, 30, b.row + 1, b.col + 4),
        None,
        "the to line"
    );
    assert_eq!(p.compose_spot_at(80, 30, 2, 10), None, "the mail");
}

#[test]
fn the_hidden_pictures_note_has_air_under_it() {
    let (mut p, _) = reading();
    let html = "<p>Hi there</p><img src='https://x.io/a.png' height='40'>";
    if let Some(m) = p.reading.as_mut().and_then(|r| r.mails.as_mut()) {
        m[0].rich = crate::inbox::rich::Rich::of_html(html, None);
    }
    let r = p.reading.as_mut().unwrap();
    r.web = false;
    let laid = r.laid(60);
    let text = |l: &crate::chatbody::CardLine| l.iter().map(|c| c.c).collect::<String>();
    let note = laid
        .lines
        .iter()
        .position(|l| text(l).contains("from the web hidden"))
        .expect("the note");
    assert_eq!(text(&laid.lines[note + 1]).trim(), "", "a row of air");
    assert!(text(&laid.lines[note + 2]).contains("Hi there"));
}
