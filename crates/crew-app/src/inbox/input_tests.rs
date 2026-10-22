use crate::inbox::compose::On;
use crate::inbox::composefiles::What;
use crate::inbox::composekeys::tests::{on, reading, typed};
use crate::inbox::InboxPane;

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

#[test]
fn a_reply_box_keeps_the_end_of_the_mail_it_answers_in_view() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = reading();
    if let Some(m) = p.reading.as_mut().and_then(|r| r.mails.as_mut()) {
        m[0].text = (1..=40).map(|n| format!("line {n}\n\n")).collect();
    }
    p.scroll_wheel(-200, 80, 30);
    let shown = |p: &InboxPane| {
        let mail = p.cards(80, 30).mail.unwrap();
        p.cells(80, 30)
            .into_iter()
            .filter(|c| c.row > mail.row && c.row < mail.row + mail.h)
            .map(|c| c.c)
            .collect::<String>()
    };
    assert!(
        shown(&p).contains("line 40"),
        "read to the end: {}",
        shown(&p)
    );
    typed(&mut p, "r");
    assert!(
        shown(&p).contains("line 40"),
        "the box opened, the end still shows"
    );
    for _ in 0..6 {
        p.press(
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter),
            80,
            30,
        );
    }
    assert!(
        shown(&p).contains("line 40"),
        "the box grew, the end still shows"
    );
    p.compose = None;
    p.scroll_wheel(200, 80, 30);
    typed(&mut p, "r");
    assert_eq!(
        p.reading.as_ref().unwrap().scroll,
        0,
        "read from the top, it stays there"
    );
}

#[test]
fn folding_a_long_mail_never_leaves_it_scrolled_past_its_end() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = reading();
    let quote: String = (1..=60).map(|n| format!("> quoted line {n}\n")).collect();
    if let Some(m) = p.reading.as_mut().and_then(|r| r.mails.as_mut()) {
        let text = format!("Short reply.\n\nOn Tue, Ann wrote:\n{quote}");
        m[0].rich = crate::inbox::rich::Rich::of_plain(&text, None);
    }
    typed(&mut p, "q");
    p.scroll_wheel(-500, 80, 30);
    assert!(
        p.reading.as_ref().unwrap().scroll > 20,
        "far down the quote"
    );
    typed(&mut p, "q");
    let mail = p.cards(80, 30).mail.unwrap();
    let shown: String = p
        .cells(80, 30)
        .into_iter()
        .filter(|c| c.row > mail.row && c.row + 1 < mail.row + mail.h)
        .map(|c| c.c)
        .collect();
    assert!(
        shown.contains("Short reply."),
        "the folded mail shows: {shown}"
    );
}

#[test]
fn a_sent_reply_shows_when_its_thread_opens_again() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = reading();
    typed(&mut p, "r");
    typed(&mut p, "Yes.");
    p.cells(80, 30);
    p.send_reply();
    p.take(crate::inbox::worker::Got::Sent("ann@x.org".into()));
    let mut mails = p.reading.as_ref().unwrap().mails.clone().unwrap();
    mails[0].text = "word\n\n".repeat(40);
    let mine = crate::inbox::api::Mail {
        from: "me <me@y.org>".into(),
        text: "Yes, Friday works for me.".into(),
        ..Default::default()
    };
    mails.push(mine);
    p.take(crate::inbox::worker::Got::Opened("t1".into(), mails));
    let shown: String = p.cells(80, 30).into_iter().map(|c| c.c).collect();
    assert!(
        shown.contains("Yes, Friday works for me."),
        "the reply on screen: scroll {} {shown}",
        p.reading.as_ref().unwrap().scroll
    );
    assert!(p.reading.as_ref().unwrap().scroll > 0, "scrolled to it");
}

#[test]
fn a_thread_opened_again_draws_its_new_mails() {
    let (mut p, _) = reading();
    p.cells(80, 30);
    let mut mails = p.reading.as_ref().unwrap().mails.clone().unwrap();
    mails[0].text = "A different word now.".into();
    p.take(crate::inbox::worker::Got::Opened("t1".into(), mails));
    let shown: String = p.cells(80, 30).into_iter().map(|c| c.c).collect();
    assert!(
        shown.contains("A different word now."),
        "not the old layout: {shown}"
    );
}
