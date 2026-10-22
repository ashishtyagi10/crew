use super::{Ask, Got, State};
use crate::inbox::api::Gmail;
use crate::inbox::forwardmail::tests::serve;
use crate::inbox::grant::{Client, Grant};

#[test]
fn a_filing_that_went_brings_the_unread_count_with_it() {
    let (base, log) = serve();
    let grant = Grant {
        client: Client {
            id: "c".into(),
            secret: "s".into(),
        },
        refresh_token: "1//r".into(),
        email: String::new(),
    };
    let mut g = Gmail::at(grant, &format!("{base}/token"), &format!("{base}/gmail")).unwrap();
    let got = State::default().answer(
        &mut g,
        Ask::Relabel {
            id: "t1".into(),
            add: vec![],
            remove: vec!["UNREAD"],
        },
    );
    assert!(
        got.iter().any(|g| matches!(g, Got::Unread(_))),
        "the pane's count, at once"
    );
    let asked: Vec<String> = log
        .lock()
        .unwrap()
        .iter()
        .map(|r| r.lines().next().unwrap_or("").to_string())
        .collect();
    assert!(
        asked
            .iter()
            .any(|l| l.starts_with("GET /gmail/labels/INBOX")),
        "{asked:#?}"
    );
}
