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
