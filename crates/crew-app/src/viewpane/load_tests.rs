use super::*;
use std::path::Path;

#[test]
fn textutil_argv_writes_to_stdout() {
    // -stdout is what keeps this a pipe read rather than a temp file we
    // would then have to clean up.
    assert_eq!(
        argv(Extractor::TextUtil, Path::new("/tmp/a b.docx")),
        vec!["-convert", "txt", "-stdout", "/tmp/a b.docx"]
    );
}

#[test]
fn pdftotext_argv_keeps_layout_and_writes_to_stdout() {
    // The trailing "-" is pdftotext's stdout sentinel; -layout preserves
    // columns, which is most of what makes an extract readable.
    assert_eq!(
        argv(Extractor::PdfToText, Path::new("/tmp/p.pdf")),
        vec!["-layout", "/tmp/p.pdf", "-"]
    );
}

#[test]
fn a_file_under_the_cap_is_not_truncated() {
    let dir = tempdir();
    let p = dir.join("small.txt");
    std::fs::write(&p, "hello\n").unwrap();
    let done = super::load_now(
        &p,
        Probe {
            textutil: false,
            pdftotext: false,
        },
    );
    let loaded = done.result.expect("small file loads");
    assert_eq!(loaded.text, "hello\n");
    assert_eq!(loaded.truncated, None, "nothing to announce");
}

#[test]
fn an_oversize_file_is_truncated_and_says_so() {
    // The cap applies to what is SHOWN, never to what may be opened — the
    // same call made for @file line ranges. A refusal here would make the
    // pane useless for exactly the files it is most wanted for.
    let dir = tempdir();
    let p = dir.join("big.log");
    let big = vec![b'x'; (MAX_VIEW_BYTES + 4096) as usize];
    std::fs::write(&p, &big).unwrap();
    let done = super::load_now(
        &p,
        Probe {
            textutil: false,
            pdftotext: false,
        },
    );
    let loaded = done.result.expect("an oversize file still loads");
    assert_eq!(loaded.text.len(), MAX_VIEW_BYTES as usize);
    assert_eq!(loaded.truncated, Some(MAX_VIEW_BYTES + 4096));
}

#[test]
fn a_missing_file_reports_why_and_names_itself() {
    let done = super::load_now(
        Path::new("/nonexistent/nope.txt"),
        Probe {
            textutil: false,
            pdftotext: false,
        },
    );
    assert_eq!(
        done.format,
        Format::Opaque {
            why: Opaque::Unreadable
        },
        "a read failure is Unreadable, not a specific (and false) claim like Binary"
    );
    let err = done.result.expect_err("a missing file fails");
    assert!(
        err.contains("nope.txt"),
        "the message names the file: {err}"
    );
}

#[test]
fn an_opaque_file_loads_no_text() {
    let dir = tempdir();
    let p = dir.join("blob.bin");
    std::fs::write(&p, [0u8, 1, 2, 3]).unwrap();
    let done = super::load_now(
        &p,
        Probe {
            textutil: false,
            pdftotext: false,
        },
    );
    assert!(matches!(done.format, Format::Opaque { .. }));
    assert_eq!(done.result.expect("opaque still succeeds").text, "");
}

#[test]
fn cap_text_leaves_short_text_alone() {
    let (text, truncated) = cap_text("hello\n".to_string());
    assert_eq!(text, "hello\n");
    assert_eq!(truncated, None);
}

#[test]
fn cap_text_caps_long_text_and_reports_the_original_length() {
    // An extractor's output has no fixed ratio to the file's byte size, so
    // this is what stands between a 300-page PDF and an unbounded `String`
    // in `Loaded.text` — the same guarantee `read_capped` gives a plain file.
    let long = "x".repeat((MAX_VIEW_BYTES + 4096) as usize);
    let (text, truncated) = cap_text(long);
    assert_eq!(text.len(), MAX_VIEW_BYTES as usize);
    assert_eq!(truncated, Some(MAX_VIEW_BYTES + 4096));
}

#[test]
fn cap_text_backs_off_a_split_multibyte_char_instead_of_panicking() {
    // A 3-byte UTF-8 char ('€') straddling the byte cap would make a raw
    // `String::truncate(MAX_VIEW_BYTES)` panic; `cap_text` must walk back to
    // the char boundary before it, not just avoid crashing on ASCII inputs.
    let mut long = "x".repeat(MAX_VIEW_BYTES as usize - 1);
    long.push('€'); // its middle byte lands exactly on the cap
    long.push_str(&"x".repeat(4096));
    let original_len = long.len() as u64;
    let (text, truncated) = cap_text(long);
    assert!(
        text.len() < MAX_VIEW_BYTES as usize,
        "backed off the split char"
    );
    assert!(text.is_char_boundary(text.len()));
    assert_eq!(truncated, Some(original_len));
}

#[test]
fn start_delivers_over_the_channel() {
    let dir = tempdir();
    let p = dir.join("chan.txt");
    std::fs::write(&p, "over the wire\n").unwrap();
    let rx = start(p);
    let done = rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
    assert_eq!(done.result.unwrap().text, "over the wire\n");
}

/// A unique temp dir for one test, created eagerly. No `tempfile` dep.
fn tempdir() -> std::path::PathBuf {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("crew-viewload-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}
