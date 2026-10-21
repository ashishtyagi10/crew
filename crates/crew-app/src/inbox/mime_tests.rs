use base64::Engine;

use super::{message, Node};

fn b64(lines: &str) -> Vec<u8> {
    let b: String = lines.split_whitespace().collect();
    base64::engine::general_purpose::STANDARD.decode(b).unwrap()
}

#[test]
fn a_file_rides_with_the_words_each_part_between_its_boundaries() {
    let tree = Node::multi(
        "mixed",
        vec![
            Node::text("plain", "hi\nthere"),
            Node::file("application/pdf", "menu \"v2\".pdf", vec![0, 1, 2, 255]),
        ],
    );
    let m = message(
        &["To: a@x.org".into(), "Subject: x\r\nBcc: e@x.org".into()],
        &tree,
    );
    assert!(
        m.starts_with("To: a@x.org\r\nSubject: x  Bcc: e@x.org\r\nMIME-Version: 1.0\r\n"),
        "{m}"
    );
    let b = m
        .split("boundary=\"")
        .nth(1)
        .and_then(|r| r.split('"').next())
        .expect("a boundary");
    let parts: Vec<&str> = m.split(&format!("--{b}")).collect();
    assert_eq!(parts.len(), 4, "preamble, two parts, the closing --");
    assert!(parts[3].starts_with("--"));
    let (h1, body1) = parts[1].trim_start().split_once("\r\n\r\n").unwrap();
    assert!(h1.contains("text/plain; charset=UTF-8"));
    assert_eq!(b64(body1), b"hi\r\nthere");
    let (h2, body2) = parts[2].trim_start().split_once("\r\n\r\n").unwrap();
    assert!(h2.contains("application/pdf; name=\"menu v2.pdf\""), "{h2}");
    assert!(h2.contains("Content-Disposition: attachment; filename=\"menu v2.pdf\""));
    assert_eq!(b64(body2), [0, 1, 2, 255]);
}

#[test]
fn every_line_fits_and_a_multipart_of_one_is_the_part() {
    let one = Node::multi("mixed", vec![Node::text("html", &"<p>x</p>".repeat(2000))]);
    assert!(matches!(one, Node::Leaf { .. }));
    let m = message(&[], &one);
    assert!(
        m.split("\r\n")
            .all(|l| l.len() <= 76 || l.starts_with("Content-")),
        "{m}"
    );
}

#[test]
fn a_picture_carries_its_cid_and_a_strange_type_is_octets() {
    let p = Node::inline("image/png", "<logo@x>", "", vec![1]);
    let m = message(&[], &p);
    assert!(
        m.contains("Content-ID: <logo@x>\r\nContent-Disposition: inline\r\n"),
        "{m}"
    );
    let f = Node::file("evil\r\nX: y", "été.txt", vec![1]);
    let m = message(&[], &f);
    assert!(
        m.contains("Content-Type: application/octet-stream; name=\"=?UTF-8?B?"),
        "{m}"
    );
}
