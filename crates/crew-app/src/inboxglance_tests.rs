use crate::inbox::wire::Thread;

use super::Card;

#[test]
fn a_count_names_the_newest_unread_by_sender_and_subject() {
    let t = Thread {
        from: "Ann Lee <ann@x.org>".into(),
        subject: " Q4 review ".into(),
        ..Thread::default()
    };
    assert_eq!(
        super::count(12, Some(&t)),
        Card::Count {
            unread: 12,
            newest: Some(("Ann Lee".into(), "Q4 review".into()))
        }
    );
    assert_eq!(
        super::count(0, None),
        Card::Count {
            unread: 0,
            newest: None
        }
    );
}

#[test]
fn a_fetch_lands_as_a_count_or_says_what_went_wrong() {
    assert_eq!(
        super::landed(Ok((3, None))),
        Card::Count {
            unread: 3,
            newest: None
        }
    );
    assert_eq!(
        super::landed(Err(
            "invalid_grant: Token has been expired or revoked.".into()
        )),
        Card::Problem("signed out \u{2014} /inbox".into())
    );
    assert_eq!(
        super::landed(Err("could not reach Gmail: dns".into())),
        Card::Problem("could not reach Gmail: dns".into())
    );
}

/// Tests have no grant (`grant::path` is `None` under test), so a tick is a
/// signed-out tick: no card, and nothing asked of Gmail.
#[test]
fn signed_out_there_is_no_card() {
    let mut app = crate::app::CrewApp::default();
    app.tick_inbox(1_000_000);
    assert_eq!(super::state(), Card::Off);
    assert_eq!(crate::navinboxcard::block(&super::state()), 0);
}
