use super::*;
use std::path::Path;

fn all() -> Probe {
    Probe {
        textutil: true,
        pdftotext: true,
    }
}

#[test]
fn extension_picks_the_rung() {
    assert!(matches!(
        detect(Path::new("a/b/main.rs"), b"fn main() {}", all()),
        Format::Code { lang: "rust" }
    ));
    assert!(matches!(
        detect(Path::new("README.md"), b"# hi", all()),
        Format::Markdown
    ));
    assert!(matches!(
        detect(Path::new("c.json"), b"{}", all()),
        Format::Data { lang: "json" }
    ));
    assert!(matches!(
        detect(Path::new("t.csv"), b"a,b", all()),
        Format::Csv { delim: ',' }
    ));
    assert!(matches!(
        detect(Path::new("t.tsv"), b"a\tb", all()),
        Format::Csv { delim: '\t' }
    ));
}

#[test]
fn a_nul_byte_outranks_the_extension() {
    // A .md file that is really a compiled object must not be handed to the
    // markdown engine — the binary verdict is the one sniff that beats the
    // name. (It used to be written with JPEG bytes, which now have a rung of
    // their own: an image is drawn, not refused.)
    let obj = [0xCF, 0xFA, 0xED, 0xFE, 0x00, 0x10];
    assert!(matches!(
        detect(Path::new("notes.md"), &obj, all()),
        Format::Opaque {
            why: Opaque::Binary
        }
    ));
}

/// A picture is a picture whatever it has been called, and it is recognised
/// before the binary sniff that would otherwise refuse it — every image format
/// is "binary" and every one of them is going to be shown.
#[test]
fn a_picture_is_read_from_its_bytes_not_its_name() {
    for (head, kind) in [
        (&b"\x89PNG\r\n\x1a\n\x00\x00"[..], "PNG"),
        (&[0xFF, 0xD8, 0xFF, 0x00, 0x10][..], "JPEG"),
        (&b"GIF89a\x00\x01"[..], "GIF"),
        (&b"RIFF\x00\x00\x00\x00WEBPVP8 "[..], "WebP"),
    ] {
        assert_eq!(
            detect(Path::new("notes.md"), head, all()),
            Format::Image { kind },
            "{kind} named .md"
        );
    }
    // …and text that merely starts with two letters is not a bitmap.
    assert!(!matches!(
        detect(Path::new("a.txt"), b"BM is a fine abbreviation", all()),
        Format::Image { .. }
    ));
}

#[test]
fn odd_text_in_a_markdown_file_is_still_markdown() {
    // Only the *binary* verdict outranks the extension. Weird prose does not.
    assert!(matches!(
        detect(Path::new("notes.md"), b"@@@ ---- ***", all()),
        Format::Markdown
    ));
}

#[test]
fn extensionless_files_are_sniffed() {
    assert!(matches!(
        detect(Path::new("run"), b"#!/bin/sh\necho hi\n", all()),
        Format::Code { lang: "shell" }
    ));
    assert!(matches!(
        detect(Path::new("changes"), b"diff --git a/x b/x\n", all()),
        Format::Diff
    ));
    assert!(matches!(
        detect(Path::new("hunk"), b"@@ -1,2 +1,3 @@\n", all()),
        Format::Diff
    ));
}

#[test]
fn invalid_utf8_without_a_nul_is_still_opaque() {
    // Latin-1 text has no NUL but cannot be rendered as UTF-8.
    assert!(matches!(
        detect(Path::new("notes.txt"), &[0xC3, 0x28, 0xA9], all()),
        Format::Opaque {
            why: Opaque::NotUtf8
        }
    ));
}

#[test]
fn a_truncated_utf8_char_at_the_sniff_boundary_is_not_opaque() {
    // The head is a prefix of the file, so a multi-byte char can be cut in
    // half by SNIFF_BYTES. That must not condemn the whole file.
    let mut head = vec![b'a'; SNIFF_BYTES - 1];
    head.push(0xE2); // first byte of a 3-byte char, rest beyond the window
    assert!(!matches!(
        detect(Path::new("notes.txt"), &head, all()),
        Format::Opaque { .. }
    ));
}

#[test]
fn extract_rungs_pick_their_tool() {
    assert!(matches!(
        detect(Path::new("memo.docx"), b"PK\x03\x04", all()),
        Format::Extract {
            via: Extractor::TextUtil
        }
    ));
    assert!(matches!(
        detect(Path::new("paper.pdf"), b"%PDF-1.7", all()),
        Format::Extract {
            via: Extractor::PdfToText
        }
    ));
}

#[test]
fn a_missing_tool_degrades_the_rung_it_does_not_error() {
    let none = Probe {
        textutil: false,
        pdftotext: false,
    };
    assert!(matches!(
        detect(Path::new("paper.pdf"), b"%PDF-1.7", none),
        Format::Opaque {
            why: Opaque::NoExtractor(Extractor::PdfToText)
        }
    ));
    assert!(matches!(
        detect(Path::new("memo.docx"), b"PK\x03\x04", none),
        Format::Opaque {
            why: Opaque::NoExtractor(Extractor::TextUtil)
        }
    ));
}

#[test]
fn an_unknown_extension_holding_text_reads_as_plain_code() {
    assert!(matches!(
        detect(Path::new("notes.wat"), b"hello there\n", all()),
        Format::Code { lang: "" }
    ));
}

#[test]
fn an_empty_file_is_not_opaque() {
    // Fix 6: the old assertion only ruled out `Opaque`, which passes for ANY
    // other rung, correct or not (e.g. it would still pass if `by_content`
    // started misclassifying an empty file as `Diff`). `.txt` carries no
    // extension mapping, so this falls to `by_content`, whose fallback for
    // unrecognised content is `Code { lang: "" }` — assert that exact rung.
    assert_eq!(
        detect(Path::new("empty.txt"), b"", all()),
        Format::Code { lang: "" }
    );
}
