use crate::chatbody::CardLine;
#[test]
fn a_mails_top_heading_is_ruled_under_and_a_viewed_files_is_not() {
    let fg = (20, 20, 20);
    let md = || crate::md::render_chat("# Big news\n\nwords", 40);
    let ruled = |lines: &[CardLine]| {
        lines.iter().any(|l| {
            l.iter()
                .map(|c| c.c)
                .collect::<String>()
                .contains("\u{2500}\u{2500}\u{2500}")
        })
    };
    assert!(
        ruled(&crate::chatmd::mail_lines(md(), 40, fg).0),
        "a mail's headline"
    );
    assert!(
        !ruled(&crate::chatmd::with_pictures(md(), 40, fg).0),
        "the viewer reads its own way"
    );
}

#[test]
fn a_mails_files_stand_apart_from_its_quote() {
    let mut m = crate::inbox::api::Mail {
        id: "m1".into(),
        from: "Bo <bo@y.org>".into(),
        ..Default::default()
    };
    m.rich = crate::inbox::rich::Rich::of_plain(
        "Works for me.\n\nOn Tue, Ann wrote:\n> Can we move it?",
        None,
    );
    m.attached = vec![crate::inbox::body::Part {
        name: "plan.pdf".into(),
        size: 2048,
        ..Default::default()
    }];
    let r = crate::inbox::Reading {
        mails: Some(vec![m]),
        ..crate::inbox::Reading::new(crate::inbox::wire::Thread::default())
    };
    let laid = r.laid(60);
    let text = |l: &crate::chatbody::CardLine| l.iter().map(|c| c.c).collect::<String>();
    let file = laid
        .lines
        .iter()
        .position(|l| text(l).contains("plan.pdf"))
        .expect("the file's row");
    assert_eq!(
        text(&laid.lines[file - 1]).trim(),
        "",
        "a row of air above it"
    );
    assert!(
        text(&laid.lines[file - 2]).contains("quoted"),
        "under the folded quote"
    );
}
