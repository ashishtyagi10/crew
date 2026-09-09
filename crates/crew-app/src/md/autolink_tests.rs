use super::*;

fn span(text: &str) -> MdSpan {
    MdSpan {
        text: text.into(),
        style: MdStyle::default(),
        link: None,
        src: None,
    }
}

/// `(text, link)` per span after autolinking one plain span.
fn linked(text: &str) -> Vec<(String, Option<String>)> {
    autolink(vec![span(text)])
        .into_iter()
        .map(|s| (s.text, s.link))
        .collect()
}

/// On main a `www.` host was prose: this asserted `link == None`.
#[test]
fn a_bare_www_host_links_with_https_in_front() {
    let out = linked("see www.example.com/x, then");
    assert_eq!(out[0], ("see ".into(), None));
    assert_eq!(
        out[1],
        (
            "www.example.com/x".into(),
            Some("https://www.example.com/x".into())
        )
    );
    assert_eq!(out[2], (", then".into(), None));
}

/// On main an address was prose: this asserted `link == None`.
#[test]
fn an_email_address_links_as_mailto() {
    let out = linked("mail name.last+tag@host.co.uk.");
    assert_eq!(
        out[1],
        (
            "name.last+tag@host.co.uk".into(),
            Some("mailto:name.last+tag@host.co.uk".into())
        )
    );
    assert_eq!(out[2], (".".into(), None));
}

/// A host without a dotted, alphabetic last label is not an address: an
/// `@handle` with a slash-word after it stays prose, as does `a@b`.
#[test]
fn handles_and_undotted_hosts_stay_prose() {
    for text in ["ping @alice/review", "a@b", "x@1.23", "awww.not.a.host"] {
        assert_eq!(linked(text), vec![(text.into(), None)], "{text}");
    }
}

/// The scheme is carried on the LINK only; the text stays as typed, and an
/// explicit `https://` URL is still as written on both.
#[test]
fn text_is_as_typed_and_schemes_pass_through() {
    let out = linked("https://a.io/p) www.b.io");
    assert_eq!(
        out[0],
        ("https://a.io/p".into(), Some("https://a.io/p".into()))
    );
    assert_eq!(out[2], ("www.b.io".into(), Some("https://www.b.io".into())));
}

/// Through the chat path a `www.` word is one link run that, like an
/// `https://` one always has, hard-cuts at the column rather than
/// overflowing it — and every cut piece still carries the whole link.
#[test]
fn www_host_hard_cuts_at_the_column_and_every_piece_carries_its_link() {
    let lines = crate::md::render_chat("go www.example.com/a/b now", 16);
    let flat: Vec<String> = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.text.as_str()).collect())
        .collect();
    assert_eq!(flat, vec!["go", "www.example.com/", "a/b now"], "{flat:?}");
    assert!(flat.iter().all(|r| r.chars().count() <= 16));
    let pieces: Vec<&str> = lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .filter_map(|s| s.link.as_deref())
        .collect();
    assert_eq!(pieces, vec!["https://www.example.com/a/b"; 2]);
}
