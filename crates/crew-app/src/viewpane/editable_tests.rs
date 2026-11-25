//! Which rungs take a caret.
use super::*;

#[test]
fn every_text_rung_takes_a_caret_and_no_other() {
    for f in [
        Format::Markdown,
        Format::Code { lang: "rust" },
        Format::Data { lang: "json" },
        Format::Text,
    ] {
        assert!(typed_in(f, false) && typed_in(f, true), "{f:?}");
    }
    assert!(
        !typed_in(Format::Csv { delim: ',' }, false),
        "a drawn table"
    );
    assert!(typed_in(Format::Csv { delim: ',' }, true), "its text");
    assert!(!typed_in(Format::Diff, false));
    assert!(!typed_in(Format::Image { kind: "PNG" }, false));
}
