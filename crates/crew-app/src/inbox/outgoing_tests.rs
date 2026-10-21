use super::{send, Outgoing};
use crate::inbox::api::Gmail;
use crate::inbox::attach::Attached;
use crate::inbox::forwardmail::tests::serve;
use crate::inbox::grant::{Client, Grant};

#[test]
fn a_reply_with_a_file_goes_up_in_its_thread_the_file_read_from_disk() {
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
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("menu.txt");
    std::fs::write(&path, "soup, bread").unwrap();
    let mut o = Outgoing {
        head: vec!["To: ann@x.org".into(), "Subject: Re: Lunch".into()],
        thread_id: Some("t1".into()),
        words: "Here it is.".into(),
        forward: None,
        files: vec![Attached {
            path: path.clone(),
            name: "menu.txt".into(),
            size: 11,
        }],
        to: "ann@x.org".into(),
    };
    send(&mut g, &o).expect("sent");
    let log = log.lock().unwrap().clone();
    let up = log
        .iter()
        .find(|r| r.starts_with("POST /upload/gmail/messages/send?uploadType=multipart "))
        .expect("a multipart upload: the thread rides beside the mail");
    assert!(up.contains("{\"threadId\":\"t1\"}"), "{up}");
    assert!(
        up.contains("Content-Type: message/rfc822\r\n\r\nTo: ann@x.org\r\n"),
        "{up}"
    );
    assert!(up.contains("filename=\"menu.txt\""), "{up}");
    // "soup, bread", base64.
    assert!(up.contains("c291cCwgYnJlYWQ="), "{up}");
    o.files[0].path = dir.path().join("gone.txt");
    let e = send(&mut g, &o).unwrap_err();
    assert!(e.starts_with("could not read menu.txt"), "{e}");
}
